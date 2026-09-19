//! Source-located candidate extraction from canonical judgment text.
//!
//! This module is deliberately pre-review. It identifies stable body/footnote
//! locators, medium-neutral and reported citation-shaped strings, lexical
//! treatment hints, and (when supplied) exact body-paragraph footnote anchors.
//! None of those observations infer proposition correspondence, CitationUse,
//! ReasoningRole, ratio, authority, applicability, residual payment or truth.

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
    pub anchor_paragraph_locator_refs: Vec<String>,
    pub anchor_paragraph_texts: Vec<String>,
    pub lexical_treatment_hints: Vec<LexicalTreatmentHint>,
    pub reviewed: bool,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootnoteAnchorObservation {
    pub footnote_id: String,
    pub paragraph_ordinal: u64,
    pub paragraph_text: String,
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

fn is_upper_reporter_token(token: &str) -> bool {
    let token = token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    let len = token.len();
    (2..=14).contains(&len)
        && token
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && token.chars().any(|ch| ch.is_ascii_uppercase())
}

fn is_year_token(token: &str) -> bool {
    let stripped = token.trim_matches(['[', ']', '(', ')']);
    stripped.len() == 4 && stripped.chars().all(|ch| ch.is_ascii_digit())
}

fn clean_token(token: &str) -> &str {
    token.trim_matches(|ch: char| matches!(ch, ',' | ';' | '.' | ':'))
}

fn is_number_token(token: &str) -> bool {
    clean_token(token).chars().all(|ch| ch.is_ascii_digit()) && !clean_token(token).is_empty()
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
        if !is_upper_reporter_token(court) {
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

fn extract_reported_citation_strings(text: &str) -> Vec<String> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let mut out = Vec::new();

    for i in 0..tokens.len() {
        let token = clean_token(tokens[i]);

        if is_year_token(token)
            && i + 3 < tokens.len()
            && is_number_token(tokens[i + 1])
            && is_upper_reporter_token(tokens[i + 2])
            && is_number_token(tokens[i + 3])
        {
            let citation = format!(
                "{} {} {} {}",
                clean_token(tokens[i]),
                clean_token(tokens[i + 1]),
                clean_token(tokens[i + 2]),
                clean_token(tokens[i + 3])
            );
            if !out.contains(&citation) {
                out.push(citation);
            }
            continue;
        }

        if is_year_token(token)
            && i + 2 < tokens.len()
            && is_upper_reporter_token(tokens[i + 1])
            && is_number_token(tokens[i + 2])
        {
            let citation = format!(
                "{} {} {}",
                clean_token(tokens[i]),
                clean_token(tokens[i + 1]),
                clean_token(tokens[i + 2])
            );
            if !out.contains(&citation) {
                out.push(citation);
            }
            continue;
        }

        let previous_is_year = i > 0 && is_year_token(clean_token(tokens[i - 1]));
        if !previous_is_year
            && i + 2 < tokens.len()
            && is_number_token(token)
            && is_upper_reporter_token(tokens[i + 1])
            && is_number_token(tokens[i + 2])
        {
            let citation = format!(
                "{} {} {}",
                clean_token(tokens[i]),
                clean_token(tokens[i + 1]),
                clean_token(tokens[i + 2])
            );
            if !out.contains(&citation) {
                out.push(citation);
            }
        }
    }

    out
}

fn extract_citation_strings(text: &str) -> Vec<String> {
    let mut out = extract_mnc_strings(text);
    for citation in extract_reported_citation_strings(text) {
        if !out.contains(&citation) {
            out.push(citation);
        }
    }
    out
}

struct CandidateObservation<'a> {
    document_ref: &'a str,
    source_revision_ref: &'a str,
    canonical_text_sha256: &'a str,
    ordinal: u64,
    locator: String,
    label: Option<String>,
    observation_text: &'a str,
    anchor_locator_refs: Vec<String>,
    anchor_texts: Vec<String>,
}

fn push_candidates_for_observation(
    candidates: &mut Vec<CitationOccurrenceCandidate>,
    observation: CandidateObservation<'_>,
) {
    let hints = treatment_hints(observation.observation_text);
    for citation in extract_citation_strings(observation.observation_text) {
        let duplicate = candidates.iter().any(|candidate| {
            candidate.paragraph_locator_ref == observation.locator
                && candidate.citation_text == citation
        });
        if duplicate {
            continue;
        }
        candidates.push(CitationOccurrenceCandidate {
            document_ref: observation.document_ref.to_string(),
            source_revision_ref: observation.source_revision_ref.to_string(),
            canonical_text_sha256: observation.canonical_text_sha256.to_string(),
            paragraph_ordinal: observation.ordinal,
            paragraph_locator_ref: observation.locator.clone(),
            reported_paragraph_label: observation.label.clone(),
            citation_text: citation,
            paragraph_text: observation.observation_text.to_string(),
            anchor_paragraph_locator_refs: observation.anchor_locator_refs.clone(),
            anchor_paragraph_texts: observation.anchor_texts.clone(),
            lexical_treatment_hints: hints.clone(),
            reviewed: false,
            candidate_only: true,
        });
    }
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
        push_candidates_for_observation(
            &mut candidates,
            CandidateObservation {
                document_ref,
                source_revision_ref,
                canonical_text_sha256,
                ordinal,
                locator: locator.clone(),
                label: reported_paragraph_label(paragraph),
                observation_text: paragraph,
                anchor_locator_refs: vec![locator],
                anchor_texts: vec![paragraph.to_string()],
            },
        );
    }
    candidates
}

