//! Deterministic local DOCX -> canonical text materialization.
//!
//! This is a carrier transformation only.  It does not establish propositions,
//! holdings, ratio, authority, applicability, treatment, truth or proof payment.
//! The refined judgment observer preserves body text and DOCX footnote bodies as
//! separate, stable carriers because legal authority identity commonly lives in
//! footnotes even when the body contains only a short case name.

use std::io::{Cursor, Read};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocxTextError {
    InvalidZip(String),
    MissingDocumentXml(String),
    InvalidDocumentXml(String),
    InvalidFootnotesXml(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDocxText {
    pub text: String,
    pub paragraph_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDocxFootnote {
    pub footnote_id: String,
    pub text: String,
    pub paragraph_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDocxJudgment {
    pub body: CanonicalDocxText,
    pub footnotes: Vec<CanonicalDocxFootnote>,
}

fn normalize_text(out: String) -> String {
    out.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn document_xml_to_canonical_text(xml: &str) -> Result<CanonicalDocxText, DocxTextError> {
    let parser = EventReader::new(xml.as_bytes());
    let mut out = String::new();
    let mut in_text = false;
    let mut paragraph_count = 0_u64;

    for event in parser {
        match event.map_err(|err| DocxTextError::InvalidDocumentXml(err.to_string()))? {
            XmlEvent::StartElement { name, .. } => match name.local_name.as_str() {
                "t" => in_text = true,
                "tab" => out.push('\t'),
                "br" | "cr" => out.push('\n'),
                _ => {}
            },
            XmlEvent::EndElement { name } => match name.local_name.as_str() {
                "t" => in_text = false,
                "p" => {
                    paragraph_count += 1;
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
                _ => {}
            },
            XmlEvent::Characters(text) | XmlEvent::CData(text) if in_text => out.push_str(&text),
            _ => {}
        }
    }

    Ok(CanonicalDocxText {
        text: normalize_text(out),
        paragraph_count,
    })
}

pub fn footnotes_xml_to_canonical_footnotes(
    xml: &str,
) -> Result<Vec<CanonicalDocxFootnote>, DocxTextError> {
    let parser = EventReader::new(xml.as_bytes());
    let mut footnotes = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_text = String::new();
    let mut current_paragraphs = 0_u64;
    let mut in_text = false;

    for event in parser {
        match event.map_err(|err| DocxTextError::InvalidFootnotesXml(err.to_string()))? {
            XmlEvent::StartElement { name, attributes, .. } => match name.local_name.as_str() {
                "footnote" => {
                    current_id = attributes
                        .iter()
                        .find(|attr| attr.name.local_name == "id")
                        .map(|attr| attr.value.clone());
                    current_text.clear();
                    current_paragraphs = 0;
                }
                "t" if current_id.is_some() => in_text = true,
                "tab" if current_id.is_some() => current_text.push('\t'),
                "br" | "cr" if current_id.is_some() => current_text.push('\n'),
                _ => {}
            },
            XmlEvent::EndElement { name } => match name.local_name.as_str() {
                "t" => in_text = false,
                "p" if current_id.is_some() => {
                    current_paragraphs += 1;
                    if !current_text.ends_with('\n') {
                        current_text.push('\n');
                    }
                }
                "footnote" => {
                    if let Some(id) = current_id.take() {
                        let is_material = id.parse::<i64>().map_or(true, |value| value > 0);
                        let text = normalize_text(current_text.clone());
                        if is_material && !text.trim().is_empty() {
                            footnotes.push(CanonicalDocxFootnote {
                                footnote_id: id,
                                text,
                                paragraph_count: current_paragraphs,
                            });
                        }
                    }
                }
                _ => {}
            },
            XmlEvent::Characters(text) | XmlEvent::CData(text) if in_text => {
                current_text.push_str(&text);
            }
            _ => {}
        }
    }

    footnotes.sort_by_key(|footnote| footnote.footnote_id.parse::<i64>().unwrap_or(i64::MAX));
    Ok(footnotes)
}

#[allow(dead_code)]
pub fn extract_docx_canonical_text(bytes: &[u8]) -> Result<CanonicalDocxText, DocxTextError> {
    Ok(extract_docx_canonical_judgment(bytes)?.body)
}

pub fn extract_docx_canonical_judgment(
    bytes: &[u8],
) -> Result<CanonicalDocxJudgment, DocxTextError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|err| DocxTextError::InvalidZip(err.to_string()))?;

    let body = {
        let mut document = archive
            .by_name("word/document.xml")
            .map_err(|err| DocxTextError::MissingDocumentXml(err.to_string()))?;
        let mut xml = String::new();
        document
            .read_to_string(&mut xml)
            .map_err(|err| DocxTextError::InvalidDocumentXml(err.to_string()))?;
        document_xml_to_canonical_text(&xml)?
    };

    let footnotes = match archive.by_name("word/footnotes.xml") {
        Ok(mut member) => {
            let mut xml = String::new();
            member
                .read_to_string(&mut xml)
                .map_err(|err| DocxTextError::InvalidFootnotesXml(err.to_string()))?;
            footnotes_xml_to_canonical_footnotes(&xml)?
        }
        Err(_) => Vec::new(),
    };

    Ok(CanonicalDocxJudgment { body, footnotes })
}

#[allow(dead_code)]
pub const fn docx_text_materialization_is_semantic_payment() -> bool { false }
#[allow(dead_code)]
pub const fn docx_text_materialization_is_legal_authority() -> bool { false }
#[allow(dead_code)]
pub const fn footnote_observation_is_citation_treatment() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_xml_preserves_text_paragraphs_tabs_and_breaks_deterministically() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Cullen v New South Wales</w:t></w:r></w:p>
    <w:p><w:r><w:t>[2026] HCA 19</w:t><w:tab/><w:t>positive operational act</w:t><w:br/><w:t>duty of care</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let materialized = document_xml_to_canonical_text(xml).unwrap();
        assert_eq!(materialized.paragraph_count, 2);
        assert_eq!(
            materialized.text,
            "Cullen v New South Wales\n[2026] HCA 19\tpositive operational act\nduty of care\n"
        );
    }

    #[test]
    fn footnotes_are_preserved_as_separate_locator_ready_carriers() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="-1"><w:p><w:r><w:t>separator</w:t></w:r></w:p></w:footnote>
  <w:footnote w:id="90"><w:p><w:r><w:t>Mallonland Pty Ltd v Advanta Seeds Pty Ltd (2024) 98 ALJR 956 at 978 [89]-[90]; 418 ALR 639 at 664-665.</w:t></w:r></w:p></w:footnote>
  <w:footnote w:id="113"><w:p><w:r><w:t>Robinson v Chief Constable of West Yorkshire Police [2018] AC 736 at 742 [13].</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#;
        let footnotes = footnotes_xml_to_canonical_footnotes(xml).unwrap();
        assert_eq!(footnotes.len(), 2);
        assert_eq!(footnotes[0].footnote_id, "90");
        assert!(footnotes[0].text.contains("98 ALJR 956"));
        assert_eq!(footnotes[1].footnote_id, "113");
        assert!(!footnote_observation_is_citation_treatment());
        assert!(!docx_text_materialization_is_semantic_payment());
        assert!(!docx_text_materialization_is_legal_authority());
    }
}
