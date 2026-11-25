-- Native SLR identity-input invalidation for the existing m089/m090
-- document anchor/family owner. The support revision journal retains
-- exact changed support payloads; this smaller input journal identifies
-- which original observations must be reopened.
-- Each event is acknowledged by exact ID, never by a sequence high-water
-- mark (transactions may commit out of sequence).
CREATE TABLE IF NOT EXISTS execution.slr_native_identity_input_event (
    event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    run_ref text,
    document_ref text,
    input_relation text NOT NULL,
    operation text NOT NULL CHECK(operation IN ('INSERT','UPDATE','DELETE','TRUNCATE')),
    sentence_refs text[] NOT NULL DEFAULT ARRAY[]::text[],
    token_ids bigint[] NOT NULL DEFAULT ARRAY[]::bigint[],
    start_char bigint,
    end_char bigint,
    whole_scope boolean NOT NULL DEFAULT false,
    CHECK ((run_ref IS NULL)=(document_ref IS NULL)),
    CHECK (run_ref IS NOT NULL OR whole_scope),
    CHECK ((start_char IS NULL)=(end_char IS NULL)),
    CHECK (start_char IS NULL OR (start_char>=0 AND end_char>start_char)),
    CHECK (whole_scope OR cardinality(sentence_refs)>0 OR cardinality(token_ids)>0 OR start_char IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS slr_native_identity_input_event_scope_idx
    ON execution.slr_native_identity_input_event(run_ref,document_ref,event_id);
CREATE TABLE IF NOT EXISTS execution.slr_native_identity_input_ack (
    scope_ref text NOT NULL,
    event_id bigint NOT NULL REFERENCES execution.slr_native_identity_input_event(event_id),
    PRIMARY KEY(scope_ref,event_id)
);
CREATE TABLE IF NOT EXISTS execution.slr_native_identity_input_baseline (
    scope_ref text PRIMARY KEY,
    contract_ref text NOT NULL,
    exact_reference_parity boolean NOT NULL CHECK(exact_reference_parity)
);

-- Noop UPDATE statements must not invalidate their inputs. Each trigger
-- compares the exact OLD/NEW relation projection read by m089/m090, not
-- timestamp or unrelated presentation columns. Cascading deletes of an
-- object's token supports are independently observed on the token lane.

CREATE OR REPLACE FUNCTION execution.slr_id_input_1_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,representation_version FROM new_rows), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,NULL::bigint AS token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_sentence','INSERT',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_1_insert ON execution.semantic_parser_sentence;
CREATE TRIGGER slr_id_input_1_insert AFTER INSERT ON execution.semantic_parser_sentence
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_1_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_1_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,representation_version FROM old_rows EXCEPT SELECT sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,representation_version FROM new_rows)
      UNION
      (SELECT sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,representation_version FROM new_rows EXCEPT SELECT sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,representation_version FROM old_rows)), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,NULL::bigint AS token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_sentence','UPDATE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_1_update ON execution.semantic_parser_sentence;
CREATE TRIGGER slr_id_input_1_update AFTER UPDATE ON execution.semantic_parser_sentence
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_1_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_1_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,representation_version FROM old_rows), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,NULL::bigint AS token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_sentence','DELETE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_1_delete ON execution.semantic_parser_sentence;
CREATE TRIGGER slr_id_input_1_delete AFTER DELETE ON execution.semantic_parser_sentence
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_1_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_1_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_parser_sentence','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_1_truncate ON execution.semantic_parser_sentence;
CREATE TRIGGER slr_id_input_1_truncate AFTER TRUNCATE ON execution.semantic_parser_sentence

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_1_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_2_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT token_id,sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id,representation_version FROM new_rows), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,r.token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,r.head_token_id,NULL::bigint,NULL::bigint FROM changed r WHERE r.head_token_id IS NOT NULL)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_token','INSERT',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_2_insert ON execution.semantic_parser_token;
CREATE TRIGGER slr_id_input_2_insert AFTER INSERT ON execution.semantic_parser_token
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_2_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_2_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT token_id,sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id,representation_version FROM old_rows EXCEPT SELECT token_id,sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id,representation_version FROM new_rows)
      UNION
      (SELECT token_id,sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id,representation_version FROM new_rows EXCEPT SELECT token_id,sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id,representation_version FROM old_rows)), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,r.token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,r.head_token_id,NULL::bigint,NULL::bigint FROM changed r WHERE r.head_token_id IS NOT NULL)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_token','UPDATE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_2_update ON execution.semantic_parser_token;
