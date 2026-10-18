use std::collections::BTreeSet;

use sensiblaw_core::canonical_evidence::{
    ConsumerAdequacyReceipt, ConsumerInformativenessReceipt, MatterPersona,
    MatterRepresentationProjection, SameMatterInformativenessReceipt,
};
use sensiblaw_pg_source_store::{
    load_database_config, load_same_matter_informativeness,
    persist_reviewed_informativeness_factorisation,
    persist_reviewed_representation_factorisation, persist_same_matter_informativeness,
    ReviewedInformativenessFactorisation, ReviewedRepresentationFactorisation,
};

fn env_ref(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} must identify an already-reviewed/persisted Smith coordinate"))
}

fn distinctions() -> BTreeSet<String> {
    [
        "smith:distinction:purpose-connected-v-direct-assistance",
        "smith:distinction:joint-reasons-v-reported-party-submission",
        "smith:distinction:meaning-v-severance-partial-disapplication",
        "smith:distinction:opinion-level-provenance",
        "smith:distinction:open-residual-v-decided-merits",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[test]
#[ignore = "requires live reviewed Smith Matter plus explicit reviewed representation-factorisation decisions"]
fn live_smith_accessible_surface_can_be_replayed_as_at_least_as_informative_for_affected_person() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let matter_ref = env_ref("ITIR_SMITH_HCA37_MATTER_REF");
    let controversy_ref = env_ref("ITIR_SMITH_HCA37_CONTROVERSY_REF");
    let consumer_ref = "consumer:smith:affected-person".to_owned();
    let query_family_ref = "query-family:understand-live-smith-interpretive-controversy".to_owned();
    let required = distinctions();

    let accessible_factor_ref = env_ref("ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REF");
    let full_factor_ref = env_ref("ITIR_SMITH_HCA37_FULL_FACTORISATION_REF");
    let accessible_admission_ref = env_ref("ITIR_SMITH_HCA37_ACCESSIBLE_ADMISSION_REF");
    let full_admission_ref = env_ref("ITIR_SMITH_HCA37_FULL_ADMISSION_REF");
    let accessible_review_ref = env_ref("ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REVIEW_REF");
    let full_review_ref = env_ref("ITIR_SMITH_HCA37_FULL_FACTORISATION_REVIEW_REF");
    let comparison_review_ref = env_ref("ITIR_SMITH_HCA37_INFORMATIVENESS_REVIEW_REF");
    let reviewer_ref = env_ref("ITIR_SMITH_HCA37_REPRESENTATION_REVIEWER_REF");
    let accessible_representation_ref = env_ref("ITIR_SMITH_HCA37_ACCESSIBLE_REPRESENTATION_REF");
    let full_representation_ref = env_ref("ITIR_SMITH_HCA37_FULL_REPRESENTATION_REF");

    let accessible_factor = ReviewedRepresentationFactorisation {
        factorisation_receipt_ref: accessible_factor_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        representation_ref: accessible_representation_ref.clone(),
        consumer_ref: consumer_ref.clone(),
        query_family_ref: query_family_ref.clone(),
        admissibility_receipt_ref: accessible_admission_ref.clone(),
        preserved_distinction_refs: required.clone(),
        reviewer_ref: reviewer_ref.clone(),
        review_decision_ref: accessible_review_ref,
        accepted: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        determines_credibility: false,
        enters_final_judgment: false,
        promotes_claim_truth: false,
    };
    let full_factor = ReviewedRepresentationFactorisation {
        factorisation_receipt_ref: full_factor_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        representation_ref: full_representation_ref.clone(),
        consumer_ref: consumer_ref.clone(),
        query_family_ref: query_family_ref.clone(),
        admissibility_receipt_ref: full_admission_ref.clone(),
        preserved_distinction_refs: required.clone(),
        reviewer_ref: reviewer_ref.clone(),
        review_decision_ref: full_review_ref,
        accepted: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        determines_credibility: false,
        enters_final_judgment: false,
        promotes_claim_truth: false,
    };
    persist_reviewed_representation_factorisation(&config, &accessible_factor)
        .expect("accessible Smith factorisation must persist/replay exactly");
    persist_reviewed_representation_factorisation(&config, &full_factor)
        .expect("full Smith factorisation must persist/replay exactly");

    let reviewed_comparison_ref = env_ref("ITIR_SMITH_HCA37_REVIEWED_INFORMATIVENESS_FACTORISATION_REF");
    let reviewed_comparison = ReviewedInformativenessFactorisation {
        reviewed_factorisation_receipt_ref: reviewed_comparison_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        left_factorisation_receipt_ref: accessible_factor_ref.clone(),
        right_factorisation_receipt_ref: full_factor_ref.clone(),
        consumer_ref: consumer_ref.clone(),
        query_family_ref: query_family_ref.clone(),
        right_preserved_distinction_refs: required.clone(),
        left_preserved_distinction_refs: required.clone(),
        reviewer_ref,
        review_decision_ref: comparison_review_ref,
        accepted: true,
        candidate_only: true,
        creates_universal_superiority: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        determines_credibility: false,
        enters_final_judgment: false,
        promotes_claim_truth: false,
    };
    persist_reviewed_informativeness_factorisation(&config, &reviewed_comparison)
        .expect("reviewed Smith informativeness comparison must persist/replay exactly");

    let accessible_adequacy = ConsumerAdequacyReceipt::new(
        accessible_representation_ref.clone(),
        consumer_ref.clone(),
        query_family_ref.clone(),
        accessible_admission_ref,
        [accessible_factor_ref],
        true,
        true,
        None,
    )
    .unwrap();
    let full_adequacy = ConsumerAdequacyReceipt::new(
        full_representation_ref.clone(),
        consumer_ref,
        query_family_ref,
        full_admission_ref,
        [full_factor_ref],
        true,
        true,
        None,
    )
    .unwrap();

    let accessible = MatterRepresentationProjection::new(
        matter_ref.clone(),
        controversy_ref.clone(),
        MatterPersona::AffectedPerson,
        accessible_representation_ref,
        accessible_adequacy.clone(),
        env_ref("ITIR_SMITH_HCA37_ACCESSIBLE_PROJECTION_RECEIPT_REF"),
    )
    .unwrap();
    let full = MatterRepresentationProjection::new(
        matter_ref.clone(),
        controversy_ref.clone(),
        MatterPersona::SolicitorCounsel,
        full_representation_ref,
        full_adequacy.clone(),
        env_ref("ITIR_SMITH_HCA37_FULL_PROJECTION_RECEIPT_REF"),
    )
    .unwrap();
    let informativeness = ConsumerInformativenessReceipt::new(
        accessible_adequacy,
        full_adequacy,
        required.clone(),
        required,
        reviewed_comparison_ref,
    )
    .unwrap();
    let receipt_ref = env_ref("ITIR_SMITH_HCA37_SAME_MATTER_INFORMATIVENESS_RECEIPT_REF");
    let receipt = SameMatterInformativenessReceipt::new(
        accessible,
        full,
        informativeness,
        receipt_ref.clone(),
    )
    .unwrap();

    let persisted = persist_same_matter_informativeness(&config, &receipt)
        .expect("Smith same-Matter informativeness must persist/replay exactly");
    let replayed = load_same_matter_informativeness(&config, &receipt_ref)
        .expect("Smith same-Matter informativeness must reopen exactly");
    assert_eq!(persisted, replayed);
    assert_eq!(replayed.matter_ref(), matter_ref);
    assert_eq!(replayed.controversy_ref(), controversy_ref);
    assert!(replayed.informativeness.left_at_least_as_informative());
    assert!(!replayed.creates_new_matter);
    assert!(!replayed.creates_semantic_authority);
    assert!(!replayed.determines_credibility);
    assert!(!replayed.enters_final_judgment);
}
