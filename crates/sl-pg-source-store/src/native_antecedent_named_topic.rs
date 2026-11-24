//! M1/M2: an actual deterministic, source-local *linguistic inference*.
//!
//! This is NOT closest-name selection and is NOT an external QID or world
//! truth. Its deliberately narrow named-topic-continuity policy only derives
//! a source-local answer when all original, pre-focus PERSON annotations have
//! the SAME normalized literal, that actor occurs in TWO DISTINCT original
//! sentences, the selected antecedent span belongs to that group, the
//! original focus is a third-person singular pronoun, and no quote boundary
//! appears in the five-source-region discourse. Any other person / quote /
//! disagreement / corrupt provenance yields an explicit unresolved result.
//!
//! The derived answer is a positive, evidence-bearing policy interpretation.
//! Independent *institutional admission* by an approved semantic owner remains
//! separate; neither S29 Accepted nor this inference grants a Wikidata QID.

use std::collections::{BTreeMap,BTreeSet};
use serde::Serialize;
use sha2::{Digest,Sha256};
use thiserror::Error;

use crate::{
    NativeAntecedentFrontier, NativeAntecedentReviewInspection, NativeReviewSentence,
    NativeCandidateNomination,
};

pub const NAMED_TOPIC_POLICY: &str =
    "slr:policy:source-local-repeated-person-topic:v1";

#[derive(Debug,Error)]
pub enum NativeNamedTopicError {
    #[error("original SLR frontier and review inspection disagree")]
    ForeignSource,
    #[error("original parser/source nomination inconsistent")]
    ContradictorySource,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NamedTopicMention {
    pub name: String,
    pub statement_ref: String,
    pub region_ref: String,
    pub start_char: u64,
    pub end_char: u64,
    pub original_observation_ref: String,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NativeNamedTopicInference {
    pub schema: &'static str,
    pub policy_ref: &'static str,
    pub derivation_ref: String,
    pub original_document_ref: String,
    pub original_source_revision_ref: String,
    pub canonical_content_sha256: String,
    pub parser_run_ref: String,
    pub review_item_ref: String,
    pub focus_mention_ref: String,
    pub focus_span: [u64;2],
    pub focus_surface: String,
    pub raw_parser_observation_count: usize,
    pub named_person_observation_count: usize,
    pub distinct_named_person_groups: usize,
    pub supporting_person_mentions: Vec<NamedTopicMention>,
    pub distinct_supporting_original_sentences: usize,
    pub named_person_groups: Vec<String>,
    pub status: &'static str,
    pub reason: &'static str,
    pub source_local_answer: Option<String>,
    pub source_local_inference_derived: bool,
    /// A machine-derived, policy-scoped linguistic answer is not a human S29
    /// action; S29 Pending does NOT suppress the inference.
    pub s29_status_at_derivation: String,
    pub requires_human_s29_to_form_policy_inference: bool,
    /// A separately approved original semantic admission has NOT occurred.
    pub governed_source_local_admission_paid: bool,
    pub complete_all_live_referents_proven: bool,
    pub external_qid_paid: bool,
    pub historical_claim_truth_paid: bool,
}

fn quoted(s: &str) -> bool {
    s.chars().any(|c|matches!(c, '"'|'“'|'”'|'„'|'«'|'»'))
}

fn noncontradictory_morph(raw: Option<&str>, surface: &str) -> bool {
    let Some(raw)=raw else {return true};
    let Ok(value)=serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    let Some(obj)=value.as_object() else {return false};
    let expected_gender=if surface=="he" {"Masc"} else {"Fem"};
    for (field,want) in [
        ("Person","3"),("Number","Sing"),("Gender",expected_gender)
    ] {
        if let Some(v)=obj.get(field) {
            let good=match v {
                serde_json::Value::String(s)=>s==want,
                serde_json::Value::Array(xs)=>
                    xs.iter().any(|x|x.as_str()==Some(want)),
                _=>false
            };
            if !good {return false;}
        }
    }
    true
}

fn original_text<'a>(
    before:&'a [NativeReviewSentence], at:&'a NativeReviewSentence,
    row:&NativeCandidateNomination,
) -> Option<&'a NativeReviewSentence> {
    before.iter().chain(std::iter::once(at))
        .find(|s|s.statement_ref==row.original_statement_ref &&
                 s.region_ref==row.original_region_ref)
}