CREATE TRIGGER slr_id_input_2_update AFTER UPDATE ON execution.semantic_parser_token
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_2_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_2_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT token_id,sentence_id,sentence_ref,run_ref,document_ref,start_char,end_char,orth_symbol_id,lemma_symbol_id,pos_symbol_id,dependency_symbol_id,head_token_id,representation_version FROM old_rows), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,r.token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,r.head_token_id,NULL::bigint,NULL::bigint FROM changed r WHERE r.head_token_id IS NOT NULL)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_token','DELETE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_2_delete ON execution.semantic_parser_token;
CREATE TRIGGER slr_id_input_2_delete AFTER DELETE ON execution.semantic_parser_token
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_2_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_2_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_parser_token','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_2_truncate ON execution.semantic_parser_token;
CREATE TRIGGER slr_id_input_2_truncate AFTER TRUNCATE ON execution.semantic_parser_token

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_2_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_3_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT entity_id,sentence_ref,run_ref,document_ref,start_char,end_char,entity_type_symbol_id,representation_version FROM new_rows), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,NULL::bigint AS token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_entity_span','INSERT',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_3_insert ON execution.semantic_parser_entity_span;
CREATE TRIGGER slr_id_input_3_insert AFTER INSERT ON execution.semantic_parser_entity_span
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_3_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_3_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT entity_id,sentence_ref,run_ref,document_ref,start_char,end_char,entity_type_symbol_id,representation_version FROM old_rows EXCEPT SELECT entity_id,sentence_ref,run_ref,document_ref,start_char,end_char,entity_type_symbol_id,representation_version FROM new_rows)
      UNION
      (SELECT entity_id,sentence_ref,run_ref,document_ref,start_char,end_char,entity_type_symbol_id,representation_version FROM new_rows EXCEPT SELECT entity_id,sentence_ref,run_ref,document_ref,start_char,end_char,entity_type_symbol_id,representation_version FROM old_rows)), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,NULL::bigint AS token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_entity_span','UPDATE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_3_update ON execution.semantic_parser_entity_span;
