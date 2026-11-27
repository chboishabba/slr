-- Production-only correction of pinned original SensibLaw m209
-- f9c4670ef04b40e8153caa7dd00fa1eba7013ef4, blob 1772922f7d67fa8057f7db7584c799f6bbfa8bbf.
-- Golden: DASHI RelationDeltaReconciliationExact (distinct membership,
-- replacement and unchanged partitions). No new candidate reducer.
-- The original count was computed after deleting mismatched current rows;
-- that incorrectly labelled replacement as remove+add in durable receipts.
-- Only patch the exact known function body. Unknown owners fail closed.
DO $slr_patch$
DECLARE
    owner_oid regprocedure := to_regprocedure('execution.reconcile_numeric_pnf_parent_candidates(bigint,bigint)');
    original_sha constant text := '730432d022e23626590a613c3ff17c6b718a077da511c03b24f62354c87ab328';
    corrected_sha constant text := '6dcf235787f35dd0dcd4b518001664fe5183a422a2fdc2c5eb542ca3db34ac6f';
    count_block constant text := $move$    SELECT count(*) INTO replaced_total
      FROM execution.semantic_pnf_demand_candidate AS candidate
      JOIN pg_temp.numeric_pnf_desired_parent_candidate AS desired
        ON desired.demand_id = candidate.demand_id
       AND desired.target_kind = candidate.target_kind
       AND desired.target_id = candidate.target_id
     WHERE EXISTS (
         SELECT 1 FROM execution.semantic_pnf_parent_affected_key AS key
          WHERE key.parent_region_id = selected_region_id
            AND key.key_family = 3 AND key.key_a = candidate.demand_id
     )
       AND ROW(candidate.ordinal, candidate.source_interface_id,
               candidate.ancestor_distance, candidate.index_rank,
               candidate.candidate_score, candidate.common_scope_interface_id,
               candidate.validation_state)
           IS DISTINCT FROM
           ROW(desired.ordinal, desired.source_interface_id,
               desired.ancestor_distance, desired.index_rank,
               desired.candidate_score, desired.common_scope_interface_id,
               desired.validation_state);

$move$;
    old_line constant text := $old$    unchanged_total := desired_total - added_total - replaced_total;$old$;
    new_line constant text := $new$    -- The DELETE/INSERT diagnostics above count physical mutations.
    -- Replacement is one deletion AND one insertion of the same key.
    IF replaced_total > added_total OR replaced_total > removed_total THEN
        RAISE EXCEPTION 'SLR m209: replacement accounting exceeds physical mutations';
    END IF;
    added_total := added_total - replaced_total;
    removed_total := removed_total - replaced_total;
    unchanged_total := desired_total - added_total - replaced_total;$new$;
    source_body text;
    patched_body text;
    definition text;
    body_sha text;
BEGIN
    -- Pre-m209 databases retain the legacy original owner; this overlay
    -- does not install an alternative implementation into them.
    IF owner_oid IS NULL THEN RETURN; END IF;
    SELECT prosrc INTO STRICT source_body FROM pg_proc WHERE oid=owner_oid;
    body_sha := encode(sha256(convert_to(source_body,'UTF8')),'hex');
    IF body_sha=corrected_sha THEN RETURN; END IF;
    IF body_sha<>original_sha THEN
        RAISE EXCEPTION 'SLR m209 owner is unrecognized; refuse to rewrite it';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtext('slr-m209-replacement-accounting'));
    SELECT prosrc INTO STRICT source_body FROM pg_proc WHERE oid=owner_oid;
    body_sha := encode(sha256(convert_to(source_body,'UTF8')),'hex');
    IF body_sha=corrected_sha THEN RETURN; END IF;
    IF body_sha<>original_sha THEN
        RAISE EXCEPTION 'SLR m209 owner changed during guarded patch';
    END IF;
    patched_body := replace(source_body,count_block,'');
    patched_body := replace(patched_body,'    WITH doomed AS (',count_block||'    WITH doomed AS (');
    patched_body := replace(patched_body,old_line,new_line);
    IF encode(sha256(convert_to(patched_body,'UTF8')),'hex')<>corrected_sha THEN
        RAISE EXCEPTION 'SLR m209 corrected body digest mismatch';
    END IF;
    SELECT pg_get_functiondef(owner_oid) INTO STRICT definition;
    EXECUTE replace(definition,source_body,patched_body);
    SELECT prosrc INTO STRICT source_body FROM pg_proc WHERE oid=owner_oid;
    IF encode(sha256(convert_to(source_body,'UTF8')),'hex')<>corrected_sha THEN
        RAISE EXCEPTION 'SLR m209 corrected owner readback mismatch';
    END IF;
END;
$slr_patch$;
