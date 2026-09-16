use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use sensiblaw_route_selector::{
    encode_route_candidate, ProducerFamily, RouteCandidate, RouteFamily, RouteSelectorError,
};
use std::io::{BufRead, Read, Write};
use thiserror::Error;

const ENTITY_PREFIX: &str = "http://www.wikidata.org/entity/";
const ENWIKI_ROOT: &str = "https://en.wikipedia.org/";
const MAX_RDF_BYTES: usize = 64 << 20;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("route candidate error: {0}")]
    Route(#[from] RouteSelectorError),
    #[error("rdf/xml error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("network error: {0}")]
    Network(Box<ureq::Error>),
    #[error("invalid provider input: {0}")]
    InvalidInput(String),
}

impl From<ureq::Error> for ProviderError {
    fn from(error: ureq::Error) -> Self { Self::Network(Box::new(error)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderReceipt {
    pub direct_property_candidates: u64,
    pub wikipedia_article_candidates: u64,
    pub producer_search_candidates: u64,
    pub parser_repair_candidates: u64,
    pub rdf_xml: bool,
    pub json_transport: bool,
    pub regex_parser: bool,
    pub route_candidate_is_claim_truth: bool,
    pub semantic_promotion: bool,
}

fn valid_qid(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('Q') else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn entity_data_rdf_url(qid: &str) -> Result<String, ProviderError> {
    if !valid_qid(qid) {
        return Err(ProviderError::InvalidInput(format!("invalid QID {qid}")));
    }
    Ok(format!(
        "https://www.wikidata.org/wiki/Special:EntityData/{qid}.rdf"
    ))
}

fn attribute_value(start: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, ProviderError> {
    for attribute in start.attributes().with_checks(false) {
        let attribute =
            attribute.map_err(|error| ProviderError::InvalidInput(error.to_string()))?;
        if attribute.key.as_ref() == key {
            return Ok(Some(
                String::from_utf8(attribute.value.into_owned())
                    .map_err(|_| ProviderError::InvalidInput("attribute is not UTF-8".into()))?,
            ));
        }
    }
    Ok(None)
}

fn qid_from_entity_uri(uri: &str) -> Option<String> {
    let qid = uri.strip_prefix(ENTITY_PREFIX)?;
    valid_qid(qid).then(|| qid.to_owned())
}

fn property_specificity(property: &str) -> Option<u32> {
    match property {
        "P31" | "P279" => Some(5),
        "P361" | "P527" => Some(4),
        "P131" | "P17" | "P1269" => Some(3),
        _ => None,
    }
}

fn property_producer(property: &str) -> Option<ProducerFamily> {
    match property {
        "P31" | "P279" => Some(ProducerFamily::ClassificationEvidence),
        "P361" | "P527" | "P131" | "P17" | "P1269" => Some(ProducerFamily::IdentitySource),
        _ => None,
    }
}

fn direct_property_candidate(
    root_qid: &str,
    property: &str,
    target_qid: &str,
) -> Option<RouteCandidate> {
    Some(RouteCandidate {
        candidate_id: format!("wikidata:{root_qid}:{property}:{target_qid}"),
        producer: property_producer(property)?,
        route_family: RouteFamily::WikidataProperty,
        source_ref: root_qid.to_owned(),
        target_ref: target_qid.to_owned(),
        property_ref: property.to_owned(),
        cross_language_gap_coverage: 0,
        source_surface_support: 0,
        root_qid_support: 1,
        typed_property_support: 1,
        route_specificity: property_specificity(property)?,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    })
}

fn article_candidate(root_qid: &str, article_url: &str) -> RouteCandidate {
    RouteCandidate {
        candidate_id: format!("wikipedia-article:{root_qid}:{article_url}"),
        producer: ProducerFamily::ArticleSemantic,
        route_family: RouteFamily::WikipediaArticle,
        source_ref: root_qid.to_owned(),
        target_ref: article_url.to_owned(),
        property_ref: String::new(),
        cross_language_gap_coverage: 0,
        source_surface_support: 1,
        root_qid_support: 1,
        typed_property_support: 0,
        route_specificity: 4,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    }
}

fn search_candidate(
    root_qid: &str,
    producer: ProducerFamily,
    route_family: RouteFamily,
    label: &str,
) -> RouteCandidate {
    RouteCandidate {
        candidate_id: format!("search:{root_qid}:{label}"),
        producer,
        route_family,
        source_ref: root_qid.to_owned(),
        target_ref: format!("search:{label}:{root_qid}"),
        property_ref: String::new(),
        cross_language_gap_coverage: 0,
        source_surface_support: 0,
        root_qid_support: 1,
        typed_property_support: 0,
        route_specificity: 1,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    }
}

fn parser_repair_candidate(root_qid: &str) -> RouteCandidate {
    search_candidate(
        root_qid,
        ProducerFamily::ParserRepair,
        RouteFamily::ParserRepair,
        "parser-repair",
    )
}

fn emit_search_candidates<W: Write>(root_qid: &str, writer: &mut W) -> Result<u64, ProviderError> {
    let rows = [
        search_candidate(
            root_qid,
            ProducerFamily::IdentitySource,
            RouteFamily::PrimarySourceSearch,
            "identity-source",
        ),
        search_candidate(
            root_qid,
            ProducerFamily::AuthoritySource,
            RouteFamily::PrimarySourceSearch,
            "authority-source",
        ),
        search_candidate(
            root_qid,
            ProducerFamily::MechanismEvidence,
            RouteFamily::PrimarySourceSearch,
            "mechanism-evidence",
        ),
        search_candidate(
            root_qid,
            ProducerFamily::MeasurementEvidence,
            RouteFamily::MeasurementSourceSearch,
            "measurement-evidence",
        ),
        search_candidate(
            root_qid,
            ProducerFamily::ComparatorEvidence,
            RouteFamily::ComparatorSourceSearch,
            "comparator-evidence",
        ),
    ];
    for row in &rows {
        encode_route_candidate(writer, row)?;
    }
    Ok(rows.len() as u64)
}

fn process_xml<R: BufRead, W: Write>(
    root_qid: &str,
    reader: R,
    writer: &mut W,
) -> Result<(u64, u64), ProviderError> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut current_entity: Option<String> = None;
    let mut current_article_url: Option<String> = None;
    let mut current_article_is_article = false;
    let mut article_about_root = false;
    let mut article_is_enwiki = false;
    let mut direct_count = 0u64;
    let mut article_count = 0u64;

    loop {
        match xml.read_event_into(&mut buffer)? {
            Event::Start(start) => match start.name().as_ref() {
                b"rdf:Description" => {
                    let about = attribute_value(&start, b"rdf:about")?;
                    current_entity = about.as_deref().and_then(qid_from_entity_uri);
                    current_article_url = about;
                    current_article_is_article = false;
                    article_about_root = false;
                    article_is_enwiki = false;
                }
                b"schema:Article" => {
                    current_article_url = attribute_value(&start, b"rdf:about")?;
                    current_article_is_article = true;
                    article_about_root = false;
                    article_is_enwiki = false;
                }
                _ => {}
            },
            Event::Empty(empty) => {
                let name = empty.name();
                let raw = name.as_ref();
                if let Some(property) = std::str::from_utf8(raw)
                    .ok()
                    .and_then(|value| value.strip_prefix("wdt:"))
                {
                    if current_entity.as_deref() == Some(root_qid)
                        && property_specificity(property).is_some()
                    {
                        if let Some(target) = attribute_value(&empty, b"rdf:resource")?
                            .and_then(|value| qid_from_entity_uri(&value))
                        {
                            if let Some(row) =
                                direct_property_candidate(root_qid, property, &target)
                            {
                                encode_route_candidate(writer, &row)?;
                                direct_count += 1;
                            }
                        }
                    }
                } else if raw == b"rdf:type" && current_article_url.is_some() {
                    current_article_is_article = attribute_value(&empty, b"rdf:resource")?
                        .as_deref()
                        == Some("http://schema.org/Article");
                } else if raw == b"schema:about" && current_article_url.is_some() {
                    article_about_root = attribute_value(&empty, b"rdf:resource")?
                        .and_then(|value| qid_from_entity_uri(&value))
                        .as_deref()
                        == Some(root_qid);
                } else if raw == b"schema:isPartOf" && current_article_url.is_some() {
                    article_is_enwiki =
                        attribute_value(&empty, b"rdf:resource")?.as_deref() == Some(ENWIKI_ROOT);
                }
            }
            Event::End(end) => match end.name().as_ref() {
                b"rdf:Description" => {
                    if current_article_is_article && article_about_root && article_is_enwiki {
                        if let Some(url) = current_article_url.as_deref() {
                            encode_route_candidate(writer, &article_candidate(root_qid, url))?;
                            article_count += 1;
                        }
                    }
                    current_entity = None;
                    current_article_url = None;
                    current_article_is_article = false;
                    article_about_root = false;
                    article_is_enwiki = false;
                }
                b"schema:Article" => {
                    if article_about_root && article_is_enwiki {
                        if let Some(url) = current_article_url.as_deref() {
                            encode_route_candidate(writer, &article_candidate(root_qid, url))?;
                            article_count += 1;
                        }
                    }
                    current_article_url = None;
                    current_article_is_article = false;
                    article_about_root = false;
                    article_is_enwiki = false;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok((direct_count, article_count))
}

pub fn emit_candidates_from_rdf<R: Read, W: Write>(
    root_qid: &str,
    mut rdf: R,
    writer: &mut W,
) -> Result<ProviderReceipt, ProviderError> {
    if !valid_qid(root_qid) {
        return Err(ProviderError::InvalidInput(format!(
            "invalid QID {root_qid}"
        )));
    }
    let mut bytes = Vec::new();
    rdf.by_ref()
        .take((MAX_RDF_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RDF_BYTES {
        return Err(ProviderError::InvalidInput(
            "RDF entity document exceeds size limit".into(),
        ));
    }
    let (direct_property_candidates, wikipedia_article_candidates) =
        process_xml(root_qid, std::io::Cursor::new(bytes), writer)?;
    let producer_search_candidates = emit_search_candidates(root_qid, writer)?;
    encode_route_candidate(writer, &parser_repair_candidate(root_qid))?;
    Ok(ProviderReceipt {
        direct_property_candidates,
        wikipedia_article_candidates,
        producer_search_candidates,
        parser_repair_candidates: 1,
        rdf_xml: true,
        json_transport: false,
        regex_parser: false,
        route_candidate_is_claim_truth: false,
        semantic_promotion: false,
    })
}

pub fn fetch_entity_rdf(qid: &str) -> Result<Vec<u8>, ProviderError> {
    let url = entity_data_rdf_url(qid)?;
    let response = ureq::get(&url)
        .set("Accept", "application/rdf+xml")
        .set(
            "User-Agent",
            "SensibLaw-SLR/0.1 (typed RDF route candidate provider)",
        )
        .call()?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take((MAX_RDF_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RDF_BYTES {
        return Err(ProviderError::InvalidInput(
            "RDF entity document exceeds size limit".into(),
        ));
    }
    Ok(bytes)
}

pub fn fetch_and_emit<W: Write>(
    qid: &str,
    writer: &mut W,
) -> Result<ProviderReceipt, ProviderError> {
    let bytes = fetch_entity_rdf(qid)?;
    emit_candidates_from_rdf(qid, std::io::Cursor::new(bytes), writer)
}