CREATE TRIGGER slr_id_input_3_update AFTER UPDATE ON execution.semantic_parser_entity_span
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_3_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_3_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT entity_id,sentence_ref,run_ref,document_ref,start_char,end_char,entity_type_symbol_id,representation_version FROM old_rows), affected AS (SELECT r.run_ref,r.document_ref,r.sentence_ref,NULL::bigint AS token_id,NULL::bigint AS start_char,NULL::bigint AS end_char FROM changed r)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_parser_entity_span','DELETE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_3_delete ON execution.semantic_parser_entity_span;
CREATE TRIGGER slr_id_input_3_delete AFTER DELETE ON execution.semantic_parser_entity_span
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_3_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_3_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_parser_entity_span','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_3_truncate ON execution.semantic_parser_entity_span;
CREATE TRIGGER slr_id_input_3_truncate AFTER TRUNCATE ON execution.semantic_parser_entity_span

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_3_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_4_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT region_id,run_ref,document_ref,run_id,document_id,start_char,end_char FROM new_rows), affected AS (SELECT r.run_ref,r.document_ref,NULL::text AS sentence_ref,NULL::bigint AS token_id,r.start_char,r.end_char FROM changed r WHERE r.end_char>r.start_char
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,s.token_id,NULL::bigint,NULL::bigint FROM changed r
      JOIN execution.semantic_pnf_object o ON o.region_id=r.region_id
      JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_region','INSERT',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_4_insert ON execution.semantic_pnf_region;
CREATE TRIGGER slr_id_input_4_insert AFTER INSERT ON execution.semantic_pnf_region
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_4_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_4_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT region_id,run_ref,document_ref,run_id,document_id,start_char,end_char FROM old_rows EXCEPT SELECT region_id,run_ref,document_ref,run_id,document_id,start_char,end_char FROM new_rows)
      UNION
      (SELECT region_id,run_ref,document_ref,run_id,document_id,start_char,end_char FROM new_rows EXCEPT SELECT region_id,run_ref,document_ref,run_id,document_id,start_char,end_char FROM old_rows)), affected AS (SELECT r.run_ref,r.document_ref,NULL::text AS sentence_ref,NULL::bigint AS token_id,r.start_char,r.end_char FROM changed r WHERE r.end_char>r.start_char
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,s.token_id,NULL::bigint,NULL::bigint FROM changed r
      JOIN execution.semantic_pnf_object o ON o.region_id=r.region_id
      JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_region','UPDATE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_4_update ON execution.semantic_pnf_region;
CREATE TRIGGER slr_id_input_4_update AFTER UPDATE ON execution.semantic_pnf_region
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_4_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_4_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT region_id,run_ref,document_ref,run_id,document_id,start_char,end_char FROM old_rows), affected AS (SELECT r.run_ref,r.document_ref,NULL::text AS sentence_ref,NULL::bigint AS token_id,r.start_char,r.end_char FROM changed r WHERE r.end_char>r.start_char
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,s.token_id,NULL::bigint,NULL::bigint FROM changed r
      JOIN execution.semantic_pnf_object o ON o.region_id=r.region_id
      JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_region','DELETE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_4_delete ON execution.semantic_pnf_region;
CREATE TRIGGER slr_id_input_4_delete AFTER DELETE ON execution.semantic_pnf_region
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_4_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_4_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_pnf_region','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_4_truncate ON execution.semantic_pnf_region;
CREATE TRIGGER slr_id_input_4_truncate AFTER TRUNCATE ON execution.semantic_pnf_region

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_4_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_5_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT object_id,region_id,head_symbol_id FROM new_rows), affected AS (SELECT r.run_ref,r.document_ref,NULL::text AS sentence_ref,NULL::bigint AS token_id,r.start_char,r.end_char
      FROM changed c JOIN execution.semantic_pnf_region r ON r.region_id=c.region_id WHERE r.end_char>r.start_char
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,s.token_id,NULL::bigint,NULL::bigint
      FROM changed c JOIN execution.semantic_pnf_region r ON r.region_id=c.region_id
      JOIN execution.semantic_pnf_object_token_support s ON s.object_id=c.object_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_object','INSERT',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_5_insert ON execution.semantic_pnf_object;
CREATE TRIGGER slr_id_input_5_insert AFTER INSERT ON execution.semantic_pnf_object
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_5_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_5_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT object_id,region_id,head_symbol_id FROM old_rows EXCEPT SELECT object_id,region_id,head_symbol_id FROM new_rows)
      UNION
      (SELECT object_id,region_id,head_symbol_id FROM new_rows EXCEPT SELECT object_id,region_id,head_symbol_id FROM old_rows)), affected AS (SELECT r.run_ref,r.document_ref,NULL::text AS sentence_ref,NULL::bigint AS token_id,r.start_char,r.end_char
      FROM changed c JOIN execution.semantic_pnf_region r ON r.region_id=c.region_id WHERE r.end_char>r.start_char
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,s.token_id,NULL::bigint,NULL::bigint
      FROM changed c JOIN execution.semantic_pnf_region r ON r.region_id=c.region_id
      JOIN execution.semantic_pnf_object_token_support s ON s.object_id=c.object_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_object','UPDATE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_5_update ON execution.semantic_pnf_object;
CREATE TRIGGER slr_id_input_5_update AFTER UPDATE ON execution.semantic_pnf_object
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_5_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_5_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT object_id,region_id,head_symbol_id FROM old_rows), affected AS (SELECT r.run_ref,r.document_ref,NULL::text AS sentence_ref,NULL::bigint AS token_id,r.start_char,r.end_char
      FROM changed c JOIN execution.semantic_pnf_region r ON r.region_id=c.region_id WHERE r.end_char>r.start_char
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,s.token_id,NULL::bigint,NULL::bigint
      FROM changed c JOIN execution.semantic_pnf_region r ON r.region_id=c.region_id
      JOIN execution.semantic_pnf_object_token_support s ON s.object_id=c.object_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_object','DELETE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_5_delete ON execution.semantic_pnf_object;
CREATE TRIGGER slr_id_input_5_delete AFTER DELETE ON execution.semantic_pnf_object
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_5_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_5_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_pnf_object','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_5_truncate ON execution.semantic_pnf_object;
CREATE TRIGGER slr_id_input_5_truncate AFTER TRUNCATE ON execution.semantic_pnf_object

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_5_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_6_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT object_id,token_id FROM new_rows), affected AS (SELECT t.run_ref,t.document_ref,t.sentence_ref,t.token_id,NULL::bigint AS start_char,NULL::bigint AS end_char
      FROM changed c JOIN execution.semantic_parser_token t ON t.token_id=c.token_id
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,c.token_id,NULL::bigint,NULL::bigint
      FROM changed c JOIN execution.semantic_pnf_object o ON o.object_id=c.object_id
      JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_object_token_support','INSERT',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_6_insert ON execution.semantic_pnf_object_token_support;
CREATE TRIGGER slr_id_input_6_insert AFTER INSERT ON execution.semantic_pnf_object_token_support
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_6_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_6_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT object_id,token_id FROM old_rows EXCEPT SELECT object_id,token_id FROM new_rows)
      UNION
      (SELECT object_id,token_id FROM new_rows EXCEPT SELECT object_id,token_id FROM old_rows)), affected AS (SELECT t.run_ref,t.document_ref,t.sentence_ref,t.token_id,NULL::bigint AS start_char,NULL::bigint AS end_char
      FROM changed c JOIN execution.semantic_parser_token t ON t.token_id=c.token_id
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,c.token_id,NULL::bigint,NULL::bigint
      FROM changed c JOIN execution.semantic_pnf_object o ON o.object_id=c.object_id
      JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_object_token_support','UPDATE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_6_update ON execution.semantic_pnf_object_token_support;
CREATE TRIGGER slr_id_input_6_update AFTER UPDATE ON execution.semantic_pnf_object_token_support
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_6_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_6_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT object_id,token_id FROM old_rows), affected AS (SELECT t.run_ref,t.document_ref,t.sentence_ref,t.token_id,NULL::bigint AS start_char,NULL::bigint AS end_char
      FROM changed c JOIN execution.semantic_parser_token t ON t.token_id=c.token_id
      UNION
      SELECT r.run_ref,r.document_ref,NULL::text,c.token_id,NULL::bigint,NULL::bigint
      FROM changed c JOIN execution.semantic_pnf_object o ON o.object_id=c.object_id
      JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id)
    INSERT INTO execution.slr_native_identity_input_event
      (run_ref,document_ref,input_relation,operation,sentence_refs,token_ids,start_char,end_char)
    SELECT a.run_ref,a.document_ref,'semantic_pnf_object_token_support','DELETE',
      COALESCE(array_agg(DISTINCT a.sentence_ref) FILTER(WHERE a.sentence_ref IS NOT NULL),ARRAY[]::text[]),
      COALESCE(array_agg(DISTINCT a.token_id) FILTER(WHERE a.token_id IS NOT NULL),ARRAY[]::bigint[]),
      a.start_char,a.end_char
      FROM affected a
      JOIN execution.semantic_run native ON native.run_ref=a.run_ref AND native.document_ref=a.document_ref
        AND native.kernel_contract='slr-native-source-parser-reuse:v1'
      WHERE a.sentence_ref IS NOT NULL OR a.token_id IS NOT NULL OR a.start_char IS NOT NULL
      GROUP BY a.run_ref,a.document_ref,a.start_char,a.end_char;
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_6_delete ON execution.semantic_pnf_object_token_support;
CREATE TRIGGER slr_id_input_6_delete AFTER DELETE ON execution.semantic_pnf_object_token_support
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_6_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_6_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_pnf_object_token_support','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_6_truncate ON execution.semantic_pnf_object_token_support;
CREATE TRIGGER slr_id_input_6_truncate AFTER TRUNCATE ON execution.semantic_pnf_object_token_support

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_6_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_7_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT constant_id,symbol_id FROM new_rows)
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    SELECT 'semantic_pnf_hot_symbol_constant','INSERT',true WHERE EXISTS(SELECT 1 FROM changed);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_7_insert ON execution.semantic_pnf_hot_symbol_constant;
CREATE TRIGGER slr_id_input_7_insert AFTER INSERT ON execution.semantic_pnf_hot_symbol_constant
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_7_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_7_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT constant_id,symbol_id FROM old_rows EXCEPT SELECT constant_id,symbol_id FROM new_rows)
      UNION
      (SELECT constant_id,symbol_id FROM new_rows EXCEPT SELECT constant_id,symbol_id FROM old_rows))
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    SELECT 'semantic_pnf_hot_symbol_constant','UPDATE',true WHERE EXISTS(SELECT 1 FROM changed);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_7_update ON execution.semantic_pnf_hot_symbol_constant;
CREATE TRIGGER slr_id_input_7_update AFTER UPDATE ON execution.semantic_pnf_hot_symbol_constant
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_7_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_7_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT constant_id,symbol_id FROM old_rows)
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    SELECT 'semantic_pnf_hot_symbol_constant','DELETE',true WHERE EXISTS(SELECT 1 FROM changed);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_7_delete ON execution.semantic_pnf_hot_symbol_constant;
CREATE TRIGGER slr_id_input_7_delete AFTER DELETE ON execution.semantic_pnf_hot_symbol_constant
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_7_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_7_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_pnf_hot_symbol_constant','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_7_truncate ON execution.semantic_pnf_hot_symbol_constant;
CREATE TRIGGER slr_id_input_7_truncate AFTER TRUNCATE ON execution.semantic_pnf_hot_symbol_constant

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_7_truncate();

