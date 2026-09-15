#[path = "../src/docx_text.rs"]
mod docx_text;

use docx_text::{extract_docx_canonical_judgment, extract_docx_canonical_text};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use zip::write::FileOptions;

fn main() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Cullen v New South Wales</w:t></w:r></w:p>
    <w:p><w:r><w:t>[2026] HCA 19</w:t></w:r></w:p>
    <w:p><w:r><w:t>positive operational act</w:t><w:tab/><w:t>duty of care</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let footnotes = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="-1"><w:p><w:r><w:t>separator</w:t></w:r></w:p></w:footnote>
  <w:footnote w:id="90"><w:p><w:r><w:t>Mallonland Pty Ltd v Advanta Seeds Pty Ltd (2024) 98 ALJR 956 at 978 [89]-[90]; 418 ALR 639 at 664-665.</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#;

    let cursor = Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    writer
        .start_file("word/document.xml", FileOptions::default())
        .expect("create DOCX document.xml member");
    writer.write_all(xml.as_bytes()).expect("write DOCX document.xml");
    writer
        .start_file("word/footnotes.xml", FileOptions::default())
        .expect("create DOCX footnotes.xml member");
    writer
        .write_all(footnotes.as_bytes())
        .expect("write DOCX footnotes.xml");
    let bytes = writer.finish().expect("finish DOCX fixture").into_inner();

    let materialized = extract_docx_canonical_text(&bytes).expect("extract canonical DOCX text");
    assert_eq!(materialized.paragraph_count, 3);
    assert_eq!(
        materialized.text,
        "Cullen v New South Wales\n[2026] HCA 19\npositive operational act\tduty of care\n"
    );
    let judgment = extract_docx_canonical_judgment(&bytes).expect("extract refined judgment");
    assert_eq!(judgment.body, materialized);
    assert_eq!(judgment.footnotes.len(), 1);
    assert_eq!(judgment.footnotes[0].footnote_id, "90");
    assert!(judgment.footnotes[0].text.contains("(2024) 98 ALJR 956"));
    let digest = format!("sha256:{:x}", Sha256::digest(materialized.text.as_bytes()));
    println!(
        "docx_text_materialization=PASS network_requests=0 paragraphs={} footnotes={} text_digest={} semantic_payment=false legal_authority=false",
        materialized.paragraph_count,
        judgment.footnotes.len(),
        digest
    );
}
