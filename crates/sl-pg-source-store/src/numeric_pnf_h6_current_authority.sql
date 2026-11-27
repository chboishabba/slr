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
-- H6 validity shares the same append-only journal shape. Install once
-- here, not on every audit pass.
CREATE TABLE IF NOT EXISTS execution.slr_native_h6_validity_current(
    evidence_id bigint PRIMARY KEY REFERENCES execution.semantic_pnf_candidate_evidence(evidence_id) ON DELETE CASCADE,
    demand_id bigint NOT NULL REFERENCES execution.semantic_pnf_demand(demand_id) ON DELETE CASCADE,
    supported boolean NOT NULL, revision bigint NOT NULL CHECK(revision>0)
);
CREATE INDEX IF NOT EXISTS slr_native_h6_validity_demand_idx
    ON execution.slr_native_h6_validity_current(demand_id,supported,evidence_id);
CREATE TABLE IF NOT EXISTS execution.slr_native_h6_validity_history(
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

-- Demand-wide proof/sufficiency authority is not consumer-local. Remove
-- only rebuildable authority-bearing outcome projections and reopen their
-- exact queue coordinates. Preserve original proof, certificate, evidence,
-- admission and validity history. Candidate-only rows retain sparse wake
-- semantics; unrelated demands are untouched.
CREATE OR REPLACE FUNCTION execution.slr_suspend_native_debt_outcomes(selected_demands bigint[])
RETURNS bigint LANGUAGE plpgsql AS $slr_suspend$
DECLARE retired bigint;
BEGIN
    IF COALESCE(cardinality(selected_demands),0)=0 THEN RETURN 0; END IF;
    WITH affected AS MATERIALIZED (
        SELECT o.demand_id,o.consumer_ref,o.query_ref,o.policy_ref,o.horizon
        FROM execution.semantic_pnf_consumer_horizon_outcome o
        JOIN execution.slr_native_h3_authority_debt d USING(demand_id)
        WHERE o.demand_id=ANY(selected_demands)
          AND (o.proof_unique OR o.consumer_sufficient OR o.outcome_state IN(3,4))
    ), requeued AS (
        INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue AS q
            (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
        SELECT demand_id,consumer_ref,query_ref,policy_ref,horizon,1::smallint,NULL FROM affected
        ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
        DO UPDATE SET work_state=1,completed_at=NULL
        WHERE q.work_state<>1 OR q.completed_at IS NOT NULL
        RETURNING demand_id
    )
    DELETE FROM execution.semantic_pnf_consumer_horizon_outcome o
    USING affected a
    WHERE (o.demand_id,o.consumer_ref,o.query_ref,o.policy_ref,o.horizon)
        =(a.demand_id,a.consumer_ref,a.query_ref,a.policy_ref,a.horizon);
    GET DIAGNOSTICS retired=ROW_COUNT;
    RETURN retired;
END;
$slr_suspend$;

CREATE OR REPLACE FUNCTION execution.slr_suspend_native_authority_on_debt()
RETURNS trigger LANGUAGE plpgsql AS $slr_debt$
BEGIN
    PERFORM execution.slr_suspend_native_debt_outcomes(
      ARRAY(SELECT demand_id FROM inserted_debt));
    PERFORM execution.slr_suspend_native_identity_derivations(
      ARRAY(SELECT demand_id FROM inserted_debt));
    RETURN NULL;
END;
$slr_debt$;
DROP TRIGGER IF EXISTS slr_native_authority_debt_suspend ON execution.slr_native_h3_authority_debt;
CREATE TRIGGER slr_native_authority_debt_suspend
AFTER INSERT ON execution.slr_native_h3_authority_debt
REFERENCING NEW TABLE AS inserted_debt
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_suspend_native_authority_on_debt();

-- Exact reverse coordinates of original m069/m072 resolution witnesses.
-- A resolution-anchor witness has NULL demand_id, so retain its original
-- resolution-interface and selected target join. Never suspend by name.
CREATE INDEX IF NOT EXISTS slr_identity_witness_demand_idx
    ON execution.semantic_pnf_identity_witness(demand_id,witness_id) WHERE demand_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS slr_identity_witness_resolution_anchor_idx
    ON execution.semantic_pnf_identity_witness(resolution_interface_id,source_object_id)
    WHERE demand_id IS NULL AND witness_kind=1;

CREATE OR REPLACE VIEW execution.slr_native_blocked_identity_fibre_v1 AS
SELECT w.source_object_id,w.authority_class,d.demand_id
FROM execution.slr_native_h3_authority_debt d
JOIN execution.semantic_pnf_identity_witness w ON w.demand_id=d.demand_id
JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id AND a.admission_state=2
UNION
SELECT w.source_object_id,w.authority_class,d.demand_id
FROM execution.slr_native_h3_authority_debt d
JOIN execution.semantic_pnf_frontier_resolution r ON r.demand_id=d.demand_id
JOIN execution.semantic_pnf_identity_witness w
  ON w.resolution_interface_id=r.interface_id
 AND w.source_object_id=r.selected_target_id
 AND w.witness_kind=1 AND w.demand_id IS NULL
JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id AND a.admission_state=2
WHERE r.selected_target_kind=1;

-- m074 current identity ABI. Test ambiguity on the original whole
-- accepted fibre; then suppress a blocked group. Never filter individual
-- blocked witnesses before uniqueness (that can manufacture uniqueness).
CREATE OR REPLACE VIEW execution.semantic_pnf_identity_projection AS
SELECT witness.source_object_id,
       witness.authority_class,
       min(witness.target_entity_id) AS target_entity_id,
       array_agg(witness.witness_id ORDER BY witness.witness_id) AS witness_ids,
       count(*)::bigint AS witness_count
FROM execution.semantic_pnf_identity_witness witness
JOIN execution.semantic_pnf_identity_witness_admission admission
  ON admission.witness_id=witness.witness_id AND admission.admission_state=2
JOIN execution.semantic_pnf_canonical_entity entity
  ON entity.entity_id=witness.target_entity_id AND entity.authority_class=witness.authority_class
WHERE witness.candidate_count=1
 AND NOT EXISTS(SELECT 1 FROM execution.slr_native_blocked_identity_fibre_v1 b
   WHERE b.source_object_id=witness.source_object_id AND b.authority_class=witness.authority_class)
GROUP BY witness.source_object_id,witness.authority_class
HAVING count(DISTINCT witness.target_entity_id)=1;

-- Exact reverse of m086 direct/shared-token support, preserving the
-- authority coordinate. Suppression must propagate across the entire
-- support fibre too: a surviving different path must not become unique
-- simply because an unresolved path has been hidden.
CREATE OR REPLACE VIEW execution.slr_native_blocked_identity_slot_v1 AS
SELECT e.factor_id,e.slot_ordinal,b.authority_class,b.demand_id
FROM execution.slr_native_blocked_identity_fibre_v1 b
JOIN execution.semantic_pnf_hyperedge e ON e.object_id=b.source_object_id
UNION
SELECT e.factor_id,e.slot_ordinal,b.authority_class,b.demand_id
FROM execution.slr_native_blocked_identity_fibre_v1 b
JOIN execution.semantic_pnf_object bo ON bo.object_id=b.source_object_id
JOIN execution.semantic_pnf_region br ON br.region_id=bo.region_id
JOIN execution.semantic_pnf_object_token_support bs ON bs.object_id=b.source_object_id
JOIN execution.semantic_pnf_object_token_support fs ON fs.token_id=bs.token_id AND fs.object_id<>bs.object_id
JOIN execution.semantic_pnf_hyperedge e ON e.object_id=fs.object_id
JOIN execution.semantic_pnf_factor f ON f.factor_id=e.factor_id
JOIN execution.semantic_pnf_region fr ON fr.region_id=f.region_id
 AND fr.run_id=br.run_id AND fr.document_id=br.document_id;

-- Original m086 unique support owner, with group-wide debt suppression.
CREATE OR REPLACE VIEW execution.semantic_pnf_factor_identity_support_unique_v1 AS
WITH unambiguous AS (
    SELECT factor_id,slot_ordinal,authority_class
    FROM execution.semantic_pnf_factor_identity_support_v1
    GROUP BY factor_id,slot_ordinal,authority_class
    HAVING count(DISTINCT target_entity_id)=1
)
SELECT DISTINCT ON (support.factor_id,support.slot_ordinal,support.authority_class) support.*
FROM execution.semantic_pnf_factor_identity_support_v1 support
JOIN unambiguous u USING(factor_id,slot_ordinal,authority_class)
WHERE NOT EXISTS(SELECT 1 FROM execution.slr_native_blocked_identity_slot_v1 b
 WHERE b.factor_id=support.factor_id AND b.slot_ordinal=support.slot_ordinal
   AND b.authority_class=support.authority_class)
ORDER BY support.factor_id,support.slot_ordinal,support.authority_class,
 support.support_kind,support.identity_source_object_id,support.support_token_id NULLS FIRST;

-- m084 entity composition is ranked over a whole region/entity/authority
-- fibre. A blocked direct projection withholds that entire group, including
-- overflow, until the independent authority owner reconciles the debt.
CREATE OR REPLACE VIEW execution.slr_native_blocked_composition_group_v1 AS
SELECT DISTINCT f.region_id,w.target_entity_id,b.authority_class,b.demand_id
FROM execution.slr_native_blocked_identity_fibre_v1 b
JOIN execution.semantic_pnf_identity_witness w
  ON w.source_object_id=b.source_object_id AND w.authority_class=b.authority_class
JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id AND a.admission_state=2
JOIN execution.semantic_pnf_hyperedge e ON e.object_id=b.source_object_id
JOIN execution.semantic_pnf_factor f ON f.factor_id=e.factor_id;

-- Original m084 bounded pair and overflow producer, pinned donor
-- f9c4670ef04b40e8153caa7dd00fa1eba7013ef4. The only semantic
-- change is withholding the complete debt-bearing entity rank group
-- BEFORE ranking and counting. Local object composition is untouched.
CREATE OR REPLACE FUNCTION execution.numeric_pnf_bounded_entity_composition_pairs(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    max_per_bridge INTEGER
)
RETURNS TABLE (
    source_interface_id BIGINT,
    region_id BIGINT,
    bridge_entity_id BIGINT,
    identity_authority_class SMALLINT,
    left_factor_id BIGINT,
    right_factor_id BIGINT,
    left_role_symbol_id BIGINT,
    right_role_symbol_id BIGINT,
    candidate_rank SMALLINT
)
LANGUAGE sql
STABLE
AS $$
WITH projected_edge AS MATERIALIZED (
    SELECT edge.factor_id,
           edge.slot_ordinal,
           edge.role_symbol_id,
           factor.region_id,
           interface.interface_id AS source_interface_id,
           projection.target_entity_id,
           projection.authority_class
      FROM execution.semantic_pnf_hyperedge AS edge
      JOIN execution.semantic_pnf_factor AS factor
        ON factor.factor_id = edge.factor_id
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = factor.region_id
      JOIN execution.semantic_pnf_interface AS interface
        ON interface.region_id = region.region_id
      JOIN execution.semantic_pnf_identity_projection AS projection
        ON projection.source_object_id = edge.object_id
     WHERE region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND NOT EXISTS (
           SELECT 1 FROM execution.slr_native_blocked_composition_group_v1 blocked
           WHERE blocked.region_id=factor.region_id
             AND blocked.target_entity_id=projection.target_entity_id
             AND blocked.authority_class=projection.authority_class
       )
), bridge AS MATERIALIZED (
    SELECT region_id,
           source_interface_id,
           target_entity_id,
           authority_class
      FROM projected_edge
     GROUP BY region_id, source_interface_id,
              target_entity_id, authority_class
    HAVING count(*) > 1
)
SELECT bridge.source_interface_id,
       bridge.region_id,
       bridge.target_entity_id,
       bridge.authority_class,
       pair.left_factor_id,
       pair.right_factor_id,
       pair.left_role_symbol_id,
       pair.right_role_symbol_id,
       pair.candidate_rank
  FROM bridge
  CROSS JOIN LATERAL (
      SELECT limited.left_factor_id,
             limited.right_factor_id,
             limited.left_role_symbol_id,
             limited.right_role_symbol_id,
             (row_number() OVER (
                 ORDER BY limited.left_factor_id,
                          limited.right_factor_id,
                          limited.left_slot_ordinal,
                          limited.right_slot_ordinal
              ) - 1)::SMALLINT AS candidate_rank
        FROM (
            SELECT left_edge.factor_id AS left_factor_id,
                   right_edge.factor_id AS right_factor_id,
                   left_edge.role_symbol_id AS left_role_symbol_id,
                   right_edge.role_symbol_id AS right_role_symbol_id,
                   left_edge.slot_ordinal AS left_slot_ordinal,
                   right_edge.slot_ordinal AS right_slot_ordinal
              FROM projected_edge AS left_edge
              JOIN projected_edge AS right_edge
                ON right_edge.region_id = left_edge.region_id
               AND right_edge.target_entity_id = left_edge.target_entity_id
               AND right_edge.authority_class = left_edge.authority_class
               AND right_edge.factor_id > left_edge.factor_id
             WHERE left_edge.region_id = bridge.region_id
               AND left_edge.target_entity_id = bridge.target_entity_id
               AND left_edge.authority_class = bridge.authority_class
             ORDER BY left_edge.factor_id,
                      right_edge.factor_id,
                      left_edge.slot_ordinal,
                      right_edge.slot_ordinal
             LIMIT max_per_bridge
        ) AS limited
  ) AS pair;
$$;

CREATE OR REPLACE FUNCTION execution.refresh_numeric_pnf_factor_composition_candidates(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    max_per_bridge INTEGER DEFAULT 16
)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    local_inserted BIGINT := 0;
    identity_inserted BIGINT := 0;
BEGIN
    IF max_per_bridge < 1 OR max_per_bridge > 256 THEN
        RAISE EXCEPTION 'max_per_bridge must be between 1 and 256';
    END IF;

    DELETE FROM execution.semantic_pnf_factor_composition_candidate AS candidate
    USING execution.semantic_pnf_region AS region
     WHERE candidate.region_id = region.region_id
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id;

    DELETE FROM execution.semantic_pnf_factor_composition_overflow
     WHERE run_id = selected_run_id
       AND document_id = selected_document_id;

    INSERT INTO execution.semantic_pnf_factor_composition_candidate
        (source_interface_id, region_id,
         left_factor_id, right_factor_id,
         left_role_symbol_id, right_role_symbol_id,
         bridge_object_id, bridge_entity_id,
         identity_authority_class, candidate_rank)
    SELECT source_interface_id,
           region_id,
           left_factor_id,
           right_factor_id,
           left_role_symbol_id,
           right_role_symbol_id,
           bridge_object_id,
           NULL,
           NULL,
           candidate_rank
      FROM execution.numeric_pnf_bounded_local_composition_pairs(
          selected_run_id, selected_document_id, max_per_bridge
      )
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS local_inserted = ROW_COUNT;

    WITH bridge AS MATERIALIZED (
        SELECT factor.region_id,
               edge.object_id,
               count(*)::BIGINT AS participant_count
          FROM execution.semantic_pnf_hyperedge AS edge
          JOIN execution.semantic_pnf_factor AS factor
            ON factor.factor_id = edge.factor_id
          JOIN execution.semantic_pnf_region AS region
            ON region.region_id = factor.region_id
         WHERE region.run_id = selected_run_id
           AND region.document_id = selected_document_id
         GROUP BY factor.region_id, edge.object_id
        HAVING count(*) > 1
    )
    INSERT INTO execution.semantic_pnf_factor_composition_overflow
        (run_id, document_id, region_id, bridge_kind,
         bridge_object_id, bridge_entity_id, identity_authority_class,
         participant_count, possible_pair_count, retained_pair_limit)
    SELECT selected_run_id,
           selected_document_id,
           region_id,
           1,
           object_id,
           NULL,
           NULL,
           participant_count,
           (participant_count * (participant_count - 1)) / 2,
           max_per_bridge::SMALLINT
      FROM bridge
     WHERE (participant_count * (participant_count - 1)) / 2 > max_per_bridge
    ON CONFLICT (run_id, document_id, region_id, bridge_object_id)
        WHERE bridge_kind = 1
    DO UPDATE SET
        participant_count = EXCLUDED.participant_count,
        possible_pair_count = EXCLUDED.possible_pair_count,
        retained_pair_limit = EXCLUDED.retained_pair_limit,
        observed_at = CURRENT_TIMESTAMP;

    INSERT INTO execution.semantic_pnf_factor_composition_candidate
        (source_interface_id, region_id,
         left_factor_id, right_factor_id,
         left_role_symbol_id, right_role_symbol_id,
         bridge_object_id, bridge_entity_id,
         identity_authority_class, candidate_rank)
    SELECT source_interface_id,
           region_id,
           left_factor_id,
           right_factor_id,
           left_role_symbol_id,
           right_role_symbol_id,
           NULL,
           bridge_entity_id,
           identity_authority_class,
           candidate_rank
      FROM execution.numeric_pnf_bounded_entity_composition_pairs(
          selected_run_id, selected_document_id, max_per_bridge
      )
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS identity_inserted = ROW_COUNT;

    WITH projected_edge AS MATERIALIZED (
        SELECT factor.region_id,
               projection.target_entity_id,
               projection.authority_class
          FROM execution.semantic_pnf_hyperedge AS edge
          JOIN execution.semantic_pnf_factor AS factor
            ON factor.factor_id = edge.factor_id
          JOIN execution.semantic_pnf_region AS region
            ON region.region_id = factor.region_id
          JOIN execution.semantic_pnf_identity_projection AS projection
            ON projection.source_object_id = edge.object_id
         WHERE region.run_id = selected_run_id
           AND region.document_id = selected_document_id
           AND NOT EXISTS (
               SELECT 1 FROM execution.slr_native_blocked_composition_group_v1 blocked
               WHERE blocked.region_id=factor.region_id
                 AND blocked.target_entity_id=projection.target_entity_id
                 AND blocked.authority_class=projection.authority_class
           )
    ), bridge AS (
        SELECT region_id,
               target_entity_id,
               authority_class,
               count(*)::BIGINT AS participant_count
          FROM projected_edge
         GROUP BY region_id, target_entity_id, authority_class
        HAVING count(*) > 1
    )
    INSERT INTO execution.semantic_pnf_factor_composition_overflow
        (run_id, document_id, region_id, bridge_kind,
         bridge_object_id, bridge_entity_id, identity_authority_class,
         participant_count, possible_pair_count, retained_pair_limit)
    SELECT selected_run_id,
           selected_document_id,
           region_id,
           2,
           NULL,
           target_entity_id,
           authority_class,
           participant_count,
           (participant_count * (participant_count - 1)) / 2,
           max_per_bridge::SMALLINT
      FROM bridge
     WHERE (participant_count * (participant_count - 1)) / 2 > max_per_bridge
    ON CONFLICT (run_id, document_id, region_id,
                 bridge_entity_id, identity_authority_class)
        WHERE bridge_kind = 2
    DO UPDATE SET
        participant_count = EXCLUDED.participant_count,
        possible_pair_count = EXCLUDED.possible_pair_count,
        retained_pair_limit = EXCLUDED.retained_pair_limit,
        observed_at = CURRENT_TIMESTAMP;

    RETURN local_inserted + identity_inserted;
END;
$$;

-- Only the two existing rebuildable L3 substitution owners are retired.
-- No witness/admission revision, immutable premise, or unrelated rule
-- is modified. The existing owner can rebuild once authority is paid.
CREATE OR REPLACE FUNCTION execution.slr_suspend_native_identity_derivations(selected_demands bigint[])
RETURNS bigint LANGUAGE plpgsql AS $slr_l3$
DECLARE retired bigint;
BEGIN
    IF COALESCE(cardinality(selected_demands),0)=0 THEN RETURN 0; END IF;
    WITH blocked AS MATERIALIZED (
        SELECT DISTINCT factor_id,authority_class
        FROM execution.slr_native_blocked_identity_slot_v1
        WHERE demand_id=ANY(selected_demands)
    )
    DELETE FROM execution.semantic_pnf_factor_derivation d
    USING execution.semantic_pnf_factor_derivation_premise p,blocked b
    WHERE d.derivation_id=p.derivation_id AND p.premise_ordinal=0
      AND p.factor_id=b.factor_id AND d.authority_class=b.authority_class
      AND d.epistemic_level=3
      AND d.rule_ref IN('identity-substitution:v1','identity-supported-substitution:v1');
    GET DIAGNOSTICS retired=ROW_COUNT;
    -- Composition uses ranked region/entity/authority fibres. Retire the
    -- entire affected rank group, not just a pair whose rank may shift.
    -- m084 remains the producer; this is withdrawal, not new composition.
    DELETE FROM execution.semantic_pnf_factor_composition_candidate c
    USING execution.slr_native_blocked_composition_group_v1 b
    WHERE b.demand_id=ANY(selected_demands)
      AND c.region_id=b.region_id AND c.bridge_entity_id=b.target_entity_id
      AND c.identity_authority_class=b.authority_class;
    DELETE FROM execution.semantic_pnf_factor_composition_overflow c
    USING execution.slr_native_blocked_composition_group_v1 b
    WHERE b.demand_id=ANY(selected_demands) AND c.bridge_kind=2
      AND c.region_id=b.region_id AND c.bridge_entity_id=b.target_entity_id
      AND c.identity_authority_class=b.authority_class;
    RETURN retired;
END;
$slr_l3$;

-- Reuse original m094/m139 reverse-dependency and consumer work queue.
-- Like m163, consume only the statement-local inserted delta. Initial
-- supported observation is not a validity change (m139 owns insertion).
-- First observed withdrawal and later true/false revisions are changes.
-- No new dependency graph, authority event or semantic refutation.
CREATE OR REPLACE FUNCTION execution.slr_wake_native_validity_delta()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Debt belongs to the original demand, not the calling consumer or
    -- the current support bit. A later restoration must not delete it.
    -- Both validity journals use this statement-local path.
    INSERT INTO execution.slr_native_h3_authority_debt(demand_id)
    SELECT DISTINCT v.demand_id FROM inserted_validity v
    JOIN execution.slr_native_h6_managed_demand m ON m.demand_id=v.demand_id
    WHERE NOT v.supported
    ON CONFLICT DO NOTHING;

    WITH atoms AS MATERIALIZED (
        SELECT e.evidence_id,e.source_region_id,e.source_interface_id
        FROM inserted_validity v
        JOIN execution.semantic_pnf_candidate_evidence e
          ON e.evidence_id=v.evidence_id AND e.demand_id=v.demand_id
        WHERE v.revision>1 OR NOT v.supported
    ), changed_source AS MATERIALIZED (
        SELECT 6::smallint AS source_kind,evidence_id AS source_id FROM atoms
        UNION SELECT 4::smallint,source_region_id FROM atoms WHERE source_region_id IS NOT NULL
        UNION SELECT 5::smallint,source_interface_id FROM atoms WHERE source_interface_id IS NOT NULL
    ), affected AS MATERIALIZED (
        SELECT d.demand_id,d.consumer_ref,d.query_ref,d.policy_ref,min(d.minimum_horizon) AS minimum_horizon
        FROM changed_source c
        JOIN execution.semantic_pnf_consumer_reverse_dependency d
          ON d.source_kind=c.source_kind AND d.source_id=c.source_id
        GROUP BY d.demand_id,d.consumer_ref,d.query_ref,d.policy_ref
    ), work AS (
        SELECT demand_id,consumer_ref,query_ref,policy_ref,minimum_horizon AS horizon FROM affected
        UNION
        SELECT q.demand_id,q.consumer_ref,q.query_ref,q.policy_ref,q.horizon
        FROM affected a
        JOIN execution.semantic_pnf_consumer_horizon_work_queue q
          ON q.demand_id=a.demand_id AND q.consumer_ref=a.consumer_ref
         AND q.query_ref=a.query_ref AND q.policy_ref=a.policy_ref AND q.horizon>=a.minimum_horizon
    )
    INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue AS q
        (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
    SELECT demand_id,consumer_ref,query_ref,policy_ref,horizon,1::smallint,NULL FROM work
    ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
    DO UPDATE SET work_state=1,completed_at=NULL
    WHERE q.work_state<>1 OR q.completed_at IS NOT NULL;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS slr_native_h3_validity_delta_wake ON execution.slr_native_h3_validity_history;
CREATE TRIGGER slr_native_h3_validity_delta_wake
AFTER INSERT ON execution.slr_native_h3_validity_history
REFERENCING NEW TABLE AS inserted_validity
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_wake_native_validity_delta();
DROP TRIGGER IF EXISTS slr_native_h6_validity_delta_wake ON execution.slr_native_h6_validity_history;
CREATE TRIGGER slr_native_h6_validity_delta_wake
AFTER INSERT ON execution.slr_native_h6_validity_history
REFERENCING NEW TABLE AS inserted_validity
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_wake_native_validity_delta();

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
        -- m086 represented_possible is TRUE for every universe row.
        -- Physical support must not join execution/admissibility history.
        SELECT u.demand_id,u.target_kind,u.target_id
          FROM execution.semantic_pnf_candidate_universe u
          JOIN ready ON ready.demand_id=u.demand_id
         WHERE u.target_kind=1 AND u.target_id<>ready.source_object_id
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
-- One keyed evidence traversal, three filtered accumulators. Keep SUM
-- numeric until the original bigint output boundary; zero evidence counts
-- separately from absent evidence. Do not prefilter away neutral atoms.
-- Lean NumericPnfRustPortContract.three_horizon_tallies_exact.
WITH candidate_bins AS NOT MATERIALIZED (
    SELECT u.demand_id,u.target_kind,u.target_id,
      COALESCE(sum(e.signed_residual) FILTER (WHERE e.horizon<=3),0) AS signed3,
      COALESCE(sum(e.signed_residual) FILTER (WHERE e.horizon<=6),0) AS signed6,
      COALESCE(sum(e.signed_residual) FILTER (WHERE e.horizon<=9),0) AS signed9,
      count(e.evidence_id) FILTER (WHERE e.horizon<=3) AS count3,
      count(e.evidence_id) FILTER (WHERE e.horizon<=6) AS count6,
      count(e.evidence_id) FILTER (WHERE e.horizon<=9) AS count9
    FROM execution.semantic_pnf_candidate_universe u
    LEFT JOIN execution.slr_native_current_candidate_evidence_v1 e
      ON e.demand_id=u.demand_id AND e.target_kind=u.target_kind AND e.target_id=u.target_id
    GROUP BY u.demand_id,u.target_kind,u.target_id
)
SELECT b.demand_id,b.target_kind,b.target_id,h.horizon,
       h.signed_residual::bigint AS signed_residual,
       sign(h.signed_residual)::smallint AS phase,
       h.evidence_count::bigint AS evidence_count
FROM candidate_bins b
CROSS JOIN LATERAL (VALUES
    (3::smallint,b.signed3,b.count3),
    (6::smallint,b.signed6,b.count6),
    (9::smallint,b.signed9,b.count9)
) h(horizon,signed_residual,evidence_count);

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

-- Reuse the original authority lanes (m086, m075 and m095). A drained
-- represented slice can retire *execution debt* when there is no current
-- authority to revalidate. It cannot retire or manufacture an authority.
-- This function is called after the Rust producer's exact current projection
-- comparison, in its serializable demand transaction. No coverage is paid.
CREATE TABLE IF NOT EXISTS execution.slr_native_candidate_reconciliation_history(
    reconciliation_id bigint GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    demand_id bigint NOT NULL,
    debt_first_observed_at timestamptz NOT NULL,
    reconciled_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reconciliation_kind text NOT NULL CHECK(reconciliation_kind='authority-free-represented-slice:v1')
);

CREATE OR REPLACE FUNCTION execution.slr_native_candidate_only_authority_absent(selected_demand_id bigint)
RETURNS boolean LANGUAGE sql STABLE AS $$
SELECT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand WHERE demand_id=selected_demand_id)
 AND EXISTS(SELECT 1 FROM execution.semantic_pnf_demand d WHERE d.demand_id=selected_demand_id
       AND d.resolved_target_kind IS NULL AND d.resolved_target_id IS NULL)
 AND NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_latest_admissibility WHERE demand_id=selected_demand_id)
 AND NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_frontier_resolution WHERE demand_id=selected_demand_id AND outcome_state=2)
 -- Identity is a distinct authority lane. A historical admitted witness
 -- may survive a changed frontier; candidate execution cannot reactivate it.
 AND NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_witness w
    JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id
    WHERE w.demand_id=selected_demand_id AND a.admission_state=2)
 AND NOT EXISTS(SELECT 1 FROM execution.slr_native_blocked_identity_fibre_v1 b
    WHERE b.demand_id=selected_demand_id)
 AND EXISTS(SELECT 1 FROM execution.semantic_pnf_demand_open_world_state WHERE demand_id=selected_demand_id AND outside_model_possible)
 AND NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_sufficiency_current_v1 WHERE demand_id=selected_demand_id AND certificate_state=1);
