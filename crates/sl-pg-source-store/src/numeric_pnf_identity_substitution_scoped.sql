-- Native SLR rollback-only exact factor-scope refinement of m072/m086.
-- No independent identity authority; both producers consume original projection views.
-- database/postgres_migrations/072_retractable_identity_and_external_alignment.sql blob 56e74f94ca606a7f9fde58c60a59d64825c8cb2e; 5 owner statements restricted to factors.
CREATE OR REPLACE FUNCTION pg_temp.slr_refresh_identity_substitution_factors(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    selected_factors BIGINT[]
)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    affected_count BIGINT := 0;
BEGIN
    IF selected_factors IS NULL OR array_position(selected_factors,NULL) IS NOT NULL
       OR EXISTS(SELECT 1 FROM unnest(selected_factors) AS ids(id)
          WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_factor f
            JOIN execution.semantic_pnf_region r ON r.region_id=f.region_id
            WHERE f.factor_id=ids.id AND r.run_id=selected_run_id AND r.document_id=selected_document_id))
    THEN RAISE EXCEPTION 'invalid identity substitution factor scope'; END IF;
    IF cardinality(selected_factors)=0 THEN RETURN 0; END IF;
    DELETE FROM execution.semantic_pnf_factor_derivation AS derivation
    USING execution.semantic_pnf_interface AS interface,
          execution.semantic_pnf_region AS region
     WHERE derivation.rule_ref = 'identity-substitution:v1'
       AND derivation.scope_interface_id = interface.interface_id
       AND interface.region_id = region.region_id
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND split_part(derivation.derivation_ref, ':', 2)::BIGINT = ANY(selected_factors);

    INSERT INTO execution.semantic_pnf_factor_derivation
        (derivation_ref, rule_ref, derivation_kind, derivation_state,
         epistemic_level, authority_class, scope_interface_id,
         conclusion_factor_type_symbol_id, conclusion_predicate_symbol_id,
         modal_state, temporal_state)
    SELECT 'identity-substitution:' || factor.factor_id::TEXT || ':'
               || projection.authority_class::TEXT,
           'identity-substitution:v1',
           1,
           2,
           3,
           projection.authority_class,
           interface.interface_id,
           factor.factor_type_symbol_id,
           factor.predicate_symbol_id,
           factor.modal_state,
           factor.temporal_state
      FROM execution.semantic_pnf_factor AS factor
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
      JOIN execution.semantic_pnf_interface AS interface
        ON interface.region_id = region.region_id
      JOIN execution.semantic_pnf_hyperedge AS edge
        ON edge.factor_id = factor.factor_id
      JOIN execution.semantic_pnf_identity_projection AS projection
        ON projection.source_object_id = edge.object_id
     WHERE region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors)
     GROUP BY factor.factor_id,
              projection.authority_class,
              interface.interface_id,
              factor.factor_type_symbol_id,
              factor.predicate_symbol_id,
              factor.modal_state,
              factor.temporal_state
    ON CONFLICT (derivation_ref) DO NOTHING;

    INSERT INTO execution.semantic_pnf_factor_derivation_premise
        (derivation_id, premise_ordinal, factor_id)
    SELECT derivation.derivation_id,
           0,
           split_part(derivation.derivation_ref, ':', 2)::BIGINT
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_interface AS interface
        ON interface.interface_id = derivation.scope_interface_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = interface.region_id
     WHERE derivation.rule_ref = 'identity-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND split_part(derivation.derivation_ref, ':', 2)::BIGINT = ANY(selected_factors)
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_factor_derivation_argument
        (derivation_id, slot_ordinal, role_symbol_id, source_object_id,
         local_object_id, identity_entity_id, identity_witness_ids)
    SELECT derivation.derivation_id,
           edge.slot_ordinal,
           edge.role_symbol_id,
           edge.object_id,
           CASE WHEN projection.target_entity_id IS NULL
                THEN edge.object_id ELSE NULL END,
           projection.target_entity_id,
           projection.witness_ids
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_factor_derivation_premise AS premise
        ON premise.derivation_id = derivation.derivation_id
       AND premise.premise_ordinal = 0
      JOIN execution.semantic_pnf_factor AS factor
        ON factor.factor_id = premise.factor_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
      JOIN execution.semantic_pnf_hyperedge AS edge
        ON edge.factor_id = factor.factor_id
      LEFT JOIN execution.semantic_pnf_identity_projection AS projection
        ON projection.source_object_id = edge.object_id
       AND projection.authority_class = derivation.authority_class
     WHERE derivation.rule_ref = 'identity-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors)
    ON CONFLICT (derivation_id, slot_ordinal) DO NOTHING;

    SELECT count(*)
      INTO affected_count
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_interface AS interface
        ON interface.interface_id = derivation.scope_interface_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = interface.region_id
     WHERE derivation.rule_ref = 'identity-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND split_part(derivation.derivation_ref, ':', 2)::BIGINT = ANY(selected_factors);

    RETURN affected_count;
END;
$$;

