-- Native horizon withdrawal regression over a REAL represented H3 atom.
-- Run only in a rollback transaction on the original numeric DB. This is
-- a negative-control projection test, not a new semantic authority event.
\set ON_ERROR_STOP on
BEGIN ISOLATION LEVEL SERIALIZABLE;
SET LOCAL statement_timeout='90000ms';
SELECT set_config('slr.test_demand', :'demand_id', true);
DO $$
DECLARE d bigint := current_setting('slr.test_demand')::bigint;
        atom record; before_row record; after_row record; h smallint;
        represented bigint; events bigint; history bigint; debt_history bigint;
BEGIN
    IF NOT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand WHERE demand_id=d) THEN
        RAISE EXCEPTION 'not the native managed demand';
    END IF;
    SELECT e.* INTO STRICT atom FROM execution.semantic_pnf_candidate_evidence e
      JOIN execution.slr_native_h3_validity_current v ON v.evidence_id=e.evidence_id
      WHERE e.demand_id=d AND e.horizon=3 AND v.supported
      AND EXISTS(SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 c WHERE c.evidence_id=e.evidence_id)
      ORDER BY e.evidence_id LIMIT 1;
    SELECT count(*) INTO represented FROM execution.semantic_pnf_candidate_universe WHERE demand_id=d;
    SELECT count(*) INTO events FROM execution.semantic_pnf_candidate_admissibility_event WHERE demand_id=d;
    SELECT count(*) INTO history FROM execution.slr_native_h3_validity_history WHERE demand_id=d;
    FOREACH h IN ARRAY ARRAY[3,6,9]::smallint[] LOOP
        SELECT signed_residual,evidence_count INTO STRICT before_row
          FROM execution.semantic_pnf_candidate_evidence_horizon_v1
          WHERE demand_id=d AND target_kind=atom.target_kind AND target_id=atom.target_id AND horizon=h;
        UPDATE execution.slr_native_h3_validity_current SET supported=false WHERE evidence_id=atom.evidence_id;
        SELECT signed_residual,evidence_count INTO STRICT after_row
          FROM execution.semantic_pnf_candidate_evidence_horizon_v1
          WHERE demand_id=d AND target_kind=atom.target_kind AND target_id=atom.target_id AND horizon=h;
        IF after_row.signed_residual<>before_row.signed_residual-atom.signed_residual
           OR after_row.evidence_count<>before_row.evidence_count-1 THEN
            RAISE EXCEPTION 'H3 withdrawal did not transport exactly to horizon %',h;
        END IF;
        IF EXISTS(SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 WHERE evidence_id=atom.evidence_id)
           OR NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_evidence WHERE evidence_id=atom.evidence_id) THEN
            RAISE EXCEPTION 'historical/current evidence distinction violated';
        END IF;
        UPDATE execution.slr_native_h3_validity_current SET supported=true WHERE evidence_id=atom.evidence_id;
    END LOOP;
    INSERT INTO execution.slr_native_h3_authority_debt(demand_id) VALUES(d) ON CONFLICT DO NOTHING;
    IF execution.slr_native_current_authority_usable(d) THEN
        RAISE EXCEPTION 'pending nonmonotone authority was reusable';
    END IF;
    SELECT count(*) INTO debt_history FROM execution.slr_native_candidate_reconciliation_history WHERE demand_id=d;
    IF execution.slr_reconcile_native_candidate_only_debt(d,false,true)
       OR execution.slr_reconcile_native_candidate_only_debt(d,true,false) THEN
        RAISE EXCEPTION 'unchecked or undrained execution cleared authority debt';
    END IF;
    IF execution.slr_native_candidate_only_authority_absent(d) THEN
        IF NOT execution.slr_reconcile_native_candidate_only_debt(d,true,true) THEN
            RAISE EXCEPTION 'checked authority-free represented slice remained blocked';
        END IF;
        IF NOT execution.slr_native_current_authority_usable(d) THEN
            RAISE EXCEPTION 'settled candidate-only debt still suspended execution';
        END IF;
        IF execution.slr_reconcile_native_candidate_only_debt(d,true,true)
           OR (SELECT count(*) FROM execution.slr_native_candidate_reconciliation_history WHERE demand_id=d)<>debt_history+1 THEN
            RAISE EXCEPTION 'candidate reconciliation was not idempotent';
        END IF;
    ELSE
        IF execution.slr_reconcile_native_candidate_only_debt(d,true,true) THEN
            RAISE EXCEPTION 'candidate-only reconciliation consumed original authority';
        END IF;
        IF execution.slr_native_current_authority_usable(d) THEN
            RAISE EXCEPTION 'original authority debt was no longer suspended';
        END IF;
    END IF;
    IF represented<>(SELECT count(*) FROM execution.semantic_pnf_candidate_universe WHERE demand_id=d)
       OR events<>(SELECT count(*) FROM execution.semantic_pnf_candidate_admissibility_event WHERE demand_id=d)
       OR history<>(SELECT count(*) FROM execution.slr_native_h3_validity_history WHERE demand_id=d) THEN
        RAISE EXCEPTION 'projection test changed original represented/admissibility/history carrier';
    END IF;
