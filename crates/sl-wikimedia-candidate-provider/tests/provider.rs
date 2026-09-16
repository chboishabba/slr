use sensiblaw_route_selector::{decode_route_candidate, ProducerFamily, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, entity_data_rdf_revision_url, entity_data_rdf_url, ProviderReceipt,
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

const MABO_RDF: &str = r#"<?xml version="1.0"?>
<rdf:RDF
 xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
 xmlns:wdt="http://www.wikidata.org/prop/direct/"
 xmlns:schema="http://schema.org/">
 <rdf:Description rdf:about="http://www.wikidata.org/entity/Q1501525">
   <wdt:P31 rdf:resource="http://www.wikidata.org/entity/Q2334719"/>
   <wdt:P17 rdf:resource="http://www.wikidata.org/entity/Q408"/>
   <wdt:P1001 rdf:resource="http://www.wikidata.org/entity/Q408"/>
   <wdt:P710 rdf:resource="http://www.wikidata.org/entity/Q203893"/>
   <wdt:P710 rdf:resource="http://www.wikidata.org/entity/Q36074"/>
   <wdt:P4884 rdf:resource="http://www.wikidata.org/entity/Q184011"/>
   <wdt:P1594 rdf:resource="http://www.wikidata.org/entity/Q310196"/>
   <wdt:P4006 rdf:resource="http://www.wikidata.org/entity/Q6852878"/>
 </rdf:Description>
 <rdf:Description rdf:about="https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)">
   <rdf:type rdf:resource="http://schema.org/Article"/>
   <schema:about rdf:resource="http://www.wikidata.org/entity/Q1501525"/>
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

fn mabo_candidates() -> Vec<sensiblaw_route_selector::RouteCandidate> {
    let mut out = Vec::new();
    emit_candidates_from_rdf("Q1501525", MABO_RDF.as_bytes(), &mut out).unwrap();
    let mut rows = Vec::new();
    let mut cursor = Cursor::new(out);
    while let Some(row) = decode_route_candidate(&mut cursor).unwrap() {
        rows.push(row);
    }
    rows
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
fn revision_pinned_entity_url_retains_exact_oldid_coordinate() {
    let url = entity_data_rdf_revision_url("Q1501525", 2_333_409_615).unwrap();
    assert_eq!(
        url,
        "https://www.wikidata.org/wiki/Special:EntityData/Q1501525.rdf?revision=2333409615"
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
fn mabo_legal_properties_emit_candidate_only_typed_edges() {
    let rows = mabo_candidates();
    for property in ["P1001", "P710", "P4884", "P1594", "P4006"] {
        let matches: Vec<_> = rows
            .iter()
            .filter(|row| row.property_ref == property)
            .collect();
        assert!(!matches.is_empty(), "missing typed Mabo property {property}");
        for row in matches {
            assert_eq!(row.route_family, RouteFamily::WikidataProperty);
            assert_eq!(row.source_ref, "Q1501525");
            assert_eq!(row.typed_property_support, 1);
        }
    }

    let overrules = rows
        .iter()
        .find(|row| row.property_ref == "P4006")
        .expect("overrules candidate");
    assert_eq!(overrules.producer, ProducerFamily::AuthoritySource);
    assert!(overrules.route_specificity >= 5);

    for property in ["P1001", "P710", "P4884", "P1594"] {
        let row = rows
            .iter()
            .find(|row| row.property_ref == property)
            .expect("identity/context candidate");
        assert_eq!(row.producer, ProducerFamily::IdentitySource);
    }
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
