#[path = "../../sl-governed-legal-provider/src/docx_text.rs"]
mod docx_text;

use docx_text::extract_docx_canonical_judgment;
use sensiblaw_proof_search_loop::judgment_candidates::{
    extract_judgment_citation_candidates_with_footnotes_and_anchors,
    FootnoteAnchorObservation,
};
use sensiblaw_proof_search_loop::live_artifact::{
    cli_paths, load_and_validate_cullen_inputs, CANDIDATE_ONLY_AUTHORITY,
};
use sensiblaw_proof_search_loop::live_artifact_validation::validate_cullen_shortlist_values;
use sensiblaw_proof_search_loop::residual_review_shortlist::{
    shortlist_anchored_citations_for_residual, ResidualAnchorCriterion,
    ResidualCitationReviewDemand, ResidualShortlistedCitation,
};
use sha2::{Digest, Sha256};
use std::fs;

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn json_array(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn shortlist_json(item: &ResidualShortlistedCitation) -> String {
    let candidate = &item.candidate;
    format!(
        concat!(
            "{{",
            "\"citation_text\":\"{}\",",
            "\"citation_locator_ref\":\"{}\",",
            "\"anchor_paragraph_locator_refs\":[{}],",
            "\"anchor_paragraph_texts\":[{}],",
            "\"matched_criterion_refs\":[{}],",
            "\"reviewed\":false,",
            "\"candidate_only\":true",
            "}}"
        ),
        json_escape(&candidate.citation_text),
        json_escape(&candidate.paragraph_locator_ref),
        json_array(&candidate.anchor_paragraph_locator_refs),
        json_array(&candidate.anchor_paragraph_texts),
        json_array(&item.matched_criterion_refs),
    )
}

fn refined_observer_text(body: &str, footnotes: &[(String, String)]) -> String {
    let mut out = String::from(body);
    for (id, text) in footnotes {
        out.push_str("\n[[footnote:");
        out.push_str(id);
        out.push_str("]]\n");
        out.push_str(text);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn criterion(
    criterion_ref: &str,
    residual_ref: &str,
    proposition_ref: &str,
    phrases: &[&str],
) -> ResidualAnchorCriterion {
    ResidualAnchorCriterion {
        criterion_ref: criterion_ref.into(),
        residual_ref: residual_ref.into(),
        proposition_ref: proposition_ref.into(),
        required_anchor_phrases: phrases.iter().map(|value| (*value).to_string()).collect(),
    }
}

fn main() {
    let (receipt_path, docx_path, output_path) =
        cli_paths("cullen-positive-operational-act-shortlist-v01.json");
    let inputs = load_and_validate_cullen_inputs(&receipt_path, &docx_path, &output_path)
        .expect("validate retained governed HCA judgment receipt and DOCX");

    let docx_bytes = fs::read(&inputs.docx_path).expect("read retained official judgment DOCX");
    let judgment = extract_docx_canonical_judgment(&docx_bytes)
        .expect("materialize body, footnotes and anchors");
    let footnotes = judgment
        .footnotes
        .iter()
        .map(|footnote| (footnote.footnote_id.clone(), footnote.text.clone()))
        .collect::<Vec<_>>();
    let refined_text = refined_observer_text(&judgment.body.text, &footnotes);
    let refined_sha256 = format!("sha256:{:x}", Sha256::digest(refined_text.as_bytes()));
    let anchors = judgment
        .body_paragraphs
        .iter()
        .flat_map(|paragraph| {
            paragraph.footnote_ids.iter().map(|footnote_id| {
                FootnoteAnchorObservation {
                    footnote_id: footnote_id.clone(),
                    paragraph_ordinal: paragraph.paragraph_ordinal,
                    paragraph_text: paragraph.text.clone(),
                }
            })
        })
        .collect::<Vec<_>>();

    let candidates = extract_judgment_citation_candidates_with_footnotes_and_anchors(
        &inputs.document_ref,
        &inputs.source_revision_ref,
        &refined_sha256,
        &judgment.body.text,
        &footnotes,
        &anchors,
    );

    let demand = ResidualCitationReviewDemand {
        residual_ref: inputs.residual_ref.clone(),
        proposition_ref: inputs.proposition_ref.clone(),
        criteria: vec![
            criterion(
                "criterion:cullen:positive-negligent-conduct",
                &inputs.residual_ref,
                &inputs.proposition_ref,
                &["positive negligent conduct", "physical injury"],
            ),
            criterion(
                "criterion:cullen:careless-act-vs-omission",
                &inputs.residual_ref,
                &inputs.proposition_ref,
                &["careless acts causing personal injury", "careless omissions"],
            ),
            criterion(
                "criterion:cullen:positive-act-vs-omission",
                &inputs.residual_ref,
                &inputs.proposition_ref,
                &["positive acts in creating risk", "omission to act"],
            ),
            criterion(
                "criterion:cullen:actions-not-failure-to-protect",
                &inputs.residual_ref,
                &inputs.proposition_ref,
                &["failed to protect her", "their actions resulted in her being injured"],
            ),
        ],
    };

    let shortlist = shortlist_anchored_citations_for_residual(&candidates, &demand)
        .expect("residual-indexed citation shortlist");
    validate_cullen_shortlist_values(candidates.len(), &shortlist)
        .expect("validate typed Cullen residual shortlist");

    let body = shortlist
        .iter()
        .map(shortlist_json)
        .collect::<Vec<_>>()
        .join(",\n    ");
    let receipt = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.residual_citation_review_shortlist.v0_1\",\n",
            "  \"authority\": \"experimental_candidate_only\",\n",
            "  \"network_requests\": 0,\n",
            "  \"source_queue_schema\": \"sl.judgment_citation_review_queue.v0_3\",\n",
            "  \"residual_ref\": \"{}\",\n",
            "  \"proposition_ref\": \"{}\",\n",
            "  \"document_ref\": \"{}\",\n",
            "  \"source_revision_ref\": \"{}\",\n",
            "  \"canonical_text_sha256\": \"{}\",\n",
            "  \"candidate_count\": {},\n",
            "  \"shortlist_count\": {},\n",
            "  \"shortlist_claimed_semantic_payment\": false,\n",
            "  \"shortlist_claimed_citation_treatment\": false,\n",
            "  \"shortlist_claimed_current_authority\": false,\n",
            "  \"shortlist_claimed_consumer_closure\": false,\n",
            "  \"shortlist\": [\n    {}\n  ]\n",
            "}}\n"
        ),
        json_escape(&inputs.residual_ref),
        json_escape(&inputs.proposition_ref),
        json_escape(&inputs.document_ref),
        json_escape(&inputs.source_revision_ref),
        json_escape(&refined_sha256),
        candidates.len(),
        shortlist.len(),
        body,
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&receipt).expect("self-validate residual shortlist JSON");
    assert_eq!(parsed["schema_version"], "sl.residual_citation_review_shortlist.v0_1");
    assert_eq!(parsed["authority"], CANDIDATE_ONLY_AUTHORITY);
    assert_eq!(parsed["network_requests"], 0);
    assert_eq!(parsed["candidate_count"], candidates.len());
    assert_eq!(parsed["shortlist_count"], shortlist.len());
    assert_eq!(parsed["shortlist_claimed_semantic_payment"], false);
    assert_eq!(parsed["shortlist_claimed_citation_treatment"], false);
    assert_eq!(parsed["shortlist_claimed_current_authority"], false);
    assert_eq!(parsed["shortlist_claimed_consumer_closure"], false);

    let robinson_count = shortlist
        .iter()
        .filter(|item| item.candidate.citation_text == "[2018] AC 736")
        .count();
    let modbury_count = shortlist
        .iter()
        .filter(|item| item.candidate.citation_text == "(2000) 205 CLR 254")
        .count();
    let mallonland_count = shortlist
        .iter()
        .filter(|item| item.candidate.citation_text == "(2024) 98 ALJR 956")
        .count();

    if let Some(parent) = inputs.output_path.parent() {
        fs::create_dir_all(parent).expect("create residual shortlist directory");
    }
    fs::write(&inputs.output_path, receipt).expect("write residual citation shortlist");
    println!(
        "cullen_residual_shortlist={} candidates={} shortlist={} robinson={} modbury={} mallonland={} network=0 authority={}",
        inputs.output_path.display(),
        candidates.len(),
        shortlist.len(),
        robinson_count,
        modbury_count,
        mallonland_count,
        CANDIDATE_ONLY_AUTHORITY,
    );
}