END;
$$;

-- Original m094 queue, m139 coordinates and m163 statement-local delta.
-- Prove that a validity revision wakes an exact interested consumer and
-- already-existing downstream horizons, not an unrelated consumer.
DO $$
DECLARE d bigint := current_setting('slr.test_demand')::bigint;
        atom record; prior_revision bigint; key_ref text := 'slr:rollback-validity:'||txid_current()::text;
        unrelated_ref text := key_ref||':unrelated';
        qref text := key_ref||':query';
BEGIN
    SELECT e.* INTO STRICT atom FROM execution.semantic_pnf_candidate_evidence e
      JOIN execution.slr_native_h3_validity_current v ON v.evidence_id=e.evidence_id
      WHERE e.demand_id=d AND e.horizon=3 AND v.supported
      ORDER BY e.evidence_id LIMIT 1;
    SELECT revision INTO STRICT prior_revision FROM execution.slr_native_h3_validity_current
      WHERE evidence_id=atom.evidence_id;
    INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
      (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
    SELECT d,c,qref,'',h,2,CURRENT_TIMESTAMP
    FROM unnest(ARRAY[key_ref,unrelated_ref]) c CROSS JOIN unnest(ARRAY[3,6,9]::smallint[]) h;
    INSERT INTO execution.semantic_pnf_consumer_reverse_dependency
      (source_kind,source_id,demand_id,consumer_ref,query_ref,policy_ref,minimum_horizon,dependency_kind)
    VALUES(6,atom.evidence_id,d,key_ref,qref,'',6,1);

    -- Isolate the journal's creation of debt from the manual debt test
    -- above. This is a rollback-only negative-control setup.
    DELETE FROM execution.slr_native_h3_authority_debt WHERE demand_id=d;

    -- Initial supported observation is not a change. An empty journal
    -- insertion must not wake either test consumer.
    INSERT INTO execution.slr_native_h3_validity_history
      (evidence_id,demand_id,revision,supported,original_atom)
    SELECT atom.evidence_id,d,prior_revision,true,to_jsonb(atom) WHERE false;
    IF EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue
      WHERE demand_id=d AND query_ref=qref AND work_state<>2) THEN
        RAISE EXCEPTION 'empty validity delta woke a consumer';
    END IF;

    UPDATE execution.slr_native_h3_validity_current
      SET supported=false,revision=prior_revision+1 WHERE evidence_id=atom.evidence_id;
    INSERT INTO execution.slr_native_h3_validity_history
      (evidence_id,demand_id,revision,supported,original_atom)
    VALUES(atom.evidence_id,d,prior_revision+1,false,to_jsonb(atom));

    IF (SELECT count(*) FROM execution.semantic_pnf_consumer_horizon_work_queue
       WHERE demand_id=d AND consumer_ref=key_ref AND query_ref=qref
       AND horizon IN(6,9) AND work_state=1 AND completed_at IS NULL)<>2
       OR EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue
       WHERE demand_id=d AND query_ref=qref AND (consumer_ref=unrelated_ref OR horizon=3)
       AND work_state<>2) THEN
        RAISE EXCEPTION 'validity withdrawal wake was not exact';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM execution.slr_native_h3_authority_debt WHERE demand_id=d)
       OR execution.slr_native_current_authority_usable(d) THEN
        RAISE EXCEPTION 'journal withdrawal failed to suspend original authority';
    END IF;

    UPDATE execution.slr_native_h3_validity_current
      SET supported=true,revision=prior_revision+2 WHERE evidence_id=atom.evidence_id;
    INSERT INTO execution.slr_native_h3_validity_history
      (evidence_id,demand_id,revision,supported,original_atom)
    VALUES(atom.evidence_id,d,prior_revision+2,true,to_jsonb(atom));
    IF NOT EXISTS(SELECT 1 FROM execution.slr_native_h3_authority_debt WHERE demand_id=d)
       OR execution.slr_native_current_authority_usable(d) THEN
        RAISE EXCEPTION 'physical restoration erased authority debt';
    END IF;
