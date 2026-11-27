-- Extend the existing fence rather than add another cache or dependency
-- engine. Source parser leaf rows are read by exact compilation key; all
-- other application tables (including parser_job, which binds the key
-- domain and product reuse) retain the original global revision.
CREATE TABLE IF NOT EXISTS slr_native_replay.parser_leaf_revision(
    relation_oid oid NOT NULL,compilation_key text NOT NULL,revision bigint NOT NULL CHECK(revision>0),
    PRIMARY KEY(relation_oid,compilation_key)
);
CREATE TABLE IF NOT EXISTS slr_native_replay.parser_leaf_epoch(
    relation_oid oid PRIMARY KEY,revision bigint NOT NULL CHECK(revision>0)
);

CREATE OR REPLACE FUNCTION slr_native_replay.capture_relation_change()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path=pg_catalog,slr_native_replay AS $$
DECLARE changed boolean; leaf boolean; keys text[];
BEGIN
    IF TG_OP='INSERT' THEN
        SELECT EXISTS(SELECT 1 FROM slr_new_rows) INTO changed;
    ELSIF TG_OP='DELETE' THEN
        SELECT EXISTS(SELECT 1 FROM slr_old_rows) INTO changed;
    ELSIF TG_OP='UPDATE' THEN
        SELECT EXISTS(
            (SELECT (o::text COLLATE "C") FROM slr_old_rows o
             EXCEPT ALL SELECT (n::text COLLATE "C") FROM slr_new_rows n)
            UNION ALL
            (SELECT (n::text COLLATE "C") FROM slr_new_rows n
             EXCEPT ALL SELECT (o::text COLLATE "C") FROM slr_old_rows o)
        ) INTO changed;
    ELSE
        changed:=true;
    END IF;
    IF changed THEN
        -- The original global revision remains authoritative for numeric
        -- and unknown source inputs. Scope only the three original source
        -- parser leaf owners, whose endpoint reads are compilation-key
        -- restricted by the selected parser_run. Old AND new keys matter.
        leaf:=TG_TABLE_SCHEMA='ingest' AND TG_TABLE_NAME IN('parser_token','parser_entity','parser_artifact');
        IF leaf THEN
            IF TG_OP='TRUNCATE' THEN
                INSERT INTO slr_native_replay.parser_leaf_epoch AS epoch(relation_oid,revision)
                  VALUES(TG_RELID,1) ON CONFLICT(relation_oid) DO UPDATE SET revision=epoch.revision+1;
            ELSE
                IF TG_OP='INSERT' THEN
                    SELECT array_agg(DISTINCT compilation_key) INTO keys FROM slr_new_rows;
                ELSIF TG_OP='DELETE' THEN
                    SELECT array_agg(DISTINCT compilation_key) INTO keys FROM slr_old_rows;
                ELSIF TG_OP='UPDATE' THEN
                    SELECT array_agg(DISTINCT compilation_key) INTO keys FROM (
                       (SELECT compilation_key,(o::text COLLATE "C") FROM slr_old_rows o
                        EXCEPT ALL SELECT compilation_key,(n::text COLLATE "C") FROM slr_new_rows n)
                       UNION ALL
                       (SELECT compilation_key,(n::text COLLATE "C") FROM slr_new_rows n
                        EXCEPT ALL SELECT compilation_key,(o::text COLLATE "C") FROM slr_old_rows o)
                    ) delta;
                END IF;
                IF keys IS NULL OR array_position(keys,NULL) IS NOT NULL THEN
                    RAISE EXCEPTION 'changed parser leaf has no exact compilation key';
                END IF;
                INSERT INTO slr_native_replay.parser_leaf_revision AS prior(relation_oid,compilation_key,revision)
                    SELECT TG_RELID,key,1 FROM unnest(keys) key ORDER BY key
                ON CONFLICT(relation_oid,compilation_key) DO UPDATE SET revision=prior.revision+1;
            END IF;
        END IF;
        UPDATE slr_native_replay.relation_revision SET revision=revision+1 WHERE relation_oid=TG_RELID;
        IF NOT FOUND THEN RAISE EXCEPTION 'missing native replay relation revision'; END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION slr_native_replay.current_source_stamp(selected_parser_run text)
RETURNS text LANGUAGE plpgsql STABLE SET search_path=pg_catalog AS $slr_scope$
DECLARE global_stamp jsonb; leaf_oids oid[]; jobs text[]; revisions jsonb; unscoped jsonb;
BEGIN
    global_stamp:=slr_native_replay.current_stamp()::jsonb;
    IF global_stamp IS NULL OR selected_parser_run IS NULL OR selected_parser_run='' THEN RETURN NULL; END IF;
    IF position('parser_leaf_revision' in pg_get_functiondef(
        'slr_native_replay.capture_relation_change()'::regprocedure))=0 THEN RETURN NULL; END IF;
    IF to_regclass('ingest.parser_job') IS NULL THEN RETURN NULL; END IF;
    leaf_oids:=ARRAY[to_regclass('ingest.parser_token')::oid,to_regclass('ingest.parser_entity')::oid,
                     to_regclass('ingest.parser_artifact')::oid];
    IF array_position(leaf_oids,NULL) IS NOT NULL
       OR (SELECT count(*) FROM slr_native_replay.application_relations
            WHERE relation_oid=ANY(leaf_oids) AND relkind='r')<>3 THEN RETURN NULL; END IF;
    -- The job table remains global and its key membership is also in the
    -- stamp. A previously absent leaf row for ANY selected job invalidates.
    -- An empty selected run cannot produce a replayable candidate.
    SELECT array_agg(compilation_key ORDER BY compilation_key COLLATE "C")
      INTO jobs FROM ingest.parser_job WHERE parser_run_ref=selected_parser_run;
    IF jobs IS NULL OR cardinality(jobs)=0 THEN RETURN NULL; END IF;
    SELECT COALESCE(jsonb_agg(x ORDER BY (x->>0)::oid),'[]'::jsonb) INTO unscoped
      FROM jsonb_array_elements(global_stamp->'tables') x WHERE NOT ((x->>0)::oid=ANY(leaf_oids));
    SELECT jsonb_agg(jsonb_build_array(leaf.oid,COALESCE(epoch.revision,0),
        (SELECT jsonb_agg(jsonb_build_array(j.key,COALESCE(v.revision,0)) ORDER BY j.key COLLATE "C")
         FROM unnest(jobs) j(key)
         LEFT JOIN slr_native_replay.parser_leaf_revision v
           ON v.relation_oid=leaf.oid AND v.compilation_key=j.key))
      ORDER BY leaf.oid) INTO revisions
      FROM unnest(leaf_oids) leaf(oid)
      LEFT JOIN slr_native_replay.parser_leaf_epoch epoch ON epoch.relation_oid=leaf.oid;
    RETURN (jsonb_set(global_stamp,'{tables}',unscoped)
      || jsonb_build_object('source_parser_leaf_scope',selected_parser_run,
                            'parser_leaf_revisions',revisions))::text;
END;
$slr_scope$;

-- A scoped fence cannot inherit a pre-upgrade stamp. The original global
-- generation is bumped once under the existing advisory installation lock.
UPDATE slr_native_replay.fence SET generation=generation+1 WHERE singleton;
