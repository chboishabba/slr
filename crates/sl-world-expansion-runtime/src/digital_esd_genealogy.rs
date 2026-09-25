use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use postgres::{Client, NoTls};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use sensiblaw_world_store::DatabaseConfig;

pub const DIGITAL_ESD_GENEALOGY_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS digital_esd;

CREATE TABLE IF NOT EXISTS digital_esd.study_family_hypothesis (
    hypothesis_ref TEXT PRIMARY KEY,
    corpus_ref TEXT NOT NULL,
    left_source_ref TEXT NOT NULL,
    right_source_ref TEXT NOT NULL,
    relation_ref TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    review_required BOOLEAN NOT NULL CHECK (review_required),
    creates_duplicate_decision BOOLEAN NOT NULL CHECK (NOT creates_duplicate_decision),
    creates_same_empirical_study BOOLEAN NOT NULL CHECK (NOT creates_same_empirical_study),
    creates_evidence_independence BOOLEAN NOT NULL CHECK (NOT creates_evidence_independence),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (left_source_ref <> right_source_ref)
);

CREATE INDEX IF NOT EXISTS digital_esd_study_family_left_idx
ON digital_esd.study_family_hypothesis(corpus_ref, left_source_ref, relation_ref);

CREATE INDEX IF NOT EXISTS digital_esd_study_family_right_idx
ON digital_esd.study_family_hypothesis(corpus_ref, right_source_ref, relation_ref);
"#;

