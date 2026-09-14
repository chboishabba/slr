use sensiblaw_route_selector::{decode_route_candidate, ProducerFamily, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, entity_data_rdf_url, ProviderReceipt,
};
use std::io::Cursor;

const RDF: &str = r#"<?xml version="1.0"?>
<rdf:RDF
 xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
 xmlns:wd="http://www.wikidata.org/entity/"
 xmlns:wdt="http://www.wikidata.org/prop/direct/"
 xmlns:schema="http://schema.org/">
 <rdf:Description rdf:about="http://www.wikidata.org/entity/Q207">
   <wdt:P31 rdf:resource="http://www.wikidata.org/entity/Q5"/>
   <wdt:P279 rdf:resource="http://www.wikidata.org/entity/Q215627"/>
   <wdt:P361 rdf:resource="http://www.wikidata.org/entity/Q30"/>
   <wdt:P17 rdf:resource="http://www.wikidata.org/entity/Q30"/>
 </rdf:Description>
 <rdf:Description rdf:about="https://en.wikipedia.org/wiki/George_W._Bush">
   <rdf:type rdf:resource="http://schema.org/Article"/>
   <schema:about rdf:resource="http://www.wikidata.org/entity/Q207"/>
   <schema:isPartOf rdf:resource="https://en.wikipedia.org/"/>
 </rdf:Description>
</rdf:RDF>"#;

fn candidates() -> (
    ProviderReceipt,
    Vec<sensiblaw_route_selector::RouteCandidate>,
) {
    let mut out = Vec::new();
    let receipt = emit_candidates_from_rdf("Q207", RDF.as_bytes(), &mut out).unwrap();
    let mut rows = Vec::new();
    let mut cursor = Cursor::new(out);
    while let Some(row) = decode_route_candidate(&mut cursor).unwrap() {
        rows.push(row);
    }
    (receipt, rows)
}

#[test]
fn entity_url_uses_rdf_not_json() {
    let url = entity_data_rdf_url("Q207").unwrap();
    assert_eq!(
        url,
        "https://www.wikidata.org/wiki/Special:EntityData/Q207.rdf"
    );
    assert!(!url.contains("json"));
}

#[test]
fn direct_properties_emit_typed_wikidata_candidates() {
    let (receipt, rows) = candidates();
    assert!(receipt.rdf_xml);
    assert!(!receipt.json_transport);
    assert!(!receipt.regex_parser);
    assert!(!receipt.route_candidate_is_claim_truth);

    let p31 = rows
        .iter()
        .find(|row| row.property_ref == "P31")
        .expect("P31 candidate");
    assert_eq!(p31.route_family, RouteFamily::WikidataProperty);
    assert_eq!(p31.source_ref, "Q207");
    assert_eq!(p31.target_ref, "Q5");
    assert_eq!(p31.producer, ProducerFamily::ClassificationEvidence);
    assert_eq!(p31.typed_property_support, 1);
    assert_eq!(p31.route_specificity, 5);

    let p279 = rows
        .iter()
        .find(|row| row.property_ref == "P279")
        .expect("P279 candidate");
    assert_eq!(p279.target_ref, "Q215627");
    assert_eq!(p279.producer, ProducerFamily::ClassificationEvidence);
}

#[test]
fn wikipedia_sitelink_emits_article_semantic_candidate() {
    let (_, rows) = candidates();
    let article = rows
        .iter()
        .find(|row| row.route_family == RouteFamily::WikipediaArticle)
        .expect("Wikipedia article candidate");
    assert_eq!(article.producer, ProducerFamily::ArticleSemantic);
    assert_eq!(article.source_ref, "Q207");
    assert_eq!(
        article.target_ref,
        "https://en.wikipedia.org/wiki/George_W._Bush"
    );
    assert!(article.source_surface_support >= 1);
}

#[test]
fn verified_qid_anchors_non_wikidata_producer_search_candidates_without_claiming_evidence() {
    let (_, rows) = candidates();
    for producer in [
        ProducerFamily::IdentitySource,
        ProducerFamily::AuthoritySource,
        ProducerFamily::MechanismEvidence,
        ProducerFamily::MeasurementEvidence,
        ProducerFamily::ComparatorEvidence,
    ] {
        let row = rows
            .iter()
            .find(|row| {
                row.producer == producer
                    && matches!(
                        row.route_family,
                        RouteFamily::PrimarySourceSearch
                            | RouteFamily::MeasurementSourceSearch
                            | RouteFamily::ComparatorSourceSearch
                    )
            })
            .unwrap_or_else(|| panic!("missing search-family producer {producer:?}"));
        assert_eq!(row.source_ref, "Q207");
        assert_eq!(row.typed_property_support, 0);
    }
}

#[test]
fn provider_dependency_surface_has_no_json_or_regex() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
