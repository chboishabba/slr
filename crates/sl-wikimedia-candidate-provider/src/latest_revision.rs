use std::io::Read;

use serde_json::Value;

use super::{fetch_entity_rdf_revision_receipt, valid_qid, AcquiredEntityRdf, ProviderError};

const MAX_REVISION_LOOKUP_BYTES: usize = 1 << 20;

/// Parse an exact MediaWiki latest-revision coordinate for one Wikidata QID.
///
/// This returns only the revision id. The caller must subsequently acquire the
/// exact RDF revision before using any content as a durable source manifestation.
pub fn parse_latest_revision_id(qid: &str, json: &[u8]) -> Result<u64, ProviderError> {
    if !valid_qid(qid) {
        return Err(ProviderError::InvalidInput(format!("invalid QID {qid}")));
    }
    if json.len() > MAX_REVISION_LOOKUP_BYTES {
        return Err(ProviderError::InvalidInput(
            "latest-revision response exceeds size limit".into(),
        ));
    }

    let value: Value = serde_json::from_slice(json).map_err(|error| {
        ProviderError::InvalidInput(format!("invalid latest-revision JSON: {error}"))
    })?;
    let pages = value
        .get("query")
        .and_then(|query| query.get("pages"))
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::InvalidInput("latest-revision response has no pages".into()))?;

    let page = pages
        .iter()
        .find(|page| page.get("title").and_then(Value::as_str) == Some(qid))
        .ok_or_else(|| {
            ProviderError::InvalidInput(format!(
                "latest-revision response does not contain requested QID {qid}"
            ))
        })?;
    let revision_id = page
        .get("revisions")
        .and_then(Value::as_array)
        .and_then(|revisions| revisions.first())
        .and_then(|revision| revision.get("revid"))
        .and_then(Value::as_u64)
        .filter(|revision_id| *revision_id > 0)
        .ok_or_else(|| {
            ProviderError::InvalidInput(format!(
                "latest-revision response has no positive revision for {qid}"
            ))
        })?;

    Ok(revision_id)
}

pub fn latest_revision_api_url(qid: &str) -> Result<String, ProviderError> {
    if !valid_qid(qid) {
        return Err(ProviderError::InvalidInput(format!("invalid QID {qid}")));
    }
    Ok(format!(
        "https://www.wikidata.org/w/api.php?action=query&format=json&formatversion=2&prop=revisions&rvprop=ids&titles={qid}"
    ))
}

pub fn fetch_latest_revision_id(qid: &str) -> Result<u64, ProviderError> {
    let url = latest_revision_api_url(qid)?;
    let response = ureq::get(&url)
        .set("Accept", "application/json")
        .set(
            "User-Agent",
            "SensibLaw-SLR/0.1 (typed latest-revision coordinate lookup)",
        )
        .call()?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take((MAX_REVISION_LOOKUP_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REVISION_LOOKUP_BYTES {
        return Err(ProviderError::InvalidInput(
            "latest-revision response exceeds size limit".into(),
        ));
    }
    parse_latest_revision_id(qid, &bytes)
}

/// Discover the current revision coordinate, then reacquire that exact RDF
/// manifestation. `latest` is therefore a lookup step only; the returned
/// semantic artifact is always revision-pinned.
pub fn fetch_latest_entity_rdf_revision_receipt(
    qid: &str,
) -> Result<AcquiredEntityRdf, ProviderError> {
    let revision_id = fetch_latest_revision_id(qid)?;
    fetch_entity_rdf_revision_receipt(qid, revision_id)
}
