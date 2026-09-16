use postgres::{Client, NoTls, Transaction};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

const BUILD_CONTRACT: &str = "slr-legal-ir-materialisation:v0_1";
const PROJECTION_CONTRACT: &str = "legal-ir-materialized-view:v0_1";

/// Reviewed PNF support supplied to the persistence layer.
///
/// The materialiser does not derive these coordinates from text. In particular,
/// it cannot manufacture a PNF factor/revision or source-span correspondence.
/// The exact source span must already occur in `observation_provenance_refs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedPropositionSupport {
    pub proposition_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub exact_span_ref: String,
    pub parser_build_ref: String,
    pub pnf_build_ref: String,
    pub refined_pnf_graph_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub structural_signature_ref: String,
    pub predicate_ref: String,
    pub observation_provenance_refs: Vec<String>,
    pub observation_residual_refs: Vec<String>,
    pub legal_system_refs: Vec<String>,
    pub jurisdiction_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub author_ref: String,
    pub institution_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedLegalIrRefs {
    pub semantic_build_ref: String,
    pub projection_ref: String,
    pub observation_ref: String,
    pub graph_revision_ref: String,
}

#[derive(Debug, Error)]
pub enum LegalIrMaterializationError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("required materialisation coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("reviewed observation provenance does not contain the exact source span")]
    ObservationMissingExactSpan,
    #[error("the requested source revision/document/span is not an eligible persisted exact source")]
    SourceCoordinateMismatch,
    #[error("the reviewed PNF factor/revision is not present on the declared PNF graph/document")]
    PnfCoordinateMismatch,
    #[error("an existing legal_ir row conflicts with the requested materialisation: {0}")]
    ExistingRowConflict(&'static str),
}

/// Persist one reviewed proposition-support coordinate into the existing
/// SensibLaw `legal_ir` schema.
///
/// This is deliberately a persistence operation only. It creates no
/// applicability, claim-truth, holding, or reader-proof authority. The SLR
/// evidence-payment consumer remains responsible for interpreting the returned
/// observation/graph coordinates for a specific query.
pub fn materialize_reviewed_proposition_support(
    config: &DatabaseConfig,
    support: &ReviewedPropositionSupport,
) -> Result<MaterializedLegalIrRefs, LegalIrMaterializationError> {
    validate_support(support)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    let canonical_text_ref = require_exact_source(&mut tx, support)?;
    require_reviewed_pnf_coordinate(&mut tx, support)?;

    let build_ref = stable_ref(
        "legal-ir-build",
        &[
            BUILD_CONTRACT,
            &support.proposition_ref,
            &support.source_revision_ref,
            &support.document_ref,
            &support.exact_span_ref,
            &support.pnf_build_ref,
            &support.refined_pnf_graph_ref,
        ],
    );
    let projection_ref = stable_ref(
        "legal-ir-projection",
        &[PROJECTION_CONTRACT, &build_ref, &support.pnf_build_ref],
    );
    let observation_ref = stable_ref(
        "legal-ir-observation",
        &[
            &projection_ref,
            &support.pnf_factor_ref,
            &support.pnf_revision_ref,
            &support.structural_signature_ref,
            &support.predicate_ref,
            &support.exact_span_ref,
        ],
    );
    let graph_revision_ref = stable_ref(
        "legal-ir-graph-revision",
        &[
            &support.proposition_ref,
            &build_ref,
            &support.exact_span_ref,
            &observation_ref,
        ],
    );

    let build_digest = digest(&[
        BUILD_CONTRACT,
        &build_ref,
        &support.document_ref,
        &support.source_revision_ref,
        &canonical_text_ref,
        &support.parser_build_ref,
        &support.pnf_build_ref,
        &support.refined_pnf_graph_ref,
        &projection_ref,
    ]);
    tx.execute(
        r#"
        INSERT INTO legal_ir.semantic_build
          (build_ref, document_ref, source_revision_ref, canonical_text_ref,
           parser_build_ref, pnf_build_ref, refined_pnf_graph_ref,
           legal_ir_projection_ref, legacy_observation_set_ref,
           comparison_ledger_ref, coverage_demand_refs,
           declaration_revision_refs, build_state_ref, provenance_refs,
           build_sha256)
        VALUES
          ($1,$2,$3,$4,$5,$6,$7,$8,
           'legacy-observation-set:none',
           'semantic-comparison-ledger:not-required-for-bounded-reader-support',
           '{}','{}','candidate',$9,$10)
        ON CONFLICT (build_ref) DO NOTHING
        "#,
        &[
            &build_ref,
            &support.document_ref,
            &support.source_revision_ref,
            &canonical_text_ref,
            &support.parser_build_ref,
            &support.pnf_build_ref,
            &support.refined_pnf_graph_ref,
            &projection_ref,
            &build_provenance(support),
            &&build_digest[..],
        ],
    )?;

    let projection_digest = digest(&[
        PROJECTION_CONTRACT,
        &projection_ref,
        &build_ref,
        &support.pnf_build_ref,
    ]);
    tx.execute(
        r#"
        INSERT INTO legal_ir.projection
          (projection_ref, build_ref, pnf_build_ref, projection_contract_ref,
           omitted_factor_refs, projection_residuals, projection_sha256)
        VALUES ($1,$2,$3,$4,'{}','{}',$5)
        ON CONFLICT (projection_ref) DO NOTHING
        "#,
        &[
            &projection_ref,
            &build_ref,
            &support.pnf_build_ref,
            &PROJECTION_CONTRACT,
            &&projection_digest[..],
        ],
    )?;

    let observation_digest = digest(&[
        &observation_ref,
        &projection_ref,
        &support.pnf_factor_ref,
        &support.pnf_revision_ref,
        &support.structural_signature_ref,
        &support.predicate_ref,
        &support.exact_span_ref,
    ]);
    tx.execute(
        r#"
        INSERT INTO legal_ir.observation
          (observation_ref, projection_ref, pnf_factor_ref, pnf_revision_ref,
           structural_signature_ref, predicate_ref, role_bindings,
           qualifier_state, wrapper_state, provenance_refs, residual_refs,
           projection_state_ref, observation_sha256)
        VALUES ($1,$2,$3,$4,$5,$6,
                '{}'::jsonb,'{}'::jsonb,'{}'::jsonb,$7,$8,
                'candidate',$9)
        ON CONFLICT (observation_ref) DO NOTHING
        "#,
        &[
            &observation_ref,
            &projection_ref,
            &support.pnf_factor_ref,
            &support.pnf_revision_ref,
            &support.structural_signature_ref,
            &support.predicate_ref,
            &support.observation_provenance_refs,
            &support.observation_residual_refs,
            &&observation_digest[..],
        ],
    )?;

    let graph_payload_hash = format!(
        "sha256:{}",
        hex(&digest(&[
            &support.proposition_ref,
            &support.exact_span_ref,
            &observation_ref,
        ]))
    );
    let graph_digest = digest(&[
        &graph_revision_ref,
        &support.proposition_ref,
        &graph_payload_hash,
        &support.exact_span_ref,
        &build_ref,
        &support.author_ref,
    ]);
    let graph_span_refs = vec![support.exact_span_ref.clone()];
    tx.execute(
        r#"
        INSERT INTO legal_ir.graph_revision
          (revision_ref, subject_ref, payload_hash, prior_revision_refs,
           source_span_refs, legal_system_refs, jurisdiction_refs, temporal_refs,
           author_ref, institution_ref, build_ref, revision_state_ref,
           revision_sha256)
        VALUES ($1,$2,$3,'{}',$4,$5,$6,$7,$8,$9,$10,'candidate',$11)
        ON CONFLICT (revision_ref) DO NOTHING
        "#,
        &[
            &graph_revision_ref,
            &support.proposition_ref,
            &graph_payload_hash,
            &graph_span_refs,
            &support.legal_system_refs,
            &support.jurisdiction_refs,
            &support.temporal_refs,
            &support.author_ref,
            &support.institution_ref,
            &build_ref,
            &&graph_digest[..],
        ],
    )?;

    verify_existing_rows(
        &mut tx,
        support,
        &build_ref,
        &projection_ref,
        &observation_ref,
        &graph_revision_ref,
    )?;

    tx.commit()?;
    Ok(MaterializedLegalIrRefs {
        semantic_build_ref: build_ref,
        projection_ref,
        observation_ref,
        graph_revision_ref,
    })
}

fn validate_support(
    support: &ReviewedPropositionSupport,
) -> Result<(), LegalIrMaterializationError> {
    for (name, value) in [
        ("proposition_ref", support.proposition_ref.as_str()),
        ("source_revision_ref", support.source_revision_ref.as_str()),
        ("document_ref", support.document_ref.as_str()),
        ("exact_span_ref", support.exact_span_ref.as_str()),
        ("parser_build_ref", support.parser_build_ref.as_str()),
        ("pnf_build_ref", support.pnf_build_ref.as_str()),
        ("refined_pnf_graph_ref", support.refined_pnf_graph_ref.as_str()),
        ("pnf_factor_ref", support.pnf_factor_ref.as_str()),
        ("pnf_revision_ref", support.pnf_revision_ref.as_str()),
        ("structural_signature_ref", support.structural_signature_ref.as_str()),
        ("predicate_ref", support.predicate_ref.as_str()),
        ("author_ref", support.author_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(LegalIrMaterializationError::EmptyCoordinate(name));
        }
    }
    if !support
        .observation_provenance_refs
        .iter()
        .any(|value| value == &support.exact_span_ref)
    {
        return Err(LegalIrMaterializationError::ObservationMissingExactSpan);
    }
    Ok(())
}

fn require_exact_source(
    tx: &mut Transaction<'_>,
    support: &ReviewedPropositionSupport,
) -> Result<String, LegalIrMaterializationError> {
    let row = tx.query_opt(
        r#"
        SELECT d.canonical_ref,
               s.start_char,
               s.end_char,
               char_length(convert_from(c.payload, 'UTF8'))
        FROM legal_source_revision AS l
        JOIN corpus.document AS d
          ON d.document_ref = l.document_ref
        JOIN corpus.canonical_content AS c
          ON c.canonical_ref = d.canonical_ref
        JOIN corpus.span AS s
          ON s.document_ref = d.document_ref
         AND s.span_ref = $3
        WHERE l.source_revision_ref = $1
          AND l.document_ref = $2
          AND l.compile_eligible = TRUE
        "#,
        &[
            &support.source_revision_ref,
            &support.document_ref,
            &support.exact_span_ref,
        ],
    )?;
    let Some(row) = row else {
        return Err(LegalIrMaterializationError::SourceCoordinateMismatch);
    };
    let start: i32 = row.get(1);
    let end: i32 = row.get(2);
    let document_len: i32 = row.get(3);
    if start < 0 || start >= end || end > document_len {
        return Err(LegalIrMaterializationError::SourceCoordinateMismatch);
    }
    Ok(row.get(0))
}

fn require_reviewed_pnf_coordinate(
    tx: &mut Transaction<'_>,
    support: &ReviewedPropositionSupport,
) -> Result<(), LegalIrMaterializationError> {
    let present: bool = tx
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM algebra.factor_revision AS fr
                JOIN pnf.graph_factor_revision AS gfr
                  ON gfr.factor_revision_ref = fr.factor_revision_ref
                JOIN pnf.graph AS g
                  ON g.graph_ref = gfr.graph_ref
                WHERE fr.factor_revision_ref = $1
                  AND fr.factor_ref = $2
                  AND g.graph_ref = $3
                  AND g.document_ref = $4
            )
            "#,
            &[
                &support.pnf_revision_ref,
                &support.pnf_factor_ref,
                &support.refined_pnf_graph_ref,
                &support.document_ref,
            ],
        )?
        .get(0);
    if !present {
        return Err(LegalIrMaterializationError::PnfCoordinateMismatch);
    }
    Ok(())
}