$$;

CREATE OR REPLACE FUNCTION execution.slr_reconcile_native_candidate_only_debt(
    selected_demand_id bigint, represented_slice_drained boolean, projection_exact boolean
) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE retired bigint;
BEGIN
    IF current_setting('transaction_isolation') <> 'serializable' THEN
        RAISE EXCEPTION 'native authority reconciliation requires serializable producer transaction';
    END IF;
    IF represented_slice_drained IS NOT TRUE OR projection_exact IS NOT TRUE THEN RETURN false; END IF;
    PERFORM 1 FROM execution.semantic_pnf_demand WHERE demand_id=selected_demand_id FOR UPDATE;
    IF NOT execution.slr_native_candidate_only_authority_absent(selected_demand_id) THEN RETURN false; END IF;
    WITH removed AS (
        DELETE FROM execution.slr_native_h3_authority_debt WHERE demand_id=selected_demand_id
        RETURNING demand_id,first_observed_at
    )
    INSERT INTO execution.slr_native_candidate_reconciliation_history
      (demand_id,debt_first_observed_at,reconciliation_kind)
    SELECT demand_id,first_observed_at,'authority-free-represented-slice:v1' FROM removed;
    GET DIAGNOSTICS retired=ROW_COUNT;
    RETURN retired=1;
