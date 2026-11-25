-- Write from the already staged exact source support. The standalone mode
-- implements the original m085/Agda source permission without changing the
-- m090 reference mode or re-reading the document.
CREATE OR REPLACE FUNCTION pg_temp.slr_write_numeric_pnf_parser_identity_bounded(
    selected_run_id BIGINT,selected_document_id BIGINT,standalone_only BOOLEAN
) RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE
    affected_count BIGINT := 0;
    max_name_targets CONSTANT INTEGER := 16;
BEGIN
    IF standalone_only IS NULL THEN RAISE EXCEPTION 'missing standalone permission mode'; END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_temp.slr_identity_stage_scope s
        WHERE s.run_id=selected_run_id AND s.document_id=selected_document_id)
    THEN RAISE EXCEPTION 'foreign identity support stage'; END IF;
    UPDATE execution.semantic_pnf_identity_evidence_candidate
       SET evidence_state=4
     WHERE run_id=selected_run_id AND document_id=selected_document_id
       AND evidence_state IN (1,2,3);

    DELETE FROM execution.semantic_pnf_proper_name_evidence_overflow
     WHERE run_id=selected_run_id AND document_id=selected_document_id;

    WITH appos AS (
        SELECT source_token.token_id AS parser_source_token_id,
               target_token.token_id AS parser_target_token_id,
               source_anchor.object_id AS parser_source_object_id,
               target_anchor.object_id AS parser_target_object_id,
               sentence.sentence_id,
               EXISTS (
                   SELECT 1 FROM slr_identity_person AS entity
                    WHERE entity.sentence_ref=source_token.sentence_ref
                      AND source_token.start_char>=entity.start_char
                      AND source_token.end_char<=entity.end_char
               ) AS source_is_person,
               EXISTS (
                   SELECT 1 FROM slr_identity_person AS entity
                    WHERE entity.sentence_ref=target_token.sentence_ref
                      AND target_token.start_char>=entity.start_char
                      AND target_token.end_char<=entity.end_char
               ) AS target_is_person
          FROM slr_identity_token AS source_token
          CROSS JOIN slr_identity_constant AS constant
          JOIN slr_identity_token AS target_token
            ON target_token.token_id=source_token.head_token_id
           AND target_token.sentence_ref=source_token.sentence_ref
          JOIN slr_identity_sentence AS sentence ON sentence.sentence_ref=source_token.sentence_ref
          JOIN slr_identity_anchor AS source_anchor ON source_anchor.token_id=source_token.token_id
          JOIN slr_identity_anchor AS target_anchor ON target_anchor.token_id=target_token.token_id
         WHERE source_token.dependency_symbol_id=constant.appos_id
           AND source_anchor.object_id<>target_anchor.object_id
    ), appos_evidence AS (
        SELECT CASE WHEN source_is_person AND NOT target_is_person
                    THEN parser_target_object_id ELSE parser_source_object_id END AS source_object_id,
               CASE WHEN source_is_person AND NOT target_is_person
                    THEN parser_source_object_id ELSE parser_target_object_id END AS target_object_id,
               CASE WHEN source_is_person<>target_is_person THEN 4 ELSE 2 END::SMALLINT AS witness_kind,
               CASE WHEN source_is_person AND NOT target_is_person
                    THEN parser_target_token_id ELSE parser_source_token_id END AS source_token_id,
               CASE WHEN source_is_person AND NOT target_is_person
                    THEN parser_source_token_id ELSE parser_target_token_id END AS target_token_id,
               sentence_id,
               'parser-appos:'||parser_source_token_id::TEXT||':'||parser_target_token_id::TEXT AS evidence_ref,
               1::SMALLINT AS candidate_count
          FROM appos
    ), proper_name_mention AS MATERIALIZED (
        -- Legacy m090 parity mode only. Golden m085/Agda permission must
        -- independently be checked for embedded_in_person before activation.
        SELECT * FROM slr_identity_mention_scope
         WHERE source_object_id IS NOT NULL AND full_count>0
           AND (NOT standalone_only OR NOT embedded_in_person)
    ), proper_name_evidence AS (
        SELECT mention.source_object_id,person.anchor_object_id AS target_object_id,
               3::SMALLINT AS witness_kind,mention.token_id AS source_token_id,
               person.head_token_id AS target_token_id,mention.sentence_id,
               'proper-name-expansion:'||mention.token_id::TEXT||':'||person.parser_entity_id::TEXT AS evidence_ref,
               LEAST(256,mention.full_count)::SMALLINT AS candidate_count
          FROM proper_name_mention AS mention
          JOIN LATERAL (
              SELECT person.* FROM slr_identity_family AS person
               WHERE person.family_lemma_symbol_id=mention.lemma_symbol_id
                 AND person.anchor_object_id<>mention.source_object_id
                 AND NOT (mention.sentence_ref=person.sentence_ref
                     AND mention.start_char>=person.start_char AND mention.end_char<=person.end_char)
               ORDER BY person.parser_entity_id,person.anchor_object_id,person.head_token_id
               LIMIT max_name_targets
          ) AS person ON mention.eligible_count>0
    ), proper_name_overflow AS (
        SELECT source_object_id,token_id AS source_token_id,lemma_symbol_id AS family_lemma_symbol_id,
               full_count,eligible_count
          FROM proper_name_mention WHERE eligible_count>max_name_targets
    ), alias_pair AS (
        SELECT left_entity.entity_id AS left_entity_id,
               right_entity.entity_id AS right_entity_id,
               sentence.sentence_id,left_head.token_id AS left_token_id,
               right_head.token_id AS right_token_id,
               left_anchor.object_id AS left_object_id,
               right_anchor.object_id AS right_object_id,
               min(cue.token_id) AS cue_token_id
          FROM slr_identity_person AS left_entity
          JOIN slr_identity_person AS right_entity
            ON right_entity.sentence_ref=left_entity.sentence_ref
           AND right_entity.start_char>left_entity.end_char
          JOIN slr_identity_sentence AS sentence ON sentence.sentence_ref=left_entity.sentence_ref
          JOIN LATERAL (
              SELECT token.token_id FROM slr_identity_token AS token
               WHERE token.sentence_ref=left_entity.sentence_ref
                 AND token.start_char>=left_entity.start_char
                 AND token.end_char<=left_entity.end_char
               ORDER BY token.end_char DESC LIMIT 1
          ) AS left_head ON TRUE
          JOIN LATERAL (
              SELECT token.token_id FROM slr_identity_token AS token
               WHERE token.sentence_ref=right_entity.sentence_ref
                 AND token.start_char>=right_entity.start_char
                 AND token.end_char<=right_entity.end_char
               ORDER BY token.end_char DESC LIMIT 1
          ) AS right_head ON TRUE
          JOIN slr_identity_anchor AS left_anchor ON left_anchor.token_id=left_head.token_id
          JOIN slr_identity_anchor AS right_anchor ON right_anchor.token_id=right_head.token_id
          JOIN slr_identity_token AS cue
            ON cue.sentence_ref=left_entity.sentence_ref
           AND cue.start_char>=left_entity.end_char
           AND cue.end_char<=right_entity.start_char
         WHERE left_anchor.object_id<>right_anchor.object_id
           AND (
               EXISTS (SELECT 1 FROM execution.semantic_pnf_hot_cue_symbol AS direct
                        WHERE direct.cue_class=1 AND direct.symbol_id=cue.orth_symbol_id)
               OR (
                   EXISTS (SELECT 1 FROM execution.semantic_pnf_hot_cue_symbol AS known
                            WHERE known.cue_class=2 AND known.symbol_id=cue.orth_symbol_id)
                   AND EXISTS (
                       SELECT 1 FROM slr_identity_token AS as_token
                       JOIN execution.semantic_pnf_hot_cue_symbol AS as_cue
                         ON as_cue.cue_class=3 AND as_cue.symbol_id=as_token.orth_symbol_id
                        WHERE as_token.sentence_ref=cue.sentence_ref
                          AND as_token.start_char>=cue.end_char
                          AND as_token.end_char<=right_entity.start_char
                   )
               )
           )
         GROUP BY left_entity.entity_id,right_entity.entity_id,sentence.sentence_id,
                  left_head.token_id,right_head.token_id,left_anchor.object_id,right_anchor.object_id
    ), alias_evidence AS (
        SELECT right_object_id AS source_object_id,left_object_id AS target_object_id,
               6::SMALLINT AS witness_kind,right_token_id AS source_token_id,
               left_token_id AS target_token_id,sentence_id,
               'explicit-alias:'||cue_token_id::TEXT||':'||left_entity_id::TEXT||':'||right_entity_id::TEXT AS evidence_ref,
               1::SMALLINT AS candidate_count
          FROM alias_pair
    ), evidence AS (
        SELECT * FROM appos_evidence
        UNION ALL SELECT * FROM proper_name_evidence
        UNION ALL SELECT * FROM alias_evidence
    ), evidence_write AS (
    INSERT INTO execution.semantic_pnf_identity_evidence_candidate
        (run_id,document_id,source_object_id,target_object_id,witness_kind,
         source_token_id,target_token_id,sentence_id,evidence_ref,candidate_count,evidence_state)
    SELECT selected_run_id,selected_document_id,source_object_id,target_object_id,
           witness_kind,source_token_id,target_token_id,sentence_id,evidence_ref,candidate_count,1
      FROM evidence
    ON CONFLICT (run_id,document_id,source_object_id,target_object_id,witness_kind,evidence_ref)
    DO UPDATE SET evidence_state=1,candidate_count=EXCLUDED.candidate_count,
                  source_token_id=EXCLUDED.source_token_id,
                  target_token_id=EXCLUDED.target_token_id,
                  sentence_id=EXCLUDED.sentence_id
    RETURNING candidate_id
    ), overflow_write AS (
        INSERT INTO execution.semantic_pnf_proper_name_evidence_overflow
            (run_id,document_id,source_object_id,source_token_id,
             family_lemma_symbol_id,possible_target_count,retained_target_limit)
        SELECT selected_run_id,selected_document_id,source_object_id,source_token_id,
               family_lemma_symbol_id,full_count,max_name_targets::SMALLINT
          FROM proper_name_overflow
        RETURNING source_token_id
    )
    SELECT (SELECT count(*) FROM evidence_write)+
           (SELECT 0*count(*) FROM overflow_write) INTO affected_count;
    RETURN affected_count;
