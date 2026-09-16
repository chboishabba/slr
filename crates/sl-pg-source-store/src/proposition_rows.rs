use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};

use crate::{DatabaseConfig, SourceStoreError};

/// Storage-owned projection of one PNF observation. The storage layer keeps
/// observation provenance distinct from graph/source-span provenance; it does
/// not decide whether the observation pays a semantic obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionObservationRow {
    pub observation_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub role_ref: String,
    pub observation_provenance_refs: Vec<String>,
    pub graph_source_span_refs: Vec<String>,
    pub residual_refs: Vec<String>,
}

/// Explicit reader-facing debt retained in PostgreSQL for a bounded proof role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionResidualRow {
    pub role_ref: String,
    pub residual_ref: String,
}

/// Typed read projection for one proposition/span pair. This is deliberately
/// not a payment result: `sl-evidence-payment` remains the owner of payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionRows {
    pub proposition_ref: String,
    pub required_span_ref: String,
    pub exact_source_paid: bool,
    pub observations: Vec<PropositionObservationRow>,
    pub residuals: Vec<PropositionResidualRow>,
}

/// Load the persisted PNF + independent graph-span coordinates for the Mabo
/// reader proposition. Unknown role bindings are ignored rather than guessed.
///
/// Exact-source readiness is checked against the canonical document and span
/// bounds. No result from this function pays proposition support,
/// applicability, or claim truth.
pub fn load_mabo_proposition_rows(
    config: &DatabaseConfig,
    proposition_ref: &str,
    required_span_ref: &str,
) -> Result<PropositionRows, SourceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;

    let exact_source_paid: bool = client
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM legal_ir.graph_revision AS gr
                JOIN legal_ir.semantic_build AS sb
                  ON sb.build_ref = gr.build_ref
                JOIN legal_source_revision AS l
                  ON l.source_revision_ref = sb.source_revision_ref
                 AND l.document_ref = sb.document_ref
                 AND l.compile_eligible = TRUE
                JOIN corpus.document AS d
                  ON d.document_ref = l.document_ref
                JOIN corpus.canonical_content AS c
                  ON c.canonical_ref = d.canonical_ref
                JOIN corpus.span AS s
                  ON s.document_ref = l.document_ref
                 AND s.span_ref = $2
                WHERE gr.subject_ref = $1
                  AND $2 = ANY(gr.source_span_refs)
                  AND s.start_char IS NOT NULL
                  AND s.end_char IS NOT NULL
                  AND s.start_char >= 0
                  AND s.start_char < s.end_char
                  AND s.end_char <= char_length(convert_from(c.payload, 'UTF8'))
            )
            "#,
            &[&proposition_ref, &required_span_ref],
        )?
        .get(0);

    let observation_rows = client.query(
        r#"
        WITH target_graph AS (
            SELECT gr.build_ref, gr.source_span_refs
            FROM legal_ir.graph_revision AS gr
            WHERE gr.subject_ref = $1
              AND $2 = ANY(gr.source_span_refs)
        )
        SELECT o.observation_ref,
               o.pnf_factor_ref,
               o.pnf_revision_ref,
               COALESCE(
                   o.role_bindings ->> 'proof_role',
                   o.role_bindings ->> 'proposition_role',
                   o.role_bindings ->> 'reader_role',
                   o.role_bindings ->> 'role'
               ) AS role_ref,
               o.provenance_refs,
               target_graph.source_span_refs,
               o.residual_refs
        FROM target_graph
        JOIN legal_ir.projection AS p
          ON p.build_ref = target_graph.build_ref
        JOIN legal_ir.observation AS o
          ON o.projection_ref = p.projection_ref
        WHERE COALESCE(
                  o.role_bindings ->> 'proposition_ref',
                  o.role_bindings ->> 'subject_ref',
                  o.role_bindings ->> 'semantic_ref',
                  o.role_bindings ->> 'claim_ref',
                  o.role_bindings ->> 'reader_proposition_ref'
              ) = $1
          AND COALESCE(
                  o.role_bindings ->> 'proof_role',
                  o.role_bindings ->> 'proposition_role',
                  o.role_bindings ->> 'reader_role',
                  o.role_bindings ->> 'role'
              ) IN ('support', 'qualifier', 'defeater', 'comparator')
        "#,
        &[&proposition_ref, &required_span_ref],
    )?;

    // A proposition may have more than one graph revision over the same build.
    // Merge exact duplicate semantic observations while unioning the independent
    // graph source-span coordinates.
    let mut observations_by_ref: BTreeMap<String, PropositionObservationRow> = BTreeMap::new();
    for row in observation_rows {
        let observation_ref: String = row.get(0);
        let graph_source_span_refs: Vec<String> = row.get(5);
        if let Some(existing) = observations_by_ref.get_mut(&observation_ref) {
            let mut spans: BTreeSet<String> = existing.graph_source_span_refs.drain(..).collect();
            spans.extend(graph_source_span_refs);
            existing.graph_source_span_refs = spans.into_iter().collect();
            continue;
        }
        observations_by_ref.insert(
            observation_ref.clone(),
            PropositionObservationRow {
                observation_ref,
                pnf_factor_ref: row.get(1),
                pnf_revision_ref: row.get(2),
                role_ref: row.get(3),
                observation_provenance_refs: row.get(4),
                graph_source_span_refs,
                residual_refs: row.get(6),
            },
        );
    }

    // Residual roles remain explicit. We accept only role-labelled residual
    // coordinates; an arbitrary projection residual is never promoted into a
    // qualifier/defeater/comparator debt by string similarity alone.
    let residual_rows = client.query(
        r#"
        WITH target_graph AS (
            SELECT DISTINCT gr.build_ref
            FROM legal_ir.graph_revision AS gr
            WHERE gr.subject_ref = $1
              AND $2 = ANY(gr.source_span_refs)
        ), target_observation AS (
            SELECT o.role_bindings
            FROM target_graph
            JOIN legal_ir.projection AS p
              ON p.build_ref = target_graph.build_ref
            JOIN legal_ir.observation AS o
              ON o.projection_ref = p.projection_ref
            WHERE COALESCE(
                      o.role_bindings ->> 'proposition_ref',
                      o.role_bindings ->> 'subject_ref',
                      o.role_bindings ->> 'semantic_ref',
                      o.role_bindings ->> 'claim_ref',
                      o.role_bindings ->> 'reader_proposition_ref'
                  ) = $1
        )
        SELECT role_ref, residual_ref
        FROM target_observation
        CROSS JOIN LATERAL (
            VALUES
              ('qualifier', COALESCE(
                  role_bindings ->> 'qualifier_residual_ref',
                  role_bindings #>> '{residual_roles,qualifier}'
              )),
              ('defeater', COALESCE(
                  role_bindings ->> 'defeater_residual_ref',
                  role_bindings #>> '{residual_roles,defeater}'
              )),
              ('comparator', COALESCE(
                  role_bindings ->> 'comparator_residual_ref',
                  role_bindings #>> '{residual_roles,comparator}'
              ))
        ) AS residual(role_ref, residual_ref)
        WHERE residual_ref IS NOT NULL
          AND residual_ref <> ''
        "#,
        &[&proposition_ref, &required_span_ref],
    )?;

    let residuals = residual_rows
        .into_iter()
        .map(|row| PropositionResidualRow {
            role_ref: row.get(0),
            residual_ref: row.get(1),
        })
        .collect::<Vec<_>>();

    Ok(PropositionRows {
        proposition_ref: proposition_ref.to_owned(),
        required_span_ref: required_span_ref.to_owned(),
        exact_source_paid,
        observations: observations_by_ref.into_values().collect(),
        residuals,
    })
}
