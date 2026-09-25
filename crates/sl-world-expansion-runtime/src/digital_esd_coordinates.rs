use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use sensiblaw_world_store::DatabaseConfig;

pub const DIGITAL_ESD_COORDINATE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS digital_esd;

CREATE TABLE IF NOT EXISTS digital_esd.study_coordinate_candidate (
    candidate_ref TEXT PRIMARY KEY,
    corpus_ref TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    source_revision_ref TEXT NOT NULL,
    statement_ref TEXT NULL,
    batch_ref TEXT NULL,
    exact_span_ref TEXT NULL,
    coordinate_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    evidence_basis_ref TEXT NOT NULL,
    candidate_span_count BIGINT NOT NULL CHECK (candidate_span_count >= 1),
    coordinate_paid BOOLEAN NOT NULL CHECK (NOT coordinate_paid),
    review_required BOOLEAN NOT NULL CHECK (review_required),
    automatic_absence_inference BOOLEAN NOT NULL CHECK (NOT automatic_absence_inference),
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (
      corpus_ref, source_revision_ref, coordinate_ref,
      statement_ref, batch_ref, evidence_basis_ref
    )
);

CREATE INDEX IF NOT EXISTS digital_esd_coordinate_source_idx
ON digital_esd.study_coordinate_candidate
  (corpus_ref, source_ref, coordinate_ref, review_required);

CREATE INDEX IF NOT EXISTS digital_esd_coordinate_review_idx
ON digital_esd.study_coordinate_candidate
  (corpus_ref, review_required, coordinate_ref, source_ref);
"#;

const DETECTOR_REF: &str = "digital-esd:pnf-coordinate-nominator:v1";

pub const CANONICAL_19_COORDINATES: [&str; 19] = [
    "sourceIdentityCoordinate",
    "sourceKindAndRoleCoordinate",
    "publicationDateCoordinate",
    "populationEducationLevelCoordinate",
    "jurisdictionInstitutionContextCoordinate",
    "digitalTechnologyOrPracticeCoordinate",
    "pedagogyCurriculumCompetenceCoordinate",
    "sustainabilityDimensionCoordinate",
    "studyOrReviewDesignCoordinate",
    "outcomeOrClaimCoordinate",
    "timeHorizonCoordinate",
    "lifecycleBoundaryCoordinate",
    "circularityRepairabilityCoordinate",
    "participantAgencyAuthorityCoordinate",
    "interoperabilityGovernanceCoordinate",
    "externalityIncidenceCoordinate",
    "sameObjectStatusCoordinate",
    "contextTransferCoordinate",
    "uncertaintyLimitationCoordinate",
];

#[derive(Debug, Error)]
pub enum DigitalEsdCoordinateError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinateNominationReceipt {
    pub corpus_ref: String,
    pub source_level_candidates: usize,
    pub statement_level_candidates: usize,
    pub total_candidates: usize,
    pub coordinate_paid: bool,
    pub review_required: bool,
    pub automatic_absence_inference: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Clone, Copy)]
struct Rule {
    coordinate_ref: &'static str,
    terms: &'static [&'static str],
}

