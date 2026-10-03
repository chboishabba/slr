//! Derived legal-follow graph projection over already-reviewed legal IR.
//!
//! This is intentionally not a rule engine. It projects one persisted
//! reviewed proposition/support coordinate into the existing legal-follow
//! graph tables so downstream workbench consumers can inspect the source and
//! provenance relation. It creates no holding, applicability, claim truth,
//! execution permission, or semantic authority.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{DatabaseConfig, MaterializedLegalIrRefs, ReviewedPropositionSupport};

const PROJECTION_CONTRACT: &str = "itir:reviewed-legal-follow:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedLegalFollowProjectionReceipt {
    pub projection_ref: String,
    pub proposition_node_ref: String,
    pub observation_node_ref: String,
    pub support_edge_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub legal_ir_projection_ref: String,
    pub legal_ir_observation_ref: String,
    pub legal_ir_graph_revision_ref: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub derived_only: bool,
    pub challengeable: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub creates_applicability: bool,
}

#[derive(Debug, Error)]
pub enum ReviewedLegalFollowProjectionError {
    #[error(transparent)]
    Pg(#[from] postgres::Error),
    #[error("legal-follow persistence tables are not installed")]
    MissingFollowSchema,
    #[error("reviewed legal-IR coordinates are missing or do not match the supplied source support")]
    WrongLegalIrOwner,
    #[error("persisted legal-follow replay differs from the reviewed source projection")]
    ChangedReplay,
}

fn digest(parts: &[&str]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        let bytes = part.as_bytes();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hasher.finalize().into()
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    format!("{prefix}:sha256:{}", hex(&digest(parts)))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn table_exists(client: &mut Client, name: &str) -> Result<bool, postgres::Error> {
    Ok(client
        .query_one("SELECT to_regclass($1)::text", &[&name])?
        .get::<_, Option<String>>(0)
        .is_some())
}

fn verify_legal_ir(
    client: &mut Client,
    support: &ReviewedPropositionSupport,
    refs: &MaterializedLegalIrRefs,
) -> Result<(), ReviewedLegalFollowProjectionError> {
    let build = client.query_opt(
        "SELECT document_ref, source_revision_ref, legal_ir_projection_ref
         FROM legal_ir.semantic_build WHERE build_ref=$1",
        &[&refs.semantic_build_ref],
    )?;
    let observation = client.query_opt(
        "SELECT projection_ref, pnf_factor_ref, pnf_revision_ref, provenance_refs
         FROM legal_ir.observation WHERE observation_ref=$1",
        &[&refs.observation_ref],
    )?;
    let graph = client.query_opt(
        "SELECT subject_ref, source_span_refs, build_ref
         FROM legal_ir.graph_revision WHERE revision_ref=$1",
        &[&refs.graph_revision_ref],
    )?;
    let (Some(build), Some(observation), Some(graph)) = (build, observation, graph) else {
        return Err(ReviewedLegalFollowProjectionError::WrongLegalIrOwner);
    };
    let provenance: Vec<String> = observation.get(3);
    let source_spans: Vec<String> = graph.get(1);
    if build.get::<_, String>(0) != support.document_ref
        || build.get::<_, String>(1) != support.source_revision_ref
        || build.get::<_, String>(2) != refs.projection_ref
        || observation.get::<_, String>(0) != refs.projection_ref
        || observation.get::<_, String>(1) != support.pnf_factor_ref
        || observation.get::<_, String>(2) != support.pnf_revision_ref
        || !provenance.iter().any(|r| r == &support.exact_span_ref)
        || graph.get::<_, String>(0) != support.proposition_ref
        || graph.get::<_, String>(2) != refs.semantic_build_ref
        || !source_spans.iter().any(|r| r == &support.exact_span_ref)
    {
        return Err(ReviewedLegalFollowProjectionError::WrongLegalIrOwner);
    }
    Ok(())
}

/// Persist a minimal, exact, challengeable graph projection of the reviewed
/// support relation. The edge means only “this reviewed source observation is
/// recorded as support for this reviewed proposition coordinate”.
pub fn persist_reviewed_legal_follow_projection(
    config: &DatabaseConfig,
    support: &ReviewedPropositionSupport,
    refs: &MaterializedLegalIrRefs,
) -> Result<ReviewedLegalFollowProjectionReceipt, ReviewedLegalFollowProjectionError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    for table in [
        "pnf_follow_projection",
        "pnf_follow_node",
        "pnf_follow_edge",
        "pnf_follow_edge_provenance",
    ] {
        if !table_exists(&mut client, table)? {
            return Err(ReviewedLegalFollowProjectionError::MissingFollowSchema);
        }
    }
    verify_legal_ir(&mut client, support, refs)?;

    let projection_ref = stable_ref(
        "legal-follow-projection",
        &[
            PROJECTION_CONTRACT,
            &refs.semantic_build_ref,
            &refs.projection_ref,
            &refs.observation_ref,
            &refs.graph_revision_ref,
        ],
    );
    let proposition_node_ref = stable_ref(
        "legal-follow-node-proposition",
        &[&projection_ref, &support.proposition_ref],
    );
    let observation_node_ref = stable_ref(
        "legal-follow-node-observation",
        &[&projection_ref, &refs.observation_ref],
    );
    let support_edge_ref = stable_ref(
        "legal-follow-edge-reviewed-support",
        &[&projection_ref, &observation_node_ref, &proposition_node_ref],
    );
    let payload = serde_json::json!({
        "schema": PROJECTION_CONTRACT,
        "source_revision_ref": support.source_revision_ref,
        "exact_span_ref": support.exact_span_ref,
        "reviewed_proposition_ref": support.proposition_ref,
        "legal_ir_build_ref": refs.semantic_build_ref,
        "legal_ir_projection_ref": refs.projection_ref,
        "legal_ir_observation_ref": refs.observation_ref,
        "legal_ir_graph_revision_ref": refs.graph_revision_ref,
        "relation": "reviewed_source_support",
        "creates_semantic_authority": false,
        "creates_claim_truth": false,
        "creates_applicability": false
    });
    let payload_text = serde_json::to_string(&payload)
        .map_err(|_| ReviewedLegalFollowProjectionError::ChangedReplay)?;
    let projection_sha256 = digest(&[
        PROJECTION_CONTRACT,
        &projection_ref,
        &support.document_ref,
        &payload_text,
    ]);
    let projection_sha256_vec = projection_sha256.to_vec();

    let mut tx = client.transaction()?;
    tx.execute(
        "INSERT INTO pnf_follow_projection
         (projection_ref, document_ref, projection_kind, authority_ceiling,
          promotion_allowed, execution_allowed, payload, projection_sha256)
         VALUES($1,$2,'legal_follow','derived_only_challengeable',FALSE,FALSE,$3::jsonb,$4)
         ON CONFLICT(projection_ref) DO NOTHING",
        &[&projection_ref, &support.document_ref, &payload_text, &projection_sha256_vec],
    )?;
    tx.execute(
        "INSERT INTO pnf_follow_node
         (projection_ref, node_ref, node_kind, label, document_ref,
          factor_revision_ref, domain_ir_ref, source_record_ref)
         VALUES($1,$2,'reviewed_proposition','Reviewed proposition',$3,NULL,NULL,$4)
         ON CONFLICT(projection_ref,node_ref) DO NOTHING",
        &[
            &projection_ref,
            &proposition_node_ref,
            &support.document_ref,
            &support.source_revision_ref,
        ],
    )?;
    tx.execute(
        "INSERT INTO pnf_follow_node
         (projection_ref, node_ref, node_kind, label, document_ref,
          factor_revision_ref, domain_ir_ref, source_record_ref)
         VALUES($1,$2,'reviewed_source_observation','Reviewed source observation',$3,$4,NULL,$5)
         ON CONFLICT(projection_ref,node_ref) DO NOTHING",
        &[
            &projection_ref,
            &observation_node_ref,
            &support.document_ref,
            &support.pnf_revision_ref,
            &support.source_revision_ref,
        ],
    )?;
    tx.execute(
        "INSERT INTO pnf_follow_edge
         (edge_ref, projection_ref, from_node_ref, to_node_ref,
          relation_kind, admissibility_state)
         VALUES($1,$2,$3,$4,'reviewed_source_support','challengeable')
         ON CONFLICT(edge_ref) DO NOTHING",
        &[
            &support_edge_ref,
            &projection_ref,
            &observation_node_ref,
            &proposition_node_ref,
        ],
    )?;
    for (provenance_ref, evidence_ref) in [
        (&refs.semantic_build_ref, &support.source_revision_ref),
        (&refs.observation_ref, &support.exact_span_ref),
        (&refs.graph_revision_ref, &support.source_revision_ref),
    ] {
        tx.execute(
            "INSERT INTO pnf_follow_edge_provenance
             (edge_ref, provenance_ref, evidence_ref)
             VALUES($1,$2,$3) ON CONFLICT DO NOTHING",
            &[&support_edge_ref, provenance_ref, evidence_ref],
        )?;
    }
    tx.commit()?;

    let projection = client.query_one(
        "SELECT document_ref, projection_kind, authority_ceiling,
                promotion_allowed, execution_allowed, projection_sha256
         FROM pnf_follow_projection WHERE projection_ref=$1",
        &[&projection_ref],
    )?;
    let node_count: i64 = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM pnf_follow_node WHERE projection_ref=$1",
            &[&projection_ref],
        )?
        .get(0);
    let edge_count: i64 = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM pnf_follow_edge WHERE projection_ref=$1",
            &[&projection_ref],
        )?
        .get(0);
    if projection.get::<_, String>(0) != support.document_ref
        || projection.get::<_, String>(1) != "legal_follow"
        || projection.get::<_, String>(2) != "derived_only_challengeable"
        || projection.get::<_, bool>(3)
        || projection.get::<_, bool>(4)
        || projection.get::<_, Vec<u8>>(5) != projection_sha256_vec
        || node_count != 2
        || edge_count != 1
    {
        return Err(ReviewedLegalFollowProjectionError::ChangedReplay);
    }

    Ok(ReviewedLegalFollowProjectionReceipt {
        projection_ref,
        proposition_node_ref,
        observation_node_ref,
        support_edge_ref,
        document_ref: support.document_ref.clone(),
        source_revision_ref: support.source_revision_ref.clone(),
        legal_ir_projection_ref: refs.projection_ref.clone(),
        legal_ir_observation_ref: refs.observation_ref.clone(),
        legal_ir_graph_revision_ref: refs.graph_revision_ref.clone(),
        node_count: node_count as usize,
        edge_count: edge_count as usize,
        derived_only: true,
        challengeable: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        creates_applicability: false,
    })
}
