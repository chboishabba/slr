use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};

use crate::{DatabaseConfig, SourceStoreError};

/// Storage-owned projection of one PNF observation welded to an independently
/// persisted proposition graph span. The storage layer does not assign the
/// reader's qualifier/defeater/comparator roles or decide semantic payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionObservationRow {
    pub observation_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub observation_provenance_refs: Vec<String>,
    pub graph_source_span_refs: Vec<String>,
    pub residual_refs: Vec<String>,
}

/// Typed read projection for one proposition/span pair. This is deliberately
/// not a payment result: `sl-evidence-payment` remains the owner of payment and
/// consumer-relative role debts are created above the persistence layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionRows {
    pub proposition_ref: String,
    pub required_span_ref: String,
    pub exact_source_paid: bool,
    pub observations: Vec<PropositionObservationRow>,
}

/// Load the persisted PNF + independent graph-span coordinates for the Mabo
/// reader proposition.
///
/// The graph revision establishes the proposition/span relation. A returned PNF
/// observation must independently retain that same span in its own provenance.
/// That conjunction is the storage-side input required by the SLR support weld;
/// this function itself does not pay support, applicability, or claim truth.
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
               o.provenance_refs,
               target_graph.source_span_refs,
               o.residual_refs
        FROM target_graph
        JOIN legal_ir.projection AS p
          ON p.build_ref = target_graph.build_ref
        JOIN legal_ir.observation AS o
          ON o.projection_ref = p.projection_ref
        WHERE $2 = ANY(o.provenance_refs)
          AND o.pnf_factor_ref <> ''
          AND o.pnf_revision_ref <> ''
        "#,
        &[&proposition_ref, &required_span_ref],
    )?;

    // A proposition may have more than one graph revision over the same build.
    // Merge duplicate PNF observations while unioning only the independently
    // persisted graph source-span coordinates.
    let mut observations_by_ref: BTreeMap<String, PropositionObservationRow> = BTreeMap::new();
    for row in observation_rows {
        let observation_ref: String = row.get(0);
        let graph_source_span_refs: Vec<String> = row.get(4);
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
                observation_provenance_refs: row.get(3),
                graph_source_span_refs,
                residual_refs: row.get(5),
            },
        );
    }

    Ok(PropositionRows {
        proposition_ref: proposition_ref.to_owned(),
        required_span_ref: required_span_ref.to_owned(),
        exact_source_paid,
        observations: observations_by_ref.into_values().collect(),
    })
}
