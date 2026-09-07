//! Source-located candidate extraction from canonical judgment text.
//!
//! This module is deliberately pre-review.  It identifies stable paragraph
//! locators, medium-neutral citation-shaped strings and lexical treatment hints.
//! It does not infer proposition correspondence, CitationUse, ReasoningRole,
//! ratio, authority, applicability or truth.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LexicalTreatmentHint {
    AppliedCandidate,
    FollowedCandidate,
    DistinguishedCandidate,
    CriticisedCandidate,
    RejectedCandidate,
    OverruledCandidate,
    ReliedOnCandidate,
    QuotedCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationOccurrenceCandidate {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub paragraph_ordinal: u64,
    pub paragraph_locator_ref: String,
    pub reported_paragraph_label: Option<String>,
    pub citation_text: String,
    pub paragraph_text: String,
    pub lexical_treatment_hints: Vec<LexicalTreatmentHint>,
    pub reviewed: bool,
    pub candidate_only: bool,
}

fn reported_paragraph_label(paragraph: &str) -> Option<String> {
    let trimmed = paragraph.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let end = rest.find(']')?;
    let digits = &rest[..end];
    (!digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| format!("[{digits}]"))
}

fn treatment_hints(paragraph: &str) -> Vec<LexicalTreatmentHint> {
    let lower = paragraph.to_ascii_lowercase();
    let mut hints = Vec::new();
    let candidates = [
        ("applied", LexicalTreatmentHint::AppliedCandidate),
        ("followed", LexicalTreatmentHint::FollowedCandidate),
        ("distinguished", LexicalTreatmentHint::DistinguishedCandidate),
        ("criticised", LexicalTreatmentHint::CriticisedCandidate),
        ("criticized", LexicalTreatmentHint::CriticisedCandidate),
        ("rejected", LexicalTreatmentHint::RejectedCandidate),
        ("overruled", LexicalTreatmentHint::OverruledCandidate),
        ("relied on", LexicalTreatmentHint::ReliedOnCandidate),
        ("quoted", LexicalTreatmentHint::QuotedCandidate),
    ];
    for (needle, hint) in candidates {
        if lower.contains(needle) && !hints.contains(&hint) {
            hints.push(hint);
        }
    }
    hints.sort();
    hints
}

fn is_court_token(token: &str) -> bool {
    let len = token.len();
    (2..=10).contains(&len)
        && token
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && token.chars().any(|ch| ch.is_ascii_uppercase())
}

fn extract_mnc_strings(paragraph: &str) -> Vec<String> {
    let bytes = paragraph.as_bytes();
    let mut out = Vec::new();
    let mut i = 0_usize;
    while i + 6 < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        if i + 6 > bytes.len()
            || !bytes[i + 1..i + 5].iter().all(u8::is_ascii_digit)
            || bytes[i + 5] != b']'
        {
            i += 1;
            continue;
        }
        let year = &paragraph[i..i + 6];
        let mut cursor = i + 6;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let court_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
            cursor += 1;
        }
        if court_start == cursor {
            i += 1;
            continue;
        }
        let court = &paragraph[court_start..cursor];
        if !is_court_token(court) {
            i += 1;
            continue;
        }
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let number_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if number_start == cursor {
            i += 1;
            continue;
        }
        let number = &paragraph[number_start..cursor];
        let citation = format!("{year} {court} {number}");
        if !out.contains(&citation) {
            out.push(citation);
        }
        i = cursor;
    }
    out
}

pub fn extract_judgment_citation_candidates(
    document_ref: &str,
    source_revision_ref: &str,
    canonical_text_sha256: &str,
    canonical_text: &str,
) -> Vec<CitationOccurrenceCandidate> {
    let mut candidates = Vec::new();
    for (index, paragraph) in canonical_text.lines().enumerate() {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        let ordinal = index as u64 + 1;
        let locator = format!("{document_ref}#paragraph-{ordinal}");
        let label = reported_paragraph_label(paragraph);
        let hints = treatment_hints(paragraph);
        for citation in extract_mnc_strings(paragraph) {
            candidates.push(CitationOccurrenceCandidate {
                document_ref: document_ref.to_string(),
                source_revision_ref: source_revision_ref.to_string(),
                canonical_text_sha256: canonical_text_sha256.to_string(),
                paragraph_ordinal: ordinal,
                paragraph_locator_ref: locator.clone(),
                reported_paragraph_label: label.clone(),
                citation_text: citation,
                paragraph_text: paragraph.to_string(),
                lexical_treatment_hints: hints.clone(),
                reviewed: false,
                candidate_only: true,
            });
        }
    }
    candidates
}

pub const fn citation_candidate_is_semantic_correspondence(
    _candidate: &CitationOccurrenceCandidate,
) -> bool {
    false
}

pub const fn lexical_hint_is_citation_use(_hint: LexicalTreatmentHint) -> bool {
    false
}

pub const fn citation_candidate_is_current_authority(
    _candidate: &CitationOccurrenceCandidate,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_source_located_mncs_without_promoting_treatment() {
        let text = "[42] We applied Mallonland Pty Ltd v Advanta Seeds Pty Ltd [2024] HCA 25 and distinguished Woolcock Street Investments Pty Ltd v CDG Pty Ltd [2004] HCA 16.\n[43] No further authority was necessary.\n";
        let candidates = extract_judgment_citation_candidates(
            "document:hca:[2026]-HCA-19:docx",
            "source-revision:fixture",
            "sha256:text-fixture",
            text,
        );
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].reported_paragraph_label.as_deref(), Some("[42]"));
        assert_eq!(candidates[0].citation_text, "[2024] HCA 25");
        assert_eq!(candidates[1].citation_text, "[2004] HCA 16");
        assert!(candidates[0]
            .lexical_treatment_hints
            .contains(&LexicalTreatmentHint::AppliedCandidate));
        assert!(candidates[0]
            .lexical_treatment_hints
            .contains(&LexicalTreatmentHint::DistinguishedCandidate));
        assert!(!candidates[0].reviewed);
        assert!(candidates[0].candidate_only);
        assert!(!citation_candidate_is_semantic_correspondence(&candidates[0]));
        assert!(!lexical_hint_is_citation_use(
            LexicalTreatmentHint::AppliedCandidate
        ));
        assert!(!citation_candidate_is_current_authority(&candidates[0]));
    }
}
