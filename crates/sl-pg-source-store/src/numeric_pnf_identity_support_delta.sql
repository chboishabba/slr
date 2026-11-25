-- Source owner: SensibLaw m089 blob b11948d2a2dc1bfb681407f36da5b7688cae79f0, original m090
-- bounded stage in SLR. Cache only unchanged source fibres; reopen exact
-- dirty sentences and both sides of affected name families. A cached row
-- is not independent semantic evidence. This producer is checked against
-- the full original bounded stage before its result is published.
CREATE OR REPLACE FUNCTION pg_temp.slr_identity_scoped_anchor(
    selected_run_id BIGINT,
    selected_document_id BIGINT,
    selected_sentences TEXT[]
)
RETURNS TABLE(token_id BIGINT, sentence_id BIGINT, object_id BIGINT)
LANGUAGE sql
STABLE
AS $$
WITH identity AS (
    SELECT run_identity.run_ref,
           document_identity.document_ref
      FROM execution.semantic_pnf_run_identity AS run_identity
      CROSS JOIN execution.semantic_pnf_document_identity AS document_identity
     WHERE run_identity.run_id = selected_run_id
       AND document_identity.document_id = selected_document_id
), constant AS MATERIALIZED (
    SELECT max(symbol_id) FILTER (WHERE constant_id = 1) AS propn_id,
           max(symbol_id) FILTER (WHERE constant_id = 2) AS noun_id,
           max(symbol_id) FILTER (WHERE constant_id = 3) AS appos_id
      FROM execution.semantic_pnf_hot_symbol_constant
), doc_token AS MATERIALIZED (
    SELECT token.token_id,
           token.sentence_id,
           token.sentence_ref,
           token.start_char,
           token.end_char,
           token.orth_symbol_id,
           token.lemma_symbol_id,
           token.pos_symbol_id,
           token.dependency_symbol_id,
           token.head_token_id
      FROM execution.semantic_parser_token AS token
      JOIN identity ON TRUE
     WHERE token.run_ref = identity.run_ref
       AND token.document_ref = identity.document_ref
       AND token.representation_version = 2
       AND token.sentence_ref=ANY(selected_sentences)
), needed_token AS MATERIALIZED (
    SELECT DISTINCT token.*
      FROM doc_token AS token
      CROSS JOIN constant
     WHERE token.pos_symbol_id IN (constant.propn_id, constant.noun_id)
        OR token.dependency_symbol_id = constant.appos_id
        OR EXISTS (
            SELECT 1
              FROM execution.semantic_parser_token AS child
              JOIN identity ON child.run_ref=identity.run_ref AND child.document_ref=identity.document_ref
             WHERE child.representation_version=2 AND child.head_token_id = token.token_id
               AND child.dependency_symbol_id = constant.appos_id
        )
), exact_support AS MATERIALIZED (
    SELECT token.token_id,
           token.sentence_id,
           support.object_id
      FROM needed_token AS token
      JOIN execution.semantic_pnf_object_token_support AS support
        ON support.token_id = token.token_id
      JOIN execution.semantic_pnf_object AS object
        ON object.object_id = support.object_id
       AND object.head_symbol_id IN (token.lemma_symbol_id, token.orth_symbol_id)
      JOIN execution.semantic_pnf_region AS region
        ON region.region_id = object.region_id
       AND region.run_id = selected_run_id
       AND region.document_id = selected_document_id
), exact_anchor AS (
    SELECT support.token_id,
           min(support.sentence_id) AS sentence_id,
           min(support.object_id) AS object_id
      FROM exact_support AS support
     GROUP BY support.token_id
    HAVING count(DISTINCT support.object_id) = 1
), token_with_any_support AS MATERIALIZED (
    SELECT DISTINCT token_id FROM exact_support
), fallback_candidate AS (
    SELECT token.token_id,
           token.sentence_id,
           object.object_id,
           (region.end_char - region.start_char) AS region_span,
           min(region.end_char - region.start_char)
               OVER (PARTITION BY token.token_id) AS minimum_region_span
      FROM needed_token AS token
      LEFT JOIN token_with_any_support AS has_support
        ON has_support.token_id = token.token_id
      JOIN execution.semantic_pnf_region AS region
        ON region.run_id = selected_run_id
       AND region.document_id = selected_document_id
       AND token.start_char >= region.start_char
       AND token.end_char <= region.end_char
      JOIN execution.semantic_pnf_object AS object
        ON object.region_id = region.region_id
       AND object.head_symbol_id IN (token.lemma_symbol_id, token.orth_symbol_id)
     WHERE has_support.token_id IS NULL
), fallback_smallest AS (
    SELECT token_id, sentence_id, object_id
      FROM fallback_candidate
     WHERE region_span = minimum_region_span
), fallback_anchor AS (
    SELECT token_id,
           min(sentence_id) AS sentence_id,
           min(object_id) AS object_id
      FROM fallback_smallest
     GROUP BY token_id
    HAVING count(DISTINCT object_id) = 1
)
SELECT token_id, sentence_id, object_id FROM exact_anchor
UNION ALL
SELECT token_id, sentence_id, object_id FROM fallback_anchor;
$$;

