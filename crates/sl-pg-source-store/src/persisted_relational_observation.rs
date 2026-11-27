//! Empirical-source bridges into the relational interlingua.
//!
//! Acceptance observations are reconstructed from durable parser/native
//! products. Callers may choose an already-persisted predicate/statement and
//! supply consumer context, but may not author role/filler structure in the
//! acceptance packet.

use thiserror::Error;
use postgres::{Client,GenericClient,NoTls};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::collections::{BTreeMap,BTreeSet};

use crate::{
    load_candidate_pnf_batch, load_source_statement,
    observation_from_candidate_pnf, observation_from_native_wikidata, CandidatePnfBatch,
    CandidatePnfRole, DatabaseConfig, ObservationContext, RelationalComparisonError,
    RelationalObservation, RelationalSourceFamily, ParserTokenRecord, RoleBinding,
    Polarity, PersistedCandidatePnfBatch, PersistedSourceStatement,
    ExactSourceSpan, audit_candidate_pnf_evidence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedRelationalObservationRequest {
    pub batch_ref: String,
    pub selected_predicate_candidate_ref: String,
    pub observation_ref: String,
    pub source_family: RelationalSourceFamily,
    pub context: ObservationContext,
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedWikidataObservationRequest {
    pub diagnostic_ref: String,
    pub native_statement_ref: String,
    pub observation_ref: String,
    pub context: ObservationContext,
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PersistedRelationalObservationError {
    #[error(transparent)]
    CandidateStore(#[from] crate::CandidatePnfStoreError),
    #[error(transparent)]
    StatementStore(#[from] crate::StatementTraceStoreError),
    #[error(transparent)]
    OntologyStore(#[from] crate::OntologyReviewError),
    #[error(transparent)]
    Relational(#[from] RelationalComparisonError),
    #[error("persisted candidate batch does not exist")]
    MissingBatch,
    #[error("persisted candidate batch owner statement does not exist")]
    MissingStatement,
    #[error("persisted candidate batch and source statement disagree on exact span")]
    WrongSpanOwner,
    #[error("persisted candidate/native source crossed candidate/non-promotion boundary")]
    PromotionBoundary,
    #[error("selected predicate is not a persisted predicate candidate in this batch")]
    MissingPredicate,
    #[error("selected native Wikidata statement is absent from the persisted diagnostic packet")]
    MissingNativeStatement,
    #[error("persisted native Wikidata diagnostic does not exist")]
    MissingDiagnostic,
    #[error("checked relational projection requires the original native DB parser receipt")]
    MissingNativeParserReceipt,
    #[error("original source bytes, parser product, or predicate dependency inverse disagree")]
    CheckedSourceMismatch,
    #[error(transparent)]
    Parser(#[from] crate::DbNativeParserError),
    #[error(transparent)]
    PnfEvidence(#[from] crate::CandidatePnfError),
    #[error(transparent)]
    GenericSource(#[from] crate::GenericSourceContentStoreError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)] Pg(#[from] postgres::Error),
}

fn load_candidate_and_statement(
    config: &DatabaseConfig,
    batch_ref: &str,
) -> Result<(PersistedCandidatePnfBatch, PersistedSourceStatement), PersistedRelationalObservationError> {
    let batch = load_candidate_pnf_batch(config, batch_ref)?
        .ok_or(PersistedRelationalObservationError::MissingBatch)?;
    let statement = load_source_statement(config, &batch.statement_ref)?
        .ok_or(PersistedRelationalObservationError::MissingStatement)?;
    validate_candidate_and_statement(batch,statement)
}

fn validate_candidate_and_statement(batch:PersistedCandidatePnfBatch,statement:PersistedSourceStatement)
    ->Result<(PersistedCandidatePnfBatch,PersistedSourceStatement),PersistedRelationalObservationError>{
    if statement.exact_span_ref != batch.exact_span_ref {
        return Err(PersistedRelationalObservationError::WrongSpanOwner);
    }
    if !statement.candidate_only
        || statement.creates_semantic_authority
        || statement.applicability_promoted
        || statement.claim_truth_promoted
        || !batch.candidate_only
        || batch.semantic_admission_paid
        || batch.proposition_support_paid
        || batch.applicability_paid
        || batch.claim_truth_paid
    {
        return Err(PersistedRelationalObservationError::PromotionBoundary);
    }
    Ok((batch, statement))
}

pub fn relational_observation_from_persisted_pnf(
    config: &DatabaseConfig,
    request: &PersistedRelationalObservationRequest,
) -> Result<RelationalObservation, PersistedRelationalObservationError> {
    let (batch, statement) = load_candidate_and_statement(config, &request.batch_ref)?;
    if !batch.factors.iter().any(|factor| {
        factor.candidate_ref == request.selected_predicate_candidate_ref
            && factor.role == CandidatePnfRole::Predicate
            && factor.candidate_only
    }) {
        return Err(PersistedRelationalObservationError::MissingPredicate);
    }
    let reconstructed = CandidatePnfBatch {
        exact_span_ref: batch.exact_span_ref.clone(),
        candidates: batch.factors.clone(),
        proposition_support_paid: false,
        applicability_paid: false,
        claim_truth_paid: false,
    };
    let mut provenance = request.provenance_refs.clone();
    provenance.extend([request.batch_ref.clone(), batch.parser_receipt_ref.clone()]);
    provenance.sort();
    provenance.dedup();
    Ok(observation_from_candidate_pnf(
        &statement.source_revision_ref,
        request.source_family,
        &request.observation_ref,
        &batch.parser_receipt_ref,
        &request.selected_predicate_candidate_ref,
        &reconstructed,
        request.context.clone(),
        provenance,
    )?)
}

pub fn relational_observation_from_persisted_wikidata(
    config: &DatabaseConfig,
    request: &PersistedWikidataObservationRequest,
) -> Result<RelationalObservation, PersistedRelationalObservationError> {
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let observation=relational_observation_from_persisted_wikidata_on_client(&mut tx,request)?;
    tx.commit()?;
    Ok(observation)
}

pub(crate) fn relational_observation_from_persisted_wikidata_on_client(
    client:&mut impl GenericClient,request:&PersistedWikidataObservationRequest,
)->Result<RelationalObservation,PersistedRelationalObservationError>{
    let read = crate::wikidata_ontology_review::load_ontology_diagnostic_on_client(client,&request.diagnostic_ref)?
        .ok_or(PersistedRelationalObservationError::MissingDiagnostic)?;
    if read.creates_semantic_authority
        || read.grants_wikidata_edit_authority
        || !read.packet.candidate_only
        || read.packet.creates_semantic_authority
        || read.packet.claim_truth_promoted
        || read.packet.grants_wikidata_edit_authority
    {
        return Err(PersistedRelationalObservationError::PromotionBoundary);
    }
    let statement = read
        .packet
        .witnesses
        .iter()
        .flat_map(|w| w.native_statements.iter())
        .find(|s| s.statement_ref == request.native_statement_ref)
        .ok_or(PersistedRelationalObservationError::MissingNativeStatement)?;
    let mut provenance = request.provenance_refs.clone();
    provenance.extend([
        request.diagnostic_ref.clone(),
        read.packet.producer_receipt_ref.clone(),
        read.packet.source_snapshot_digest_ref.clone(),
        statement.statement_revision_ref.clone(),
    ]);
    provenance.sort();
    provenance.dedup();
    Ok(observation_from_native_wikidata(
        &read.packet.source_revision_ref,
        statement,
        &read.packet.producer_receipt_ref,
        provenance,
        request.context.clone(),
    )?)
}

// The legacy batch adapter remains replay-compatible. The checked path below
// is the production-facing native parser projection: one predicate occurrence
// owns only its direct original dependency children, never all sentence roles.
// Roles are syntactic coordinates (not asserted semantic actor/patient roles).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckedRelationalBatch {
    pub batch_ref: String,
    pub source_revision_ref: String,
    pub source_content_digest_ref: String,
    pub original_parser_run_ref: String,
    pub original_region_ref: String,
    pub original_compilation_key: String,
    pub original_token_count: usize,
    pub predicate_candidate_refs: Vec<String>,
    pub observations: Vec<RelationalObservation>,
    pub candidate_only: bool,
    pub semantic_coverage_paid: bool,
}

struct CheckedDependencyFibre<'a> {
    tokens: &'a [ParserTokenRecord],
    batch: &'a CandidatePnfBatch,
    index: BTreeMap<&'a str, usize>,
    children: BTreeMap<u32, Vec<usize>>,
    by_ordinal: BTreeMap<u32, usize>,
}

impl<'a> CheckedDependencyFibre<'a> {
    fn new(tokens: &'a [ParserTokenRecord], batch: &'a CandidatePnfBatch)
        -> Result<Self, PersistedRelationalObservationError>
    {
        if tokens.len() != batch.candidates.len() {
            return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
        }
        let mut index = BTreeMap::new();
        let mut by_ordinal = BTreeMap::new();
        let mut children: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, (token, factor)) in tokens.iter().zip(&batch.candidates).enumerate() {
            if token.start_char != factor.source_start_char
                || token.end_char != factor.source_end_char
                || token.surface != factor.surface || token.lemma != factor.lemma
                || token.dependency_ref != factor.dependency_ref
                || factor.role != crate::candidate_pnf::role_for(&token.pos, &token.dependency_ref)
                || !factor.candidate_only
                || index.insert(factor.candidate_ref.as_str(), i).is_some()
                || by_ordinal.insert(token.token_ordinal, i).is_some()
            {
                return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
            }
            if let Some(head) = token.head_ordinal {
                if head != token.token_ordinal {
                    children.entry(head).or_default().push(i);
                }
            }
        }
        if children.keys().any(|id| !by_ordinal.contains_key(id)) {
            return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
        }
        // Validate ancestor chains without confusing parser-root self heads
        // with cross-token cycles. No semantic interpretation is inferred.
        let mut finished = BTreeSet::new();
        for token in tokens {
            let mut path = BTreeSet::new();
            let mut current = token.token_ordinal;
            loop {
                if finished.contains(&current) { break; }
                if !path.insert(current) {
                    return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
                }
                let t = &tokens[*by_ordinal.get(&current)
                    .ok_or(PersistedRelationalObservationError::CheckedSourceMismatch)?];
                match t.head_ordinal {
                    Some(head) if head != current => current = head,
                    _ => break,
                }
            }
            finished.extend(path);
        }
        Ok(Self { tokens, batch, index, children, by_ordinal })
    }

    fn project(&self, selected: &str)
        -> Result<(Vec<RoleBinding>, Vec<String>), PersistedRelationalObservationError>
    {
        let i = *self.index.get(selected)
            .ok_or(PersistedRelationalObservationError::MissingPredicate)?;
        if self.batch.candidates[i].role != CandidatePnfRole::Predicate {
            return Err(PersistedRelationalObservationError::MissingPredicate);
        }
        let predicate = &self.tokens[i];
        let mut metadata = vec![
            format!("syntax:predicate-ordinal:{}", predicate.token_ordinal),
            format!("syntax:predicate-span:{}:{}", predicate.start_char, predicate.end_char),
            format!("syntax:predicate-dependency:{}", predicate.dependency_ref),
        ];
        if let Some(head) = predicate.head_ordinal.filter(|h| *h != predicate.token_ordinal) {
            let parent = &self.batch.candidates[self.by_ordinal[&head]];
            metadata.push(format!("syntax:parent-factor:{}", parent.candidate_ref));
        }
        let mut positions = BTreeMap::<String,u32>::new();
        let mut roles = Vec::new();
        for &j in self.children.get(&predicate.token_ordinal).into_iter().flatten() {
            let token = &self.tokens[j];
            let factor = &self.batch.candidates[j];
            // The original dependency relation is retained even when the
            // coarse PNF role would classify both active/passive as Actor.
            let role_ref = format!("syntax:{}", token.dependency_ref);
            let occurrence = positions.entry(role_ref.clone()).or_default();
            roles.push(RoleBinding {
                role_ref,
                filler_candidate_ref: factor.candidate_ref.clone(),
                occurrence: *occurrence,
            });
            *occurrence += 1;
            if token.dependency_ref == "neg" {
                metadata.push(format!("syntax:literal-negation-factor:{}", factor.candidate_ref));
            }
        }
        Ok((roles, metadata))
    }
}

/// Reopen the original native source bytes, persisted PNF batch and its
/// checked parser product, then enumerate *every* predicate candidate.
/// Caller-provided context remains candidate context, not a reviewed licence.
/// Neither missing negation nor a syntax child is a truth/polarity proof.
pub fn relational_observations_from_checked_persisted_batch(
    config: &DatabaseConfig,
    batch_ref: &str,
    source_family: RelationalSourceFamily,
    context: &ObservationContext,
    provenance_refs: &[String],
) -> Result<CheckedRelationalBatch, PersistedRelationalObservationError> {
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let result=relational_observations_from_checked_batch_on_client(&mut tx,batch_ref,source_family,context,provenance_refs)?;
    tx.commit()?;
    Ok(result)
}

pub(crate) fn relational_observations_from_checked_batch_on_client(
    client:&mut impl GenericClient,batch_ref:&str,source_family:RelationalSourceFamily,
    context:&ObservationContext,provenance_refs:&[String],
)->Result<CheckedRelationalBatch,PersistedRelationalObservationError>{
    let batch=crate::candidate_pnf_store::load_candidate_pnf_batch_with_client(client,batch_ref)?
        .ok_or(PersistedRelationalObservationError::MissingBatch)?;
    let statement=crate::statement_trace_store::load_source_statement_with_client(client,&batch.statement_ref)?
        .ok_or(PersistedRelationalObservationError::MissingStatement)?;
    let (batch,statement)=validate_candidate_and_statement(batch,statement)?;
    let suffix = format!(":{}", batch.exact_span_ref);
    let run = batch.parser_receipt_ref.strip_prefix("db-parser:")
        .and_then(|s| s.strip_suffix(suffix.as_str()))
        .filter(|s| !s.is_empty())
        .ok_or(PersistedRelationalObservationError::MissingNativeParserReceipt)?;
    let evidence = crate::db_native_parser::load_parser_evidence_region_on_client(client, run, &batch.exact_span_ref)?;
    let (start_char, end_char) = evidence.span();
    if end_char<=start_char {return Err(PersistedRelationalObservationError::CheckedSourceMismatch);}
    let (source, text) = crate::generic_source_content_store::load_generic_text_source_on_client(client, &statement.source_revision_ref)?;
    let source_literal = text.chars().skip(start_char as usize)
        .take((end_char-start_char) as usize).collect::<String>();
    if source.document_ref != statement.document_ref
        || evidence.source_revision_ref() != statement.source_revision_ref.as_str()
        || source_literal != statement.literal_text
        || !evidence.matches_source_literal(&source_literal)
        || !evidence.matches_observation_coverage(&source_literal)
    {
        return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
    }
    let pnf = CandidatePnfBatch {
        exact_span_ref: batch.exact_span_ref.clone(), candidates: batch.factors.clone(),
        proposition_support_paid:false, applicability_paid:false, claim_truth_paid:false,
    };
    let span = ExactSourceSpan { span_ref: batch.exact_span_ref.clone(), start_char, end_char };
    audit_candidate_pnf_evidence(&span, &statement.source_revision_ref, &pnf, &evidence)?;
    let fibre = CheckedDependencyFibre::new(evidence.tokens(), &pnf)?;
    let mut provenance = provenance_refs.to_vec();
    provenance.extend([batch_ref.to_owned(), batch.parser_receipt_ref.clone(),
        statement.statement_ref.clone(), source.acquisition_receipt_ref.clone(),
        source.content_digest_ref.clone()]);
    provenance.sort(); provenance.dedup();
    let predicate_candidate_refs = pnf.candidates.iter()
        .filter(|f| f.role == CandidatePnfRole::Predicate)
        .map(|f| f.candidate_ref.clone()).collect::<Vec<_>>();
    let mut observations = Vec::with_capacity(predicate_candidate_refs.len());
    for predicate in &predicate_candidate_refs {
        let (role_bindings, mut native_metadata_refs) = fibre.project(predicate)?;
        native_metadata_refs.extend([
            format!("native-source-family:{}", source.source_family_ref),
            format!("native-source-provider:{}", source.provider_ref),
            format!("native-parser-compilation:{}", evidence.compilation_key()),
        ]);
        let mut observation = RelationalObservation {
            observation_ref: String::new(),
            source_revision_ref: statement.source_revision_ref.clone(), source_family,
            source_span_ref: batch.exact_span_ref.clone(),
            parser_or_producer_ref: batch.parser_receipt_ref.clone(),
            predicate_candidate_ref: predicate.clone(), role_bindings,
            role_type_hypotheses: vec![], context: context.clone(),
            candidate_type_refs:vec![], provenance_refs:provenance.clone(),
            native_metadata_refs, polarity:Polarity::Undetermined,
            candidate_only:true, creates_semantic_authority:false, claim_truth_promoted:false,
        };
        // A checked syntax observation must not collide with the legacy
        // whole-sentence role projection or a changed candidate context.
        observation.observation_ref = format!("checked-pnf-observation:sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&observation)?));
        observations.push(observation);
    }
    Ok(CheckedRelationalBatch {
        batch_ref: batch_ref.into(), source_revision_ref:statement.source_revision_ref,
        source_content_digest_ref:source.content_digest_ref,
        original_parser_run_ref:run.into(), original_region_ref:batch.exact_span_ref.clone(),
        original_compilation_key:evidence.compilation_key().into(),
        original_token_count:evidence.tokens().len(), predicate_candidate_refs,
        observations, candidate_only:true, semantic_coverage_paid:false,
    })
}

pub fn relational_observation_from_checked_persisted_pnf(
    config: &DatabaseConfig,
    request: &PersistedRelationalObservationRequest,
) -> Result<RelationalObservation, PersistedRelationalObservationError> {
    let batch = relational_observations_from_checked_persisted_batch(config,
        &request.batch_ref, request.source_family, &request.context, &request.provenance_refs)?;
    batch.observations.into_iter()
        .find(|o| o.predicate_candidate_ref == request.selected_predicate_candidate_ref)
        .ok_or(PersistedRelationalObservationError::MissingPredicate)
}


/// Source-local query over checked predicate occurrences. The caller supplies
/// exact mention factors, not resolved person identifiers. Keep every predicate
/// in the answer (including unlinked ones): absence of a direct role is not
/// evidence that the person did not act.
pub fn recorded_action_inventory(batch: &CheckedRelationalBatch, mentions: &[String])
    -> serde_json::Value {
    let selected: BTreeSet<&str> = mentions.iter().map(String::as_str).collect();
    let rows: Vec<_> = batch.observations.iter().map(|o| {
        let matched: Vec<_> = o.role_bindings.iter().filter(|r|
            selected.contains(r.filler_candidate_ref.as_str())).collect();
        let active: Vec<_> = matched.iter().filter(|r|
            matches!(r.role_ref.as_str(), "syntax:nsubj" | "syntax:csubj")).collect();
        let passive: Vec<_> = matched.iter().filter(|r|
            matches!(r.role_ref.as_str(), "syntax:nsubjpass" | "syntax:nsubj:pass" |
                "syntax:csubjpass" | "syntax:csubj:pass")).collect();
        let embedded = o.native_metadata_refs.iter().any(|m|
            m.starts_with("syntax:parent-factor:"));
        let local_negation = o.native_metadata_refs.iter().any(|m|
            m.starts_with("syntax:literal-negation-factor:"));
        serde_json::json!({
            "original_observation": o,
            "matched_roles": matched,
            "active_subject_candidates": active,
            "passive_subject_candidates": passive,
            "embedded_predicate": embedded,
            "local_negation_recorded": local_negation,
            "actor_attribution_paid": false,
            "asserted_action_paid": false
        })
    }).collect();
    serde_json::json!({
        "schema": "slr.recorded-action-inventory.v1",
        "batch_ref": batch.batch_ref,
        "source_revision_ref": batch.source_revision_ref,
        "source_content_digest_ref": batch.source_content_digest_ref,
        "original_parser_run_ref": batch.original_parser_run_ref,
        "original_region_ref": batch.original_region_ref,
        "mention_factor_refs": selected,
        "predicate_count": rows.len(),
        "rows": rows,
        "all_live_semantic_readings_covered": false,
        "governed_identity_paid": false,
        "world_truth_paid": false
    })
}

/// Reopen a whole list of checked source regions in one snapshot. Reject
/// revision mixing and duplicate regions rather than silently merging charts.
/// This is an inventory of recorded syntax, not proof of article completeness.
pub fn read_recorded_article_actions(config: &DatabaseConfig, revision: &str,
    batches: &[String], mentions: &[String])
    -> Result<serde_json::Value, PersistedRelationalObservationError> {
    if revision.trim().is_empty() || batches.is_empty()
        || batches.iter().collect::<BTreeSet<_>>().len() != batches.len()
        || mentions.iter().any(|m| m.trim().is_empty()) {
        return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let mut regions = BTreeSet::new();
    let mut rows = Vec::new();
    let mut digest = None;
    for batch_ref in batches {
        let batch = relational_observations_from_checked_batch_on_client(&mut tx,
            batch_ref, RelationalSourceFamily::Wikipedia, &ObservationContext::default(), &[])?;
        if batch.source_revision_ref != revision || !regions.insert(batch.original_region_ref.clone())
            || digest.as_ref().is_some_and(|d| d != &batch.source_content_digest_ref) {
            return Err(PersistedRelationalObservationError::CheckedSourceMismatch);
        }
        digest = Some(batch.source_content_digest_ref.clone());
        rows.push(recorded_action_inventory(&batch, mentions));
    }
    tx.commit()?;
    Ok(serde_json::json!({
        "schema": "slr.recorded-article-actions.v1",
        "source_revision_ref": revision,
        "source_content_digest_ref": digest,
        "regions": rows,
        "article_region_coverage_paid": false,
        "source_family_verified": false,
        "semantic_coverage_paid": false,
        "world_truth_paid": false
    }))
}

/// Exact source-local inverse of the existing checked predicate projection.
/// No QID, title, surface string or grammatical subject establishes identity.
/// All roles survive; callers can select active-subject candidates separately
/// from passive patients, mentions and unresolved syntax.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateParticipantIncidence {
    pub batch_ref: String,
    pub source_revision_ref: String,
    pub source_span_ref: String,
    pub observation_ref: String,
    pub predicate_candidate_ref: String,
    pub role: RoleBinding,
    pub provenance_refs: Vec<String>,
}

impl CheckedRelationalBatch {
    /// Enumerate the original incidence, retaining occurrence and role.
    pub fn participant_incidence(&self) -> Vec<PredicateParticipantIncidence> {
        self.observations.iter().flat_map(|o| {
            o.role_bindings.iter().map(move |role| PredicateParticipantIncidence {
                batch_ref: self.batch_ref.clone(),
                source_revision_ref: o.source_revision_ref.clone(),
                source_span_ref: o.source_span_ref.clone(),
                observation_ref: o.observation_ref.clone(),
                predicate_candidate_ref: o.predicate_candidate_ref.clone(),
                role: role.clone(),
                provenance_refs: o.provenance_refs.clone(),
            })
        }).collect()
    }

    /// Reverse traversal by an exact persisted factor, not by spelling.
    pub fn predicates_for_participant(&self, factor_ref: &str)
        -> Vec<PredicateParticipantIncidence>
    {
        self.participant_incidence().into_iter()
            .filter(|i| i.role.filler_candidate_ref == factor_ref).collect()
    }
}

#[cfg(test)]
mod checked_dependency_tests {
    use super::*;
    use crate::CandidatePnfFactor;

    fn fixture() -> (Vec<ParserTokenRecord>, CandidatePnfBatch) {
        // Alice said Bob did not leave: subjects and negation belong to
        // distinct predicate occurrences; no reporting-verb dictionary.
        let rows = [
            ("Alice","PROPN","nsubj",1), ("said","VERB","ROOT",1),
            ("Bob","PROPN","nsubj",5), ("did","AUX","aux",5),
            ("not","PART","neg",5), ("leave","VERB","ccomp",1),
        ];
        let tokens = rows.iter().enumerate().map(|(i,(word,pos,dep,head))|
            ParserTokenRecord { token_ordinal:i as u32,start_char:i as u32,
                end_char:i as u32+1,surface:(*word).into(),lemma:(*word).into(),
                pos:(*pos).into(),morph_json:None,head_ordinal:Some(*head),
                dependency_ref:(*dep).into() }).collect::<Vec<_>>();
        let candidates = tokens.iter().map(|t| CandidatePnfFactor {
            candidate_ref:format!("factor:{}",t.token_ordinal),
            role:crate::candidate_pnf::role_for(&t.pos,&t.dependency_ref),
            source_start_char:t.start_char, source_end_char:t.end_char,
            surface:t.surface.clone(),lemma:t.lemma.clone(),
            dependency_ref:t.dependency_ref.clone(),candidate_only:true,
        }).collect();
        (tokens,CandidatePnfBatch{exact_span_ref:"span:1".into(),candidates,
            proposition_support_paid:false,applicability_paid:false,claim_truth_paid:false})
    }

    #[test]
    fn nested_predicate_and_local_negation_do_not_share_all_sentence_roles() {
        let (tokens,batch)=fixture();
        let f=CheckedDependencyFibre::new(&tokens,&batch).unwrap();
        let (outer,outer_meta)=f.project("factor:1").unwrap();
        let (inner,inner_meta)=f.project("factor:5").unwrap();
        assert_eq!(outer.iter().map(|r|r.filler_candidate_ref.as_str()).collect::<Vec<_>>(),
            vec!["factor:0","factor:5"]);
        assert_eq!(inner.iter().map(|r|r.filler_candidate_ref.as_str()).collect::<Vec<_>>(),
            vec!["factor:2","factor:3","factor:4"]);
        assert!(!outer_meta.iter().any(|s|s.contains("literal-negation")));
        assert!(inner_meta.contains(&"syntax:literal-negation-factor:factor:4".into()));
        assert!(inner_meta.contains(&"syntax:parent-factor:factor:1".into()));
    }

    #[test]
    fn article_action_inventory_preserves_unmatched_and_embedded_predicates() {
        let (tokens,pnf)=fixture();
        let fibre=CheckedDependencyFibre::new(&tokens,&pnf).unwrap();
        let observations=["factor:1","factor:5"].iter().map(|predicate|{
            let (role_bindings,native_metadata_refs)=fibre.project(predicate).unwrap();
            RelationalObservation {
                observation_ref:format!("obs:{predicate}"),source_revision_ref:"rev".into(),
                source_family:RelationalSourceFamily::Wikipedia,source_span_ref:"span:1".into(),
                parser_or_producer_ref:"parser".into(),predicate_candidate_ref:(*predicate).into(),
                role_bindings,role_type_hypotheses:vec![],context:ObservationContext::default(),
                candidate_type_refs:vec![],provenance_refs:vec![],native_metadata_refs,
                polarity:Polarity::Undetermined,candidate_only:true,creates_semantic_authority:false,
                claim_truth_promoted:false,
            }
        }).collect();
        let batch=CheckedRelationalBatch {
            batch_ref:"batch".into(),source_revision_ref:"rev".into(),
            source_content_digest_ref:"digest".into(),original_parser_run_ref:"run".into(),
            original_region_ref:"span:1".into(),original_compilation_key:"key".into(),
            original_token_count:tokens.len(),predicate_candidate_refs:vec!["factor:1".into(),"factor:5".into()],
            observations,candidate_only:true,semantic_coverage_paid:false,
        };
        let result=recorded_action_inventory(&batch,&["factor:0".into()]);
        assert_eq!(result["predicate_count"],2);
        let rows=result["rows"].as_array().unwrap();
        assert_eq!(rows[0]["active_subject_candidates"].as_array().unwrap().len(),1);
        assert_eq!(rows[1]["matched_roles"].as_array().unwrap().len(),0);
        assert_eq!(rows[1]["embedded_predicate"],true);
        assert_eq!(rows[1]["local_negation_recorded"],true);
        assert!(rows.iter().all(|r| r["actor_attribution_paid"]==false
            && r["asserted_action_paid"]==false));
    }

    #[test]
    fn foreign_head_cycle_and_non_predicate_fail_closed() {
        let (mut tokens,batch)=fixture();
        assert!(CheckedDependencyFibre::new(&tokens,&batch).unwrap().project("factor:0").is_err());
        tokens[1].head_ordinal=Some(5);
        assert!(CheckedDependencyFibre::new(&tokens,&batch).is_err());
        tokens[1].head_ordinal=Some(999);
        assert!(CheckedDependencyFibre::new(&tokens,&batch).is_err());
    }
}
