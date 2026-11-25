-- SLR production refinement of original m086/m088/m110 (donor f9c4670ef04b40e8153caa7dd00fa1eba7013ef4).
-- Only demands explicitly registered by native SLR change their read semantics.
-- Historical evidence IDs and admission events remain immutable. Physical
-- validity is computed from the original live factor-role carrier, not from
-- a cached claim or a signed-score compensation.
CREATE TABLE IF NOT EXISTS execution.slr_native_h6_managed_demand(
    demand_id bigint PRIMARY KEY REFERENCES execution.semantic_pnf_demand(demand_id) ON DELETE CASCADE
);
-- Current native H3 support is reconstructed by the original native producer
-- pass. Unknown/unowned original H3 evidence is not silently withdrawn.
CREATE TABLE IF NOT EXISTS execution.slr_native_h3_validity_current(
    evidence_id bigint PRIMARY KEY REFERENCES execution.semantic_pnf_candidate_evidence(evidence_id) ON DELETE CASCADE,
    demand_id bigint NOT NULL REFERENCES execution.semantic_pnf_demand(demand_id) ON DELETE CASCADE,
    supported boolean NOT NULL, revision bigint NOT NULL CHECK(revision>0)
);
CREATE INDEX IF NOT EXISTS slr_native_h3_validity_demand_idx
    ON execution.slr_native_h3_validity_current(demand_id,supported,evidence_id);