END;
$$;

-- Derived authority projection isolation. These are synthetic rollback
-- outcome rows on an actual managed demand, not proof/certificate writes.
DO $slr_debt_test$
DECLARE d bigint := current_setting('slr.test_demand')::bigint;
        c text := 'slr:rollback-debt:'||txid_current()::text;
        q text := c||':query';
        unaffected text := c||':candidate-only';
BEGIN
    DELETE FROM execution.slr_native_h3_authority_debt WHERE demand_id=d;
    INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
      (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
    VALUES(d,c,q,'',6,2,CURRENT_TIMESTAMP),(d,unaffected,q,'',6,2,CURRENT_TIMESTAMP);
    INSERT INTO execution.semantic_pnf_consumer_horizon_outcome
      (demand_id,consumer_ref,query_ref,policy_ref,horizon,outcome_state,
       evidence_count,nonneutral_evidence_count,represented_candidate_count,
       preferred_candidate_count,proof_unique,consumer_sufficient,residual_required)
    VALUES(d,c,q,'',6,4,1,1,1,1,true,false,false),
          (d,unaffected,q,'',6,2,1,1,1,1,false,false,true);
    -- No reverse dependency is registered by either fixture consumer.
    -- Demand-level authority debt must nevertheless invalidate the old
    -- authority outcome, without waking a candidate-only unrelated fibre.
    INSERT INTO execution.slr_native_h3_authority_debt(demand_id) VALUES(d);
    IF EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_outcome o
      WHERE o.demand_id=d AND o.consumer_ref=c AND o.query_ref=q)
       OR NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue w
      WHERE w.demand_id=d AND w.consumer_ref=c AND w.query_ref=q
      AND w.horizon=6 AND w.work_state=1 AND w.completed_at IS NULL)
       OR NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_outcome o
      WHERE o.demand_id=d AND o.consumer_ref=unaffected AND o.query_ref=q AND o.outcome_state=2)
       OR NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue w
      WHERE w.demand_id=d AND w.consumer_ref=unaffected AND w.query_ref=q AND w.work_state=2)
    THEN RAISE EXCEPTION 'demand-wide authority suspension or candidate isolation failed'; END IF;
    IF execution.slr_suspend_native_debt_outcomes(ARRAY[d])<>0 THEN
        RAISE EXCEPTION 'repeated debt suspension was not a no-op';
    END IF;
END;
$slr_debt_test$;

-- The original outcome refresh must not mutate a stable projection.
DO $slr_noop_test$
DECLARE d bigint := current_setting('slr.test_demand')::bigint;
        c text := 'slr:rollback-noop:'||txid_current()::text;
        q text := c||':query';
        run_id_value bigint;
        document_id_value bigint;
        first_write bigint;
        second_write bigint;
        before_row jsonb;
        after_row jsonb;
