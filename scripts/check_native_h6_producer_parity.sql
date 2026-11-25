-- Exact m110 reference parity, rollback only. Historical donor:
-- SensibLaw f9c4670ef04b40e8153caa7dd00fa1eba7013ef4 blob 41bf2232b43a53fb8e0373559206e5fec430d629.
-- The independent reference SELECT is deliberately not production code.
\set ON_ERROR_STOP on
BEGIN ISOLATION LEVEL SERIALIZABLE;
SET LOCAL statement_timeout='90000ms';
SELECT set_config('slr.test_demand', :'demand_id', true),
       set_config('slr.test_consumer', :'consumer_ref', true),
       set_config('slr.test_query', :'query_ref', true),
       set_config('slr.test_policy', :'policy_ref', true);
DO $slr_parity$
DECLARE
    selected_demand_id bigint := current_setting('slr.test_demand')::bigint;
    selected_consumer_ref text := current_setting('slr.test_consumer');
    selected_query_ref text := current_setting('slr.test_query');
    selected_policy_ref text := current_setting('slr.test_policy');
    selected_run_id bigint;
    selected_document_id bigint;
    selected_reprocess_completed boolean := false;
    expected_new bigint;
    actual_new bigint;
    desired_count bigint;
BEGIN
    SELECT r.run_id,r.document_id INTO STRICT selected_run_id,selected_document_id
      FROM execution.semantic_pnf_demand d
      JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
      WHERE d.demand_id=selected_demand_id;
    IF NOT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand m
         WHERE m.demand_id=selected_demand_id)
       OR NOT execution.slr_native_current_authority_usable(selected_demand_id) THEN
        RAISE EXCEPTION 'H6 parity requires the checked managed demand';
    END IF;
    -- Re-open only scheduling in the rollback fixture; no semantic event.
    INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
      (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
    VALUES(selected_demand_id,selected_consumer_ref,selected_query_ref,selected_policy_ref,6,1,NULL)
    ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
    DO UPDATE SET work_state=1,completed_at=NULL;

    CREATE TEMP TABLE slr_h6_reference_expected ON COMMIT DROP AS
    WITH ready AS MATERIALIZED (
        SELECT demand.demand_id,demand.source_object_id,demand.source_interface_id,
               demand.source_region_id,demand.expected_factor_type_symbol_id,
               demand.role_symbol_id
          FROM execution.semantic_pnf_consumer_horizon_work_queue AS work
          JOIN execution.semantic_pnf_demand AS demand
            ON demand.demand_id=work.demand_id
          JOIN execution.semantic_pnf_region AS region
            ON region.region_id=demand.source_region_id
         WHERE work.consumer_ref=selected_consumer_ref
           AND work.query_ref=selected_query_ref
           AND work.policy_ref=selected_policy_ref
           AND work.horizon=6
           AND (work.work_state=1 OR (selected_reprocess_completed AND work.work_state=2))
           AND demand.demand_id=selected_demand_id
           AND region.run_id=selected_run_id
           AND region.document_id=selected_document_id
           AND demand.source_object_id IS NOT NULL
           AND NOT execution.numeric_pnf_consumer_stop_at_horizon(
               demand.demand_id,selected_consumer_ref,selected_query_ref,
               selected_policy_ref,3::smallint
           )
           AND NOT EXISTS (
               SELECT 1 FROM execution.semantic_pnf_frontier_resolution AS proof
                WHERE proof.demand_id=demand.demand_id
                  AND proof.outcome_state=2 AND proof.candidate_count=1
           )
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
           evidence_rows.target_id,evidence_rows.evidence_ref,2::smallint AS evidence_family,6::smallint AS horizon,
           evidence_rows.signed_residual,evidence_rows.evidence_kind,evidence_rows.provenance_ref,
           evidence_rows.source_interface_id,evidence_rows.source_region_id
    FROM evidence_rows;
    CREATE TEMP TABLE slr_h6_native_expected ON COMMIT DROP AS
    SELECT p.demand_id,p.target_kind,p.target_id,p.evidence_ref,2::smallint AS evidence_family,6::smallint AS horizon,
           p.signed_residual,p.evidence_kind,p.provenance_ref,p.source_interface_id,p.source_region_id
      FROM execution.slr_current_m110_h6_atoms(selected_demand_id) p
      WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_latest_admissibility a
          WHERE a.demand_id=p.demand_id AND a.target_kind=p.target_kind AND a.target_id=p.target_id AND a.event_kind=1)
        AND NOT execution.numeric_pnf_consumer_stop_at_horizon(
          selected_demand_id,selected_consumer_ref,selected_query_ref,selected_policy_ref,3::smallint)
        AND NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_frontier_resolution proof
          WHERE proof.demand_id=selected_demand_id AND proof.outcome_state=2 AND proof.candidate_count=1);
    IF EXISTS((SELECT * FROM slr_h6_reference_expected EXCEPT ALL SELECT * FROM slr_h6_native_expected)
      UNION ALL (SELECT * FROM slr_h6_native_expected EXCEPT ALL SELECT * FROM slr_h6_reference_expected)) THEN
        RAISE EXCEPTION 'native H6 desired relation differs from m110';
    END IF;
    SELECT count(*) INTO desired_count FROM slr_h6_reference_expected;
    SELECT count(*) INTO expected_new FROM slr_h6_reference_expected r
      WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_evidence e
        WHERE e.demand_id=r.demand_id AND e.target_kind=r.target_kind AND e.target_id=r.target_id AND e.evidence_ref=r.evidence_ref);
    actual_new := execution.slr_produce_native_h6_for_demand(
        selected_demand_id,selected_consumer_ref,selected_query_ref,selected_policy_ref);
    IF actual_new<>expected_new THEN RAISE EXCEPTION 'native H6 insert count mismatch'; END IF;
    IF EXISTS(SELECT * FROM slr_h6_reference_expected EXCEPT ALL
        SELECT e.demand_id,e.target_kind,e.target_id,e.evidence_ref,e.evidence_family,e.horizon,
               e.signed_residual,e.evidence_kind,e.provenance_ref,e.source_interface_id,e.source_region_id
        FROM execution.semantic_pnf_candidate_evidence e WHERE e.demand_id=selected_demand_id AND e.horizon=6) THEN
        RAISE EXCEPTION 'native H6 exact persisted payload mismatch';
    END IF;
    IF execution.slr_produce_native_h6_for_demand(
        selected_demand_id,selected_consumer_ref,selected_query_ref,selected_policy_ref)<>0 THEN
        RAISE EXCEPTION 'native H6 replay inserted duplicate evidence';
    END IF;
    RAISE NOTICE 'H6 reference desired %, new %, rollback only (zero desired is vacuous for nonempty parity)',desired_count,actual_new;
END;
$slr_parity$;
ROLLBACK;