CREATE OR REPLACE FUNCTION pg_temp.slr_stage_identity_support_delta(
    selected_run_id bigint,selected_document_id bigint,
    selected_scope text,selected_sentences text[]
) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE
    selected_run_ref text;
    selected_document_ref text;
    affected_families bigint[];
BEGIN
    IF selected_sentences IS NULL OR array_position(selected_sentences,NULL) IS NOT NULL
       OR NOT EXISTS(SELECT 1 FROM execution.slr_native_identity_input_baseline
           WHERE scope_ref=selected_scope AND contract_ref='slr-identity-input-delta:v1' AND exact_reference_parity)
    THEN RAISE EXCEPTION 'identity delta requires a checked source baseline'; END IF;
    SELECT run_ref INTO STRICT selected_run_ref FROM execution.semantic_pnf_run_identity WHERE run_id=selected_run_id;
    SELECT document_ref INTO STRICT selected_document_ref FROM execution.semantic_pnf_document_identity WHERE document_id=selected_document_id;
    IF NOT EXISTS(SELECT 1 FROM execution.slr_native_identity_support_scope
        WHERE scope_ref=selected_scope AND scope_payload->>'run_ref'=selected_run_ref
          AND scope_payload->>'document_ref'=selected_document_ref)
    THEN RAISE EXCEPTION 'foreign identity delta scope'; END IF;

    CREATE TEMP TABLE slr_identity_stage_scope ON COMMIT DROP AS
        SELECT selected_run_id AS run_id,selected_document_id AS document_id;
    CREATE TEMP TABLE slr_identity_constant ON COMMIT DROP AS
        SELECT max(symbol_id) FILTER (WHERE constant_id=1) AS propn_id,
               max(symbol_id) FILTER (WHERE constant_id=3) AS appos_id,
               max(symbol_id) FILTER (WHERE constant_id=4) AS person_id
          FROM execution.semantic_pnf_hot_symbol_constant;

    CREATE TEMP TABLE slr_identity_token ON COMMIT DROP AS
    SELECT t.token_id,t.sentence_id,t.sentence_ref,t.start_char,t.end_char,t.orth_symbol_id,
           t.lemma_symbol_id,t.pos_symbol_id,t.dependency_symbol_id,t.head_token_id
      FROM execution.semantic_parser_token t
     WHERE t.run_ref=selected_run_ref AND t.document_ref=selected_document_ref
       AND t.representation_version=2 AND t.sentence_ref=ANY(selected_sentences)
    UNION ALL
    SELECT p.token_id,p.sentence_id,p.sentence_ref,p.start_char,p.end_char,p.orth_symbol_id,
           p.lemma_symbol_id,p.pos_symbol_id,p.dependency_symbol_id,p.head_token_id
      FROM execution.slr_native_identity_support_index i
      CROSS JOIN LATERAL jsonb_populate_record(NULL::execution.semantic_parser_token,i.payload) p
     WHERE i.scope_ref=selected_scope AND i.lane=1 AND NOT p.sentence_ref=ANY(selected_sentences);
    CREATE UNIQUE INDEX ON slr_identity_token(token_id);
    CREATE INDEX ON slr_identity_token(sentence_ref,start_char,end_char,token_id);
    CREATE INDEX ON slr_identity_token(lemma_symbol_id,pos_symbol_id);

    CREATE TEMP TABLE slr_identity_sentence ON COMMIT DROP AS
    SELECT s.sentence_id,s.sentence_ref,s.start_char,s.end_char
      FROM execution.semantic_parser_sentence s
     WHERE s.run_ref=selected_run_ref AND s.document_ref=selected_document_ref
       AND s.sentence_ref=ANY(selected_sentences)
    UNION ALL
    SELECT p.sentence_id,p.sentence_ref,p.start_char,p.end_char
      FROM execution.slr_native_identity_support_index i
      CROSS JOIN LATERAL jsonb_populate_record(NULL::execution.semantic_parser_sentence,i.payload) p
     WHERE i.scope_ref=selected_scope AND i.lane=8 AND NOT p.sentence_ref=ANY(selected_sentences);
    CREATE UNIQUE INDEX ON slr_identity_sentence(sentence_ref);

    CREATE TEMP TABLE slr_identity_person ON COMMIT DROP AS
    SELECT e.entity_id,e.sentence_ref,e.start_char,e.end_char
      FROM execution.semantic_parser_entity_span e CROSS JOIN slr_identity_constant c
     WHERE e.run_ref=selected_run_ref AND e.document_ref=selected_document_ref
       AND e.entity_type_symbol_id=c.person_id AND e.sentence_ref=ANY(selected_sentences)
    UNION ALL
    SELECT p.entity_id,p.sentence_ref,p.start_char,p.end_char
      FROM execution.slr_native_identity_support_index i
      CROSS JOIN LATERAL jsonb_populate_record(NULL::execution.semantic_parser_entity_span,i.payload) p
     WHERE i.scope_ref=selected_scope AND i.lane=2 AND NOT p.sentence_ref=ANY(selected_sentences);
    CREATE UNIQUE INDEX ON slr_identity_person(entity_id);
    CREATE INDEX ON slr_identity_person(sentence_ref,start_char,end_char);

    CREATE TEMP TABLE slr_identity_anchor ON COMMIT DROP AS
    SELECT * FROM pg_temp.slr_identity_scoped_anchor(selected_run_id,selected_document_id,selected_sentences)
    UNION ALL
    SELECT p.token_id,p.sentence_id,p.object_id
      FROM execution.slr_native_identity_support_index i
      CROSS JOIN LATERAL jsonb_to_record(i.payload) p(token_id bigint,sentence_id bigint,object_id bigint)
      JOIN slr_identity_token t ON t.token_id=p.token_id
     WHERE i.scope_ref=selected_scope AND i.lane=3 AND NOT t.sentence_ref=ANY(selected_sentences);
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
         WHERE entity.sentence_ref=ANY(selected_sentences)
           AND (SELECT count(*) FROM slr_identity_token AS member
                 WHERE member.sentence_ref=entity.sentence_ref
                   AND member.start_char>=entity.start_char
                   AND member.end_char<=entity.end_char) >= 2
    UNION ALL
    SELECT p.parser_entity_id,p.sentence_ref,p.start_char,p.end_char,p.head_token_id,
           p.family_lemma_symbol_id,p.anchor_object_id
      FROM execution.slr_native_identity_support_index i
      CROSS JOIN LATERAL jsonb_to_record(i.payload) p(
        parser_entity_id bigint,sentence_ref text,start_char bigint,end_char bigint,
        head_token_id bigint,family_lemma_symbol_id bigint,anchor_object_id bigint)
     WHERE i.scope_ref=selected_scope AND i.lane=4
       AND NOT p.sentence_ref=ANY(selected_sentences);

    CREATE UNIQUE INDEX ON slr_identity_family(parser_entity_id);
    CREATE INDEX ON slr_identity_family(family_lemma_symbol_id,parser_entity_id,anchor_object_id,head_token_id);
    CREATE INDEX ON slr_identity_family(family_lemma_symbol_id,sentence_ref,start_char,end_char);
    CREATE INDEX ON slr_identity_family(family_lemma_symbol_id,anchor_object_id);
    ANALYZE slr_identity_family;

    -- Both old and new family memberships are essential: empty -> nonempty
    -- must reopen the earlier mention even when its own sentence is unchanged.
    SELECT COALESCE(array_agg(DISTINCT family_id),ARRAY[]::bigint[]) INTO affected_families
      FROM (
        SELECT i.family_id FROM execution.slr_native_identity_support_index i
         WHERE i.scope_ref=selected_scope AND i.lane=4
           AND (i.payload->>'sentence_ref')=ANY(selected_sentences)
        UNION SELECT f.family_lemma_symbol_id FROM slr_identity_family f
         WHERE f.sentence_ref=ANY(selected_sentences)
      ) families WHERE family_id IS NOT NULL;

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
     WHERE mention.pos_symbol_id=constant.propn_id
       AND (mention.sentence_ref=ANY(selected_sentences) OR mention.lemma_symbol_id=ANY(affected_families))
    UNION ALL
    SELECT p.token_id,p.sentence_ref,p.start_char,p.end_char,p.lemma_symbol_id,p.source_object_id,
           p.sentence_id,p.full_count,p.embedded_in_person,p.eligible_count
      FROM execution.slr_native_identity_support_index i
      CROSS JOIN LATERAL jsonb_to_record(i.payload) p(
        token_id bigint,sentence_ref text,start_char bigint,end_char bigint,lemma_symbol_id bigint,
        source_object_id bigint,sentence_id bigint,full_count bigint,embedded_in_person boolean,eligible_count bigint)
     WHERE i.scope_ref=selected_scope AND i.lane=5
       AND NOT p.sentence_ref=ANY(selected_sentences)
       AND NOT p.lemma_symbol_id=ANY(affected_families);

    CREATE UNIQUE INDEX ON slr_identity_mention_scope(token_id);
    CREATE INDEX ON slr_identity_mention_scope(lemma_symbol_id,token_id);
    CREATE INDEX ON slr_identity_mention_scope(sentence_id,token_id);
    ANALYZE slr_identity_mention_scope;
    RETURN cardinality(selected_sentences)::bigint;
END;
$$;