BEGIN
    SELECT r.run_id,r.document_id INTO STRICT run_id_value,document_id_value
      FROM execution.semantic_pnf_demand demand
      JOIN execution.semantic_pnf_region r ON r.region_id=demand.source_region_id
      WHERE demand.demand_id=d;
    INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
      (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
    VALUES(d,c,q,'',3,2,CURRENT_TIMESTAMP);
    first_write := execution.refresh_numeric_pnf_consumer_horizon_outcome(
        run_id_value,document_id_value,3::smallint,c,q,'');
    SELECT to_jsonb(o) INTO STRICT before_row FROM execution.semantic_pnf_consumer_horizon_outcome o
      WHERE o.demand_id=d AND o.consumer_ref=c AND o.query_ref=q AND o.horizon=3;
    second_write := execution.refresh_numeric_pnf_consumer_horizon_outcome(
        run_id_value,document_id_value,3::smallint,c,q,'');
    SELECT to_jsonb(o) INTO STRICT after_row FROM execution.semantic_pnf_consumer_horizon_outcome o
      WHERE o.demand_id=d AND o.consumer_ref=c AND o.query_ref=q AND o.horizon=3;
    IF first_write<>1 OR second_write<>0 OR before_row IS DISTINCT FROM after_row THEN
        RAISE EXCEPTION 'stable consumer outcome caused physical rewrite';
    END IF;
END;
$slr_noop_test$;

-- Actual admitted witness fibres only: do not insert a synthetic witness
-- or close the open world to make this test nonvacuous. An absent focus
-- witness is reported as zero, never as live identity acceptance.
DO $slr_l3_test$
DECLARE d bigint := current_setting('slr.test_demand')::bigint;
        run_value bigint;
        doc_value bigint;
        before_admissions jsonb;
        after_admissions jsonb;
        blocked_groups bigint;
        blocked_slots bigint;
        removed_l3 bigint;
BEGIN
    SELECT r.run_id,r.document_id INTO STRICT run_value,doc_value
    FROM execution.semantic_pnf_demand demand
    JOIN execution.semantic_pnf_region r ON r.region_id=demand.source_region_id
    WHERE demand.demand_id=d;

    DELETE FROM execution.slr_native_h3_authority_debt WHERE demand_id=d;
    CREATE TEMP TABLE slr_l3_before ON COMMIT DROP AS
    SELECT x.derivation_id,x.authority_class,p.factor_id,x.rule_ref,x.epistemic_level
    FROM execution.semantic_pnf_factor_derivation x
    JOIN execution.semantic_pnf_factor_derivation_premise p ON p.derivation_id=x.derivation_id AND p.premise_ordinal=0
    JOIN execution.semantic_pnf_factor f ON f.factor_id=p.factor_id
    JOIN execution.semantic_pnf_region r ON r.region_id=f.region_id
    WHERE r.run_id=run_value AND r.document_id=doc_value;
    SELECT jsonb_agg(to_jsonb(a) ORDER BY a.witness_id) INTO before_admissions
    FROM execution.semantic_pnf_identity_witness_admission a
    JOIN execution.semantic_pnf_identity_witness w USING(witness_id)
    JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
    JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
    WHERE r.run_id=run_value AND r.document_id=doc_value;

    INSERT INTO execution.slr_native_h3_authority_debt(demand_id) VALUES(d);
    SELECT count(*) INTO blocked_groups FROM execution.slr_native_blocked_identity_fibre_v1 WHERE demand_id=d;
    SELECT count(*) INTO blocked_slots FROM execution.slr_native_blocked_identity_slot_v1 WHERE demand_id=d;
    IF blocked_groups>0 AND (execution.slr_native_candidate_only_authority_absent(d)
       OR execution.slr_reconcile_native_candidate_only_debt(d,true,true)) THEN
        RAISE EXCEPTION 'candidate-only execution reactivated suspended admitted identity';
    END IF;
    IF EXISTS(SELECT 1 FROM execution.slr_native_blocked_identity_fibre_v1 b
        JOIN execution.semantic_pnf_identity_projection p USING(source_object_id,authority_class)
        WHERE b.demand_id=d)
       OR EXISTS(SELECT 1 FROM execution.slr_native_blocked_identity_slot_v1 b
        JOIN execution.semantic_pnf_factor_identity_support_unique_v1 p USING(factor_id,slot_ordinal,authority_class)
        WHERE b.demand_id=d) THEN
        RAISE EXCEPTION 'blocked identity or shared-token fibre still projects';
    END IF;
    -- Compare every prior derivation ID to the exact reverse cone.
    IF EXISTS(SELECT 1 FROM slr_l3_before old
      WHERE (NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation current_row
                          WHERE current_row.derivation_id=old.derivation_id))
      IS DISTINCT FROM
        (old.epistemic_level=3 AND old.rule_ref IN('identity-substitution:v1','identity-supported-substitution:v1')
         AND EXISTS(SELECT 1 FROM execution.slr_native_blocked_identity_slot_v1 b
            WHERE b.demand_id=d AND b.factor_id=old.factor_id AND b.authority_class=old.authority_class)))
    THEN RAISE EXCEPTION 'Level-3 retirement was not the exact reverse cone'; END IF;

    SELECT jsonb_agg(to_jsonb(a) ORDER BY a.witness_id) INTO after_admissions
    FROM execution.semantic_pnf_identity_witness_admission a
    JOIN execution.semantic_pnf_identity_witness w USING(witness_id)
    JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
    JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
    WHERE r.run_id=run_value AND r.document_id=doc_value;
    IF before_admissions IS DISTINCT FROM after_admissions THEN
        RAISE EXCEPTION 'execution suspension modified original admission authority';
    END IF;
    SELECT count(*) INTO removed_l3 FROM slr_l3_before old
    WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation x WHERE x.derivation_id=old.derivation_id);
    IF execution.slr_suspend_native_identity_derivations(ARRAY[d])<>0 THEN
        RAISE EXCEPTION 'repeat Level-3 suspension was not a no-op';
    END IF;
    RAISE NOTICE 'actual blocked identity groups %, slots %, retired L3 %; zero counts do not pay live identity acceptance',
        blocked_groups,blocked_slots,removed_l3;
END;
$slr_l3_test$;

ROLLBACK;
