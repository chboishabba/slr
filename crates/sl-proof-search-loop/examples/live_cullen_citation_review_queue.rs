use sensiblaw_proof_search_loop::judgment_candidates::{
    extract_judgment_citation_candidates, CitationOccurrenceCandidate, LexicalTreatmentHint,
};
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
        hints,
    )
}

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing required environment variable {name}"))
}

fn main() {
    let text_path = PathBuf::from(required_env("SENSIBLAW_CANONICAL_JUDGMENT_TEXT"));
    let output_path = PathBuf::from(required_env("SENSIBLAW_CULLEN_REVIEW_QUEUE"));
    let document_ref = required_env("SENSIBLAW_DOCUMENT_REF");
    let source_revision_ref = required_env("SENSIBLAW_SOURCE_REVISION_REF");
    let canonical_text_sha256 = required_env("SENSIBLAW_CANONICAL_TEXT_SHA256");
    let residual_ref = required_env("SENSIBLAW_BOUND_RESIDUAL_REF");
    let proposition_ref = required_env("SENSIBLAW_BOUND_PROPOSITION_REF");
    let producer_ref = required_env("SENSIBLAW_BOUND_PRODUCER_REF");
    let hypothesis_ref = required_env("SENSIBLAW_BOUND_HYPOTHESIS_REF");

    let text = fs::read_to_string(&text_path).expect("read canonical judgment text");
    let candidates = extract_judgment_citation_candidates(
        &document_ref,
        &source_revision_ref,
        &canonical_text_sha256,
        &text,
    );

    let body = candidates
        .iter()
        .map(candidate_json)
        .collect::<Vec<_>>()
        .join(",\n    ");
    let receipt = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.judgment_citation_review_queue.v0_1\",\n",
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
            "  \"canonical_text_sha256\": \"{}\",\n",
            "  \"candidate_count\": {},\n",
            "  \"all_candidates_reviewed\": false,\n",
            "  \"candidate_extraction_claimed_semantic_correspondence\": false,\n",
            "  \"candidate_extraction_claimed_citation_treatment\": false,\n",
            "  \"candidate_extraction_claimed_current_authority\": false,\n",
            "  \"candidates\": [\n    {}\n  ]\n",
            "}}\n"
        ),
        json_escape(&residual_ref),
        json_escape(&proposition_ref),
        json_escape(&producer_ref),
        json_escape(&hypothesis_ref),
        json_escape(&document_ref),
        json_escape(&source_revision_ref),
        json_escape(&canonical_text_sha256),
        candidates.len(),
        body,
    );

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("create review queue directory");
    }
    fs::write(&output_path, receipt).expect("write citation review queue");
    println!(
        "cullen_review_queue={} candidates={} network=0 authority=experimental_candidate_only",
        output_path.display(),
        candidates.len()
    );
}
