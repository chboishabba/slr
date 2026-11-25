-- Native SLR endpoint replay fence. Conservative database-wide invalidation
-- over application relations; no claim of minimal consumer dirty frontier.
-- Install only for the explicit native endpoint replay campaign. A trigger
-- or catalog mismatch invalidates the entire fence. No authority writes.
CREATE SCHEMA IF NOT EXISTS slr_native_replay;
CREATE TABLE IF NOT EXISTS slr_native_replay.fence(
    singleton boolean PRIMARY KEY CHECK(singleton),
    instance_ref text NOT NULL,
    generation bigint NOT NULL CHECK(generation>0),
    last_catalog_snapshot text
);
INSERT INTO slr_native_replay.fence(singleton,instance_ref,generation) VALUES(true,
    md5(clock_timestamp()::text||random()::text||current_database()),1)
ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS slr_native_replay.relation_revision(
    relation_oid oid PRIMARY KEY,
    revision bigint NOT NULL DEFAULT 0 CHECK(revision>=0)
);
CREATE TABLE IF NOT EXISTS slr_native_replay.endpoint_receipt(
    key_ref text PRIMARY KEY,source_stamp text NOT NULL,numeric_stamp text NOT NULL,
    payload_sha256 text NOT NULL,payload jsonb NOT NULL
);

-- Actual changed transition rows, not statement counts or wall-clock times.
-- Equal UPDATE rows need not invalidate. Use composite type output under binary collation so array bounds, nullable
-- and JSON-typed columns compare without assuming every SQL type has equality.
CREATE OR REPLACE FUNCTION slr_native_replay.capture_relation_change()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path=pg_catalog,slr_native_replay AS $$
DECLARE changed boolean;
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
        UPDATE slr_native_replay.relation_revision SET revision=revision+1 WHERE relation_oid=TG_RELID;
        IF NOT FOUND THEN RAISE EXCEPTION 'missing native replay relation revision'; END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- This contract is deliberately conservative: all ordinary/partitioned
-- application tables are watched. Foreign and materialized relation inputs
-- are not supported by this fence, because external mutation/REFRESH is not
-- observable through these DML triggers.
CREATE OR REPLACE VIEW slr_native_replay.application_relations AS
SELECT c.oid AS relation_oid,n.nspname,c.relname,c.relkind,c.relpersistence
FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
WHERE n.nspname NOT IN('pg_catalog','information_schema','slr_native_replay')
  AND n.nspname NOT LIKE 'pg_%'
  AND c.relpersistence<>'t'
  AND c.relkind IN('r','p','f','m');

CREATE OR REPLACE FUNCTION slr_native_replay.fence_intact()
RETURNS boolean LANGUAGE sql STABLE AS $$
SELECT NOT EXISTS(
    SELECT 1 FROM slr_native_replay.application_relations r
    WHERE r.relkind IN('f','m') OR NOT EXISTS(
        SELECT 1 FROM slr_native_replay.relation_revision v WHERE v.relation_oid=r.relation_oid)
    OR EXISTS(
        SELECT 1 FROM (VALUES
            ('slr_endpoint_replay_insert',4::smallint,NULL::name,'slr_new_rows'::name),
            ('slr_endpoint_replay_update',16::smallint,'slr_old_rows'::name,'slr_new_rows'::name),
            ('slr_endpoint_replay_delete',8::smallint,'slr_old_rows'::name,NULL::name),
            ('slr_endpoint_replay_truncate',32::smallint,NULL::name,NULL::name)
        ) expected(name,kind,old_name,new_name)
        WHERE NOT EXISTS(
            SELECT 1 FROM pg_catalog.pg_trigger t
            WHERE t.tgrelid=r.relation_oid AND t.tgname=expected.name
              AND t.tgtype=expected.kind AND NOT t.tgisinternal AND t.tgenabled='A'
              AND t.tgfoid='slr_native_replay.capture_relation_change()'::regprocedure
              AND t.tgoldtable IS NOT DISTINCT FROM expected.old_name
              AND t.tgnewtable IS NOT DISTINCT FROM expected.new_name
        )
    )
);
$$;

