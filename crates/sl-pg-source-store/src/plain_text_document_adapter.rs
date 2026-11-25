//! INGEST-1A clean long-document adapter.
//!
//! This adapter is intentionally structural, not semantic. It provides stable
//! chapter/paragraph/sentence candidate regions over an immutable text revision
//! so the existing M12 compiler can operate at book scale. Segmentation errors
//! remain structural residuals/candidates; they do not alter source identity or
//! create semantic authority.

use sha2::{Digest, Sha256};
use sensiblaw_core::source_ingest::{
    DocumentRegion, DocumentRegionKind, IngestRoleClass, LongDocumentSource,
    SourceFamily, SourceIngestEnvelope,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlainTextDocumentAdapterError {
    #[error("source ref is empty")]
    EmptySourceRef,
    #[error("source revision ref is empty")]
    EmptyRevisionRef,
    #[error("provider ref is empty")]
    EmptyProviderRef,
    #[error("acquisition receipt ref is empty")]
    EmptyAcquisitionReceipt,
    #[error("document text is empty")]
    EmptyText,
    #[error("source family is not eligible for generic long-text compilation")]
    UnsupportedLongTextFamily,
    #[error("heading repair rejected: {0}")]
    InvalidHeadingRepair(&'static str),
    #[error("generated source structure failed validation: {0:?}")]
    InvalidSource(sensiblaw_core::source_ingest::SourceIngestError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainTextSegmentationReceipt {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub char_count: usize,
    pub chapter_count: usize,
    pub paragraph_count: usize,
    pub sentence_count: usize,
    pub structural_region_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub segmentation_claims_semantic_completeness: bool,
}

fn sha256_ref(text: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(text.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

pub(crate) fn chapter_heading(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower == "chapter"
        || lower.starts_with("chapter ")
        || lower.starts_with("chapter\t")
        || lower.starts_with("book ")
        || lower.starts_with("part ")
}

fn char_line_ranges(text: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for line in text.split_inclusive('\n') {
        let len = line.chars().count();
        out.push((start, start + len, line));
        start += len;
    }
    out
}

fn paragraph_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut current_start: Option<usize> = None;
    let mut current_end = 0usize;

    for (start, end, line) in char_line_ranges(text) {
        if line.trim().is_empty() {
            if let Some(paragraph_start) = current_start.take() {
                if paragraph_start < current_end {
                    out.push((paragraph_start, current_end));
                }
            }
            continue;
        }
        if current_start.is_none() {
            current_start = Some(start);
        }
        current_end = end;
    }

    if let Some(paragraph_start) = current_start {
        if paragraph_start < current_end {
            out.push((paragraph_start, current_end));
        }
    }
    out
}

fn char_at(chars: &[char], index: usize) -> Option<char> {
    chars.get(index).copied()
}

fn is_sentence_terminal(c: char) -> bool {
    matches!(c, '.' | '?' | '!')
}

fn sentence_ranges(text: &str, paragraph_start: usize, paragraph_end: usize) -> Vec<(usize, usize)> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut sentence_start = paragraph_start;
    let mut i = paragraph_start;

    while i < paragraph_end {
        let c = chars[i];
        if is_sentence_terminal(c) {
            let mut end = i + 1;
            while end < paragraph_end
                && matches!(
                    char_at(&chars, end),
                    Some('"' | '\'' | '”' | '’' | ')' | ']')
                )
            {
                end += 1;
            }

            let boundary = end >= paragraph_end
                || char_at(&chars, end).is_some_and(char::is_whitespace);
            if boundary {
                while sentence_start < end
                    && char_at(&chars, sentence_start).is_some_and(char::is_whitespace)
                {
                    sentence_start += 1;
                }
                if sentence_start < end {
                    out.push((sentence_start, end));
                }
                sentence_start = end;
                while sentence_start < paragraph_end
                    && char_at(&chars, sentence_start).is_some_and(char::is_whitespace)
                {
                    sentence_start += 1;
                }
                i = sentence_start;
                continue;
            }
        }
        i += 1;
    }

    while sentence_start < paragraph_end
        && char_at(&chars, sentence_start).is_some_and(char::is_whitespace)
    {
        sentence_start += 1;
    }
    let mut trailing_end = paragraph_end;
    while trailing_end > sentence_start
        && char_at(&chars, trailing_end - 1).is_some_and(char::is_whitespace)
    {
        trailing_end -= 1;
    }
    if sentence_start < trailing_end {
        out.push((sentence_start, trailing_end));
    }

    out
}

/// Versioned additive repair of one *original* heading-excluded paragraph.
/// The caller supplies the exact paragraph literal, not a guessed gap.
/// Heading-shaped first lines become a separate candidate region, not a
/// semantic assertion; prose is segmented by the original sentence owner.
/// Standalone headings retain the original structural-only policy.
pub(crate) fn heading_exclusion_repair_regions(
    paragraph: &DocumentRegion,
    literal: &str,
) -> Result<Vec<DocumentRegion>, PlainTextDocumentAdapterError> {
    use PlainTextDocumentAdapterError::InvalidHeadingRepair;
    let chars = literal.chars().collect::<Vec<_>>();
    if paragraph.kind != DocumentRegionKind::Paragraph
        || paragraph.end_char.checked_sub(paragraph.start_char)
            != Some(chars.len() as u64)
        || paragraph.region_ref.is_empty()
        || paragraph.source_revision_ref.is_empty()
    {
        return Err(InvalidHeadingRepair("foreign paragraph coordinates"));
    }
    let mut lines = char_line_ranges(literal).into_iter();
    let Some((first_start, first_end, first_line)) = lines.next() else {
        return Ok(Vec::new());
    };
    if !chapter_heading(literal) || !chapter_heading(first_line)
        || !lines.any(|(_, _, line)| !line.trim().is_empty())
    {
        return Ok(Vec::new());
    }
    let mut heading_start = first_start;
    let mut heading_end = first_end;
    while heading_start < heading_end && chars[heading_start].is_whitespace() {
        heading_start += 1;
    }
    while heading_end > heading_start && chars[heading_end - 1].is_whitespace() {
        heading_end -= 1;
    }
    let mut spans = vec![(heading_start, heading_end)];
    spans.extend(sentence_ranges(literal, first_end, chars.len()));
    let mut cursor = 0usize;
    let mut added = Vec::new();
    for (a, b) in spans {
        if a < cursor || a >= b || b > chars.len()
            || chars[cursor..a].iter().any(|c| !c.is_whitespace())
        {
            return Err(InvalidHeadingRepair("non-exact local partition"));
        }
        cursor = b;
        let start = paragraph.start_char + a as u64;
        let end = paragraph.start_char + b as u64;
        added.push(DocumentRegion {
            region_ref: format!(
                "document-region:{}:sentence:heading-repair-v1:{start}-{end}",
                paragraph.source_revision_ref
            ),
            source_revision_ref: paragraph.source_revision_ref.clone(),
            parent_region_ref: Some(paragraph.region_ref.clone()),
            kind: DocumentRegionKind::Sentence,
            start_char: start,
            end_char: end,
        });
    }
    if chars[cursor..].iter().any(|c| !c.is_whitespace()) {
        return Err(InvalidHeadingRepair("uncovered paragraph suffix"));
    }
    Ok(added)
}

/// Prepare an exact additive structural delta for selected original paragraphs.
/// Existing refs and source bytes remain unchanged, even on repeat calls.
/// This does not persist, schedule a parser or pay typed identity coverage.
pub fn extend_plain_text_heading_exclusions(
    original: &LongDocumentSource,
    text: &str,
    paragraph_refs: &[String],
) -> Result<(LongDocumentSource, Vec<DocumentRegion>), PlainTextDocumentAdapterError> {
    use PlainTextDocumentAdapterError::InvalidHeadingRepair;
    original.validate().map_err(PlainTextDocumentAdapterError::InvalidSource)?;
    if original.ingest.content_digest_ref != sha256_ref(text) {
        return Err(InvalidHeadingRepair("source digest mismatch"));
    }
    let chars = text.chars().collect::<Vec<_>>();
    let mut extended = original.clone();
    let mut added = Vec::new();
    for parent_ref in paragraph_refs {
        let paragraph = original.regions.iter()
            .find(|r| &r.region_ref == parent_ref && r.kind == DocumentRegionKind::Paragraph)
            .ok_or(InvalidHeadingRepair("original paragraph missing"))?;
        let a = usize::try_from(paragraph.start_char)
            .map_err(|_| InvalidHeadingRepair("paragraph start overflow"))?;
        let b = usize::try_from(paragraph.end_char)
            .map_err(|_| InvalidHeadingRepair("paragraph end overflow"))?;
        let literal = chars.get(a..b)
            .ok_or(InvalidHeadingRepair("paragraph outside canonical source"))?
            .iter().collect::<String>();
        for region in heading_exclusion_repair_regions(paragraph, &literal)? {
            if let Some(existing) = extended.regions.iter()
                .find(|r| r.region_ref == region.region_ref)
            {
                if existing != &region {
                    return Err(InvalidHeadingRepair("repair ref collision"));
                }
                continue;
            }
            if extended.regions.iter().any(|r| r.kind == DocumentRegionKind::Sentence
                && r.start_char < region.end_char && region.start_char < r.end_char)
            {
                return Err(InvalidHeadingRepair("repair overlaps original sentence"));
            }
            extended.regions.push(region.clone());
            added.push(region);
        }
    }
    extended.validate().map_err(PlainTextDocumentAdapterError::InvalidSource)?;
    Ok((extended, added))
}

fn chapter_ranges(text: &str) -> Vec<(usize, usize)> {
    let lines = char_line_ranges(text);
    let mut headings = lines
        .iter()
        .filter(|(_, _, line)| chapter_heading(line))
        .map(|(start, _, _)| *start)
        .collect::<Vec<_>>();
    headings.sort_unstable();
    headings.dedup();

    if headings.is_empty() {
        return vec![];
    }

    let text_len = text.chars().count();
    headings
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = headings.get(index + 1).copied().unwrap_or(text_len);
            (*start, end)
        })
        .collect()
}

fn parent_chapter_ref(
    chapters: &[(usize, usize, String)],
    start: usize,
    end: usize,
) -> Option<String> {
    chapters
        .iter()
        .find(|(chapter_start, chapter_end, _)| {
            *chapter_start <= start && end <= *chapter_end
        })
        .map(|(_, _, reference)| reference.clone())
}

pub fn build_plain_text_long_document_source(
    source_ref: &str,
    source_revision_ref: &str,
    provider_ref: &str,
    acquisition_receipt_ref: &str,
    title: Option<String>,
    edition_ref: Option<String>,
    text: &str,
) -> Result<(LongDocumentSource, PlainTextSegmentationReceipt), PlainTextDocumentAdapterError> {
    build_plain_text_long_source_with_family(
        source_ref,
        source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        SourceFamily::Document,
        title,
        edition_ref,
        text,
    )
}

pub fn build_plain_text_long_source_with_family(
    source_ref: &str,
    source_revision_ref: &str,
    provider_ref: &str,
    acquisition_receipt_ref: &str,
    family: SourceFamily,
    title: Option<String>,
    edition_ref: Option<String>,
    text: &str,
) -> Result<(LongDocumentSource, PlainTextSegmentationReceipt), PlainTextDocumentAdapterError> {
    if source_ref.trim().is_empty() {
        return Err(PlainTextDocumentAdapterError::EmptySourceRef);
    }
    if source_revision_ref.trim().is_empty() {
        return Err(PlainTextDocumentAdapterError::EmptyRevisionRef);
    }
    if provider_ref.trim().is_empty() {
        return Err(PlainTextDocumentAdapterError::EmptyProviderRef);
    }
    if acquisition_receipt_ref.trim().is_empty() {
        return Err(PlainTextDocumentAdapterError::EmptyAcquisitionReceipt);
    }
    if text.is_empty() {
        return Err(PlainTextDocumentAdapterError::EmptyText);
    }
    if !matches!(
        family,
        SourceFamily::Document
            | SourceFamily::Web
            | SourceFamily::Wiki
            | SourceFamily::Transcript
            | SourceFamily::ImageOcr
            | SourceFamily::LegalAuthority
            | SourceFamily::NoteResearch
    ) {
        return Err(PlainTextDocumentAdapterError::UnsupportedLongTextFamily);
    }

    let chapter_candidates = chapter_ranges(text);
    let mut regions = Vec::new();
    let mut chapters = Vec::new();

    for (index, (start, end)) in chapter_candidates.into_iter().enumerate() {
        let reference = format!("document-region:{source_revision_ref}:chapter:{index}:{start}-{end}");
        regions.push(DocumentRegion {
            region_ref: reference.clone(),
            source_revision_ref: source_revision_ref.to_owned(),
            parent_region_ref: None,
            kind: DocumentRegionKind::Chapter,
            start_char: start as u64,
            end_char: end as u64,
        });
        chapters.push((start, end, reference));
    }

    let paragraphs = paragraph_ranges(text);
    let mut paragraph_refs = Vec::new();
    for (index, (start, end)) in paragraphs.iter().copied().enumerate() {
        let reference =
            format!("document-region:{source_revision_ref}:paragraph:{index}:{start}-{end}");
        regions.push(DocumentRegion {
            region_ref: reference.clone(),
            source_revision_ref: source_revision_ref.to_owned(),
            parent_region_ref: parent_chapter_ref(&chapters, start, end),
            kind: DocumentRegionKind::Paragraph,
            start_char: start as u64,
            end_char: end as u64,
        });
        paragraph_refs.push((start, end, reference));
    }

    let all_chars = text.chars().collect::<Vec<_>>();
    let mut sentence_count = 0usize;
    for (paragraph_start, paragraph_end, paragraph_ref) in &paragraph_refs {
        let paragraph_text = all_chars[*paragraph_start..*paragraph_end]
            .iter()
            .collect::<String>();
        if chapter_heading(&paragraph_text) {
            continue;
        }
        for (start, end) in sentence_ranges(text, *paragraph_start, *paragraph_end) {
            let reference = format!(
                "document-region:{source_revision_ref}:sentence:{sentence_count}:{start}-{end}"
            );
            regions.push(DocumentRegion {
                region_ref: reference,
                source_revision_ref: source_revision_ref.to_owned(),
                parent_region_ref: Some(paragraph_ref.clone()),
                kind: DocumentRegionKind::Sentence,
                start_char: start as u64,
                end_char: end as u64,
            });
            sentence_count += 1;
        }
    }

    let source = LongDocumentSource {
        ingest: SourceIngestEnvelope {
            source_ref: source_ref.to_owned(),
            source_revision_ref: source_revision_ref.to_owned(),
            provider_ref: provider_ref.to_owned(),
            family,
            role_class: IngestRoleClass::ContentSource,
            content_digest_ref: sha256_ref(text),
            acquisition_receipt_ref: acquisition_receipt_ref.to_owned(),
            media_type_ref: "text/plain".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        title,
        edition_ref,
        regions,
    };

    source
        .validate()
        .map_err(PlainTextDocumentAdapterError::InvalidSource)?;

    let receipt = PlainTextSegmentationReceipt {
        source_ref: source_ref.to_owned(),
        source_revision_ref: source_revision_ref.to_owned(),
        char_count: text.chars().count(),
        chapter_count: chapters.len(),
        paragraph_count: paragraphs.len(),
        sentence_count,
        structural_region_count: source.regions.len(),
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
        segmentation_claims_semantic_completeness: false,
    };

    Ok((source, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_book_text_becomes_reopenable_chapter_paragraph_sentence_regions() {
        let text = "# Chapter One\n\nAlice called Bob. Bob replied!\n\n# Chapter Two\n\nA final sentence.";
        let (source, receipt) = build_plain_text_long_document_source(
            "book:fixture",
            "book-revision:fixture",
            "plain-text",
            "receipt:book:fixture",
            Some("Fixture Book".into()),
            Some("edition:1".into()),
            text,
        )
        .unwrap();

        assert_eq!(receipt.chapter_count, 2);
        assert_eq!(receipt.paragraph_count, 4);
        assert_eq!(receipt.sentence_count, 3);
        assert_eq!(
            source
                .regions
                .iter()
                .filter(|region| region.kind == DocumentRegionKind::Sentence)
                .count(),
            3
        );
        assert!(source
            .regions
            .iter()
            .filter(|region| region.kind == DocumentRegionKind::Sentence)
            .all(|region| region.parent_region_ref.is_some()));
        assert!(!receipt.segmentation_claims_semantic_completeness);
        assert!(!receipt.creates_semantic_authority);
    }

    #[test]
    fn unicode_document_offsets_remain_character_coordinates() {
        let text = "αβγ. Delta.";
        let (source, receipt) = build_plain_text_long_document_source(
            "book:unicode",
            "revision:unicode",
            "plain-text",
            "receipt:unicode",
            None,
            None,
            text,
        )
        .unwrap();

        assert_eq!(receipt.char_count, text.chars().count());
        let first_sentence = source
            .regions
            .iter()
            .find(|region| region.kind == DocumentRegionKind::Sentence)
            .unwrap();
        assert_eq!(first_sentence.start_char, 0);
        assert_eq!(first_sentence.end_char, 4);
    }

    #[test]
    fn heading_repair_recovers_body_and_heading_without_renumbering() {
        let text = "Opening.\r\n\r\nChapter One\r\nαβ called Bob.\r\nHe replied!\r\n\r\nLater sentence.";
        let (old, _) = build_plain_text_long_document_source(
            "book:repair", "revision:repair", "plain", "receipt:repair",
            None, None, text,
        ).unwrap();
        let parent = old.regions.iter()
            .find(|r| r.kind == DocumentRegionKind::Paragraph
                && r.region_ref.contains(":paragraph:1:")).unwrap();
        let refs = vec![parent.region_ref.clone()];
        let (new, delta) = extend_plain_text_heading_exclusions(&old, text, &refs).unwrap();
        assert_eq!(delta.len(), 3);
        assert_eq!(&new.regions[..old.regions.len()], old.regions.as_slice());
        let chars = text.chars().collect::<Vec<_>>();
        let literals = delta.iter().map(|r|
            chars[r.start_char as usize..r.end_char as usize].iter().collect::<String>()
        ).collect::<Vec<_>>();
        assert_eq!(literals, vec!["Chapter One", "αβ called Bob.", "He replied!"]);
        assert!(delta.iter().all(|r| r.parent_region_ref.as_ref() == Some(&parent.region_ref)));
        let (again, replay) = extend_plain_text_heading_exclusions(&new, text, &refs).unwrap();
        assert!(replay.is_empty());
        assert_eq!(again, new);
        assert!(extend_plain_text_heading_exclusions(&old, "different", &refs).is_err());
    }

    #[test]
    fn heading_repair_preserves_standalone_and_unrelated_paragraphs() {
        let text = "# Standalone\n\nPart of the story.\nThis is prose.\n\nOrdinary paragraph.";
        let (old, _) = build_plain_text_long_document_source(
            "book:repair", "revision:repair", "plain", "receipt:repair",
            None, None, text,
        ).unwrap();
        let paragraphs = old.regions.iter().filter(|r| r.kind == DocumentRegionKind::Paragraph)
            .map(|r| r.region_ref.clone()).collect::<Vec<_>>();
        let (_, standalone) = extend_plain_text_heading_exclusions(&old, text, &paragraphs[..1]).unwrap();
        assert!(standalone.is_empty());
        let (_, prose) = extend_plain_text_heading_exclusions(&old, text, &paragraphs[1..2]).unwrap();
        assert_eq!(prose.len(), 2);
        let (same, unrelated) = extend_plain_text_heading_exclusions(&old, text, &paragraphs[2..]).unwrap();
        assert!(unrelated.is_empty());
        assert_eq!(same, old);
    }

    #[test]
    fn operational_family_cannot_enter_generic_long_text_compiler() {
        let result = build_plain_text_long_source_with_family(
            "calendar:fixture",
            "revision:calendar",
            "calendar-provider",
            "receipt:calendar",
            SourceFamily::Calendar,
            None,
            None,
            "Meeting with Alice on Monday.",
        );
        assert!(matches!(
            result,
            Err(PlainTextDocumentAdapterError::UnsupportedLongTextFamily)
        ));
    }

    #[test]
    fn structural_segmentation_does_not_require_chapter_markers() {
        let text = "First paragraph sentence.\n\nSecond paragraph.";
        let (_, receipt) = build_plain_text_long_document_source(
            "book:no-chapters",
            "revision:no-chapters",
            "plain-text",
            "receipt:no-chapters",
            None,
            None,
            text,
        )
        .unwrap();
        assert_eq!(receipt.chapter_count, 0);
        assert_eq!(receipt.paragraph_count, 2);
        assert_eq!(receipt.sentence_count, 2);
    }
}
