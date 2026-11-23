//! M1 integrated native source/parser antecedent nomination and consumer residual.
//!
//! Does not parse text, re-run spaCy, decide a pronoun, or infer world identity.
//! C4.12 owns source/S29 reopening; db_native_parser owns persisted POS,
//! morphology, entities and original parser product. The output is a complete
//! *bounded source observation* frontier, NOT complete semantic readings.

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use std::collections::BTreeSet;

use crate::{
    inspect_native_antecedent_review, load_parser_evidence_region,
    DatabaseConfig, NativeAntecedentReviewError, NativeReviewSentence,
    ParserEvidenceRegion, ParserTokenRecord, ParserEntityRecord, DbNativeParserError,
};

#[derive(Debug,Error)]
pub enum NativeAntecedentFrontierError {
    #[error(transparent)]
    Source(#[from] NativeAntecedentReviewError),
    #[error(transparent)]
    Parser(#[from] DbNativeParserError),
    #[error("original source and parser run/region/coverage do not agree")]
    ParserSourceMismatch,
    #[error("no exact original focus pronoun token or conflicting tokens")]
    MissingFocusToken,
    #[error("original C4.10 Bush nomination lacks a matching parser token or entity")]
    MissingNamedSourceAnchor,
    #[error("parser run reference cannot be empty")]
    MissingParserRun,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NativeCandidateNomination {
    pub nomination_ref: String,
    pub original_statement_ref: String,
    pub original_region_ref: String,
    pub parser_compilation_key: String,
    pub original_start_char: u64,
    pub original_end_char: u64,
    pub surface: String,
    /// "parser_token" or "parser_entity"; neither establishes a referent.
    pub observation_kind: &'static str,
    pub parser_observation_ref: String,
    pub parser_pos_or_label: String,
    pub parser_lemma: Option<String>,
    pub parser_morph_json: Option<String>,
    pub parser_dependency_ref: Option<String>,
    /// Note: this is only temporal/source accessibility; *no* quote/scope proof.
    pub in_preceding_source: bool,
    pub same_original_statement_as_focus: bool,
    pub source_verified: bool,
    pub morphological_resolution: &'static str,
    pub quotation_scope: &'static str,
    pub semantic_admission_paid: bool,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NativeAntecedentFrontier {
    pub schema: &'static str,
    pub original_source_ref: String,
    pub source_revision_ref: String,
    pub canonical_content_sha256: String,
    pub document_ref: String,
    pub review_item_ref: String,
    pub s29_review_status: String,
    pub parser_run_ref: String,
    pub focus_mention_ref: String,
    pub focus_start_char: u64,
    pub focus_end_char: u64,
    pub focus_surface: String,
    pub nominated_source_anchor_ref: String,
    pub nominated_source_anchor_start_char: u64,
    pub nominated_source_anchor_end_char: u64,
    pub candidate_set_ref: String,
    pub parser_nomination_count: usize,
    pub original_regions_covered: usize,
    pub structural_region_coverage_complete: bool,
    /// This deliberately never claims all possible semantic readings.
    pub semantic_candidate_exhaustiveness_paid: bool,
    pub nominations: Vec<NativeCandidateNomination>,
    pub source_relative_answer: &'static str,
    pub local_antecedent_admission_paid: bool,
    pub external_qid_paid: bool,
    pub claim_truth_paid: bool,
    pub unresolved_obligations: Vec<&'static str>,
}

fn sha_parts(parts: &[&str]) -> String {
    let mut d=Sha256::new();
    for p in std::iter::once("slr-native-antecedent-frontier:v1")
        .chain(parts.iter().copied()) {
        d.update((p.len() as u64).to_be_bytes());
        d.update(p.as_bytes());
    }
    format!("{:x}",d.finalize())
}

fn token_eligible(t: &ParserTokenRecord, focus_start: u64) -> bool {
    t.end_char as u64 <= focus_start &&
    matches!(t.pos.as_str(), "PROPN" | "NOUN" | "PRON")
}
fn entity_eligible(e: &ParserEntityRecord, focus_start: u64) -> bool {
    e.end_char as u64 <= focus_start
}

fn nomination_key(
    run: &str, compilation: &str, kind: &str,
    ordinal: u32, st: u32, en: u32,
) -> String {
    let ord=ordinal.to_string();
    let from=st.to_string();
    let to=en.to_string();
    format!("native-parser-nomination:sha256:{}",sha_parts(&[
        run,compilation,kind,&ord,&from,&to
    ]))
}

fn token_nomination(
    t: &ParserTokenRecord, s: &NativeReviewSentence,
    e: &ParserEvidenceRegion, run: &str, focus_start: u64,
    focus_statement_ref: &str,
) -> Option<NativeCandidateNomination> {
    if !token_eligible(t,focus_start) {return None;}
    let n=nomination_key(run,e.compilation_key(),"token",
                          t.token_ordinal,t.start_char,t.end_char);
    Some(NativeCandidateNomination {
        nomination_ref:n,
        original_statement_ref:s.statement_ref.clone(),
        original_region_ref:s.region_ref.clone(),
        parser_compilation_key:e.compilation_key().into(),
        original_start_char:t.start_char as u64,
        original_end_char:t.end_char as u64,
        surface:t.surface.clone(),
        observation_kind:"parser_token",
        parser_observation_ref:format!("{}:token:{}",e.compilation_key(),t.token_ordinal),
        parser_pos_or_label:t.pos.clone(),
        parser_lemma:Some(t.lemma.clone()),
        parser_morph_json:t.morph_json.clone(),
        parser_dependency_ref:Some(t.dependency_ref.clone()),
        in_preceding_source:true,
        same_original_statement_as_focus:s.statement_ref==focus_statement_ref,
        source_verified:true,morphological_resolution:"UNASSESSED",
        quotation_scope:"UNASSESSED",
        semantic_admission_paid:false
    })
}
fn entity_nomination(
    a: &ParserEntityRecord, s: &NativeReviewSentence,
    e: &ParserEvidenceRegion, run: &str, focus_start: u64,
    focus_statement_ref: &str,
) -> Option<NativeCandidateNomination> {
    if !entity_eligible(a,focus_start) {return None;}
    Some(NativeCandidateNomination {
        nomination_ref:nomination_key(run,e.compilation_key(),"entity",
                                       a.entity_ordinal,a.start_char,a.end_char),
        original_statement_ref:s.statement_ref.clone(),
        original_region_ref:s.region_ref.clone(),
        parser_compilation_key:e.compilation_key().into(),
        original_start_char:a.start_char as u64,
        original_end_char:a.end_char as u64,
        surface:a.surface.clone(),
        observation_kind:"parser_entity",
        parser_observation_ref:format!("{}:entity:{}",e.compilation_key(),a.entity_ordinal),
        parser_pos_or_label:a.label_ref.clone(),
        parser_lemma:None,parser_morph_json:None,parser_dependency_ref:None,
        in_preceding_source:true,
        same_original_statement_as_focus:s.statement_ref==focus_statement_ref,
        source_verified:true,morphological_resolution:"UNASSESSED",
        quotation_scope:"UNASSESSED",
        semantic_admission_paid:false
    })
}

/// The source statement and *original persisted parser product* must agree
/// before even a lexical candidate can enter this bounded frontier.
fn audited_parser_region(
    config: &DatabaseConfig, run: &str, expected_source_revision: &str,
    sentence: &NativeReviewSentence,
) -> Result<ParserEvidenceRegion,NativeAntecedentFrontierError> {
    let e=load_parser_evidence_region(config,run,&sentence.region_ref)?;
    if e.source_revision_ref()!=expected_source_revision
        || e.region_ref()!=sentence.region_ref
        || e.span()!=(sentence.start_char as u32,sentence.end_char as u32)
        || sentence.start_char>u32::MAX as u64
        || sentence.end_char>u32::MAX as u64
        || !e.candidate_only()
        || !e.matches_source_literal(&sentence.literal)
        || !e.matches_observation_coverage(&sentence.literal) {
        return Err(NativeAntecedentFrontierError::ParserSourceMismatch);
    }
    Ok(e)
}

/// No independent authority is inferred from having a single token candidate.
/// A real decision must use the original typed binding + S29 + separately
/// admitted semantic authority; this function *always* emits unresolved.
pub fn native_antecedent_frontier(
    config: &DatabaseConfig,
    review_item_ref: &str,
    parser_run_ref: &str,
) -> Result<NativeAntecedentFrontier,NativeAntecedentFrontierError> {
    if parser_run_ref.trim().is_empty() {
        return Err(NativeAntecedentFrontierError::MissingParserRun);
    }
    let inspect=inspect_native_antecedent_review(config,review_item_ref)?;
    let mut nominations=Vec::new();
    let focus=inspect.focus.clone();
    let mut sentences=inspect.before.clone();
    sentences.push(inspect.focus_statement.clone());
    let mut focus_count=0usize;
    let mut has_named=false;
    let mut seen=BTreeSet::new();
    for sentence in &sentences {
        let parser=audited_parser_region(
            config,parser_run_ref,&inspect.source_revision_ref,sentence
        )?;
        if sentence.statement_ref==focus.statement_ref {
            focus_count+=parser.tokens().iter().filter(|t| {
                t.start_char as u64==focus.start_char
                    && t.end_char as u64==focus.end_char
                    && t.surface==focus.surface && t.pos=="PRON"
            }).count();
        }
        for t in parser.tokens() {
            if let Some(n)=token_nomination(
                t,sentence,&parser,parser_run_ref,focus.start_char,
                &focus.statement_ref
            ) {
                if n.original_start_char==inspect.candidate_antecedent.start_char
                    && n.original_end_char==inspect.candidate_antecedent.end_char
                    && n.surface==inspect.candidate_antecedent.surface
                    && n.original_statement_ref==inspect.candidate_antecedent.statement_ref {
                    has_named=true;
                }
                if seen.insert(n.nomination_ref.clone()) {nominations.push(n);}
            }
        }
        for ent in parser.entities() {
            if let Some(n)=entity_nomination(
                ent,sentence,&parser,parser_run_ref,focus.start_char,
                &focus.statement_ref
            ) {
                if n.original_start_char==inspect.candidate_antecedent.start_char
                    && n.original_end_char==inspect.candidate_antecedent.end_char
                    && n.surface==inspect.candidate_antecedent.surface
                    && n.original_statement_ref==inspect.candidate_antecedent.statement_ref {
                    has_named=true;
                }
                if seen.insert(n.nomination_ref.clone()) {nominations.push(n);}
            }
        }
    }
    if focus_count!=1 {return Err(NativeAntecedentFrontierError::MissingFocusToken);}
    if !has_named {return Err(NativeAntecedentFrontierError::MissingNamedSourceAnchor);}
    nominations.sort_by(|a,b| {
        (a.original_start_char,a.original_end_char,a.observation_kind,&a.nomination_ref)
        .cmp(&(b.original_start_char,b.original_end_char,b.observation_kind,&b.nomination_ref))
    });
    let mut refs=vec![review_item_ref,parser_run_ref,inspect.source_revision_ref.as_str()];
    for n in &nominations {refs.push(&n.nomination_ref);}
    let candidate_set_ref=format!("native-antecedent-bounded-frontier:sha256:{}",
                                  sha_parts(&refs));
    Ok(NativeAntecedentFrontier {
        schema:"slr-native-antecedent-bounded-frontier-v1",
        original_source_ref:inspect.original_source_ref,
        source_revision_ref:inspect.source_revision_ref,
        canonical_content_sha256:inspect.canonical_content_sha256,
        document_ref:inspect.document_ref,
        review_item_ref:review_item_ref.into(),
        s29_review_status:inspect.s29_review_status,
        parser_run_ref:parser_run_ref.into(),
        focus_mention_ref:focus.mention_ref,
        focus_start_char:focus.start_char,focus_end_char:focus.end_char,
        focus_surface:focus.surface,
        nominated_source_anchor_ref:inspect.candidate_antecedent.statement_ref,
        nominated_source_anchor_start_char:inspect.candidate_antecedent.start_char,
        nominated_source_anchor_end_char:inspect.candidate_antecedent.end_char,
        candidate_set_ref,
        parser_nomination_count:nominations.len(),
        original_regions_covered:sentences.len(),
        structural_region_coverage_complete:true,
        semantic_candidate_exhaustiveness_paid:false,
        nominations,
        source_relative_answer:"UNRESOLVED_REVIEW_REQUIRED",
        local_antecedent_admission_paid:false,
        external_qid_paid:false,
        claim_truth_paid:false,
        unresolved_obligations:vec![
            "full_document_antecedent_accessibility_and_quotations",
            "typed_binding_candidate_set_and_rival_exclusions",
            "human_s29_semantic_review_decision",
            "independent_antecedent_resolution_receipt",
            "consumer_relevant_semantic_fibre_coverage",
            "external_world_qid_and_article_kb_alignment",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(st:u32,en:u32,pos:&str,surf:&str)->ParserTokenRecord {
        ParserTokenRecord {token_ordinal:1,start_char:st,end_char:en,
            surface:surf.into(),lemma:surf.into(),pos:pos.into(),
            morph_json:Some(r#"{"Number":["Sing"]}"#.into()),
            head_ordinal:None,dependency_ref:"nsubj".into()}
    }
    #[test] fn lexical_nominations_are_not_antecedent_decisions() {
        let b=t(4310,4314,"PROPN","Bush");
        assert!(token_eligible(&b,4534));
        assert!(!token_eligible(&t(4534,4536,"PRON","he"),4534));
        assert!(!token_eligible(&t(4539,4543,"PROPN","Ford"),4534));
    }
    #[test] fn rival_candidates_cannot_be_silently_filtered_by_distance() {
        assert!(token_eligible(&t(2,6,"PROPN","Bush"),100));
        assert!(token_eligible(&t(50,55,"PROPN","Reagan"),100));
        assert!(token_eligible(&t(1,7,"PRON","another"),100));
    }
    #[test] fn entity_before_focus_is_not_authority() {
        let e=ParserEntityRecord{entity_ordinal:0,start_char:1,
            end_char:5,surface:"Bush".into(),label_ref:"PERSON".into()};
        assert!(entity_eligible(&e,4534));
        assert!(!entity_eligible(&e,5-1));
    }
    #[test] fn source_candidate_refs_change_with_run_and_span() {
        let a=nomination_key("runA","comp","token",1,4310,4314);
        assert_ne!(a,nomination_key("runB","comp","token",1,4310,4314));
        assert_ne!(a,nomination_key("runA","comp","token",1,4310,4315));
        assert_ne!(a,nomination_key("runA","comp","entity",1,4310,4314));
    }
    #[test] fn no_empty_parser_run() {
        assert_eq!(NativeAntecedentFrontierError::MissingParserRun.to_string(),
                   "parser run reference cannot be empty");
    }
}