END;
$$;

-- Original m095 certificate state is historical authority. A physical
-- withdrawal does not fabricate a replacement certificate or restore a
-- refuted candidate. It only suspends consumption until the owner checks it.
CREATE OR REPLACE FUNCTION execution.slr_native_current_authority_usable(selected_demand_id bigint)
RETURNS boolean LANGUAGE sql STABLE AS $$
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
$$;

-- Preserve the m095 public stop ABI and all query/policy certificate rules.
-- The additional conjunction is a consumption guard, not new authority.
CREATE OR REPLACE FUNCTION execution.numeric_pnf_consumer_stop_at_horizon(
    selected_demand_id bigint, selected_consumer_ref text, selected_query_ref text,
    selected_policy_ref text, selected_horizon smallint
) RETURNS boolean LANGUAGE sql STABLE AS $$
SELECT execution.slr_native_current_authority_usable(selected_demand_id) AND EXISTS(
    SELECT 1 FROM execution.semantic_pnf_consumer_sufficiency_current_v1 certificate
    WHERE certificate.demand_id=selected_demand_id
      AND certificate.consumer_ref=selected_consumer_ref AND certificate.query_ref=selected_query_ref
      AND certificate.policy_ref=selected_policy_ref AND certificate.certificate_state=1
      AND certificate.horizon<=selected_horizon
      AND ((selected_policy_ref='' AND certificate.certificate_kind IN(1,3))
        OR (selected_policy_ref<>'' AND certificate.certificate_kind IN(2,3)))
);
$$;

