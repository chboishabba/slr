use crate::{
    select_adaptive_cone, AdaptiveConePolicy, AdaptiveExplanationCone, ConeCandidate,
    CoordinateCoverage, ExplanationCone, SemanticNodeKind,
};

fn paid_candidates(
    coverage: &CoordinateCoverage,
    kind: SemanticNodeKind,
    out: &mut Vec<ConeCandidate>,
) {
    match coverage {
        CoordinateCoverage::Paid { evidence_refs } => {
            out.extend(
                evidence_refs
                    .iter()
                    .map(|reference| ConeCandidate::new(reference.clone(), 1, 100, kind).mandatory()),
            );
        }
        CoordinateCoverage::Residualised(residual_ref) => {
            let mut candidate = ConeCandidate::new(
                residual_ref.as_str().to_owned(),
                1,
                100,
                SemanticNodeKind::Residual,
            )
            .mandatory();
            candidate.residual_ref = Some(residual_ref.as_str().to_owned());
            out.push(candidate);
        }
    }
}

/// Convert a paid bounded explanation into mandatory adaptive-cone candidates,
/// then merge optional world/context candidates under the normal reader budget.
/// Source/support/residual coordinates therefore cannot be displaced by merely
/// interesting context.
#[must_use]
pub fn select_from_explanation_cone(
    cone: &ExplanationCone,
    extras: &[ConeCandidate],
    policy: AdaptiveConePolicy,
) -> AdaptiveExplanationCone {
    let mut candidates = Vec::new();

    let mut source = ConeCandidate::new(
        cone.source.span_ref().as_str().to_owned(),
        0,
        100,
        SemanticNodeKind::Source,
    )
    .mandatory();
    source
        .provenance_refs
        .push(cone.source.source_revision_ref().as_str().to_owned());
    source
        .source_span_refs
        .push(cone.source.span_ref().as_str().to_owned());
    candidates.push(source);

    candidates.extend(
        cone.support_refs
            .iter()
            .map(|reference| {
                ConeCandidate::new(reference.clone(), 1, 100, SemanticNodeKind::Support).mandatory()
            }),
    );
    paid_candidates(&cone.qualifier, SemanticNodeKind::Qualifier, &mut candidates);
    paid_candidates(&cone.defeater, SemanticNodeKind::Defeater, &mut candidates);
    paid_candidates(&cone.comparator, SemanticNodeKind::Comparator, &mut candidates);
    candidates.extend_from_slice(extras);

    select_adaptive_cone(cone.proposition_ref.as_str(), &candidates, policy)
}