-- database/postgres_migrations/086_consumer_indexed_reopenable_runtime.sql blob 908401502686036250ccf0a988d572748efe1a36; 5 owner statements restricted to factors.
CREATE OR REPLACE FUNCTION pg_temp.slr_refresh_supported_identity_substitution_factors(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    selected_factors BIGINT[]
)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE affected_count BIGINT := 0;
BEGIN
    IF selected_factors IS NULL OR array_position(selected_factors,NULL) IS NOT NULL
       OR EXISTS(SELECT 1 FROM unnest(selected_factors) AS ids(id)
          WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_factor f
            JOIN execution.semantic_pnf_region r ON r.region_id=f.region_id
            WHERE f.factor_id=ids.id AND r.run_id=selected_run_id AND r.document_id=selected_document_id))
    THEN RAISE EXCEPTION 'invalid identity substitution factor scope'; END IF;
    IF cardinality(selected_factors)=0 THEN RETURN 0; END IF;
    -- Supported m086 producer is insert-only. Rebuild exactly the selected
    -- current projection, never stale slots or unrelated premises.
    DELETE FROM execution.semantic_pnf_factor_derivation d
    USING execution.semantic_pnf_interface i,execution.semantic_pnf_region r
     WHERE d.rule_ref='identity-supported-substitution:v1'
       AND d.scope_interface_id=i.interface_id AND i.region_id=r.region_id
       AND r.run_id=selected_run_id AND r.document_id=selected_document_id
       AND split_part(d.derivation_ref,':',2)::bigint=ANY(selected_factors);

    -- Create one Level-3 derivation per factor/authority only when at least one
    -- argument gains identity applicability through exact token support that is
    -- not already the direct edge object.
    INSERT INTO execution.semantic_pnf_factor_derivation
        (derivation_ref, rule_ref, derivation_kind, derivation_state,
         epistemic_level, authority_class, scope_interface_id,
         conclusion_factor_type_symbol_id, conclusion_predicate_symbol_id,
         modal_state, temporal_state)
    SELECT 'identity-supported-substitution:' || factor.factor_id::TEXT || ':'
               || support.authority_class::TEXT,
           'identity-supported-substitution:v1',
           1,
           2,
           3,
           support.authority_class,
           interface.interface_id,
           factor.factor_type_symbol_id,
           factor.predicate_symbol_id,
           factor.modal_state,
           factor.temporal_state
      FROM execution.semantic_pnf_factor AS factor
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
      JOIN execution.semantic_pnf_interface AS interface
        ON interface.region_id = region.region_id
      JOIN execution.semantic_pnf_factor_identity_support_unique_v1 AS support
        ON support.factor_id = factor.factor_id
       AND support.support_kind = 2
     WHERE region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors)
     GROUP BY factor.factor_id, support.authority_class,
              interface.interface_id,
              factor.factor_type_symbol_id,
              factor.predicate_symbol_id,
              factor.modal_state,
              factor.temporal_state
    ON CONFLICT (derivation_ref) DO NOTHING;

    INSERT INTO execution.semantic_pnf_factor_derivation_premise
        (derivation_id, premise_ordinal, factor_id)
    SELECT derivation.derivation_id, 0, factor.factor_id
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_factor AS factor
        ON derivation.derivation_ref =
           'identity-supported-substitution:' || factor.factor_id::TEXT || ':'
           || derivation.authority_class::TEXT
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
     WHERE derivation.rule_ref = 'identity-supported-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors)
    ON CONFLICT DO NOTHING;

    INSERT INTO execution.semantic_pnf_factor_derivation_argument
        (derivation_id, slot_ordinal, role_symbol_id, source_object_id,
         local_object_id, identity_entity_id, identity_witness_ids)
    SELECT derivation.derivation_id,
           edge.slot_ordinal,
           edge.role_symbol_id,
           edge.object_id,
           CASE WHEN support.target_entity_id IS NULL
                THEN edge.object_id ELSE NULL END,
           support.target_entity_id,
           support.witness_ids
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_factor_derivation_premise AS premise
        ON premise.derivation_id = derivation.derivation_id
       AND premise.premise_ordinal = 0
      JOIN execution.semantic_pnf_factor AS factor
        ON factor.factor_id = premise.factor_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
      JOIN execution.semantic_pnf_hyperedge AS edge
        ON edge.factor_id = factor.factor_id
      LEFT JOIN execution.semantic_pnf_factor_identity_support_unique_v1 AS support
        ON support.factor_id = edge.factor_id
       AND support.slot_ordinal = edge.slot_ordinal
       AND support.authority_class = derivation.authority_class
     WHERE derivation.rule_ref = 'identity-supported-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors)
    ON CONFLICT (derivation_id, slot_ordinal) DO NOTHING;

    INSERT INTO execution.semantic_pnf_factor_derivation_support
        (derivation_id, slot_ordinal, identity_source_object_id,
         target_entity_id, authority_class, support_kind, support_token_id,
         identity_witness_ids)
    SELECT derivation.derivation_id,
           edge.slot_ordinal,
           support.identity_source_object_id,
           support.target_entity_id,
           support.authority_class,
           support.support_kind,
           support.support_token_id,
           support.witness_ids
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_factor_derivation_premise AS premise
        ON premise.derivation_id = derivation.derivation_id
       AND premise.premise_ordinal = 0
      JOIN execution.semantic_pnf_factor AS factor
        ON factor.factor_id = premise.factor_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
      JOIN execution.semantic_pnf_hyperedge AS edge
        ON edge.factor_id = factor.factor_id
      JOIN execution.semantic_pnf_factor_identity_support_unique_v1 AS support
        ON support.factor_id = edge.factor_id
       AND support.slot_ordinal = edge.slot_ordinal
       AND support.authority_class = derivation.authority_class
     WHERE derivation.rule_ref = 'identity-supported-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors)
    ON CONFLICT (derivation_id, slot_ordinal) DO NOTHING;

    SELECT count(*)
      INTO affected_count
      FROM execution.semantic_pnf_factor_derivation AS derivation
      JOIN execution.semantic_pnf_factor_derivation_premise AS premise
        ON premise.derivation_id = derivation.derivation_id
      JOIN execution.semantic_pnf_factor AS factor
        ON factor.factor_id = premise.factor_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
     WHERE derivation.rule_ref = 'identity-supported-substitution:v1'
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND factor.factor_id = ANY(selected_factors);
    RETURN affected_count;
END;
$$;
