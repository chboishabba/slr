//! Deterministic local DOCX -> canonical text materialization.
//!
//! This is a carrier transformation only.  It does not establish propositions,
//! holdings, ratio, authority, applicability, treatment, truth or proof payment.

use std::io::{Cursor, Read};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocxTextError {
    InvalidZip(String),
    MissingDocumentXml(String),
    InvalidDocumentXml(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDocxText {
    pub text: String,
    pub paragraph_count: u64,
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

    let normalized = out.replace("\r\n", "\n").replace('\r', "\n");
    Ok(CanonicalDocxText {
        text: normalized,
        paragraph_count,
    })
}

pub fn extract_docx_canonical_text(bytes: &[u8]) -> Result<CanonicalDocxText, DocxTextError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|err| DocxTextError::InvalidZip(err.to_string()))?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|err| DocxTextError::MissingDocumentXml(err.to_string()))?;
    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .map_err(|err| DocxTextError::InvalidDocumentXml(err.to_string()))?;
    document_xml_to_canonical_text(&xml)
}

pub const fn docx_text_materialization_is_semantic_payment() -> bool { false }
pub const fn docx_text_materialization_is_legal_authority() -> bool { false }

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
        assert!(!docx_text_materialization_is_semantic_payment());
        assert!(!docx_text_materialization_is_legal_authority());
    }
}
