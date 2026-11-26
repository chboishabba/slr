//! P0 source-relative consumer, not another antecedent parser.
//!
//! Reuses exact original SLR source, native M12 parser frontier and the
//! versioned named-topic inference. Returns a *consumer usable* answer and
//! all provenance, with a read-only replay/revocation contract.
//!
//! Policy-derived local linguistic interpretation is not a uniquely
//! resolved typed PNF object, an accepted numeric identity witness,
//! world QID, source proposition truth, or legal applicability.
//! Native S29 Pending does NOT prevent use of this scoped answer.
//!
//! Never writes, silently repairs, or synthesizes an identity admission.
//! The canonical original SensibLaw identity owner (migrations 072-080)
//! must later issue its own uniquely resolved typed-demand witness.

use crate::{
    derive_native_named_topic, NativeAntecedentFrontier,
    NativeAntecedentReviewInspection, NamedTopicMention, NativeNamedTopicError,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeSourceLocalAnswer {
    pub schema: &'static str,
    pub consumer_scope_ref: &'static str,
    pub answer_receipt_ref: String,
    pub source_document_ref: String,
    pub source_revision_ref: String,
    pub source_content_sha256: String,
    pub original_focus_mention_ref: String,
    pub original_focus_statement_ref: String,
    pub original_focus_region_ref: String,
    pub original_focus_span: [u64; 2],
    pub original_focus_surface: String,
    pub original_selected_antecedent_statement_ref: String,
    pub original_selected_antecedent_region_ref: String,
    pub original_selected_antecedent_span: [u64; 2],
    pub original_selected_antecedent_surface: String,
    pub original_parser_run_ref: String,
    pub original_candidate_set_ref: String,
    pub original_review_item_ref: String,
    pub original_s29_status: String,
    pub original_policy_ref: String,
    pub original_derivation_ref: String,
    pub original_person_support: Vec<NamedTopicMention>,
    pub answer_kind: &'static str,
    pub answer_value: Option<String>,
    pub source_local_policy_inference_paid: bool,
    pub typed_unique_identity_witness_paid: bool,
    pub governed_semantic_identity_paid: bool,
    pub level_three_substitution_paid: bool,
    pub external_qid_paid: bool,
    pub historical_truth_paid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeSourceAnswerReplay {
    pub schema: &'static str,
    pub result: &'static str,
    pub current: NativeSourceLocalAnswer,
    pub prior_receipt_ref: Option<String>,
    pub original_level_one_source_preserved: bool,
    pub previous_interpretation_usable: bool,
    pub original_level_three_retracted_here: bool,
    pub external_identity_inferred: bool,
}

fn push_digest(d: &mut Sha256, part: &str) {
    d.update((part.len() as u64).to_be_bytes());
    d.update(part.as_bytes());
}

fn receipt_ref(
    review: &NativeAntecedentReviewInspection,
    frontier: &NativeAntecedentFrontier,
    derived_ref: &str,
    answer: Option<&str>,
) -> String {
    let mut d = Sha256::new();
    for part in [
        "slr-source-local-consumer-v1",
        review.document_ref.as_str(), review.source_revision_ref.as_str(),
        review.canonical_content_sha256.as_str(),
        review.focus.mention_ref.as_str(), review.focus.statement_ref.as_str(),
        frontier.parser_run_ref.as_str(), frontier.candidate_set_ref.as_str(),
        frontier.review_item_ref.as_str(), review.s29_review_status.as_str(),
        derived_ref, answer.unwrap_or("UNRESOLVED"),
    ] { push_digest(&mut d, part); }
    format!("source-relative-answer:sha256:{:x}", d.finalize())
}

/// This is the actual source answer under the named-topic linguistic policy.
/// It is not the generic identity admission API; that is a separate owner.
pub fn compile_native_source_local_answer(
    review: &NativeAntecedentReviewInspection,
    frontier: &NativeAntecedentFrontier,
) -> Result<NativeSourceLocalAnswer, NativeNamedTopicError> {
    let inferred = derive_native_named_topic(review, frontier)?;
    let answer = inferred.source_local_answer.clone();
    let receipt_ref = receipt_ref(
        review, frontier, &inferred.derivation_ref, answer.as_deref()
    );
    Ok(NativeSourceLocalAnswer {
        schema: "slr-native-source-local-consumer-v1",
        consumer_scope_ref: "source-local-antecedent-query:declared-named-topic-policy-v1",
        answer_receipt_ref: receipt_ref,
        source_document_ref: review.document_ref.clone(),
        source_revision_ref: review.source_revision_ref.clone(),
        source_content_sha256: review.canonical_content_sha256.clone(),
        original_focus_mention_ref: review.focus.mention_ref.clone(),
        original_focus_statement_ref: review.focus.statement_ref.clone(),
        original_focus_region_ref: review.focus.region_ref.clone(),
        original_focus_span: [review.focus.start_char,review.focus.end_char],
        original_focus_surface: review.focus.surface.clone(),
        original_selected_antecedent_statement_ref:
            review.candidate_antecedent.statement_ref.clone(),
        original_selected_antecedent_region_ref:
            review.candidate_antecedent.region_ref.clone(),
        original_selected_antecedent_span: [
            review.candidate_antecedent.start_char,
            review.candidate_antecedent.end_char,
        ],
        original_selected_antecedent_surface:
            review.candidate_antecedent.surface.clone(),
        original_parser_run_ref: frontier.parser_run_ref.clone(),
        original_candidate_set_ref: frontier.candidate_set_ref.clone(),
        original_review_item_ref: review.review_item_ref.clone(),
        original_s29_status: review.s29_review_status.clone(),
        original_policy_ref: inferred.policy_ref.to_owned(),
        original_derivation_ref: inferred.derivation_ref,
        original_person_support: inferred.supporting_person_mentions,
        answer_kind: if inferred.source_local_inference_derived {
            "DERIVED_UNDER_NAMED_TOPIC_POLICY"
        } else { "EXPLICIT_UNRESOLVED" },
        answer_value: answer,
        source_local_policy_inference_paid:
            inferred.source_local_inference_derived,
        typed_unique_identity_witness_paid: false,
        governed_semantic_identity_paid: false,
        level_three_substitution_paid: false,
        external_qid_paid: false,
        historical_truth_paid: false,
    })
}

/// Pure read-only replay: a consumer may continue using the historical answer
/// only if the exact current, source-revalidated certificate is identical.
/// Changed source, parser run, policy, actor/evidence, or review state are
/// invalidations; we do not mutate or pretend to retract external Level-3 DB.
pub fn evaluate_native_source_local_replay(
    prior: &serde_json::Value,
    current: NativeSourceLocalAnswer,
) -> NativeSourceAnswerReplay {
    let same = serde_json::to_value(&current)
        .map(|actual| actual == *prior).unwrap_or(false);
    // A current unresolved receipt is useful evidence of uncertainty, but
    // must never license consumption of a positive interpretation.
    let current_is_positive = current.source_local_policy_inference_paid
        && current.answer_kind == "DERIVED_UNDER_NAMED_TOPIC_POLICY"
        && current.answer_value.as_ref().is_some_and(|value| !value.trim().is_empty());
    NativeSourceAnswerReplay {
        schema: "slr-native-source-local-answer-replay-v1",
        result: if same { "CURRENT" } else { "INVALIDATED" },
        current,
        prior_receipt_ref: prior.get("answer_receipt_ref")
            .and_then(serde_json::Value::as_str).map(str::to_owned),
        original_level_one_source_preserved: true,
        previous_interpretation_usable: same && current_is_positive,
        original_level_three_retracted_here: false,
        external_identity_inferred: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(value: Option<&str>) -> NativeSourceLocalAnswer {
        NativeSourceLocalAnswer {
            schema:"slr-native-source-local-consumer-v1",
            consumer_scope_ref:
                "source-local-antecedent-query:declared-named-topic-policy-v1",
            answer_receipt_ref:"source-relative-answer:sha256:sample".into(),
            source_document_ref:"document:book".into(),
            source_revision_ref:"revision:book".into(),
            source_content_sha256:"digest:book".into(),
            original_focus_mention_ref:"mention:he".into(),
            original_focus_statement_ref:"stmt:23".into(),
            original_focus_region_ref:"reg:23".into(),
            original_focus_span:[4534,4536],
            original_focus_surface:"he".into(),
            original_selected_antecedent_statement_ref:"stmt:22".into(),
            original_selected_antecedent_region_ref:"reg:22".into(),
            original_selected_antecedent_span:[4310,4314],
            original_selected_antecedent_surface:"Bush".into(),
            original_parser_run_ref:"run:actual".into(),
            original_candidate_set_ref:"frontier:actual".into(),
            original_review_item_ref:"review:pending".into(),
            original_s29_status:"Pending".into(),
            original_policy_ref:"slr:policy:source-local-repeated-person-topic:v1".into(),
            original_derivation_ref:"derivation:actual".into(),
            original_person_support:vec![],
            answer_kind:if value.is_some() {
                "DERIVED_UNDER_NAMED_TOPIC_POLICY"
            } else {"EXPLICIT_UNRESOLVED"},
            answer_value:value.map(str::to_owned),
            source_local_policy_inference_paid:value.is_some(),
            typed_unique_identity_witness_paid:false,
            governed_semantic_identity_paid:false,
            level_three_substitution_paid:false,
            external_qid_paid:false,
            historical_truth_paid:false,
        }
    }
    #[test] fn exact_original_source_answer_replays_current() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let result=evaluate_native_source_local_replay(&json,original);
        assert_eq!(result.result,"CURRENT");
        assert!(result.previous_interpretation_usable);
        assert!(!result.external_identity_inferred);
    }
    #[test] fn unresolved_receipt_replays_without_licensing_interpretation() {
        let current=sample(None);
        let prior=serde_json::to_value(&current).unwrap();
        let result=evaluate_native_source_local_replay(&prior,current);
        assert_eq!(result.result,"CURRENT");
        assert!(!result.previous_interpretation_usable);
        assert!(result.original_level_one_source_preserved);
    }
    #[test] fn positive_flag_without_answer_cannot_license_interpretation() {
        let mut current=sample(None);
        current.source_local_policy_inference_paid=true;
        current.answer_kind="DERIVED_UNDER_NAMED_TOPIC_POLICY";
        let prior=serde_json::to_value(&current).unwrap();
        assert!(!evaluate_native_source_local_replay(&prior,current).previous_interpretation_usable);
    }
    #[test] fn candidate_change_invalidates_original_answer() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let next=sample(None);
        assert_eq!(evaluate_native_source_local_replay(&json,next).result,"INVALIDATED");
    }
    #[test] fn changed_source_revision_invalidates_answer() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let mut next=original;
        next.source_revision_ref="revision:new".into();
        assert_eq!(evaluate_native_source_local_replay(&json,next).result,"INVALIDATED");
    }
    #[test] fn changed_policy_invalidates_answer() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let mut next=original;
        next.original_policy_ref="policy:new".into();
        assert_eq!(evaluate_native_source_local_replay(&json,next).result,"INVALIDATED");
    }
    #[test] fn changed_parser_run_invalidates_answer() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let mut next=original;
        next.original_parser_run_ref="run:new".into();
        assert_eq!(evaluate_native_source_local_replay(&json,next).result,"INVALIDATED");
    }
    #[test] fn forged_semantic_or_external_promotion_cannot_replay() {
        let current=sample(Some("Bush"));
        let mut prior=serde_json::to_value(&current).unwrap();
        prior["governed_semantic_identity_paid"]=serde_json::json!(true);
        assert!(!evaluate_native_source_local_replay(
            &prior,current).previous_interpretation_usable);
    }
    #[test] fn changed_s29_status_does_not_silently_reuse_old_receipt() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let mut next=original;
        next.original_s29_status="Rejected".into();
        assert_eq!(evaluate_native_source_local_replay(&json,next).result,"INVALIDATED");
    }
    #[test] fn changed_supporting_evidence_invalidates_answer() {
        let original=sample(Some("Bush"));
        let json=serde_json::to_value(&original).unwrap();
        let mut next=original;
        next.original_person_support.push(NamedTopicMention{
            name:"Bush".into(),statement_ref:"stmt:19".into(),
            region_ref:"reg:19".into(),start_char:4061,end_char:4065,
            original_observation_ref:"entity:19".into(),
        });
        assert_eq!(evaluate_native_source_local_replay(&json,next).result,"INVALIDATED");
    }
}