-- Production SLR H6 producer. Reuse the single physical signature
-- relation above, but intersect it with original m110 execution permission:
-- ready consumer/query/policy, no prior stop/proof, admissible target.
-- Physical support and permission remain different relations.
CREATE OR REPLACE FUNCTION execution.slr_produce_native_h6_for_demand(
    selected_demand_id bigint, selected_consumer_ref text,
    selected_query_ref text, selected_policy_ref text
) RETURNS bigint LANGUAGE plpgsql AS $slr_h6$
DECLARE inserted bigint;
BEGIN
    IF NOT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand
                  WHERE demand_id=selected_demand_id) THEN
        RAISE EXCEPTION 'unmanaged native H6 producer scope';
    END IF;
    IF NOT execution.slr_native_current_authority_usable(selected_demand_id) THEN
        RETURN 0;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue w
        WHERE w.demand_id=selected_demand_id AND w.consumer_ref=selected_consumer_ref
          AND w.query_ref=selected_query_ref AND w.policy_ref=selected_policy_ref
          AND w.horizon=6 AND w.work_state=1)
        OR execution.numeric_pnf_consumer_stop_at_horizon(
           selected_demand_id,selected_consumer_ref,selected_query_ref,selected_policy_ref,3::smallint)
        OR EXISTS(SELECT 1 FROM execution.semantic_pnf_frontier_resolution proof
          WHERE proof.demand_id=selected_demand_id AND proof.outcome_state=2 AND proof.candidate_count=1)
    THEN RETURN 0; END IF;

    INSERT INTO execution.semantic_pnf_candidate_evidence
      (demand_id,target_kind,target_id,evidence_ref,evidence_family,horizon,
       signed_residual,evidence_kind,provenance_ref,source_interface_id,source_region_id)
    SELECT p.demand_id,p.target_kind,p.target_id,p.evidence_ref,2,6,
           p.signed_residual,p.evidence_kind,p.provenance_ref,p.source_interface_id,p.source_region_id
    FROM execution.slr_current_m110_h6_atoms(selected_demand_id) p
    WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_latest_admissibility a
        WHERE a.demand_id=p.demand_id AND a.target_kind=p.target_kind
          AND a.target_id=p.target_id AND a.event_kind=1)
    ON CONFLICT(demand_id,target_kind,target_id,evidence_ref) DO NOTHING;
    GET DIAGNOSTICS inserted=ROW_COUNT;
    RETURN inserted;
