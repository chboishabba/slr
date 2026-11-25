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

ROLLBACK;
