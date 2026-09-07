#[path = "../src/docx_text.rs"]
mod docx_text;

use docx_text::extract_docx_canonical_text;
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

    let cursor = Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    writer
        .start_file("word/document.xml", FileOptions::default())
        .expect("create DOCX document.xml member");
    writer.write_all(xml.as_bytes()).expect("write DOCX document.xml");
    let bytes = writer.finish().expect("finish DOCX fixture").into_inner();

    let materialized = extract_docx_canonical_text(&bytes).expect("extract canonical DOCX text");
    assert_eq!(materialized.paragraph_count, 3);
    assert_eq!(
        materialized.text,
        "Cullen v New South Wales\n[2026] HCA 19\npositive operational act\tduty of care\n"
    );
    let digest = format!("sha256:{:x}", Sha256::digest(materialized.text.as_bytes()));
    println!(
        "docx_text_materialization=PASS network_requests=0 paragraphs={} text_digest={} semantic_payment=false legal_authority=false",
        materialized.paragraph_count, digest
    );
}