END;
$slr_h6$;

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
    INSERT INTO execution.semantic_pnf_consumer_horizon_outcome AS prior
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
        refreshed_at=EXCLUDED.refreshed_at
    -- A replay observes the same outcome; it is not a new publication.
    -- Retain its original change timestamp and avoid triggering the
    -- whole-endpoint revision fence for an unchanged payload.
    WHERE ROW(prior.outcome_state,prior.evidence_count,prior.nonneutral_evidence_count,
              prior.represented_candidate_count,prior.preferred_candidate_count,
              prior.proof_unique,prior.consumer_sufficient,prior.residual_required)
      IS DISTINCT FROM
          ROW(EXCLUDED.outcome_state,EXCLUDED.evidence_count,EXCLUDED.nonneutral_evidence_count,
              EXCLUDED.represented_candidate_count,EXCLUDED.preferred_candidate_count,
              EXCLUDED.proof_unique,EXCLUDED.consumer_sufficient,EXCLUDED.residual_required);
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
-- A pre-v4 journal may have recorded withdrawal then restoration without
-- durable debt. Conservatively reopen that historical obligation once,
-- rather than assuming an earlier certificate remains usable.
INSERT INTO execution.slr_native_h3_authority_debt(demand_id)
SELECT DISTINCT historical.demand_id
FROM (
    SELECT demand_id FROM execution.slr_native_h3_validity_history WHERE NOT supported
    UNION
    SELECT demand_id FROM execution.slr_native_h6_validity_history WHERE NOT supported
) historical
JOIN execution.slr_native_h6_managed_demand m USING(demand_id)
WHERE NOT EXISTS(SELECT 1 FROM execution.slr_native_current_authority_contract
    WHERE contract_ref='slr-native-current-authority:v4')
