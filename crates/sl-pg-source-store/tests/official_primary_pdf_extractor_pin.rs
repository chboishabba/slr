use sensiblaw_pg_source_store::{
    validate_official_primary_extraction_receipt, OfficialPrimaryExtractionError,
    OfficialPrimaryExtractionReceipt,
};

fn receipt() -> OfficialPrimaryExtractionReceipt {
    OfficialPrimaryExtractionReceipt {
        schema: "sensiblaw-official-primary-extraction/v1".into(),
        citation: "Smith v Chief Executive, Queensland Corrective Services [2026] HCA 37".into(),
        source_provider_ref: "provider:official-hca-pdf".into(),
        source_url: "https://www.hcourt.gov.au/example.pdf".into(),
        source_media_type: "application/pdf".into(),
        source_sha256_hex: "a".repeat(64),
        source_size_bytes: 123,
        extractor_repo: "https://github.com/isaacus-dev/open-australian-legal-corpus-creator".into(),
        extractor_revision: "dbdb9e3752394a87a9c966c9a9199f9685db9af2".into(),
        extractor_path: "src/oalc_creator/ocr.py".into(),
        extractor_function: "pdf2txt".into(),
        extractor_license: "MIT".into(),
        parameters: serde_json::json!({"pdf_scale": 2}),
        output_sha256_hex: "b".repeat(64),
        output_size_bytes: 456,
        authority: "source-observation-only".into(),
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn pdf_receipt_rejects_a_different_upstream_extractor_path() {
    let mut value = receipt();
    value.extractor_path = "src/oalc_creator/not_the_pdf_extractor.py".into();
    assert!(matches!(
        validate_official_primary_extraction_receipt(&value),
        Err(OfficialPrimaryExtractionError::WrongExtractor)
    ));
}