const RULES: &[Rule] = &[
    Rule {
        coordinate_ref: "populationEducationLevelCoordinate",
        terms: &[
            "student", "students", "learner", "learners", "teacher", "teachers",
            "school", "schools", "university", "universities", "undergraduate",
            "postgraduate", "adolescent", "adolescents", "child", "children",
        ],
    },
    Rule {
        coordinate_ref: "jurisdictionInstitutionContextCoordinate",
        terms: &[
            "institution", "institutional", "school", "university", "college",
            "district", "ministry", "department", "campus", "classroom",
        ],
    },
    Rule {
        coordinate_ref: "digitalTechnologyOrPracticeCoordinate",
        terms: &[
            "digital", "online", "computer", "technology", "technologies",
            "software", "platform", "internet", "virtual", "ai",
            "artificial intelligence", "mobile", "simulation",
        ],
    },
    Rule {
        coordinate_ref: "pedagogyCurriculumCompetenceCoordinate",
        terms: &[
            "pedagogy", "pedagogical", "curriculum", "curricular", "competence",
            "competency", "learning", "teaching", "instruction", "education",
            "literacy", "skill", "skills",
        ],
    },
    Rule {
        coordinate_ref: "sustainabilityDimensionCoordinate",
        terms: &[
            "sustainability", "sustainable", "esd", "climate", "environmental",
            "environment", "social", "economic", "ecological",
        ],
    },
    Rule {
        coordinate_ref: "studyOrReviewDesignCoordinate",
        terms: &[
            "randomized", "randomised", "trial", "experiment", "experimental",
            "survey", "interview", "qualitative", "quantitative", "mixed methods",
            "longitudinal", "cohort", "case study", "systematic review",
            "literature review", "meta-analysis", "meta analysis",
        ],
    },
    Rule {
        coordinate_ref: "outcomeOrClaimCoordinate",
        terms: &[
            "outcome", "outcomes", "effect", "effects", "result", "results",
            "finding", "findings", "score", "scores", "performance", "knowledge",
            "achievement", "engagement",
        ],
    },
    Rule {
        coordinate_ref: "timeHorizonCoordinate",
        terms: &[
            "follow-up", "follow up", "longitudinal", "semester", "year",
            "years", "month", "months", "week", "weeks", "day", "days",
        ],
    },
    Rule {
        coordinate_ref: "lifecycleBoundaryCoordinate",
        terms: &[
            "lifecycle", "life cycle", "embodied", "hardware", "infrastructure",
            "data center", "data centre", "energy", "carbon", "emission",
            "emissions", "manufacturing",
        ],
    },
    Rule {
        coordinate_ref: "circularityRepairabilityCoordinate",
        terms: &[
            "circular", "circularity", "repair", "repairability", "reuse",
            "reusable", "recycle", "recycling", "e-waste", "ewaste",
            "refurbish", "refurbishment", "waste",
        ],
    },
    Rule {
        coordinate_ref: "participantAgencyAuthorityCoordinate",
        terms: &[
            "agency", "voice", "participation", "participatory", "consent",
            "autonomy", "authority", "co-design", "codesign", "choice",
            "student-led", "learner-led",
        ],
    },
    Rule {
        coordinate_ref: "interoperabilityGovernanceCoordinate",
        terms: &[
            "interoperability", "interoperable", "governance", "privacy",
            "data governance", "portability", "open source", "open-source",
            "procurement", "security", "standard", "standards",
        ],
    },
    Rule {
        coordinate_ref: "externalityIncidenceCoordinate",
        terms: &[
            "externality", "externalities", "burden", "burdens", "benefit",
            "benefits", "cost", "costs", "energy", "water", "emission",
            "emissions", "labour", "labor", "supply chain",
        ],
    },
    Rule {
        coordinate_ref: "contextTransferCoordinate",
        terms: &[
            "transfer", "transferability", "generalisability", "generalizability",
            "external validity", "context", "contexts", "scale-up", "scaling",
            "replication",
        ],
    },
    Rule {
        coordinate_ref: "uncertaintyLimitationCoordinate",
        terms: &[
            "limitation", "limitations", "uncertainty", "confidence interval",
            "confidence intervals", "bias", "attrition", "missing", "caveat",
            "caveats", "uncertain",
        ],
    },
];