CREATE OR REPLACE FUNCTION execution.slr_id_input_8_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT cue_class,symbol_id FROM new_rows)
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    SELECT 'semantic_pnf_hot_cue_symbol','INSERT',true WHERE EXISTS(SELECT 1 FROM changed);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_8_insert ON execution.semantic_pnf_hot_cue_symbol;
CREATE TRIGGER slr_id_input_8_insert AFTER INSERT ON execution.semantic_pnf_hot_cue_symbol
REFERENCING NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_8_insert();

CREATE OR REPLACE FUNCTION execution.slr_id_input_8_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS ((SELECT cue_class,symbol_id FROM old_rows EXCEPT SELECT cue_class,symbol_id FROM new_rows)
      UNION
      (SELECT cue_class,symbol_id FROM new_rows EXCEPT SELECT cue_class,symbol_id FROM old_rows))
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    SELECT 'semantic_pnf_hot_cue_symbol','UPDATE',true WHERE EXISTS(SELECT 1 FROM changed);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_8_update ON execution.semantic_pnf_hot_cue_symbol;
CREATE TRIGGER slr_id_input_8_update AFTER UPDATE ON execution.semantic_pnf_hot_cue_symbol
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_8_update();

CREATE OR REPLACE FUNCTION execution.slr_id_input_8_delete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH changed AS (SELECT DISTINCT cue_class,symbol_id FROM old_rows)
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    SELECT 'semantic_pnf_hot_cue_symbol','DELETE',true WHERE EXISTS(SELECT 1 FROM changed);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_8_delete ON execution.semantic_pnf_hot_cue_symbol;
CREATE TRIGGER slr_id_input_8_delete AFTER DELETE ON execution.semantic_pnf_hot_cue_symbol
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_8_delete();

CREATE OR REPLACE FUNCTION execution.slr_id_input_8_truncate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO execution.slr_native_identity_input_event(input_relation,operation,whole_scope)
    VALUES('semantic_pnf_hot_cue_symbol','TRUNCATE',true);
    RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS slr_id_input_8_truncate ON execution.semantic_pnf_hot_cue_symbol;
CREATE TRIGGER slr_id_input_8_truncate AFTER TRUNCATE ON execution.semantic_pnf_hot_cue_symbol

FOR EACH STATEMENT EXECUTE FUNCTION execution.slr_id_input_8_truncate();

-- The original source/family scope can acknowledge a baseline only after
-- exact reference parity. Installing this contract never certifies old
-- cached support rows or a previously completed consumer.
CREATE TABLE IF NOT EXISTS execution.slr_native_identity_input_contract (
    contract_ref text PRIMARY KEY
);
INSERT INTO execution.slr_native_identity_input_contract VALUES('slr-identity-input-delta:v1')
ON CONFLICT DO NOTHING;