fn exact_source(row:&NativeCandidateNomination,
    sentence:&NativeReviewSentence) -> bool {
    let (a,b)=(row.original_start_char,row.original_end_char);
    if a<sentence.start_char || b>sentence.end_char || a>=b {
        return false;
    }
    let first=(a-sentence.start_char) as usize;
    let last=(b-sentence.start_char) as usize;
    let chars=sentence.literal.chars().collect::<Vec<_>>();
    last<=chars.len() &&
        chars[first..last].iter().collect::<String>()==row.surface
}

fn derivation_ref(
    frontier:&NativeAntecedentFrontier,
    support:&[NamedTopicMention],
    status:&str,
) -> String {
    let mut d=Sha256::new();
    for part in [
        NAMED_TOPIC_POLICY,
        frontier.document_ref.as_str(),
        frontier.source_revision_ref.as_str(),
        frontier.canonical_content_sha256.as_str(),
        frontier.parser_run_ref.as_str(),
        frontier.focus_mention_ref.as_str(),
        frontier.candidate_set_ref.as_str(),
        status,
    ] {
        d.update((part.len() as u64).to_be_bytes());
        d.update(part.as_bytes());
    }
    for s in support {
        for part in [&s.original_observation_ref,&s.statement_ref,
                     &s.region_ref,&s.name] {
            d.update((part.len() as u64).to_be_bytes());
            d.update(part.as_bytes());
        }
        d.update(s.start_char.to_be_bytes());
        d.update(s.end_char.to_be_bytes());
    }
    format!("source-local-topic-inference:sha256:{:x}",d.finalize())
}

