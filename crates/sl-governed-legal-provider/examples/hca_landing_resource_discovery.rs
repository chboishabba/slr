#[path = "../src/official_resource.rs"]
mod official_resource;

use official_resource::{
    discover_hca_judgment_resources, preferred_hca_judgment_resource, JudgmentResourceKind,
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

fn main() {
    let input = PathBuf::from(
        std::env::var("SENSIBLAW_HCA_LANDING_HTML")
            .unwrap_or_else(|_| "fixtures/hca_cullen_landing_minimal.html".into()),
    );
    let output = PathBuf::from(
        std::env::var("SENSIBLAW_HCA_RESOURCE_RECEIPT")
            .unwrap_or_else(|_| "/tmp/hca-judgment-resource-discovery-v01.json".into()),
    );
    let body = fs::read(&input).expect("read locally persisted HCA landing HTML");
    let resources = discover_hca_judgment_resources(&body);
    assert_eq!(resources.len(), 2, "expected DOCX and PDF judgment resources");
    let preferred = preferred_hca_judgment_resource(&resources).expect("preferred HCA judgment resource");
    assert_eq!(preferred.kind, JudgmentResourceKind::Docx);

    let docx = resources
        .iter()
        .find(|resource| resource.kind == JudgmentResourceKind::Docx)
        .expect("DOCX resource");
    let pdf = resources
        .iter()
        .find(|resource| resource.kind == JudgmentResourceKind::Pdf)
        .expect("PDF resource");

    let receipt = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.official_judgment_resource_discovery.v0_1\",\n",
            "  \"authority\": \"experimental_candidate_only\",\n",
            "  \"provider\": \"HighCourtAustralia\",\n",
            "  \"source_identity_ref\": \"case:[2026]-HCA-19\",\n",
            "  \"medium_neutral_citation\": \"[2026] HCA 19\",\n",
            "  \"network_requests\": 0,\n",
            "  \"docx_reference\": \"{}\",\n",
            "  \"pdf_reference\": \"{}\",\n",
            "  \"preferred_reference\": \"{}\",\n",
            "  \"preferred_kind\": \"Docx\",\n",
            "  \"resource_discovery_claimed_semantic_payment\": false,\n",
            "  \"resource_discovery_claimed_legal_authority\": false\n",
            "}}\n"
        ),
        json_escape(&docx.reference),
        json_escape(&pdf.reference),
        json_escape(&preferred.reference),
    );
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("create resource receipt directory");
    }
    fs::write(&output, receipt).expect("write resource discovery receipt");
    println!(
        "hca_resource_discovery=PASS citation=[2026] HCA 19 resources=2 preferred=Docx network_requests=0 receipt={}",
        output.display()
    );
}