CREATE OR REPLACE FUNCTION slr_native_replay.install_fence()
RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE r record; spec record; changed boolean:=false;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtext('slr-native-endpoint-replay:v1'));
    IF EXISTS(SELECT 1 FROM slr_native_replay.application_relations WHERE relkind IN('f','m')) THEN
        RETURN false;
    END IF;
    FOR r IN SELECT * FROM slr_native_replay.application_relations ORDER BY relation_oid LOOP
        INSERT INTO slr_native_replay.relation_revision(relation_oid) VALUES(r.relation_oid)
        ON CONFLICT DO NOTHING;
        FOR spec IN SELECT * FROM (VALUES
            ('slr_endpoint_replay_insert','INSERT','REFERENCING NEW TABLE AS slr_new_rows',4::smallint,NULL::name,'slr_new_rows'::name),
            ('slr_endpoint_replay_update','UPDATE','REFERENCING OLD TABLE AS slr_old_rows NEW TABLE AS slr_new_rows',16::smallint,'slr_old_rows'::name,'slr_new_rows'::name),
            ('slr_endpoint_replay_delete','DELETE','REFERENCING OLD TABLE AS slr_old_rows',8::smallint,'slr_old_rows'::name,NULL::name),
            ('slr_endpoint_replay_truncate','TRUNCATE','',32::smallint,NULL::name,NULL::name)
        ) v(name,operation,transition,kind,old_name,new_name) LOOP
            IF NOT EXISTS(SELECT 1 FROM pg_trigger t
                WHERE t.tgrelid=r.relation_oid AND t.tgname=spec.name AND t.tgtype=spec.kind
                  AND NOT t.tgisinternal AND t.tgenabled='A'
                  AND t.tgfoid='slr_native_replay.capture_relation_change()'::regprocedure
                  AND t.tgoldtable IS NOT DISTINCT FROM spec.old_name
                  AND t.tgnewtable IS NOT DISTINCT FROM spec.new_name) THEN
                EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I.%I',spec.name,r.nspname,r.relname);
                EXECUTE format('CREATE TRIGGER %I AFTER %s ON %I.%I %s FOR EACH STATEMENT EXECUTE FUNCTION slr_native_replay.capture_relation_change()',
                    spec.name,spec.operation,r.nspname,r.relname,spec.transition);
                EXECUTE format('ALTER TABLE %I.%I ENABLE ALWAYS TRIGGER %I',r.nspname,r.relname,spec.name);
                changed:=true;
            END IF;
        END LOOP;
    END LOOP;
    IF changed THEN
        UPDATE slr_native_replay.fence SET generation=generation+1 WHERE singleton;
    END IF;
    RETURN slr_native_replay.fence_intact();
END;
$$;