fn verify_existing_rows(
    tx: &mut Transaction<'_>,
    support: &ReviewedPropositionSupport,
    build_ref: &str,
    projection_ref: &str,
    observation_ref: &str,
    graph_revision_ref: &str,
) -> Result<(), LegalIrMaterializationError> {
    let build = tx.query_one(
        "SELECT document_ref, source_revision_ref, legal_ir_projection_ref FROM legal_ir.semantic_build WHERE build_ref=$1",
        &[&build_ref],
    )?;
    if build.get::<_, String>(0) != support.document_ref
        || build.get::<_, String>(1) != support.source_revision_ref
        || build.get::<_, String>(2) != projection_ref
    {
        return Err(LegalIrMaterializationError::ExistingRowConflict("semantic_build"));
    }

    let observation = tx.query_one(
        "SELECT pnf_factor_ref, pnf_revision_ref, provenance_refs FROM legal_ir.observation WHERE observation_ref=$1",
        &[&observation_ref],
    )?;
    let provenance: Vec<String> = observation.get(2);
    if observation.get::<_, String>(0) != support.pnf_factor_ref
        || observation.get::<_, String>(1) != support.pnf_revision_ref
        || !provenance.iter().any(|value| value == &support.exact_span_ref)
    {
        return Err(LegalIrMaterializationError::ExistingRowConflict("observation"));
    }

    let graph = tx.query_one(
        "SELECT subject_ref, source_span_refs, build_ref FROM legal_ir.graph_revision WHERE revision_ref=$1",
        &[&graph_revision_ref],
    )?;
    let spans: Vec<String> = graph.get(1);
    if graph.get::<_, String>(0) != support.proposition_ref
        || graph.get::<_, String>(2) != build_ref
        || !spans.iter().any(|value| value == &support.exact_span_ref)
    {
        return Err(LegalIrMaterializationError::ExistingRowConflict("graph_revision"));
    }
    Ok(())
}

fn build_provenance(support: &ReviewedPropositionSupport) -> Vec<String> {
    let mut values = vec![
        BUILD_CONTRACT.to_owned(),
        support.source_revision_ref.clone(),
        support.document_ref.clone(),
        support.exact_span_ref.clone(),
        support.pnf_factor_ref.clone(),
        support.pnf_revision_ref.clone(),
    ];
    values.sort();
    values.dedup();
    values
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    format!("{prefix}:sha256:{}", hex(&digest(parts)))
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

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}
