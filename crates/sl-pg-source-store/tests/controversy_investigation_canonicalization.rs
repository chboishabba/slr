use sensiblaw_pg_source_store::{
    canonicalize_controversy_acquisition_routes, AccessDisposition,
    AcquisitionRouteCandidate, ControversyInvestigationError, DuplicateRelation,
    EvidenceIndependence,
};

fn route(id: &str) -> AcquisitionRouteCandidate {
    AcquisitionRouteCandidate {
        route_ref: id.into(),
        obligation_ref: "controversy-acquisition:1".into(),
        route_description: format!("route {id}"),
        source_locator_ref: format!("locator:{id}"),
        access_disposition: AccessDisposition::Public,
        authority_receipt_ref: None,
        provenance_genealogy_ref: format!("genealogy:{id}"),
        independence: EvidenceIndependence::Unknown,
        independence_receipt_ref: None,
        duplicate_relation: DuplicateRelation::Unknown,
        information_gain: 1,
        dependency_closure_impact: 1,
        residual_coverage: 1,
        provenance_novelty: 1,
        acquisition_cost: 1,
        axis_estimation_receipt_ref: "axis:reviewed".into(),
        candidate_only: true,
        creates_acquisition_authority: false,
    }
}

#[test]
fn route_discovery_order_is_not_part_of_immutable_identity() {
    let canonical = canonicalize_controversy_acquisition_routes(&[
        route("route:z"),
        route("route:a"),
        route("route:m"),
    ])
    .expect("distinct durable route refs canonicalize");

    assert_eq!(
        canonical
            .iter()
            .map(|candidate| candidate.route_ref.as_str())
            .collect::<Vec<_>>(),
        vec!["route:a", "route:m", "route:z"]
    );
}

#[test]
fn duplicate_durable_route_identity_fails_closed() {
    let error = canonicalize_controversy_acquisition_routes(&[
        route("route:a"),
        route("route:a"),
    ])
    .expect_err("duplicate route refs cannot be silently merged");

    assert_eq!(error, ControversyInvestigationError::InvalidIdentity);
}