CREATE TABLE IF NOT EXISTS execution.slr_native_h3_validity_history(
    evidence_id bigint NOT NULL,demand_id bigint NOT NULL,revision bigint NOT NULL,
    supported boolean NOT NULL,original_atom jsonb NOT NULL,
    PRIMARY KEY(evidence_id,revision)
);
-- A physical support journal delta alone cannot reconcile the original
-- proof/coverage authority. This debt survives later empty passes.
CREATE TABLE IF NOT EXISTS execution.slr_native_h3_authority_debt(
    demand_id bigint PRIMARY KEY REFERENCES execution.semantic_pnf_demand(demand_id) ON DELETE CASCADE,
    first_observed_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- Source: original SensibLaw m110 at f9c4670ef04b40e8153caa7dd00fa1eba7013ef4, blob 41bf2232b43a53fb8e0373559206e5fec430d629.
-- Reopen the actual discourse/temporal signature relation independently of
-- scheduling/stop or admissibility state. An already-completed horizon or later stop does not
-- withdraw its physical support. This relation is evidence validity only.
-- Never use absence from this relation as semantic candidate refutation.
CREATE OR REPLACE FUNCTION execution.slr_current_m110_h6_atoms(
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


CREATE OR REPLACE VIEW execution.slr_native_current_candidate_evidence_v1 AS
SELECT e.* FROM execution.semantic_pnf_candidate_evidence e
WHERE (NOT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand m WHERE m.demand_id=e.demand_id)
   OR (e.horizon=6 AND e.evidence_family=2 AND
        ((e.evidence_kind='h6_discourse_factor_role_signature' AND e.provenance_ref='numeric-factor-signature')
        OR (e.evidence_kind='h6_temporal_factor_role_signature' AND e.provenance_ref='numeric-temporal-state'))) IS NOT TRUE
   OR EXISTS(SELECT 1 FROM execution.slr_current_m110_h6_atoms(e.demand_id) p
        WHERE p.target_kind=e.target_kind AND p.target_id=e.target_id
          AND p.evidence_ref=e.evidence_ref AND p.signed_residual=e.signed_residual
          AND p.evidence_kind=e.evidence_kind AND p.provenance_ref IS NOT DISTINCT FROM e.provenance_ref
          AND p.source_interface_id IS NOT DISTINCT FROM e.source_interface_id
          AND p.source_region_id IS NOT DISTINCT FROM e.source_region_id))
  AND NOT EXISTS (SELECT 1 FROM execution.slr_native_h3_validity_current v
      JOIN execution.slr_native_h6_managed_demand m ON m.demand_id=v.demand_id
      WHERE v.evidence_id=e.evidence_id AND v.demand_id=e.demand_id AND NOT v.supported);

-- Preserve the original public ABI, including neutral represented rows.
CREATE OR REPLACE VIEW execution.semantic_pnf_candidate_evidence_horizon_v1 AS
WITH horizon(horizon) AS (
    VALUES (3::SMALLINT), (6::SMALLINT), (9::SMALLINT)
)
SELECT universe.demand_id,
       universe.target_kind,
       universe.target_id,
       horizon.horizon,
       COALESCE(sum(evidence.signed_residual), 0)::BIGINT AS signed_residual,
       sign(COALESCE(sum(evidence.signed_residual), 0))::SMALLINT AS phase,
       count(evidence.evidence_id)::BIGINT AS evidence_count
  FROM execution.semantic_pnf_candidate_universe AS universe
 CROSS JOIN horizon
  LEFT JOIN execution.slr_native_current_candidate_evidence_v1 AS evidence
    ON evidence.demand_id = universe.demand_id
   AND evidence.target_kind = universe.target_kind
   AND evidence.target_id = universe.target_id
   AND evidence.horizon <= horizon.horizon
 GROUP BY universe.demand_id, universe.target_kind, universe.target_id,
          horizon.horizon;

CREATE OR REPLACE VIEW execution.semantic_pnf_candidate_evidence_classification_v1 AS
SELECT evidence.evidence_id,
       evidence.demand_id,
       evidence.target_kind,
       evidence.target_id,
       evidence.evidence_family,
       evidence.horizon,
       evidence.signed_residual,
       CASE
           WHEN evidence.signed_residual < 0 THEN -1
           WHEN evidence.signed_residual > 0 THEN 1
           ELSE 0
       END::SMALLINT AS evidence_polarity,
       CASE
           WHEN evidence.signed_residual > 0 THEN 2  -- candidate-supporting pressure
           WHEN evidence.signed_residual < 0 THEN 3  -- explicit pressure against candidate
           WHEN universe.demand_id IS NOT NULL THEN 1 -- represented-neutral evidence
           ELSE 4                                    -- demand-level/other neutral evidence
       END::SMALLINT AS evidence_class,
       evidence.evidence_kind,
       evidence.provenance_ref,
       evidence.source_interface_id,
       evidence.source_region_id,
       evidence.created_at
  FROM execution.slr_native_current_candidate_evidence_v1 AS evidence
  LEFT JOIN execution.semantic_pnf_candidate_universe AS universe
    ON universe.demand_id=evidence.demand_id
   AND universe.target_kind=evidence.target_kind
   AND universe.target_id=evidence.target_id;

-- Original m095 certificate state is historical authority. A physical
-- withdrawal does not fabricate a replacement certificate or restore a
-- refuted candidate. It only suspends consumption until the owner checks it.
CREATE OR REPLACE FUNCTION execution.slr_native_current_authority_usable(selected_demand_id bigint)
RETURNS boolean LANGUAGE sql STABLE AS $
SELECT NOT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand WHERE demand_id=selected_demand_id)
 OR (
    NOT EXISTS(SELECT 1 FROM execution.slr_native_h3_authority_debt WHERE demand_id=selected_demand_id)
    AND NOT EXISTS(
      SELECT 1 FROM execution.semantic_pnf_candidate_latest_admissibility a
      WHERE a.demand_id=selected_demand_id AND NOT EXISTS(
        SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 e
        WHERE e.evidence_id=a.evidence_id AND e.demand_id=a.demand_id
          AND e.target_kind=a.target_kind AND e.target_id=a.target_id))
    AND NOT (
      EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_evidence e WHERE e.demand_id=selected_demand_id
        AND e.horizon IN(3,6) AND NOT EXISTS(
          SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 c WHERE c.evidence_id=e.evidence_id))
      AND (EXISTS(SELECT 1 FROM execution.semantic_pnf_frontier_resolution WHERE demand_id=selected_demand_id AND outcome_state=2)
        OR EXISTS(SELECT 1 FROM execution.semantic_pnf_demand_open_world_state WHERE demand_id=selected_demand_id AND NOT outside_model_possible)
        OR EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_sufficiency_current_v1 WHERE demand_id=selected_demand_id AND certificate_state=1)))
 );
$;

-- Preserve the m095 public stop ABI and all query/policy certificate rules.
-- The additional conjunction is a consumption guard, not new authority.
CREATE OR REPLACE FUNCTION execution.numeric_pnf_consumer_stop_at_horizon(
    selected_demand_id bigint, selected_consumer_ref text, selected_query_ref text,
    selected_policy_ref text, selected_horizon smallint
) RETURNS boolean LANGUAGE sql STABLE AS $
SELECT execution.slr_native_current_authority_usable(selected_demand_id) AND EXISTS(
    SELECT 1 FROM execution.semantic_pnf_consumer_sufficiency_current_v1 certificate
    WHERE certificate.demand_id=selected_demand_id
      AND certificate.consumer_ref=selected_consumer_ref AND certificate.query_ref=selected_query_ref
      AND certificate.policy_ref=selected_policy_ref AND certificate.certificate_state=1
      AND certificate.horizon<=selected_horizon
      AND ((selected_policy_ref='' AND certificate.certificate_kind IN(1,3))
        OR (selected_policy_ref<>'' AND certificate.certificate_kind IN(2,3)))
);
$;

