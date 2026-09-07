#[path = "../../sl-governed-legal-provider/src/docx_text.rs"]
mod docx_text;

use docx_text::extract_docx_canonical_judgment;
use sensiblaw_proof_search_loop::judgment_candidates::{
    extract_judgment_citation_candidates_with_footnotes_and_anchors,
    CitationOccurrenceCandidate, FootnoteAnchorObservation, LexicalTreatmentHint,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn json_string_array(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn hint_name(hint: LexicalTreatmentHint) -> &'static str {
    match hint {
        LexicalTreatmentHint::AppliedCandidate => "AppliedCandidate",
        LexicalTreatmentHint::FollowedCandidate => "FollowedCandidate",
        LexicalTreatmentHint::DistinguishedCandidate => "DistinguishedCandidate",
        LexicalTreatmentHint::CriticisedCandidate => "CriticisedCandidate",
        LexicalTreatmentHint::RejectedCandidate => "RejectedCandidate",
        LexicalTreatmentHint::OverruledCandidate => "OverruledCandidate",
        LexicalTreatmentHint::ReliedOnCandidate => "ReliedOnCandidate",
        LexicalTreatmentHint::QuotedCandidate => "QuotedCandidate",
    }
}

fn candidate_json(candidate: &CitationOccurrenceCandidate) -> String {
    let label = candidate
        .reported_paragraph_label
        .as_ref()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".into());
    let hints = candidate
        .lexical_treatment_hints
        .iter()
        .map(|hint| format!("\"{}\"", hint_name(*hint)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{",
            "\"document_ref\":\"{}\",",
            "\"source_revision_ref\":\"{}\",",
            "\"canonical_text_sha256\":\"{}\",",
            "\"paragraph_ordinal\":{},",
            "\"paragraph_locator_ref\":\"{}\",",
            "\"reported_paragraph_label\":{},",
            "\"citation_text\":\"{}\",",
            "\"paragraph_text\":\"{}\",",
            "\"anchor_paragraph_locator_refs\":[{}],",
            "\"anchor_paragraph_texts\":[{}],",
            "\"lexical_treatment_hints\":[{}],",
            "\"reviewed\":false,",
            "\"candidate_only\":true",
            "}}"
        ),
        json_escape(&candidate.document_ref),
        json_escape(&candidate.source_revision_ref),
        json_escape(&candidate.canonical_text_sha256),
        candidate.paragraph_ordinal,
        json_escape(&candidate.paragraph_locator_ref),
        label,
        json_escape(&candidate.citation_text),
        json_escape(&candidate.paragraph_text),
        json_string_array(&candidate.anchor_paragraph_locator_refs),
        json_string_array(&candidate.anchor_paragraph_texts),
        hints,
    )
}

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing required environment variable {name}"))
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

fn main() {
    let docx_path = PathBuf::from(required_env("SENSIBLAW_JUDGMENT_DOCX"));
    let output_path = PathBuf::from(required_env("SENSIBLAW_CULLEN_REVIEW_QUEUE"));
    let document_ref = required_env("SENSIBLAW_DOCUMENT_REF");
    let source_revision_ref = required_env("SENSIBLAW_SOURCE_REVISION_REF");
    let body_only_text_sha256 = required_env("SENSIBLAW_CANONICAL_TEXT_SHA256");
    let residual_ref = required_env("SENSIBLAW_BOUND_RESIDUAL_REF");
    let proposition_ref = required_env("SENSIBLAW_BOUND_PROPOSITION_REF");
    let producer_ref = required_env("SENSIBLAW_BOUND_PRODUCER_REF");
    let hypothesis_ref = required_env("SENSIBLAW_BOUND_HYPOTHESIS_REF");

    let docx_bytes = fs::read(&docx_path).expect("read retained official judgment DOCX");
    let judgment = extract_docx_canonical_judgment(&docx_bytes)
        .expect("materialize body plus footnote judgment observer");
    let footnotes = judgment
        .footnotes
        .iter()
        .map(|footnote| (footnote.footnote_id.clone(), footnote.text.clone()))
        .collect::<Vec<_>>();
    let anchors = judgment
        .body_paragraphs
        .iter()
        .flat_map(|paragraph| {
            paragraph.footnote_ids.iter().map(|footnote_id| FootnoteAnchorObservation {
                footnote_id: footnote_id.clone(),
                paragraph_ordinal: paragraph.paragraph_ordinal,
                paragraph_text: paragraph.text.clone(),
            })
        })
        .collect::<Vec<_>>();
    let refined_text = refined_observer_text(&judgment.body.text, &footnotes);
    let refined_sha256 = format!("sha256:{:x}", Sha256::digest(refined_text.as_bytes()));

    let candidates = extract_judgment_citation_candidates_with_footnotes_and_anchors(
        &document_ref,
        &source_revision_ref,
        &refined_sha256,
        &judgment.body.text,
        &footnotes,
        &anchors,
    );
    let footnote_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.paragraph_locator_ref.contains("#footnote-"))
        .count();
    let anchored_footnote_candidate_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.paragraph_locator_ref.contains("#footnote-")
                && !candidate.anchor_paragraph_locator_refs.is_empty()
        })
        .count();

    let body = candidates
        .iter()
        .map(candidate_json)
        .collect::<Vec<_>>()
        .join(",\n    ");
    let receipt = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.judgment_citation_review_queue.v0_3\",\n",
            "  \"authority\": \"experimental_candidate_only\",\n",
            "  \"network_requests\": 0,\n",
            "  \"binding\": {{",
            "\"residual_ref\":\"{}\",",
            "\"proposition_ref\":\"{}\",",
            "\"scheduled_producer_ref\":\"{}\",",
            "\"hypothesis_ref\":\"{}\"",
            "}},\n",
            "  \"document_ref\": \"{}\",\n",
            "  \"source_revision_ref\": \"{}\",\n",
            "  \"body_only_canonical_text_sha256\": \"{}\",\n",
            "  \"canonical_text_sha256\": \"{}\",\n",
            "  \"observer_refinement\": {{",
            "\"body_footnotes_preserved\":true,",
            "\"body_footnote_anchors_preserved\":true,",
            "\"body_paragraph_count\":{},",
            "\"footnote_count\":{},",
            "\"footnote_anchor_count\":{}",
            "}},\n",
            "  \"candidate_count\": {},\n",
            "  \"footnote_candidate_count\": {},\n",
            "  \"anchored_footnote_candidate_count\": {},\n",
            "  \"all_candidates_reviewed\": false,\n",
            "  \"candidate_extraction_claimed_semantic_correspondence\": false,\n",
            "  \"candidate_extraction_claimed_citation_treatment\": false,\n",
            "  \"candidate_extraction_claimed_current_authority\": false,\n",
            "  \"anchor_observation_claimed_residual_payment\": false,\n",
            "  \"candidates\": [\n    {}\n  ]\n",
            "}}\n"
        ),
        json_escape(&residual_ref),
        json_escape(&proposition_ref),
        json_escape(&producer_ref),
        json_escape(&hypothesis_ref),
        json_escape(&document_ref),
        json_escape(&source_revision_ref),
        json_escape(&body_only_text_sha256),
        json_escape(&refined_sha256),
        judgment.body.paragraph_count,
        footnotes.len(),
        anchors.len(),
        candidates.len(),
        footnote_candidate_count,
        anchored_footnote_candidate_count,
        body,
    );

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("create review queue directory");
    }
    fs::write(&output_path, receipt).expect("write citation review queue");
    println!(
        "cullen_review_queue={} candidates={} footnotes={} anchors={} anchored_footnote_candidates={} network=0 authority=experimental_candidate_only",
        output_path.display(),
        candidates.len(),
        footnotes.len(),
        anchors.len(),
        anchored_footnote_candidate_count,
    );
}