#[derive(Debug, Error)]
pub enum DigitalEsdGenealogyError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("study-family hypothesis lacks required identity/relation coordinates")]
    InvalidHypothesis,
    #[error("study-family hypothesis references source outside corpus: {0}")]
    OrphanSource(String),
    #[error("study-family hypothesis attempted an authority promotion")]
    PromotionBoundary,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudyFamilyHypothesisReceipt {
    pub artifact_rows: usize,
    pub persisted_hypotheses: i64,
    pub candidate_only: bool,
    pub review_required: bool,
    pub creates_duplicate_decision: bool,
    pub creates_same_empirical_study: bool,
    pub creates_evidence_independence: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

fn text(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or("").trim().to_owned()
}

fn normalize(row: &Value) -> Result<(String, String, String, String, String), DigitalEsdGenealogyError> {
    let candidate_only = row.get("candidate_only").and_then(Value::as_bool).unwrap_or(true);
    let duplicate = row
        .get("creates_duplicate_decision")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let same_study = row
        .get("creates_same_empirical_study")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !candidate_only || duplicate || same_study {
        return Err(DigitalEsdGenealogyError::PromotionBoundary);
    }

    let hypothesis_ref = {
        let direct = text(row.get("hypothesis_reference"));
        if direct.is_empty() { text(row.get("fibre_reference")) } else { direct }
    };

    let left = text(row.get("left_source_identity_reference"));
    let right = text(row.get("right_source_identity_reference"));
    let (left, right) = if !left.is_empty() && !right.is_empty() {
        (left, right)
    } else {
        let members = row
            .get("member_source_references")
            .and_then(Value::as_array)
            .ok_or(DigitalEsdGenealogyError::InvalidHypothesis)?;
        if members.len() != 2 {
            return Err(DigitalEsdGenealogyError::InvalidHypothesis);
        }
        (text(members.first()), text(members.get(1)))
    };

    let relation = {
        let proposed = text(row.get("proposed_relation"));
        if proposed.is_empty() { text(row.get("relation_kind")) } else { proposed }
    };

    if hypothesis_ref.is_empty() || left.is_empty() || right.is_empty() || relation.is_empty() || left == right {
        return Err(DigitalEsdGenealogyError::InvalidHypothesis);
    }

    let evidence = row
        .get("evidence")
        .cloned()
        .or_else(|| row.get("evidence_reference").cloned())
        .unwrap_or(Value::Null);
    Ok((
        hypothesis_ref,
        left,
        right,
        relation,
        serde_json::to_string(&evidence)?,
    ))
}

pub fn ingest_study_family_hypotheses(
    config: &DatabaseConfig,
    corpus_ref: &str,
    path: Option<&Path>,
) -> Result<StudyFamilyHypothesisReceipt, DigitalEsdGenealogyError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DIGITAL_ESD_GENEALOGY_SCHEMA_SQL)?;

    let Some(path) = path.filter(|path| path.exists()) else {
        return Ok(StudyFamilyHypothesisReceipt {
            artifact_rows: 0,
            persisted_hypotheses: 0,
            candidate_only: true,
            review_required: true,
            creates_duplicate_decision: false,
            creates_same_empirical_study: false,
            creates_evidence_independence: false,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        });
    };

    let corpus_sources = client
        .query(
            "SELECT source_ref FROM digital_esd.corpus_source WHERE corpus_ref=$1",
            &[&corpus_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<std::collections::BTreeSet<_>>();

    let file = BufReader::new(File::open(path)?);
    let mut artifact_rows = 0usize;
    let mut tx = client.transaction()?;
    for line in file.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        artifact_rows += 1;
        let row: Value = serde_json::from_str(&line)?;
        let (hypothesis_ref, left, right, relation, evidence_json) = normalize(&row)?;
        for source in [&left, &right] {
            if !corpus_sources.contains(source) {
                return Err(DigitalEsdGenealogyError::OrphanSource(source.clone()));
            }
        }
        tx.execute(
            r#"INSERT INTO digital_esd.study_family_hypothesis (
                hypothesis_ref, corpus_ref, left_source_ref, right_source_ref,
                relation_ref, evidence_json, candidate_only, review_required,
                creates_duplicate_decision, creates_same_empirical_study,
                creates_evidence_independence, creates_semantic_authority,
                claim_truth_promoted
            ) VALUES (
                $1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE
            ) ON CONFLICT (hypothesis_ref) DO NOTHING"#,
            &[&hypothesis_ref, &corpus_ref, &left, &right, &relation, &evidence_json],
        )?;
    }
    tx.commit()?;

    let persisted_hypotheses: i64 = client
        .query_one(
            r#"SELECT COUNT(*)::BIGINT
               FROM digital_esd.study_family_hypothesis
               WHERE corpus_ref=$1 AND candidate_only AND review_required
                 AND NOT creates_duplicate_decision
                 AND NOT creates_same_empirical_study
                 AND NOT creates_evidence_independence
                 AND NOT creates_semantic_authority
                 AND NOT claim_truth_promoted"#,
            &[&corpus_ref],
        )?
        .get(0);

    Ok(StudyFamilyHypothesisReceipt {
        artifact_rows,
        persisted_hypotheses,
        candidate_only: true,
        review_required: true,
        creates_duplicate_decision: false,
        creates_same_empirical_study: false,
        creates_evidence_independence: false,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_hypothesis_stays_candidate_only() {
        let row = serde_json::json!({
            "candidate_only": true,
            "creates_duplicate_decision": false,
            "creates_same_empirical_study": false,
            "hypothesis_reference": "study-family-hypothesis:test",
            "left_source_identity_reference": "ERIC:A",
            "right_source_identity_reference": "ERIC:B",
            "proposed_relation": "publicationDuplicate",
            "evidence": {"basis": "exact-normalized-title"}
        });
        let (_, left, right, relation, _) = normalize(&row).unwrap();
        assert_eq!(left, "ERIC:A");
        assert_eq!(right, "ERIC:B");
        assert_eq!(relation, "publicationDuplicate");
    }

    #[test]
    fn promoted_hypothesis_is_rejected() {
        let row = serde_json::json!({
            "candidate_only": true,
            "creates_same_empirical_study": true,
            "hypothesis_reference": "study-family-hypothesis:test",
            "left_source_identity_reference": "ERIC:A",
            "right_source_identity_reference": "ERIC:B",
            "proposed_relation": "sameStudy"
        });
        assert!(matches!(normalize(&row), Err(DigitalEsdGenealogyError::PromotionBoundary)));
    }
}
