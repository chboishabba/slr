-- Original SensibLaw m080, donor blob 8b5813d7c9d1527d43a05411126f8f37b35dc8be at f9c4670ef04b40e8153caa7dd00fa1eba7013ef4.
-- Target-scoped specialization for native SLR rollback comparison ONLY.
-- Requires complete affected target set and original pre-admission candidate state.
-- No new authority: exact original m080 direct/corroborated witness rules.
-- External shared-witness closure obstruction fails closed.
CREATE OR REPLACE FUNCTION pg_temp.slr_admit_numeric_pnf_parser_identity_targets(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    selected_targets BIGINT[]
)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    admitted_count BIGINT := 0;
BEGIN
    IF selected_targets IS NULL OR array_position(selected_targets,NULL) IS NOT NULL
       OR EXISTS(SELECT 1 FROM unnest(selected_targets) t(id) WHERE id<=0)
       OR EXISTS(SELECT 1 FROM unnest(selected_targets) t(id)
          WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_object o
            JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
            WHERE o.object_id=t.id AND r.run_id=selected_run_id AND r.document_id=selected_document_id))
    THEN RAISE EXCEPTION 'invalid identity target scope'; END IF;
    IF cardinality(selected_targets)=0 THEN RETURN 0; END IF;
    -- A witness shared with an outside candidate is an explicit scope
    -- obstruction, not permission to revoke another target's authority.
    IF EXISTS(
      SELECT 1 FROM execution.semantic_pnf_identity_evidence_candidate c
      JOIN execution.semantic_pnf_identity_evidence_witness p ON p.candidate_id=c.candidate_id
      JOIN execution.semantic_pnf_identity_evidence_witness other ON other.witness_id=p.witness_id
      JOIN execution.semantic_pnf_identity_evidence_candidate d ON d.candidate_id=other.candidate_id
      WHERE c.run_id=selected_run_id AND c.document_id=selected_document_id
        AND c.target_object_id=ANY(selected_targets)
        AND (d.run_id<>selected_run_id OR d.document_id<>selected_document_id
          OR NOT d.target_object_id=ANY(selected_targets))
    ) THEN RAISE EXCEPTION 'identity target scope has external witness dependency'; END IF;

    -- Retract only witnesses whose current authority came from parser evidence in
    -- this document.  Typed-demand anchors/anaphor witnesses are untouched.
    UPDATE execution.semantic_pnf_identity_witness_admission AS admission
       SET admission_state = 4,
           revision = admission.revision + 1,
           updated_at = CURRENT_TIMESTAMP
      FROM execution.semantic_pnf_identity_evidence_witness AS provenance
      JOIN execution.semantic_pnf_identity_evidence_candidate AS candidate
        ON candidate.candidate_id = provenance.candidate_id
     WHERE admission.witness_id = provenance.witness_id
       AND admission.admission_state = 2
       AND candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets);

    INSERT INTO execution.semantic_pnf_canonical_entity
        (entity_ref, authority_class, canonical_symbol_id, anchor_object_id)
    SELECT DISTINCT
           'document-object:' || encode(target.object_digest, 'hex'),
           2,
           target.head_symbol_id,
           target.object_id
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_object AS target
        ON target.object_id = candidate.target_object_id
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.candidate_count = 1
       AND candidate.witness_kind IN (2, 4, 6)
    ON CONFLICT DO NOTHING;

    -- Strong evidence: create/reuse anchor and source witnesses.
    INSERT INTO execution.semantic_pnf_identity_witness
        (source_object_id, target_entity_id, witness_kind, authority_class,
         candidate_count)
    SELECT DISTINCT candidate.target_object_id,
           entity.entity_id, 1, 2, 1
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.anchor_object_id = candidate.target_object_id
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.candidate_count = 1
       AND candidate.witness_kind IN (2, 4, 6)
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_identity_witness
        (source_object_id, target_entity_id, witness_kind, authority_class,
         source_interface_id, candidate_count)
    SELECT candidate.source_object_id,
           entity.entity_id,
           candidate.witness_kind,
           2,
           interface.interface_id,
           1
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.anchor_object_id = candidate.target_object_id
      JOIN execution.semantic_pnf_object AS source
        ON source.object_id = candidate.source_object_id
      LEFT JOIN execution.semantic_pnf_interface AS interface
        ON interface.region_id = source.region_id
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.candidate_count = 1
       AND candidate.witness_kind IN (2, 4, 6)
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_identity_evidence_witness
        (candidate_id, witness_id, witness_role)
    SELECT candidate.candidate_id,
           anchor.witness_id,
           1
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.anchor_object_id = candidate.target_object_id
      JOIN execution.semantic_pnf_identity_witness AS anchor
        ON anchor.source_object_id = candidate.target_object_id
       AND anchor.target_entity_id = entity.entity_id
       AND anchor.witness_kind = 1
       AND anchor.authority_class = 2
       AND anchor.candidate_count = 1
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.candidate_count = 1
       AND candidate.witness_kind IN (2, 4, 6)
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_identity_evidence_witness
        (candidate_id, witness_id, witness_role)
    SELECT candidate.candidate_id,
           witness.witness_id,
           2
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.anchor_object_id = candidate.target_object_id
      JOIN execution.semantic_pnf_identity_witness AS witness
        ON witness.source_object_id = candidate.source_object_id
       AND witness.target_entity_id = entity.entity_id
       AND witness.witness_kind = candidate.witness_kind
       AND witness.authority_class = 2
       AND witness.candidate_count = 1
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.candidate_count = 1
       AND candidate.witness_kind IN (2, 4, 6)
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_identity_witness_admission
        (witness_id, admission_state, revision)
    SELECT DISTINCT provenance.witness_id, 2, 1
      FROM execution.semantic_pnf_identity_evidence_witness AS provenance
      JOIN execution.semantic_pnf_identity_evidence_candidate AS candidate
        ON candidate.candidate_id = provenance.candidate_id
      JOIN execution.semantic_pnf_identity_witness AS witness
        ON witness.witness_id = provenance.witness_id
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.entity_id = witness.target_entity_id
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.candidate_count = 1
       AND candidate.witness_kind IN (2, 4, 6)
       AND witness.candidate_count = 1
       AND witness.authority_class = entity.authority_class
    ON CONFLICT (witness_id) DO UPDATE SET
        admission_state = 2,
        revision = execution.semantic_pnf_identity_witness_admission.revision + 1,
        updated_at = CURRENT_TIMESTAMP;

    -- Corroborated name expansion.  Its target must already carry an accepted
    -- non-anchor proof independent of this name-expansion candidate.
    INSERT INTO execution.semantic_pnf_identity_witness
        (source_object_id, target_entity_id, witness_kind, authority_class,
         source_interface_id, candidate_count)
    SELECT candidate.source_object_id,
           entity.entity_id,
           3,
           2,
           interface.interface_id,
           1
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.anchor_object_id = candidate.target_object_id
      JOIN execution.semantic_pnf_object AS source
        ON source.object_id = candidate.source_object_id
      LEFT JOIN execution.semantic_pnf_interface AS interface
        ON interface.region_id = source.region_id
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.witness_kind = 3
       AND candidate.candidate_count = 1
       AND EXISTS (
           SELECT 1
             FROM execution.semantic_pnf_identity_witness AS corroborating
             JOIN execution.semantic_pnf_identity_witness_admission AS corroborating_admission
               ON corroborating_admission.witness_id = corroborating.witness_id
              AND corroborating_admission.admission_state = 2
            WHERE corroborating.target_entity_id = entity.entity_id
              AND corroborating.witness_kind IN (2, 4, 5, 6, 8)
       )
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_identity_evidence_witness
        (candidate_id, witness_id, witness_role)
    SELECT candidate.candidate_id,
           witness.witness_id,
           2
      FROM execution.semantic_pnf_identity_evidence_candidate AS candidate
      JOIN execution.semantic_pnf_canonical_entity AS entity
        ON entity.anchor_object_id = candidate.target_object_id
      JOIN execution.semantic_pnf_identity_witness AS witness
        ON witness.source_object_id = candidate.source_object_id
       AND witness.target_entity_id = entity.entity_id
       AND witness.witness_kind = 3
       AND witness.authority_class = 2
       AND witness.candidate_count = 1
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.witness_kind = 3
       AND candidate.candidate_count = 1
       AND EXISTS (
           SELECT 1
             FROM execution.semantic_pnf_identity_witness AS corroborating
             JOIN execution.semantic_pnf_identity_witness_admission AS corroborating_admission
               ON corroborating_admission.witness_id = corroborating.witness_id
              AND corroborating_admission.admission_state = 2
            WHERE corroborating.target_entity_id = entity.entity_id
              AND corroborating.witness_kind IN (2, 4, 5, 6, 8)
       )
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_identity_witness_admission
        (witness_id, admission_state, revision)
    SELECT provenance.witness_id, 2, 1
      FROM execution.semantic_pnf_identity_evidence_witness AS provenance
      JOIN execution.semantic_pnf_identity_evidence_candidate AS candidate
        ON candidate.candidate_id = provenance.candidate_id
      JOIN execution.semantic_pnf_identity_witness AS witness
        ON witness.witness_id = provenance.witness_id
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state = 1
       AND candidate.witness_kind = 3
       AND candidate.candidate_count = 1
       AND provenance.witness_role = 2
       AND EXISTS (
           SELECT 1
             FROM execution.semantic_pnf_identity_witness AS corroborating
             JOIN execution.semantic_pnf_identity_witness_admission AS corroborating_admission
               ON corroborating_admission.witness_id = corroborating.witness_id
              AND corroborating_admission.admission_state = 2
            WHERE corroborating.target_entity_id = witness.target_entity_id
              AND corroborating.witness_kind IN (2, 4, 5, 6, 8)
       )
    ON CONFLICT (witness_id) DO UPDATE SET
        admission_state = 2,
        revision = execution.semantic_pnf_identity_witness_admission.revision + 1,
        updated_at = CURRENT_TIMESTAMP;

    UPDATE execution.semantic_pnf_identity_evidence_candidate AS candidate
       SET evidence_state = CASE
           WHEN candidate.candidate_count > 1 THEN 3
           WHEN EXISTS (
               SELECT 1
                 FROM execution.semantic_pnf_identity_evidence_witness AS provenance
                 JOIN execution.semantic_pnf_identity_witness_admission AS admission
                   ON admission.witness_id = provenance.witness_id
                  AND admission.admission_state = 2
                WHERE provenance.candidate_id = candidate.candidate_id
                  AND provenance.witness_role = 2
           ) THEN 2
           ELSE 1
       END
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND candidate.evidence_state IN (1, 2, 3);

    SELECT count(DISTINCT provenance.witness_id)
      INTO admitted_count
      FROM execution.semantic_pnf_identity_evidence_witness AS provenance
      JOIN execution.semantic_pnf_identity_evidence_candidate AS candidate
        ON candidate.candidate_id = provenance.candidate_id
      JOIN execution.semantic_pnf_identity_witness_admission AS admission
        ON admission.witness_id = provenance.witness_id
       AND admission.admission_state = 2
     WHERE candidate.run_id = selected_run_id
       AND candidate.document_id = selected_document_id
       AND candidate.target_object_id = ANY(selected_targets)
       AND provenance.witness_role = 2;
    RETURN admitted_count;
END;
$$;
