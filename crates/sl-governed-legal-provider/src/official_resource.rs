//! Official-court landing-page resource discovery.
//!
//! This module is intentionally acquisition-only.  It turns already-local
//! official landing-page HTML into typed document references; it does not parse
//! holdings, propositions, treatment, authority, applicability or truth.

pub mod nsw_legislation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JudgmentResourceKind {
    Docx,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JudgmentResourceReference {
    pub kind: JudgmentResourceKind,
    pub reference: String,
    pub media_type: &'static str,
}

fn absolutize_hca(href: &str) -> Option<String> {
    if href.starts_with("https://www.hcourt.gov.au/") || href.starts_with("https://hcourt.gov.au/") {
        return Some(href.to_string());
    }
    href.starts_with('/').then(|| format!("https://www.hcourt.gov.au{href}"))
}

fn classify_hca_resource(href: &str) -> Option<(JudgmentResourceKind, &'static str)> {
    let lower = href.to_ascii_lowercase();
    if lower.ends_with(".docx") {
        return Some((
            JudgmentResourceKind::Docx,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ));
    }
    if lower.ends_with(".pdf") {
        return Some((JudgmentResourceKind::Pdf, "application/pdf"));
    }
    None
}

pub fn discover_hca_judgment_resources(body: &[u8]) -> Vec<JudgmentResourceReference> {
    let html = String::from_utf8_lossy(body);
    let mut resources = Vec::new();
    for marker in ["href=\"", "href='"] {
        let mut rest = html.as_ref();
        while let Some(start) = rest.find(marker) {
            rest = &rest[start + marker.len()..];
            let quote = if marker.ends_with('"') { '"' } else { '\'' };
            let Some(end) = rest.find(quote) else { break };
            let href = &rest[..end];
            if let (Some(reference), Some((kind, media_type))) =
                (absolutize_hca(href), classify_hca_resource(href))
            {
                let resource = JudgmentResourceReference { kind, reference, media_type };
                if !resources.contains(&resource) {
                    resources.push(resource);
                }
            }
            rest = &rest[end + 1..];
        }
    }
    resources.sort();
    resources
}

pub fn preferred_hca_judgment_resource(
    resources: &[JudgmentResourceReference],
) -> Option<&JudgmentResourceReference> {
    resources
        .iter()
        .find(|resource| resource.kind == JudgmentResourceKind::Docx)
        .or_else(|| resources.iter().find(|resource| resource.kind == JudgmentResourceKind::Pdf))
}

#[allow(dead_code)]
pub const fn resource_discovery_is_semantic_payment() -> bool { false }
#[allow(dead_code)]
pub const fn resource_discovery_is_legal_authority() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    const CULLEN_FIXTURE: &[u8] = br#"
      <span class="citation">[2026] HCA 19</span>
      <a href="/sites/default/files/eresources/2026-06-17/HCA/Cullen%20v%20New%20South%20Wales%20%28S47-2025%29%20%5B2026%5D%20HCA%2019.docx"
         type="application/vnd.openxmlformats-officedocument.wordprocessingml.document">DOCX</a>
      <a href="/sites/default/files/eresources/2026-06-17/HCA/Cullen%20v%20New%20South%20Wales%20%28S47-2025%29%20%5B2026%5D%20HCA%2019.pdf"
         type="application/pdf">PDF</a>
    "#;

    #[test]
    fn cullen_landing_yields_docx_and_pdf_and_prefers_docx() {
        let resources = discover_hca_judgment_resources(CULLEN_FIXTURE);
        assert_eq!(resources.len(), 2);
        assert!(resources.iter().any(|r| r.kind == JudgmentResourceKind::Docx));
        assert!(resources.iter().any(|r| r.kind == JudgmentResourceKind::Pdf));
        let preferred = preferred_hca_judgment_resource(&resources).unwrap();
        assert_eq!(preferred.kind, JudgmentResourceKind::Docx);
        assert!(preferred.reference.starts_with("https://www.hcourt.gov.au/"));
        assert!(!resource_discovery_is_semantic_payment());
        assert!(!resource_discovery_is_legal_authority());
    }
}