ON CONFLICT DO NOTHING;
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v4') ON CONFLICT DO NOTHING;
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v5') ON CONFLICT DO NOTHING;
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v6') ON CONFLICT DO NOTHING;
-- Repair stale derived projections from earlier versions only once.
SELECT execution.slr_suspend_native_debt_outcomes(
    ARRAY(SELECT demand_id FROM execution.slr_native_h3_authority_debt))
WHERE NOT EXISTS(SELECT 1 FROM execution.slr_native_current_authority_contract
    WHERE contract_ref='slr-native-current-authority:v7');
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v7') ON CONFLICT DO NOTHING;
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v8') ON CONFLICT DO NOTHING;
SELECT execution.slr_suspend_native_identity_derivations(
    ARRAY(SELECT demand_id FROM execution.slr_native_h3_authority_debt))
WHERE NOT EXISTS(SELECT 1 FROM execution.slr_native_current_authority_contract
    WHERE contract_ref='slr-native-current-authority:v9');
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v9') ON CONFLICT DO NOTHING;
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v10') ON CONFLICT DO NOTHING;

-- Upgrade existing debt groups without changing original admission rows.
SELECT execution.slr_suspend_native_identity_derivations(
    ARRAY(SELECT demand_id FROM execution.slr_native_h3_authority_debt))
WHERE NOT EXISTS(SELECT 1 FROM execution.slr_native_current_authority_contract
    WHERE contract_ref='slr-native-current-authority:v11');
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v11') ON CONFLICT DO NOTHING;

-- Upgrade current debt once to retire stale rank overflow as well as pairs.
SELECT execution.slr_suspend_native_identity_derivations(
    ARRAY(SELECT demand_id FROM execution.slr_native_h3_authority_debt))
WHERE NOT EXISTS(SELECT 1 FROM execution.slr_native_current_authority_contract
    WHERE contract_ref='slr-native-current-authority:v12');
INSERT INTO execution.slr_native_current_authority_contract VALUES('slr-native-current-authority:v12') ON CONFLICT DO NOTHING;