fn digest_ref(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("coordinate-candidate:sha256:{:x}", hasher.finalize())
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn matched_terms<'a>(text: &str, terms: &'a [&'a str]) -> Vec<&'a str> {
    let tokens = text
        .split(|ch: char| !ch.is_alphanumeric() && ch != '-')
        .filter(|token| !token.is_empty())
        .collect::<BTreeSet<_>>();
    let mut matched = terms
        .iter()
        .copied()
        .filter(|term| {
            if term.contains(' ') {
                text.contains(term)
            } else {
                tokens.contains(term)
            }
        })
        .collect::<Vec<_>>();
    matched.sort_unstable();
    matched.dedup();
    matched
}

fn persist_candidate(
    tx: &mut impl postgres::GenericClient,
    corpus_ref: &str,
    source_ref: &str,
    source_revision_ref: &str,
    statement_ref: Option<&str>,
    batch_ref: Option<&str>,
    exact_span_ref: Option<&str>,
    coordinate_ref: &str,
    evidence_basis_ref: &str,
) -> Result<bool, postgres::Error> {
    let candidate_ref = digest_ref(&[
        "digital-esd-study-coordinate:v1",
        corpus_ref,
        source_ref,
        source_revision_ref,
        coordinate_ref,
        statement_ref.unwrap_or(""),
        batch_ref.unwrap_or(""),
        evidence_basis_ref,
    ]);
    let affected = tx.execute(
        r#"INSERT INTO digital_esd.study_coordinate_candidate (
            candidate_ref, corpus_ref, source_ref, source_revision_ref,
            statement_ref, batch_ref, exact_span_ref, coordinate_ref,
            detector_ref, evidence_basis_ref, candidate_span_count,
            coordinate_paid, review_required, automatic_absence_inference,
            candidate_only, creates_semantic_authority,
            applicability_promoted, claim_truth_promoted
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,1,
            FALSE,TRUE,FALSE,TRUE,FALSE,FALSE,FALSE
        ) ON CONFLICT (candidate_ref) DO NOTHING"#,
        &[
            &candidate_ref,
            &corpus_ref,
            &source_ref,
            &source_revision_ref,
            &statement_ref,
            &batch_ref,
            &exact_span_ref,
            &coordinate_ref,
            &DETECTOR_REF,
            &evidence_basis_ref,
        ],
    )?;
    Ok(affected == 1)
}

pub fn materialize_study_coordinate_candidates(
    config: &DatabaseConfig,
    corpus_ref: &str,
) -> Result<CoordinateNominationReceipt, DigitalEsdCoordinateError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DIGITAL_ESD_COORDINATE_SCHEMA_SQL)?;

    let substrate_ready: bool = client
        .query_one(
            "SELECT
               to_regclass('ingest.generic_source_revision') IS NOT NULL
               AND to_regclass('corpus.source_statement') IS NOT NULL
               AND to_regclass('pnf.statement_candidate_batch') IS NOT NULL
               AND to_regclass('pnf.statement_candidate_factor') IS NOT NULL",
            &[],
        )?
        .get(0);
    if !substrate_ready {
        return Ok(CoordinateNominationReceipt {
            corpus_ref: corpus_ref.to_owned(),
            source_level_candidates: 0,
            statement_level_candidates: 0,
            total_candidates: 0,
            coordinate_paid: false,
            review_required: true,
            automatic_absence_inference: false,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        });
    }

    let source_rows = client.query(
        r#"
        SELECT cs.source_ref, r.source_revision_ref, r.source_family_ref,
               r.ingest_role_class_ref, r.content_digest_ref
        FROM digital_esd.corpus_source cs
        JOIN ingest.generic_source_revision r ON r.source_ref=cs.source_ref
        WHERE cs.corpus_ref=$1
          AND cs.candidate_only
          AND NOT cs.creates_semantic_authority
          AND r.candidate_only
          AND NOT r.creates_semantic_authority
          AND NOT r.applicability_promoted
          AND NOT r.claim_truth_promoted
        ORDER BY cs.source_ref, r.source_revision_ref
        "#,
        &[&corpus_ref],
    )?;

    let mut tx = client.transaction()?;
    let mut source_level = 0usize;
    for row in source_rows {
        let source_ref: String = row.get(0);
        let source_revision_ref: String = row.get(1);
        let family: String = row.get(2);
        let role: String = row.get(3);
        let digest: String = row.get(4);

        source_level += if persist_candidate(
            &mut tx,
            corpus_ref,
            &source_ref,
            &source_revision_ref,
            None,
            None,
            None,
            "sourceIdentityCoordinate",
            &format!("source-revision:{source_revision_ref}"),
        )? { 1 } else { 0 };
        source_level += if persist_candidate(
            &mut tx,
            corpus_ref,
            &source_ref,
            &source_revision_ref,
            None,
            None,
            None,
            "sourceKindAndRoleCoordinate",
            &format!("family:{family}|role:{role}"),
        )? { 1 } else { 0 };
        source_level += if persist_candidate(
            &mut tx,
            corpus_ref,
            &source_ref,
            &source_revision_ref,
            None,
            None,
            None,
            "sameObjectStatusCoordinate",
            &format!("revision:{source_revision_ref}|digest:{digest}"),
        )? { 1 } else { 0 };
    }

    let statement_rows = tx.query(
        r#"
        SELECT r.source_ref, s.source_revision_ref, s.statement_ref,
               b.batch_ref, b.exact_span_ref, s.literal_text
        FROM digital_esd.corpus_source cs
        JOIN ingest.generic_source_revision r ON r.source_ref=cs.source_ref
        JOIN corpus.source_statement s
          ON s.source_revision_ref=r.source_revision_ref
        JOIN pnf.statement_candidate_batch b ON b.statement_ref=s.statement_ref
        WHERE cs.corpus_ref=$1
          AND s.candidate_only
          AND NOT s.creates_semantic_authority
          AND NOT s.applicability_promoted
          AND NOT s.claim_truth_promoted
          AND b.candidate_only
          AND NOT b.semantic_admission_paid
          AND NOT b.proposition_support_paid
          AND NOT b.applicability_paid
          AND NOT b.claim_truth_paid
        ORDER BY r.source_ref, s.exact_span_ref, b.batch_ref
        "#,
        &[&corpus_ref],
    )?;

    let mut statement_level = 0usize;
    for row in statement_rows {
        let source_ref: String = row.get(0);
        let source_revision_ref: String = row.get(1);
        let statement_ref: String = row.get(2);
        let batch_ref: String = row.get(3);
        let exact_span_ref: String = row.get(4);
        let literal_text: String = row.get(5);

        let factor_rows = tx.query(
            r#"
            SELECT candidate_ref, lemma, surface
            FROM pnf.statement_candidate_factor
            WHERE batch_ref=$1 AND candidate_only
            ORDER BY source_start_char, candidate_ref
            "#,
            &[&batch_ref],
        )?;
        let mut lexical_parts = vec![literal_text];
        let mut factor_refs = BTreeSet::new();
        for factor in factor_rows {
            factor_refs.insert(factor.get::<_, String>(0));
            lexical_parts.push(factor.get::<_, String>(1));
            lexical_parts.push(factor.get::<_, String>(2));
        }
        let normalized = normalize(&lexical_parts.join(" "));

        for rule in RULES {
            let hits = matched_terms(&normalized, rule.terms);
            if hits.is_empty() {
                continue;
            }
            let evidence_basis_ref = format!(
                "pnf:{}|terms:{}|factors:{}",
                batch_ref,
                hits.join(","),
                factor_refs.iter().cloned().collect::<Vec<_>>().join(",")
            );
            statement_level += if persist_candidate(
                &mut tx,
                corpus_ref,
                &source_ref,
                &source_revision_ref,
                Some(&statement_ref),
                Some(&batch_ref),
                Some(&exact_span_ref),
                rule.coordinate_ref,
                &evidence_basis_ref,
            )? { 1 } else { 0 };
        }
    }

    tx.commit()?;
    Ok(CoordinateNominationReceipt {
        corpus_ref: corpus_ref.to_owned(),
        source_level_candidates: source_level,
        statement_level_candidates: statement_level,
        total_candidates: source_level + statement_level,
        coordinate_paid: false,
        review_required: true,
        automatic_absence_inference: false,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_matching_nominates_without_absence_inference() {
        let text = normalize(
            "Students used a digital platform for sustainability learning; limitations remain.",
        );
        let coordinates = RULES
            .iter()
            .filter(|rule| !matched_terms(&text, rule.terms).is_empty())
            .map(|rule| rule.coordinate_ref)
            .collect::<BTreeSet<_>>();

        assert!(coordinates.contains("populationEducationLevelCoordinate"));
        assert!(coordinates.contains("digitalTechnologyOrPracticeCoordinate"));
        assert!(coordinates.contains("pedagogyCurriculumCompetenceCoordinate"));
        assert!(coordinates.contains("sustainabilityDimensionCoordinate"));
        assert!(coordinates.contains("uncertaintyLimitationCoordinate"));
        assert!(!coordinates.contains("circularityRepairabilityCoordinate"));
    }

    #[test]
    fn runtime_vocabulary_is_exactly_the_nineteen_coordinate_schema() {
        assert_eq!(CANONICAL_19_COORDINATES.len(), 19);
        assert!(CANONICAL_19_COORDINATES.contains(&"publicationDateCoordinate"));
        assert!(CANONICAL_19_COORDINATES.contains(&"sameObjectStatusCoordinate"));
    }

    #[test]
    fn candidate_identity_is_deterministic() {
        assert_eq!(
            digest_ref(&["a", "b", "c"]),
            digest_ref(&["a", "b", "c"])
        );
        assert_ne!(
            digest_ref(&["a", "b", "c"]),
            digest_ref(&["a", "b", "d"])
        );
    }
}