CREATE OR REPLACE FUNCTION execution.refresh_numeric_pnf_consumer_horizon_outcome(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    selected_horizon SMALLINT,
    selected_consumer_ref TEXT,
    selected_query_ref TEXT,
    selected_policy_ref TEXT DEFAULT ''
) RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE affected BIGINT := 0;
BEGIN
    IF selected_horizon NOT IN (3,6,9) THEN
        RAISE EXCEPTION 'selected_horizon must be 3, 6, or 9';
    END IF;

    WITH scoped AS MATERIALIZED (
        SELECT demand.demand_id
          FROM execution.semantic_pnf_demand AS demand
          JOIN execution.semantic_pnf_region AS region
            ON region.region_id=demand.source_region_id
          JOIN execution.semantic_pnf_consumer_horizon_work_queue AS work
            ON work.demand_id=demand.demand_id
           AND work.consumer_ref=selected_consumer_ref
           AND work.query_ref=selected_query_ref
           AND work.policy_ref=selected_policy_ref
           AND work.horizon=selected_horizon
         WHERE region.run_id=selected_run_id
           AND region.document_id=selected_document_id
           AND (work.work_state=2 OR (
             NOT execution.slr_native_current_authority_usable(demand.demand_id)
             AND EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_outcome old
               WHERE old.demand_id=demand.demand_id AND old.consumer_ref=selected_consumer_ref
                 AND old.query_ref=selected_query_ref AND old.policy_ref=selected_policy_ref
                 AND old.horizon=selected_horizon)))
    ), evidence_count AS MATERIALIZED (
        SELECT scoped.demand_id,
               count(evidence.evidence_id)::BIGINT AS evidence_count,
               count(evidence.evidence_id) FILTER (
                   WHERE evidence.signed_residual<>0
               )::BIGINT AS nonneutral_evidence_count
          FROM scoped
          LEFT JOIN execution.slr_native_current_candidate_evidence_v1 AS evidence
            ON evidence.demand_id=scoped.demand_id
           AND evidence.horizon<=selected_horizon
         GROUP BY scoped.demand_id
    ), candidate_count AS MATERIALIZED (
        SELECT scoped.demand_id,count(universe.target_id)::BIGINT AS represented_candidate_count
          FROM scoped
          LEFT JOIN execution.semantic_pnf_candidate_universe AS universe
            ON universe.demand_id=scoped.demand_id
         GROUP BY scoped.demand_id
    ), preferred_count AS MATERIALIZED (
        SELECT scoped.demand_id,
               count(preference.target_id) FILTER (WHERE preference.preferred)::BIGINT
                   AS preferred_candidate_count
          FROM scoped
          LEFT JOIN execution.semantic_pnf_candidate_current_preference AS preference
            ON preference.demand_id=scoped.demand_id
           AND preference.horizon=selected_horizon
         GROUP BY scoped.demand_id
    ), state AS MATERIALIZED (
        SELECT scoped.demand_id,
               evidence_count.evidence_count,
               evidence_count.nonneutral_evidence_count,
               candidate_count.represented_candidate_count,
               preferred_count.preferred_candidate_count,
               execution.slr_native_current_authority_usable(scoped.demand_id) AND EXISTS (
                   SELECT 1 FROM execution.semantic_pnf_frontier_resolution AS proof
                    WHERE proof.demand_id=scoped.demand_id
                      AND proof.outcome_state=2
                      AND proof.candidate_count=1
               ) AS proof_unique,
               execution.numeric_pnf_consumer_stop_at_horizon(
                   scoped.demand_id,selected_consumer_ref,selected_query_ref,
                   selected_policy_ref,selected_horizon
               ) AS consumer_sufficient
          FROM scoped
          JOIN evidence_count USING (demand_id)
          JOIN candidate_count USING (demand_id)
          JOIN preferred_count USING (demand_id)
    )
    INSERT INTO execution.semantic_pnf_consumer_horizon_outcome
        (demand_id,consumer_ref,query_ref,policy_ref,horizon,outcome_state,
         evidence_count,nonneutral_evidence_count,represented_candidate_count,
         preferred_candidate_count,proof_unique,consumer_sufficient,
         residual_required,refreshed_at)
    SELECT state.demand_id,selected_consumer_ref,selected_query_ref,selected_policy_ref,
           selected_horizon,
           CASE
               WHEN state.proof_unique THEN 4
               WHEN state.consumer_sufficient THEN 3
               WHEN state.evidence_count=0 THEN 1
               ELSE 2
           END::SMALLINT,
           state.evidence_count,state.nonneutral_evidence_count,
           state.represented_candidate_count,state.preferred_candidate_count,
           state.proof_unique,state.consumer_sufficient,
           NOT state.proof_unique AND NOT state.consumer_sufficient,
           CURRENT_TIMESTAMP
      FROM state
    ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
    DO UPDATE SET
        outcome_state=EXCLUDED.outcome_state,
        evidence_count=EXCLUDED.evidence_count,
        nonneutral_evidence_count=EXCLUDED.nonneutral_evidence_count,
        represented_candidate_count=EXCLUDED.represented_candidate_count,
        preferred_candidate_count=EXCLUDED.preferred_candidate_count,
        proof_unique=EXCLUDED.proof_unique,
        consumer_sufficient=EXCLUDED.consumer_sufficient,
        residual_required=EXCLUDED.residual_required,
        refreshed_at=EXCLUDED.refreshed_at;
    GET DIAGNOSTICS affected=ROW_COUNT;
    RETURN affected;