END;
$$;

-- SLR production-owner specialization of original SensibLaw m090:
-- f9c4670ef04b40e8153caa7dd00fa1eba7013ef4, blob f5f0dc9e43f50dec52c604614d3492f2cb216bbd.
-- Original m085 bounded family/overflow carrier, preserving m090's actual
-- same-target-span exclusion, head selection, hot symbols and direct lanes.
-- TEMPORARY function + indexed scratch carriers. Invoked only in the native
-- rollback parity probe until original direct/reference admission parity passes.
-- No alternative identity authority, no coverage or world promotion.
CREATE OR REPLACE FUNCTION pg_temp.slr_refresh_numeric_pnf_parser_identity_evidence_bounded(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    standalone_only BOOLEAN DEFAULT FALSE
)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    selected_run_ref TEXT;
    selected_document_ref TEXT;
BEGIN
    SELECT run_ref INTO selected_run_ref
      FROM execution.semantic_pnf_run_identity WHERE run_id=selected_run_id;
    SELECT document_ref INTO selected_document_ref
      FROM execution.semantic_pnf_document_identity WHERE document_id=selected_document_id;
    IF selected_run_ref IS NULL OR selected_document_ref IS NULL THEN
        RAISE EXCEPTION 'unknown numeric run/document %, %', selected_run_id, selected_document_id;
    END IF;

    CREATE TEMP TABLE slr_identity_stage_scope ON COMMIT DROP AS
        SELECT selected_run_id AS run_id,selected_document_id AS document_id;
    -- Index the original observation fibres once. Keep the exact m090
    -- selection rules, including its original document anchor owner.
    CREATE TEMP TABLE slr_identity_constant ON COMMIT DROP AS
        SELECT max(symbol_id) FILTER (WHERE constant_id=1) AS propn_id,
               max(symbol_id) FILTER (WHERE constant_id=3) AS appos_id,
               max(symbol_id) FILTER (WHERE constant_id=4) AS person_id
          FROM execution.semantic_pnf_hot_symbol_constant;
    CREATE TEMP TABLE slr_identity_token ON COMMIT DROP AS
        SELECT token_id,sentence_id,sentence_ref,start_char,end_char,
               orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id
          FROM execution.semantic_parser_token
         WHERE run_ref=selected_run_ref AND document_ref=selected_document_ref
           AND representation_version=2;
    CREATE UNIQUE INDEX ON slr_identity_token(token_id);
    CREATE INDEX ON slr_identity_token(sentence_ref,start_char,end_char,token_id);
    CREATE INDEX ON slr_identity_token(lemma_symbol_id,pos_symbol_id);
    CREATE TEMP TABLE slr_identity_sentence ON COMMIT DROP AS
        SELECT sentence_id,sentence_ref,start_char,end_char FROM execution.semantic_parser_sentence
         WHERE run_ref=selected_run_ref AND document_ref=selected_document_ref;
    CREATE UNIQUE INDEX ON slr_identity_sentence(sentence_ref);
    CREATE TEMP TABLE slr_identity_person ON COMMIT DROP AS
        SELECT entity_id,sentence_ref,start_char,end_char
          FROM execution.semantic_parser_entity_span CROSS JOIN slr_identity_constant AS constant
         WHERE run_ref=selected_run_ref AND document_ref=selected_document_ref
           AND entity_type_symbol_id=person_id;
    CREATE UNIQUE INDEX ON slr_identity_person(entity_id);
    CREATE INDEX ON slr_identity_person(sentence_ref,start_char,end_char);
    CREATE TEMP TABLE slr_identity_anchor ON COMMIT DROP AS
        SELECT * FROM execution.numeric_pnf_document_parser_object_anchor(
            selected_run_id,selected_document_id);
    CREATE UNIQUE INDEX ON slr_identity_anchor(token_id);
    ANALYZE slr_identity_token;
    ANALYZE slr_identity_person;
    ANALYZE slr_identity_anchor;
    CREATE TEMP TABLE slr_identity_family ON COMMIT DROP AS
        SELECT entity.entity_id AS parser_entity_id,entity.sentence_ref,
               entity.start_char,entity.end_char,head.token_id AS head_token_id,
               head.lemma_symbol_id AS family_lemma_symbol_id,
               anchor.object_id AS anchor_object_id
          FROM slr_identity_person AS entity
          CROSS JOIN slr_identity_constant AS constant
          JOIN LATERAL (
              SELECT token.token_id,token.lemma_symbol_id,token.end_char
                FROM slr_identity_token AS token
               WHERE token.sentence_ref=entity.sentence_ref
                 AND token.start_char>=entity.start_char
                 AND token.end_char<=entity.end_char
                 AND token.pos_symbol_id=constant.propn_id
               ORDER BY token.end_char DESC,token.token_id DESC LIMIT 1
          ) AS head ON TRUE
          JOIN slr_identity_anchor AS anchor ON anchor.token_id=head.token_id
         WHERE (SELECT count(*) FROM slr_identity_token AS member
                 WHERE member.sentence_ref=entity.sentence_ref
                   AND member.start_char>=entity.start_char
                   AND member.end_char<=entity.end_char) >= 2;
    CREATE UNIQUE INDEX ON slr_identity_family(parser_entity_id);
    CREATE INDEX ON slr_identity_family(family_lemma_symbol_id,parser_entity_id,anchor_object_id,head_token_id);
    CREATE INDEX ON slr_identity_family(family_lemma_symbol_id,sentence_ref,start_char,end_char);
    CREATE INDEX ON slr_identity_family(family_lemma_symbol_id,anchor_object_id);
    ANALYZE slr_identity_family;

    -- Exact indexed family dependency carrier. Include zero-member families,
    -- unanchored mentions, and embedded PERSON tokens: these are negative/
    -- unresolved dependencies, not absent work. Numeric IDs are local lookup
    -- coordinates, not independent semantic identity.
    CREATE TEMP TABLE slr_identity_mention_scope ON COMMIT DROP AS
    WITH family_cardinality AS MATERIALIZED (
        SELECT family_lemma_symbol_id,count(*)::BIGINT AS full_count
          FROM slr_identity_family GROUP BY family_lemma_symbol_id
    ), family_anchor_cardinality AS MATERIALIZED (
        SELECT family_lemma_symbol_id,anchor_object_id,count(*)::BIGINT AS anchor_count
          FROM slr_identity_family GROUP BY family_lemma_symbol_id,anchor_object_id
    )
    SELECT mention.token_id,mention.sentence_ref,mention.start_char,mention.end_char,
           mention.lemma_symbol_id,mention_anchor.object_id AS source_object_id,
           sentence.sentence_id,COALESCE(cardinality.full_count,0)::BIGINT AS full_count,
           EXISTS (SELECT 1 FROM slr_identity_person p
             WHERE p.sentence_ref=mention.sentence_ref AND p.start_char<=mention.start_char
               AND p.end_char>=mention.end_char) AS embedded_in_person,
           CASE WHEN mention_anchor.object_id IS NULL THEN NULL
             ELSE COALESCE(cardinality.full_count,0) - COALESCE(ac.anchor_count,0) -
                 (SELECT count(*) FROM slr_identity_family excluded
                   WHERE excluded.family_lemma_symbol_id=mention.lemma_symbol_id
                     AND excluded.sentence_ref=mention.sentence_ref
                     AND excluded.start_char<=mention.start_char
                     AND excluded.end_char>=mention.end_char
                     AND excluded.anchor_object_id<>mention_anchor.object_id)
           END AS eligible_count
      FROM slr_identity_token mention
      CROSS JOIN slr_identity_constant AS constant
      JOIN slr_identity_sentence sentence ON sentence.sentence_ref=mention.sentence_ref
      LEFT JOIN slr_identity_anchor mention_anchor ON mention_anchor.token_id=mention.token_id
      LEFT JOIN family_cardinality cardinality ON cardinality.family_lemma_symbol_id=mention.lemma_symbol_id
      LEFT JOIN family_anchor_cardinality ac ON ac.family_lemma_symbol_id=mention.lemma_symbol_id
           AND ac.anchor_object_id=mention_anchor.object_id
     WHERE mention.pos_symbol_id=constant.propn_id;
    CREATE UNIQUE INDEX ON slr_identity_mention_scope(token_id);
    CREATE INDEX ON slr_identity_mention_scope(lemma_symbol_id,token_id);
    CREATE INDEX ON slr_identity_mention_scope(sentence_id,token_id);
    ANALYZE slr_identity_mention_scope;

    RETURN pg_temp.slr_write_numeric_pnf_parser_identity_bounded(
        selected_run_id,selected_document_id,standalone_only);
END;
$$;