/// Pure analysis; the native source and parser output MUST already have
/// passed original database reinspection. Never commits an S29 reviewer
/// action or an external identity.
pub fn derive_native_named_topic(
    review:&NativeAntecedentReviewInspection,
    frontier:&NativeAntecedentFrontier,
) -> Result<NativeNamedTopicInference,NativeNamedTopicError> {
    if review.schema!="slr-native-antecedent-s29-inspection-v1"
        || frontier.schema!="slr-native-antecedent-bounded-frontier-v1"
        || frontier.review_item_ref!=review.review_item_ref
        || frontier.s29_review_status!=review.s29_review_status
        || frontier.document_ref!=review.document_ref
        || frontier.original_source_ref!=review.original_source_ref
        || frontier.source_revision_ref!=review.source_revision_ref
        || frontier.canonical_content_sha256!=review.canonical_content_sha256
        || frontier.focus_mention_ref!=review.focus.mention_ref
        || frontier.focus_start_char!=review.focus.start_char
        || frontier.focus_end_char!=review.focus.end_char
        || frontier.focus_surface!=review.focus.surface
        || frontier.nominated_source_anchor_ref!=
                review.candidate_antecedent.statement_ref
        || frontier.nominated_source_anchor_start_char!=
                review.candidate_antecedent.start_char
        || frontier.nominated_source_anchor_end_char!=
                review.candidate_antecedent.end_char
        || !frontier.structural_region_coverage_complete
        || frontier.original_regions_covered!=review.before.len()+1
        || frontier.parser_nomination_count!=frontier.nominations.len()
        || frontier.source_relative_answer!="UNRESOLVED_REVIEW_REQUIRED"
        || frontier.local_antecedent_admission_paid
        || frontier.external_qid_paid || frontier.claim_truth_paid
        || review.identity_admission_paid || review.external_qid_paid
        || review.claim_truth_paid
    {return Err(NativeNamedTopicError::ForeignSource);}
    let mut groups:BTreeMap<String,Vec<NamedTopicMention>>=BTreeMap::new();
    let mut distinct_mentions=BTreeSet::new();
    for n in frontier.nominations.iter().filter(|n|
        n.observation_kind=="parser_entity" &&
        n.parser_pos_or_label=="PERSON"
    ) {
        let s=original_text(&review.before,&review.focus_statement,n)
            .ok_or(NativeNamedTopicError::ContradictorySource)?;
        if !n.source_verified || !n.in_preceding_source
            || n.semantic_admission_paid || !exact_source(n,s)
            || n.original_end_char>review.focus.start_char
        {return Err(NativeNamedTopicError::ContradictorySource);}
        let normalized=n.surface.split_whitespace()
            .collect::<Vec<_>>().join(" ").to_lowercase();
        if normalized.trim().is_empty() {
            return Err(NativeNamedTopicError::ContradictorySource);
        }
        let dedup=(n.original_statement_ref.clone(),
            n.original_start_char,n.original_end_char);
        if !distinct_mentions.insert(dedup) {continue;}
        groups.entry(normalized).or_default().push(NamedTopicMention {
            name:n.surface.clone(),
            statement_ref:n.original_statement_ref.clone(),
            region_ref:n.original_region_ref.clone(),
            start_char:n.original_start_char,
            end_char:n.original_end_char,
            original_observation_ref:n.parser_observation_ref.clone(),
        });
    }
    let named_person_observation_count=frontier.nominations.iter().filter(|n|
        n.observation_kind=="parser_entity" && n.parser_pos_or_label=="PERSON"
    ).count();
    let scope_quotes=review.before.iter()
        .chain(std::iter::once(&review.focus_statement))
        .any(|s|quoted(&s.literal));
    let matched_groups=groups.iter().filter(|(_,mentions)|
        mentions.iter().any(|n|
            n.original_statement_ref==review.candidate_antecedent.statement_ref &&
            n.region_ref==review.candidate_antecedent.region_ref &&
            n.start_char==review.candidate_antecedent.start_char &&
            n.end_char==review.candidate_antecedent.end_char &&
            n.name==review.candidate_antecedent.surface
        )
    ).collect::<Vec<_>>();
    let supported=if groups.len()==1 {
        groups.values().next().cloned().unwrap_or_default()
    } else {Vec::new()};
    let distinct_sentences=supported.iter()
        .map(|n|n.statement_ref.as_str()).collect::<BTreeSet<_>>().len();
    let morph_compatible=noncontradictory_morph(
        frontier.focus_parser_morph_json.as_deref(),
        frontier.focus_surface.as_str(),
    );
    // This is one declared bounded discourse convention: exactly one named
    // human actor is repeatedly made explicit; independent person names,
    // quote boundaries, nonpersonal pronouns and morphological conflicts
    // defeat it. It does NOT claim completeness of all semantic readings.
    let resolved=groups.len()==1
        && matched_groups.len()==1
        && distinct_sentences>=2
        && !scope_quotes
        && frontier.focus_parser_pos=="PRON"
        && ["he","she"].contains(&frontier.focus_surface.as_str())
        && morph_compatible
        && !matches!(review.s29_review_status.as_str(),"Rejected"|"Superseded");
    let (status,reason)=if resolved {
        ("DERIVED_SOURCE_LOCAL_NAMED_TOPIC",
         "unique repeatedly named PERSON across original scope; no competing PERSON or quote boundary")
    } else if groups.len()>1 {
        ("AMBIGUOUS_NAMED_ACTORS","more than one distinct PERSON name is in original pre-focus source")
    } else if scope_quotes {
        ("UNRESOLVED_QUOTATION_SCOPE","unreviewed quotation boundary in original source")
    } else if !morph_compatible {
        ("UNRESOLVED_PRONOUN_MORPHOLOGY","parser morphology conflicts with third-person pronoun")
    } else {
        ("UNRESOLVED_INSUFFICIENT_DISCOURSE","named-person continuity cannot be independently established")
    };
    let answer=if resolved {
        Some(review.candidate_antecedent.surface.clone())
    } else {None};
    Ok(NativeNamedTopicInference {
        schema:"slr-native-named-topic-inference-v1",
        policy_ref:NAMED_TOPIC_POLICY,
        derivation_ref:derivation_ref(frontier,&supported,status),
        original_document_ref:frontier.document_ref.clone(),
        original_source_revision_ref:frontier.source_revision_ref.clone(),
        canonical_content_sha256:frontier.canonical_content_sha256.clone(),
        parser_run_ref:frontier.parser_run_ref.clone(),
        review_item_ref:frontier.review_item_ref.clone(),
        focus_mention_ref:frontier.focus_mention_ref.clone(),
        focus_span:[frontier.focus_start_char,frontier.focus_end_char],
        focus_surface:frontier.focus_surface.clone(),
        raw_parser_observation_count:frontier.nominations.len(),
        named_person_observation_count,
        distinct_named_person_groups:groups.len(),
        supporting_person_mentions:supported,
        distinct_supporting_original_sentences:distinct_sentences,
        named_person_groups:groups.keys().cloned().collect(),
        status,reason,
        source_local_answer:answer,
        source_local_inference_derived:resolved,
        s29_status_at_derivation:review.s29_review_status.clone(),
        requires_human_s29_to_form_policy_inference:false,
        governed_source_local_admission_paid:false,
        complete_all_live_referents_proven:false,
        external_qid_paid:false,
        historical_claim_truth_paid:false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (NativeAntecedentReviewInspection, NativeAntecedentFrontier) {
        let doc="document:sha256:example".to_string();
        let rev="source-revision:example".to_string();
        let r=NativeAntecedentReviewInspection {
            schema:"slr-native-antecedent-s29-inspection-v1",
            review_item_ref:"review:one".into(),proposal_ref:"proposal:one".into(),
            original_source_ref:"source:one".into(),source_revision_ref:rev.clone(),
            document_ref:doc.clone(),canonical_content_sha256:"digest".into(),
            original_context_sha256:"context".into(),
            focus:crate::NativeSourceMention{
                mention_ref:"mention:he".into(),candidate_ref:"candidate:he".into(),
                statement_ref:"stmt:23".into(),region_ref:"reg:23".into(),
                start_char:60,end_char:62,surface:"he".into()},
            candidate_antecedent:crate::NativeSourceSpan{
                statement_ref:"stmt:22".into(),region_ref:"reg:22".into(),
                start_char:42,end_char:46,surface:"Bush".into()},
            before:vec![
                NativeReviewSentence{statement_ref:"stmt:19".into(),
                    region_ref:"reg:19".into(),start_char:0,end_char:4,literal:"Bush".into()},
                NativeReviewSentence{statement_ref:"stmt:20".into(),
                    region_ref:"reg:20".into(),start_char:5,end_char:10,literal:"other".into()},
                NativeReviewSentence{statement_ref:"stmt:21".into(),
                    region_ref:"reg:21".into(),start_char:11,end_char:16,literal:"about".into()},
                NativeReviewSentence{statement_ref:"stmt:22".into(),
                    region_ref:"reg:22".into(),start_char:40,end_char:47,literal:"a Bush.".into()},
            ],
            focus_statement:NativeReviewSentence{
                statement_ref:"stmt:23".into(),region_ref:"reg:23".into(),
                start_char:60,end_char:62,literal:"he".into()},
            after:vec![],s29_review_status:"Pending".into(),
            s29_receipt_commands:vec![],candidate_only:true,
            antecedent_resolution:"REVIEW_REQUIRED",
            identity_admission_paid:false,external_qid_paid:false,
            claim_truth_paid:false,
        };
        let nomination=|p:u64,s:&str,stmt:&str,reg:&str|{
            NativeCandidateNomination {
                nomination_ref:format!("n:{p}"),
                original_statement_ref:stmt.into(),original_region_ref:reg.into(),
                parser_compilation_key:format!("comp:{reg}"),
                original_start_char:p,original_end_char:p+4,surface:s.into(),
                observation_kind:"parser_entity",
                parser_observation_ref:format!("{reg}:ent"),
                parser_pos_or_label:"PERSON".into(),parser_lemma:None,
                parser_morph_json:None,parser_dependency_ref:None,
                in_preceding_source:true,same_original_statement_as_focus:false,
                source_verified:true,morphological_resolution:"UNASSESSED",
                quotation_scope:"UNASSESSED",semantic_admission_paid:false,
            }
        };
        let noms=vec![
            nomination(0,"Bush","stmt:19","reg:19"),
            nomination(42,"Bush","stmt:22","reg:22"),
        ];
        let f=NativeAntecedentFrontier {
            schema:"slr-native-antecedent-bounded-frontier-v1",
            original_source_ref:r.original_source_ref.clone(),
            source_revision_ref:rev,canonical_content_sha256:"digest".into(),
            document_ref:doc,review_item_ref:r.review_item_ref.clone(),
            s29_review_status:"Pending".into(),parser_run_ref:"run:original".into(),
            focus_mention_ref:"mention:he".into(),focus_start_char:60,
            focus_end_char:62,focus_surface:"he".into(),
            focus_parser_pos:"PRON".into(),
            focus_parser_morph_json:Some(r#"{"Person":"3","Number":"Sing","Gender":"Masc"}"#.into()),
            focus_parser_dependency_ref:"nsubj".into(),
            nominated_source_anchor_ref:"stmt:22".into(),
            nominated_source_anchor_start_char:42,nominated_source_anchor_end_char:46,
            candidate_set_ref:"frontier:one".into(),
            parser_nomination_count:noms.len(),original_regions_covered:5,
            structural_region_coverage_complete:true,
            semantic_candidate_exhaustiveness_paid:false,nominations:noms,
            source_relative_answer:"UNRESOLVED_REVIEW_REQUIRED",
            local_antecedent_admission_paid:false,external_qid_paid:false,
            claim_truth_paid:false,unresolved_obligations:vec![],
        };
        (r,f)
    }
    #[test] fn repeated_named_topic_derives_without_human_s29() {
        let (r,f)=fixture();
        let d=derive_native_named_topic(&r,&f).unwrap();
        assert_eq!(d.status,"DERIVED_SOURCE_LOCAL_NAMED_TOPIC");
        assert_eq!(d.source_local_answer.as_deref(),Some("Bush"));
        assert_eq!(d.distinct_supporting_original_sentences,2);
        assert_eq!(d.named_person_observation_count,2);
        assert!(d.source_local_inference_derived);
        assert!(!d.governed_source_local_admission_paid);
    }
    #[test] fn extra_person_must_not_be_hidden_by_proximity() {
        let (mut r,mut f)=fixture();
        r.before[1].literal="Reagan".into();
        r.before[1].end_char=11;
        let mut x=f.nominations[0].clone();
        x.nomination_ref="n:reagan".into();
        x.parser_observation_ref="reg:20:ent".into();
        x.original_statement_ref="stmt:20".into();
        x.original_region_ref="reg:20".into();
        x.original_start_char=5;x.original_end_char=11;
        x.surface="Reagan".into();
        f.nominations.push(x);f.parser_nomination_count=3;
        let d=derive_native_named_topic(&r,&f).unwrap();
        assert_eq!(d.status,"AMBIGUOUS_NAMED_ACTORS");
        assert!(d.source_local_answer.is_none());
    }
    #[test] fn a_nearer_nonperson_name_never_outvotes_repeated_person_topic() {
        let (mut r,mut f)=fixture();
        r.before[1].literal="Place".into();
        let mut n=f.nominations[0].clone();
        n.nomination_ref="nearer:place".into();
        n.parser_observation_ref="reg:20:token:3".into();
        n.original_statement_ref="stmt:20".into();
        n.original_region_ref="reg:20".into();
        n.original_start_char=5;
        n.original_end_char=10;
        n.surface="Place".into();
        n.observation_kind="parser_token";
        n.parser_pos_or_label="PROPN".into();
        f.nominations.push(n);
        f.parser_nomination_count=f.nominations.len();
        let d=derive_native_named_topic(&r,&f).unwrap();
        assert_eq!(d.source_local_answer.as_deref(),Some("Bush"));
        assert_eq!(d.distinct_named_person_groups,1);
    }
    #[test] fn quote_boundary_blocks_automatic_topic_continuity() {
        let (mut r,f)=fixture();
        r.before[1].literal="\"text".into();
        let d=derive_native_named_topic(&r,&f).unwrap();
        assert_eq!(d.status,"UNRESOLVED_QUOTATION_SCOPE");
    }
    #[test] fn wrong_pronoun_morphology_blocks_derivation() {
        let (r,mut f)=fixture();
        f.focus_parser_morph_json=Some(r#"{"Person":"1","Number":"Plur"}"#.into());
        assert_eq!(derive_native_named_topic(&r,&f).unwrap().status,
            "UNRESOLVED_PRONOUN_MORPHOLOGY");
    }
    #[test] fn human_review_pending_does_not_block_linguistic_inference() {
        let (r,f)=fixture();
        assert_eq!(r.s29_review_status,"Pending");
        assert!(derive_native_named_topic(&r,&f).unwrap().source_local_inference_derived);
    }
    #[test] fn source_hash_or_review_mismatch_fail_closed() {
        let (r,mut f)=fixture();
        f.source_revision_ref="another".into();
        assert!(derive_native_named_topic(&r,&f).is_err());
        let (r,mut f)=fixture();
        f.review_item_ref="review:forged".into();
        assert!(derive_native_named_topic(&r,&f).is_err());
    }
    #[test] fn only_one_named_mention_is_insufficient_for_continuity() {
        let (r,mut f)=fixture();
        f.nominations.remove(0);f.parser_nomination_count=1;
        assert!(!derive_native_named_topic(&r,&f).unwrap().source_local_inference_derived);
    }
    #[test] fn s29_rejected_does_not_produce_active_inference() {
        let (mut r,mut f)=fixture();
        r.s29_review_status="Rejected".into();
        f.s29_review_status="Rejected".into();
        assert!(!derive_native_named_topic(&r,&f).unwrap().source_local_inference_derived);
    }
}