END;
$$;

-- Original m088 strict-winner relation owns ranking. Revision both gains
-- AND losses, without revising a stable current preference or touching proof.
CREATE OR REPLACE FUNCTION execution.slr_refresh_native_preference(
    selected_demand_id bigint, selected_horizon smallint
) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE changed bigint;
BEGIN
    IF selected_horizon NOT IN(3,6,9) OR NOT EXISTS(
        SELECT 1 FROM execution.slr_native_h6_managed_demand WHERE demand_id=selected_demand_id)
    THEN RAISE EXCEPTION 'unmanaged native preference scope'; END IF;
    WITH desired AS (
        SELECT h.demand_id,h.target_kind,h.target_id,h.horizon,
            COALESCE(p.strict_winner,false) AS preferred,
            CASE WHEN COALESCE(p.strict_winner,false) THEN h.signed_residual ELSE 0 END AS margin,
            h.evidence_count
        FROM execution.semantic_pnf_candidate_horizon_state_v1 h
        LEFT JOIN execution.semantic_pnf_progressive_preference_candidate_v1 p
          ON p.demand_id=h.demand_id AND p.target_kind=h.target_kind AND p.target_id=h.target_id AND p.horizon=h.horizon
        WHERE h.demand_id=selected_demand_id AND h.horizon=selected_horizon
    ), changes AS (
        SELECT d.*,COALESCE(c.revision,0)+1 AS next_revision
        FROM desired d
        LEFT JOIN execution.semantic_pnf_candidate_latest_preference c
          ON c.demand_id=d.demand_id AND c.target_kind=d.target_kind AND c.target_id=d.target_id AND c.horizon=d.horizon
        WHERE (c.preferred,c.margin,c.evidence_count) IS DISTINCT FROM (d.preferred,d.margin,d.evidence_count)
          AND (c.revision IS NOT NULL OR d.preferred)
    )
    INSERT INTO execution.semantic_pnf_candidate_preference
      (demand_id,target_kind,target_id,horizon,revision,preferred,margin,evidence_count)
    SELECT demand_id,target_kind,target_id,horizon,next_revision,preferred,margin,evidence_count FROM changes;
    GET DIAGNOSTICS changed=ROW_COUNT;
    RETURN changed;
END;
$$;

CREATE TABLE IF NOT EXISTS execution.slr_native_h6_current_contract(contract_ref text PRIMARY KEY);
INSERT INTO execution.slr_native_h6_current_contract VALUES('slr-native-h6-current:v1') ON CONFLICT DO NOTHING;

-- Version marker is written last, after all dependent views/functions.
CREATE TABLE IF NOT EXISTS execution.slr_native_h3_current_contract(contract_ref text PRIMARY KEY);
INSERT INTO execution.slr_native_h3_current_contract VALUES('slr-native-h3-current:v1') ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS execution.slr_native_current_authority_contract(contract_ref text PRIMARY KEY);
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v1') ON CONFLICT DO NOTHING;
