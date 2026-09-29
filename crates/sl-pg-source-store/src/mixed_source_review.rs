//! SCALE-2E.2 / M10.2 source-context read boundary.
//!
//! This is a comparison of *persisted* sources, not a text-matching engine.
//! PNF/L2 signatures nominate contextual questions, native source joins record
//! lineage when independently supported, and StatiBaker links remain observer
//! annotations. None of these implies source identity, copying, or witness
//! independence by itself. A missing scoped observer is not negative evidence.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{DatabaseConfig, ChatSourceJoin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextVisibility {
    Available,
    NotObserved,
    Unavailable,
    ExcludedByScope,
    Redacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticComparison {
    SharedCandidateFingerprint,
    NoSharedCandidateFingerprint,
    InsufficientPnf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenealogyStatus {
    ExactNativeTextCorrespondence,
    SourceBackreferenceUnverified,
    NoRecordedLineage,
}

/// Values are deliberately orthogonal: common candidate factors do not pay
/// source genealogy, review status or independent corroboration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedSourceOperationalLink {
    pub link_ref: String,
    pub operational_event_ref: String,
    pub source_revision_ref: String,
    pub relation_kind: String,
    pub relationship_receipt_ref: String,
    pub producer_ref: String,
    pub producer_event_ref: String,
    pub label: String,
    pub start_time_ref: String,
    pub end_time_ref: String,
    pub provenance_refs: Vec<String>,
    pub reviewed_link: bool,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedSourceComparison {
    pub left_source_revision_ref: String,
    pub right_source_revision_ref: String,
    pub left_statement_refs: Vec<String>,
    pub right_statement_refs: Vec<String>,
    pub left_source_excerpt: String,
    pub right_source_excerpt: String,
    pub shared_entity_candidate_refs: Vec<String>,
    pub shared_proposition_candidate_refs: Vec<String>,
    pub shared_event_candidate_refs: Vec<String>,
    pub semantic_comparison: SemanticComparison,
    pub genealogy: GenealogyStatus,
    pub native_join_refs: Vec<String>,
    pub operational_context_refs: Vec<String>,
    pub operational_links: Vec<MixedSourceOperationalLink>,
    pub operational_visibility: ContextVisibility,
    pub semantic_review_pending: bool,
    pub independent_witnesses_established: Option<usize>,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum MixedSourceReviewError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    ChatStore(#[from] crate::ChatSourceStoreError),
    #[error(transparent)]
    ChatJoin(#[from] crate::ChatSourceJoinError),
    #[error(transparent)]
    Operational(#[from] crate::OperationalStateStoreError),
    #[error("both source revisions must be explicitly selected")]
    MissingSourceSelection,
    #[error("a mixed-source comparison requires two distinct source revisions")]
    SameSourceSelection,
    #[error("selected source revision is not persisted")]
    MissingSourceRevision,
    #[error("persisted semantic candidate rows crossed a non-promotion boundary")]
    CandidatePromotion,
}

fn registered(client: &mut Client, name: &str) -> Result<bool, postgres::Error> {
    let row = client.query_one("SELECT to_regclass($1)::text", &[&name])?;
    Ok(row.get::<_, Option<String>>(0).is_some())
}

fn load_native_excerpt(
    client: &mut Client, revision: &str,
    generic_present: bool, chat_present: bool,
) -> Result<String,postgres::Error> {
    let result=if generic_present {
        client.query_opt(
            "SELECT convert_from(c.payload,'UTF8')
             FROM ingest.generic_source_revision r
             JOIN corpus.document d ON d.document_ref=r.document_ref
             JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
             WHERE r.source_revision_ref=$1", &[&revision],
        )?
    } else {None};
    let result=match result {
        Some(row)=>Some(row.get::<_,String>(0)),
        None if chat_present=>client.query_opt(
            "SELECT convert_from(c.payload,'UTF8')
             FROM corpus.chat_archive_message m
             JOIN corpus.document d ON d.document_ref=m.document_ref
             JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
             WHERE m.source_revision_ref=$1 LIMIT 1", &[&revision],
        )?.map(|row|row.get::<_,String>(0)),
        None=>None,
    };
    let mut excerpt=result.unwrap_or_default().chars().take(500).collect::<String>();
    if excerpt.chars().count()==500 {excerpt.push('…');}
    Ok(excerpt)
}

fn source_statements(
    client: &mut Client,
    revision: &str,
) -> Result<Vec<String>, postgres::Error> {
    Ok(client.query(
        "SELECT statement_ref FROM corpus.source_statement
         WHERE source_revision_ref=$1 AND candidate_only=TRUE
         AND creates_semantic_authority=FALSE
         AND applicability_promoted=FALSE AND claim_truth_promoted=FALSE
         ORDER BY statement_ref", &[&revision],
    )?.into_iter().map(|r| r.get(0)).collect())
}

fn fingerprint_refs(
    client: &mut Client, table: &str,
    fingerprint_col: &str, revision: &str,
) -> Result<BTreeSet<String>, postgres::Error> {
    // Both names come from the fixed callers, never user input.
    let sql = format!(
        "SELECT DISTINCT {fingerprint_col} FROM {table}
         WHERE source_revision_ref=$1 AND candidate_only=TRUE
         AND creates_semantic_authority=FALSE AND claim_truth_promoted=FALSE"
    );
    Ok(client.query(&sql, &[&revision])?
        .into_iter().map(|row| row.get(0)).collect())
}

fn entity_fingerprints(
    client: &mut Client, revision: &str,
) -> Result<BTreeSet<String>, postgres::Error> {
    Ok(client.query(
        "SELECT DISTINCT m.entity_fingerprint_ref
         FROM semantic.entity_mention_candidate m
         JOIN corpus.source_statement s ON s.statement_ref=m.statement_ref
         WHERE s.source_revision_ref=$1 AND m.candidate_only=TRUE
         AND m.creates_entity_identity=FALSE
         AND m.creates_semantic_authority=FALSE
         AND m.claim_truth_promoted=FALSE", &[&revision],
    )?.into_iter().map(|row| row.get(0)).collect())
}

fn has_l2_candidates(
    client: &mut Client, revision: &str,
) -> Result<bool, postgres::Error> {
    // Presence is distinct from overlap. A source with no completed L2
    // candidate materialization is 'insufficient', not a negative match.
    let present:bool=client.query_one(
        "SELECT EXISTS(
             SELECT 1 FROM semantic.proposition_candidate_occurrence
             WHERE source_revision_ref=$1 AND candidate_only=TRUE
         ) OR EXISTS(
             SELECT 1 FROM semantic.event_candidate_occurrence
             WHERE source_revision_ref=$1 AND candidate_only=TRUE
         ) OR EXISTS(
             SELECT 1 FROM semantic.entity_mention_candidate m
             JOIN corpus.source_statement s ON s.statement_ref=m.statement_ref
             WHERE s.source_revision_ref=$1 AND m.candidate_only=TRUE
         )", &[&revision],
    )?.get(0);
    Ok(present)
}

fn intersection(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.intersection(right).cloned().collect()
}

/// Read through established SLR semantic/source stores. The optional
/// operational field is controlled by the requesting workspace's explicit
/// scope; no implicit StatiBaker access occurs.
pub fn load_mixed_source_comparison(
    config: &DatabaseConfig,
    left_revision: &str,
    right_revision: &str,
    chat_message_ref: Option<&str>,
    operational_visibility: ContextVisibility,
) -> Result<MixedSourceComparison, MixedSourceReviewError> {
    if left_revision.trim().is_empty() || right_revision.trim().is_empty() {
        return Err(MixedSourceReviewError::MissingSourceSelection);
    }
    if left_revision == right_revision {
        return Err(MixedSourceReviewError::SameSourceSelection);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    // Chat source revisions live in the original chat archive table rather
    // than ingest.generic_source_revision. Do not demand a fake duplicate
    // generic source just to make cross-family comparisons succeed.
    let generic_present=registered(&mut client,"ingest.generic_source_revision")?;
    let chat_present=registered(&mut client,"corpus.chat_archive_message")?;
    for rev in [left_revision,right_revision] {
        let generic=if generic_present {
            client.query_opt(
                "SELECT 1 FROM ingest.generic_source_revision
                 WHERE source_revision_ref=$1", &[&rev],
            )?.is_some()
        } else {false};
        let chat=if chat_present {
            client.query_opt(
                "SELECT 1 FROM corpus.chat_archive_message
                 WHERE source_revision_ref=$1", &[&rev],
            )?.is_some()
        } else {false};
        if !generic && !chat {
            return Err(MixedSourceReviewError::MissingSourceRevision);
        }
    }
    let left_excerpt=load_native_excerpt(&mut client,left_revision,generic_present,chat_present)?;
    let right_excerpt=load_native_excerpt(&mut client,right_revision,generic_present,chat_present)?;
    let left_statements = source_statements(&mut client,left_revision)?;
    let right_statements = source_statements(&mut client,right_revision)?;
    let pnf_tables = [
        "semantic.entity_mention_candidate",
        "semantic.proposition_candidate_occurrence",
        "semantic.event_candidate_occurrence",
    ];
    let pnf_available = pnf_tables.iter().map(|name| registered(&mut client,name))
        .collect::<Result<Vec<_>,_>>()?.into_iter().all(|flag|flag);
    let (shared_entities,shared_propositions,shared_events) = if pnf_available {
        let ent_left=entity_fingerprints(&mut client,left_revision)?;
        let ent_right=entity_fingerprints(&mut client,right_revision)?;
        let prop_left=fingerprint_refs(&mut client,"semantic.proposition_candidate_occurrence","proposition_fingerprint_ref",left_revision)?;
        let prop_right=fingerprint_refs(&mut client,"semantic.proposition_candidate_occurrence","proposition_fingerprint_ref",right_revision)?;
        let event_left=fingerprint_refs(&mut client,"semantic.event_candidate_occurrence","event_fingerprint_ref",left_revision)?;
        let event_right=fingerprint_refs(&mut client,"semantic.event_candidate_occurrence","event_fingerprint_ref",right_revision)?;
        (
            intersection(&ent_left,&ent_right),
            intersection(&prop_left,&prop_right),
            intersection(&event_left,&event_right),
        )
    } else { (vec![],vec![],vec![]) };
    let l2_populated = if pnf_available {
        has_l2_candidates(&mut client,left_revision)?
            && has_l2_candidates(&mut client,right_revision)?
    } else {false};
    let semantic_comparison = if !pnf_available || !l2_populated
        || left_statements.is_empty() || right_statements.is_empty()
    {
        SemanticComparison::InsufficientPnf
    } else if shared_entities.is_empty() && shared_propositions.is_empty()
        && shared_events.is_empty()
    {
        SemanticComparison::NoSharedCandidateFingerprint
    } else { SemanticComparison::SharedCandidateFingerprint };

    let mut join_refs=Vec::new();
    let mut genealogy=GenealogyStatus::NoRecordedLineage;
    if let Some(message_ref) = chat_message_ref {
        // A join belongs to this comparison only if its canonical chat
        // message actually owns one selected source revision and its producer
        // locator is the *other* source. Matching a random message to either
        // side would manufacture a relationship not in the source records.
        let owner_revision=client.query_opt(
            "SELECT source_revision_ref FROM corpus.chat_archive_message
             WHERE message_ref=$1", &[&message_ref],
        )?.map(|r|r.get::<_,String>(0));
        let other_revision=match owner_revision.as_deref() {
            Some(rev) if rev==left_revision=>Some(right_revision),
            Some(rev) if rev==right_revision=>Some(left_revision),
            _=>None,
        };
        if let Some(other)=other_revision {
            let joins: Vec<ChatSourceJoin>=crate::load_chat_source_joins_for_message(
                config,message_ref,
            )?;
            for join in &joins {
                if join.source_locator_ref == other {
                    // Stored exact_digest only verifies the chat side;
                    // source genealogy is not promoted by this reader.
                    genealogy=GenealogyStatus::SourceBackreferenceUnverified;
                    join_refs.push(
                        crate::chat_source_fold::canonical_chat_source_join_ref(join)
                    );
                }
            }
        }
    }

    let operation_schema_ready = if operational_visibility == ContextVisibility::Available {
        registered(&mut client,"operational.semantic_link")?
    } else {false};
    let requested_operational_scope = if operational_visibility == ContextVisibility::Available
        && !operation_schema_ready { ContextVisibility::Unavailable }
        else {operational_visibility};
    let mut op_links=Vec::new();
    if operation_schema_ready {
        for target in [left_revision,right_revision] {
            for relation in crate::load_operational_semantic_links_for_target(config,target)? {
                let event=crate::load_operational_event(
                    config,&relation.operational_event_ref,
                )?.ok_or(MixedSourceReviewError::CandidatePromotion)?;
                op_links.push(MixedSourceOperationalLink {
                    link_ref:relation.link_ref,
                    operational_event_ref:event.operational_event_ref,
                    source_revision_ref:target.to_owned(),
                    relation_kind:format!("{:?}",relation.relation_kind),
                    relationship_receipt_ref:relation.relationship_receipt_ref,
                    producer_ref:event.producer_ref,
                    producer_event_ref:event.producer_event_ref,
                    label:event.label,
                    start_time_ref:event.start_time_ref,
                    end_time_ref:event.end_time_ref,
                    provenance_refs:event.provenance_refs,
                    reviewed_link:relation.reviewed_link,
                    creates_semantic_authority:false,
                    pays_evidence:false,
                });
            }
        }
        op_links.sort_by(|a,b| (&a.start_time_ref,&a.link_ref).cmp(&(&b.start_time_ref,&b.link_ref)));
        op_links.dedup_by(|a,b|a.link_ref==b.link_ref);
    }
    let op_refs = op_links.iter().map(|item|item.link_ref.clone()).collect::<Vec<_>>();

    let resolved_visibility=if requested_operational_scope==ContextVisibility::Available
        && op_refs.is_empty() { ContextVisibility::NotObserved }
        else { requested_operational_scope };
    Ok(MixedSourceComparison {
        left_source_revision_ref:left_revision.into(),
        right_source_revision_ref:right_revision.into(),
        left_statement_refs:left_statements,
        right_statement_refs:right_statements,
        left_source_excerpt:left_excerpt,
        right_source_excerpt:right_excerpt,
        shared_entity_candidate_refs:shared_entities,
        shared_proposition_candidate_refs:shared_propositions,
        shared_event_candidate_refs:shared_events,
        semantic_comparison,
        genealogy,
        native_join_refs:join_refs,
        operational_context_refs:op_refs,
        operational_links:op_links,
        operational_visibility: resolved_visibility,
        semantic_review_pending:true,
        independent_witnesses_established:None,
        creates_semantic_authority:false,
        pays_evidence:false,
        claim_truth_promoted:false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidate_fingerprint_intersection_is_not_evidence_independence() {
        let a=BTreeSet::from(["entity:paper".into(),"entity:source".into()]);
        let b=BTreeSet::from(["entity:paper".into(),"entity:judgment".into()]);
        assert_eq!(intersection(&a,&b),vec!["entity:paper".to_string()]);
        // A shared candidate is *only* a comparison scheduling signal.
        assert_ne!(GenealogyStatus::SourceBackreferenceUnverified,
                   GenealogyStatus::ExactNativeTextCorrespondence);
    }
}