-- Catalog continuity is separate from DML versions. In particular,
-- disabling a capture trigger, mutating, and enabling it again cannot
-- silently return to the same stamp. Require the database DDL owner to
-- support an event trigger; a normal unprivileged installation is denied
-- rather than pretending that continuity is proven.
CREATE OR REPLACE FUNCTION slr_native_replay.catalog_snapshot()
RETURNS text LANGUAGE sql STABLE SET search_path=pg_catalog AS $$
SELECT jsonb_build_object(
    'tables',(SELECT jsonb_agg(to_jsonb(r) ORDER BY r.relation_oid) FROM slr_native_replay.application_relations r),
    'columns',(SELECT jsonb_agg(jsonb_build_array(a.attrelid,a.attnum,a.attname,a.atttypid,a.atttypmod,a.attnotnull,a.attgenerated,a.attidentity,
                      pg_get_expr(d.adbin,d.adrelid)) ORDER BY a.attrelid,a.attnum)
               FROM slr_native_replay.application_relations r JOIN pg_attribute a ON a.attrelid=r.relation_oid
               LEFT JOIN pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum
               WHERE a.attnum>0 AND NOT a.attisdropped),
    'views',(SELECT jsonb_agg(jsonb_build_array(c.oid,n.nspname,c.relname,pg_get_viewdef(c.oid)) ORDER BY c.oid)
             FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
             WHERE c.relkind='v' AND n.nspname NOT LIKE 'pg_%' AND n.nspname NOT IN('information_schema','slr_native_replay')),
    'functions',(SELECT jsonb_agg(jsonb_build_array(p.oid,n.nspname,p.proowner,p.proacl,pg_get_functiondef(p.oid)) ORDER BY p.oid)
                 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
                 WHERE p.prokind IN('f','p') AND n.nspname NOT LIKE 'pg_%' AND n.nspname NOT IN('information_schema','slr_native_replay')),
    'triggers',(SELECT jsonb_agg(jsonb_build_array(t.oid,t.tgenabled,pg_get_triggerdef(t.oid)) ORDER BY t.oid)
                FROM pg_trigger t JOIN slr_native_replay.application_relations r ON r.relation_oid=t.tgrelid),
    'constraints',(SELECT jsonb_agg(jsonb_build_array(c.oid,pg_get_constraintdef(c.oid)) ORDER BY c.oid)
                   FROM pg_constraint c JOIN slr_native_replay.application_relations r ON r.relation_oid=c.conrelid),
    'relation_definitions',(SELECT jsonb_agg(jsonb_build_array(c.oid,c.relfilenode,c.relowner,c.relacl,c.relrowsecurity,c.relforcerowsecurity,c.relispartition,
                               CASE WHEN c.relispartition THEN pg_get_expr(c.relpartbound,c.oid) ELSE NULL END) ORDER BY c.oid)
                             FROM pg_class c JOIN slr_native_replay.application_relations r ON r.relation_oid=c.oid),
    'policies',(SELECT jsonb_agg(jsonb_build_array(p.oid,p.polname,p.polcmd,p.polpermissive,p.polroles,pg_get_expr(p.polqual,p.polrelid),
                               pg_get_expr(p.polwithcheck,p.polrelid)) ORDER BY p.oid)
                 FROM pg_policy p JOIN slr_native_replay.application_relations r ON r.relation_oid=p.polrelid),
    'schemas',(SELECT jsonb_agg(jsonb_build_array(n.oid,n.nspname,n.nspowner,n.nspacl) ORDER BY n.oid)
               FROM pg_namespace n WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname NOT IN('information_schema','slr_native_replay')),
    'roles',(SELECT jsonb_agg(jsonb_build_array(oid,rolname,rolsuper,rolinherit,rolbypassrls,rolconfig) ORDER BY oid) FROM pg_roles),
    'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY m.roleid,m.member,m.grantor) FROM pg_auth_members)

)::text;
$$;
CREATE OR REPLACE FUNCTION slr_native_replay.capture_catalog_change()
RETURNS event_trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE observed text;
BEGIN
    observed:=slr_native_replay.catalog_snapshot();
    UPDATE slr_native_replay.fence SET generation=generation+1,last_catalog_snapshot=observed
    WHERE singleton AND last_catalog_snapshot IS DISTINCT FROM observed;
END;
$$;
CREATE EVENT TRIGGER slr_endpoint_replay_catalog_change ON ddl_command_end
EXECUTE FUNCTION slr_native_replay.capture_catalog_change();
ALTER EVENT TRIGGER slr_endpoint_replay_catalog_change ENABLE ALWAYS;
UPDATE slr_native_replay.fence SET last_catalog_snapshot=slr_native_replay.catalog_snapshot() WHERE singleton;

-- Catalog and revision stamp is metadata only; no application data scan.
-- Requiring event trigger continuity also rejects silently altered DDL
-- configuration. A superuser capable of altering the fence is trusted,
-- just as the original database authority is trusted.
CREATE OR REPLACE FUNCTION slr_native_replay.current_stamp()
RETURNS text LANGUAGE sql STABLE SET search_path=pg_catalog AS $$
SELECT CASE WHEN slr_native_replay.fence_intact()
 AND EXISTS(SELECT 1 FROM pg_event_trigger e WHERE e.evtname='slr_endpoint_replay_catalog_change'
    AND e.evtevent='ddl_command_end' AND e.evtenabled='A'
    AND e.evtfoid='slr_native_replay.capture_catalog_change()'::regprocedure)
 AND f.last_catalog_snapshot=slr_native_replay.catalog_snapshot()
 THEN jsonb_build_object(
    'instance',f.instance_ref,'generation',f.generation,'database',current_database(),
    'server_version',current_setting('server_version_num'),'postmaster',pg_postmaster_start_time(),
    'catalog',f.last_catalog_snapshot,
    'tables',(SELECT jsonb_agg(jsonb_build_array(r.relation_oid,v.revision) ORDER BY r.relation_oid)
              FROM slr_native_replay.application_relations r
              JOIN slr_native_replay.relation_revision v USING(relation_oid)),
    'user',current_user,'row_security',current_setting('row_security'),
    'session_authorization',session_user
 )::text ELSE NULL END
FROM slr_native_replay.fence f WHERE f.singleton;
$$;
