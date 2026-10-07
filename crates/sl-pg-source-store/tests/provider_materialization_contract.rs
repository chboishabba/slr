use sensiblaw_pg_source_store::{
    canonical_provider_materialization_ref, canonical_sha256_hex,
    validate_provider_pin, verify_provider_rematerialization,
    ProviderCandidatePnfError, ProviderCandidatePnfReceipt,
    ProviderMaterializationIdentity, ProviderMaterializationPolicy,
};

fn identity() -> ProviderMaterializationIdentity {
    ProviderMaterializationIdentity {
        provider_ref: "provider:oalc-hf".into(),
        dataset_ref: "isaacus/open-australian-legal-corpus".into(),
        dataset_revision_ref: "4f4a1e12d6b73bd77a45b9cc71c4ebd907edb922".into(),
        split_ref: "train".into(),
        external_version_ref: "case:[1992] HCA 23".into(),
        citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
        source_ref: "oalc:case:[1992]-HCA-23".into(),
        jurisdiction_ref: "AU".into(),
        source_url: Some("hf://datasets/isaacus/open-australian-legal-corpus".into()),
        acquisition_receipt_ref: "receipt:oalc:fixture".into(),
    }
}

#[test]
fn provider_pin_requires_an_immutable_revision() {
    for mutable in ["", "main", "master", "latest", "HEAD", "refs/heads/main"] {
        let mut value = identity();
        value.dataset_revision_ref = mutable.into();
        assert!(validate_provider_pin(&value).is_err(), "accepted mutable pin {mutable:?}");
    }

    assert!(validate_provider_pin(&identity()).is_ok());
}

#[test]
fn oalc_hf_pin_requires_a_commit_like_revision_not_an_arbitrary_branch_name() {
    for mutable_or_unverifiable in ["feature/provider-pin", "release-2026", "v1.2.3", "refs/tags/current"] {
        let mut value = identity();
        value.dataset_revision_ref = mutable_or_unverifiable.into();
        assert!(
            validate_provider_pin(&value).is_err(),
            "accepted non-commit OALC/HF pin {mutable_or_unverifiable:?}"
        );
    }

    let mut sha256 = identity();
    sha256.dataset_revision_ref =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into();
    assert!(validate_provider_pin(&sha256).is_ok());
}

#[test]
fn materialization_identity_is_content_addressed_but_provider_pinned() {
    let value = identity();
    let digest = canonical_sha256_hex("exact provider bytes");
    let a = canonical_provider_materialization_ref(&value, &digest).unwrap();
    let b = canonical_provider_materialization_ref(&value, &digest).unwrap();
    assert_eq!(a, b);

    let mut other_revision = value.clone();
    other_revision.dataset_revision_ref = "7b799e2c241ce91a5b59b7fe9b1d65530aeab111".into();
    let c = canonical_provider_materialization_ref(&other_revision, &digest).unwrap();
    assert_ne!(a, c);
}

#[test]
fn rematerialization_must_reproduce_the_exact_digest() {
    let expected = canonical_sha256_hex("pinned bytes");
    assert!(verify_provider_rematerialization(&expected, "pinned bytes").is_ok());
    assert!(verify_provider_rematerialization(&expected, "silently changed bytes").is_err());
}

#[test]
fn default_policy_is_ephemeral_and_non_promoting() {
    let policy = ProviderMaterializationPolicy::default();
    assert!(policy.allow_byte_eviction);
    assert!(!policy.retain_full_text_by_default);
    assert!(!policy.creates_semantic_authority);
    assert!(!policy.creates_legal_authority);
    assert!(!policy.promotes_applicability);
    assert!(!policy.promotes_claim_truth);
}

#[test]
fn provider_candidate_pnf_public_contract_remains_non_promoting() {
    let receipt = ProviderCandidatePnfReceipt {
        materialization_ref: "provider-materialization:fixture".into(),
        source_slice_ref: "source-slice:fixture".into(),
        legal_source_revision_ref: "external-source-revision:fixture".into(),
        statement_ref: "statement:fixture".into(),
        candidate_batch_ref: "candidate-pnf-batch:fixture".into(),
        parser_receipt_ref: "parser:fixture".into(),
        candidate_factor_count: 1,
        byte_coordinate_contract: true,
        candidate_only: true,
        creates_semantic_authority: false,
        proposition_support_paid: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    assert!(receipt.byte_coordinate_contract);
    assert!(receipt.candidate_only);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.proposition_support_paid);
    assert!(!receipt.applicability_promoted);
    assert!(!receipt.claim_truth_promoted);

    assert_eq!(
        ProviderCandidatePnfError::BytesNotResident.to_string(),
        "provider bytes are evicted; candidate-PNF compilation requires verified resident bytes"
    );
}