pub fn extract_judgment_citation_candidates_with_footnotes(
    document_ref: &str,
    source_revision_ref: &str,
    canonical_text_sha256: &str,
    canonical_text: &str,
    footnotes: &[(String, String)],
) -> Vec<CitationOccurrenceCandidate> {
    extract_judgment_citation_candidates_with_footnotes_and_anchors(
        document_ref,
        source_revision_ref,
        canonical_text_sha256,
        canonical_text,
        footnotes,
        &[],
    )
}

pub fn extract_judgment_citation_candidates_with_footnotes_and_anchors(
    document_ref: &str,
    source_revision_ref: &str,
    canonical_text_sha256: &str,
    canonical_text: &str,
    footnotes: &[(String, String)],
    anchors: &[FootnoteAnchorObservation],
) -> Vec<CitationOccurrenceCandidate> {
    let mut candidates = extract_judgment_citation_candidates(
        document_ref,
        source_revision_ref,
        canonical_text_sha256,
        canonical_text,
    );

    for (footnote_id, footnote_text) in footnotes {
        let ordinal = footnote_id.parse::<u64>().unwrap_or(0);
        let matching_anchors = anchors
            .iter()
            .filter(|anchor| anchor.footnote_id == *footnote_id)
            .collect::<Vec<_>>();
        let anchor_locator_refs = matching_anchors
            .iter()
            .map(|anchor| format!("{document_ref}#paragraph-{}", anchor.paragraph_ordinal))
            .collect::<Vec<_>>();
        let anchor_texts = matching_anchors
            .iter()
            .map(|anchor| anchor.paragraph_text.clone())
            .collect::<Vec<_>>();

        push_candidates_for_observation(
            &mut candidates,
            CandidateObservation {
                document_ref,
                source_revision_ref,
                canonical_text_sha256,
                ordinal,
                locator: format!("{document_ref}#footnote-{footnote_id}"),
                label: Some(format!("footnote:{footnote_id}")),
                observation_text: footnote_text.trim(),
                anchor_locator_refs,
                anchor_texts,
            },
        );
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

pub const fn footnote_citation_candidate_is_treatment(
    _candidate: &CitationOccurrenceCandidate,
) -> bool {
    false
}

pub const fn footnote_anchor_is_residual_payment(
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
        assert_eq!(candidates[0].anchor_paragraph_locator_refs.len(), 1);
        assert!(!citation_candidate_is_semantic_correspondence(&candidates[0]));
        assert!(!lexical_hint_is_citation_use(
            LexicalTreatmentHint::AppliedCandidate
        ));
        assert!(!citation_candidate_is_current_authority(&candidates[0]));
    }

    #[test]
    fn extracts_reported_and_parallel_citations_from_footnotes() {
        let footnotes = vec![
            (
                "90".to_string(),
                "Mallonland Pty Ltd v Advanta Seeds Pty Ltd (2024) 98 ALJR 956 at 978 [89]-[90]; 418 ALR 639 at 664-665.".to_string(),
            ),
            (
                "113".to_string(),
                "Robinson v Chief Constable of West Yorkshire Police [2018] AC 736 at 742 [13].".to_string(),
            ),
        ];
        let candidates = extract_judgment_citation_candidates_with_footnotes(
            "document:hca:[2026]-HCA-19:docx",
            "source-revision:fixture",
            "sha256:text-fixture",
            "[2026] HCA 19\n",
            &footnotes,
        );
        assert!(candidates.iter().any(|candidate| {
            candidate.paragraph_locator_ref.ends_with("#footnote-90")
                && candidate.citation_text == "(2024) 98 ALJR 956"
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.paragraph_locator_ref.ends_with("#footnote-90")
                && candidate.citation_text == "418 ALR 639"
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.paragraph_locator_ref.ends_with("#footnote-113")
                && candidate.citation_text == "[2018] AC 736"
        }));
        assert!(candidates.iter().all(|candidate| !candidate.reviewed && candidate.candidate_only));
        assert!(!footnote_citation_candidate_is_treatment(&candidates[0]));
    }

    #[test]
    fn footnote_candidate_retains_exact_body_anchor_without_becoming_payment() {
        let footnotes = vec![(
            "105".to_string(),
            "AA (2026) 100 ALJR 170; Robinson [2018] AC 736 at 761 [73].".to_string(),
        )];
        let anchors = vec![FootnoteAnchorObservation {
            footnote_id: "105".into(),
            paragraph_ordinal: 64,
            paragraph_text: "Any liability would be based upon positive acts in creating risk, not an omission.".into(),
        }];
        let candidates = extract_judgment_citation_candidates_with_footnotes_and_anchors(
            "document:hca:[2026]-HCA-19:docx",
            "source-revision:fixture",
            "sha256:refined",
            "Cullen v New South Wales\n",
            &footnotes,
            &anchors,
        );
        let robinson = candidates
            .iter()
            .find(|candidate| candidate.citation_text == "[2018] AC 736")
            .unwrap();
        assert_eq!(
            robinson.anchor_paragraph_locator_refs,
            vec!["document:hca:[2026]-HCA-19:docx#paragraph-64"]
        );
        assert!(robinson.anchor_paragraph_texts[0].contains("positive acts in creating risk"));
        assert!(!footnote_anchor_is_residual_payment(robinson));
    }
}
