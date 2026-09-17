use sensiblaw_pg_source_store::{
    collapse_discovery_identity_baseline, DiscoveryIdentityBaselineError, DiscoveryIdentityBaselineRow,
};

#[test]
fn baseline_counts_unique_identity_classes_not_lineage_rows() {
    let baseline = collapse_discovery_identity_baseline(&[
        DiscoveryIdentityBaselineRow {
            object_ref: "Q975866".into(),
            identity_class_ref: "world-object:eddie-mabo".into(),
        },
        DiscoveryIdentityBaselineRow {
            object_ref: "https://en.wikipedia.org/wiki/Eddie_Mabo".into(),
            identity_class_ref: "world-object:eddie-mabo".into(),
        },
        DiscoveryIdentityBaselineRow {
            object_ref: "case:[1992]-HCA-23".into(),
            identity_class_ref: "world-object:mabo-case-1992-hca-23".into(),
        },
    ])
    .unwrap();

    assert_eq!(baseline.identity_class_refs.len(), 2);
    assert_eq!(
        baseline.representation_identity_class_refs.get("Q975866").map(String::as_str),
        Some("world-object:eddie-mabo")
    );
    assert_eq!(
        baseline.representation_identity_class_refs
            .get("https://en.wikipedia.org/wiki/Eddie_Mabo")
            .map(String::as_str),
        Some("world-object:eddie-mabo")
    );
}

#[test]
fn conflicting_durable_representation_identity_fails_closed() {
    let error = collapse_discovery_identity_baseline(&[
        DiscoveryIdentityBaselineRow {
            object_ref: "Q975866".into(),
            identity_class_ref: "world-object:eddie-mabo".into(),
        },
        DiscoveryIdentityBaselineRow {
            object_ref: "Q975866".into(),
            identity_class_ref: "world-object:incorrect-second-class".into(),
        },
    ])
    .unwrap_err();

    assert_eq!(
        error,
        DiscoveryIdentityBaselineError::RepresentationIdentityConflict {
            object_ref: "Q975866".into(),
            existing_identity_class_ref: "world-object:eddie-mabo".into(),
            conflicting_identity_class_ref: "world-object:incorrect-second-class".into(),
        }
    );
}
