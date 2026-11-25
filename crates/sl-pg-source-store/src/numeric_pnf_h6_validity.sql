-- Source: original SensibLaw m110 at f9c4670ef04b40e8153caa7dd00fa1eba7013ef4, blob 41bf2232b43a53fb8e0373559206e5fec430d629.
-- Reopen the actual discourse/temporal signature relation independently of
-- scheduling/stop state. An already-completed horizon or later stop does not
-- withdraw its physical support. This relation is evidence validity only.
-- Never use absence from this relation as semantic candidate refutation.
CREATE OR REPLACE FUNCTION pg_temp.slr_current_m110_h6_atoms(
    selected_demand_id bigint
) RETURNS TABLE(
    demand_id bigint,target_kind smallint,target_id bigint,evidence_ref text,
    signed_residual bigint,evidence_kind text,provenance_ref text,
    source_interface_id bigint,source_region_id bigint
) LANGUAGE sql STABLE AS $$
    WITH ready AS MATERIALIZED (
        SELECT d.demand_id,d.source_object_id,d.source_interface_id,
               d.source_region_id,d.expected_factor_type_symbol_id,d.role_symbol_id
        FROM execution.semantic_pnf_demand d
        WHERE d.demand_id=selected_demand_id AND d.source_object_id IS NOT NULL
    ), source_signature AS MATERIALIZED (
        SELECT DISTINCT ready.demand_id,ready.source_object_id,
               ready.source_interface_id,ready.source_region_id,
               factor.factor_type_symbol_id,factor.predicate_symbol_id,
               edge.role_symbol_id,factor.temporal_state
          FROM ready
          JOIN execution.semantic_pnf_hyperedge AS edge
            ON edge.object_id=ready.source_object_id
          JOIN execution.semantic_pnf_factor AS factor
            ON factor.factor_id=edge.factor_id AND factor.active
         WHERE (ready.expected_factor_type_symbol_id IS NULL
                OR factor.factor_type_symbol_id=ready.expected_factor_type_symbol_id)
           AND (ready.role_symbol_id IS NULL
                OR edge.role_symbol_id=ready.role_symbol_id)
    ), candidate AS MATERIALIZED (
        SELECT DISTINCT state.demand_id,state.target_kind,state.target_id
          FROM execution.semantic_pnf_candidate_state_v1 AS state
          JOIN ready ON ready.demand_id=state.demand_id
         WHERE state.target_kind=1
           AND state.represented_possible
           AND state.admissible
           AND state.target_id<>ready.source_object_id
    ), matched AS MATERIALIZED (
        SELECT DISTINCT candidate.demand_id,candidate.target_kind,candidate.target_id,
               source_signature.source_interface_id,source_signature.source_region_id,
               source_signature.factor_type_symbol_id,
               source_signature.predicate_symbol_id,source_signature.role_symbol_id,
               source_signature.temporal_state AS source_temporal_state,
               candidate_factor.temporal_state AS candidate_temporal_state
          FROM candidate
          JOIN execution.semantic_pnf_hyperedge AS candidate_edge
            ON candidate_edge.object_id=candidate.target_id
          JOIN execution.semantic_pnf_factor AS candidate_factor
            ON candidate_factor.factor_id=candidate_edge.factor_id
           AND candidate_factor.active
          JOIN source_signature
            ON source_signature.demand_id=candidate.demand_id
           AND source_signature.role_symbol_id=candidate_edge.role_symbol_id
           AND source_signature.factor_type_symbol_id=candidate_factor.factor_type_symbol_id
           AND source_signature.predicate_symbol_id=candidate_factor.predicate_symbol_id
    ), evidence_rows AS (
        SELECT matched.demand_id,matched.target_kind,matched.target_id,
               'h6:discourse:' || matched.factor_type_symbol_id::TEXT || ':' ||
                   matched.predicate_symbol_id::TEXT || ':' || matched.role_symbol_id::TEXT
                   AS evidence_ref,
               1::BIGINT AS signed_residual,
               'h6_discourse_factor_role_signature'::TEXT AS evidence_kind,
               'numeric-factor-signature'::TEXT AS provenance_ref,
               matched.source_interface_id,matched.source_region_id
          FROM matched
        UNION ALL
        SELECT matched.demand_id,matched.target_kind,matched.target_id,
               'h6:temporal:' || matched.factor_type_symbol_id::TEXT || ':' ||
                   matched.predicate_symbol_id::TEXT || ':' || matched.role_symbol_id::TEXT || ':' ||
                   matched.source_temporal_state::TEXT AS evidence_ref,
               1::BIGINT AS signed_residual,
               'h6_temporal_factor_role_signature'::TEXT AS evidence_kind,
               'numeric-temporal-state'::TEXT AS provenance_ref,
               matched.source_interface_id,matched.source_region_id
          FROM matched
         WHERE matched.source_temporal_state<>0
           AND matched.candidate_temporal_state=matched.source_temporal_state
    )

    SELECT DISTINCT evidence_rows.demand_id,evidence_rows.target_kind,
           evidence_rows.target_id,evidence_rows.evidence_ref,evidence_rows.signed_residual,
           evidence_rows.evidence_kind,evidence_rows.provenance_ref,
           evidence_rows.source_interface_id,evidence_rows.source_region_id
    FROM evidence_rows;
$$;
