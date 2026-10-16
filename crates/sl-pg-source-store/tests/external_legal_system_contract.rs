use sensiblaw_pg_source_store::{
    actionstep_matter_binding, netdocuments_provider_identity, netdocuments_version_binding,
    ActionstepMatterBindingInput, ExternalLegalSystem, ExternalObjectKind,
    ExternalLegalSystemError, NetDocumentsVersionInput,
};

#[test]
fn actionstep_binding_keeps_actionstep_authoritative_and_does_not_promote_meaning() {
    let binding = actionstep_matter_binding(&ActionstepMatterBindingInput {
        tenant_ref: "actionstep-org:acme".into(),
        action_id: "12345".into(),
        observation_ref: "actionstep-observation:etag:abc".into(),
        matter_ref: "matter:12345".into(),
        user_ref: "actionstep-user:42".into(),
        access_context_ref: "actionstep-oauth:user:42".into(),
        acquisition_receipt_ref: "receipt:actionstep:12345".into(),
    })
    .expect("valid Actionstep binding");

    assert_eq!(binding.system, ExternalLegalSystem::Actionstep);
    assert_eq!(binding.object_kind, ExternalObjectKind::Matter);
    assert_eq!(binding.external_object_ref, "12345");
    assert_eq!(binding.canonical_matter_ref.as_deref(), Some("matter:12345"));
    assert!(binding.authoritative_externally);
    assert!(!binding.copied_as_system_of_record);
    assert_eq!(binding.principal_ref.as_deref(), Some("actionstep-user:42"));
    assert_eq!(
        binding.external_identity_receipt_ref.as_deref(),
        Some("actionstep-observation:etag:abc")
    );
    assert!(binding.candidate_only);
    assert!(!binding.creates_semantic_authority);
    assert!(!binding.creates_legal_authority);
    assert!(!binding.applicability_promoted);
    assert!(!binding.claim_truth_promoted);
}

fn netdocuments_input(version_id: &str, identity_receipt: &str) -> NetDocumentsVersionInput {
    NetDocumentsVersionInput {
        repository_ref: "nd-repository:au-law-firm".into(),
        document_id: "48291".into(),
        version_id: version_id.into(),
        version_identity_receipt_ref: identity_receipt.into(),
        citation: format!("NetDocuments 48291 {version_id}"),
        jurisdiction_ref: "AU-QLD".into(),
        source_url: Some(format!("https://example.invalid/nd/48291/{version_id}")),
        access_context_ref: "nd-user-access:abc".into(),
        acquisition_receipt_ref: format!("receipt:nd:48291:{version_id}"),
    }
}

#[test]
fn netdocuments_version_maps_to_existing_provider_materialisation_identity_only_with_version_receipt() {
    let input = netdocuments_input("17", "receipt:nd:immutable-version:48291:17");
    let identity = netdocuments_provider_identity(&input)
        .expect("provider-confirmed immutable NetDocuments version identity");

    assert_eq!(identity.provider_ref, "netdocuments");
    assert_eq!(identity.dataset_ref, "nd-repository:au-law-firm");
    assert_eq!(identity.dataset_revision_ref, "17");
    assert_eq!(identity.split_ref, "document_version");
    assert_eq!(identity.external_version_ref, "48291:17");
    assert_eq!(
        identity.source_ref,
        "netdocuments:nd-repository:au-law-firm:48291"
    );
    assert_eq!(identity.acquisition_receipt_ref, "receipt:nd:48291:17");

    let binding = netdocuments_version_binding(&input, "source:nd:48291:17")
        .expect("identity receipt is retained on the external binding");
    assert_eq!(
        binding.external_identity_receipt_ref.as_deref(),
        Some("receipt:nd:immutable-version:48291:17")
    );
}

#[test]
fn netdocuments_version_without_identity_receipt_fails_closed() {
    let error = netdocuments_provider_identity(&netdocuments_input("17", ""))
        .expect_err("version string shape alone cannot establish immutability");
    assert!(matches!(
        error,
        ExternalLegalSystemError::MissingNetDocumentsVersionIdentityReceipt
    ));
}

#[test]
fn netdocuments_mutable_alias_is_rejected_even_with_identity_receipt() {
    let error = netdocuments_provider_identity(&netdocuments_input(
        "latest",
        "receipt:nd:purported-version:latest",
    ))
    .expect_err("obvious mutable version aliases must fail closed");
    assert!(matches!(
        error,
        ExternalLegalSystemError::MutableNetDocumentsVersion
    ));
}
