use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldStoreError {
    #[error("DATABASE_URL is not configured")]
    MissingDatabaseUrl,
    #[error("env file does not exist: {0}")]
    MissingEnvFile(PathBuf),
    #[error("env file error: {0}")]
    EnvFile(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("invalid world record: {0}")]
    InvalidRecord(String),
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    database_url: String,
}
impl DatabaseConfig {
    pub fn database_url(&self) -> &str { &self.database_url }
    pub fn redacted_description(&self) -> &'static str { "DATABASE_URL configured" }
}

pub fn load_database_config(explicit_env_file: Option<&Path>) -> Result<DatabaseConfig, WorldStoreError> {
    if let Ok(value) = env::var("DATABASE_URL") {
        if !value.trim().is_empty() { return Ok(DatabaseConfig { database_url: value }); }
    }
    if let Some(path) = explicit_env_file {
        if !path.exists() { return Err(WorldStoreError::MissingEnvFile(path.to_path_buf())); }
        dotenvy::from_path(path).map_err(|e| WorldStoreError::EnvFile(e.to_string()))?;
        let value = env::var("DATABASE_URL").map_err(|_| WorldStoreError::MissingDatabaseUrl)?;
        if value.trim().is_empty() { return Err(WorldStoreError::MissingDatabaseUrl); }
        return Ok(DatabaseConfig { database_url: value });
    }
    Err(WorldStoreError::MissingDatabaseUrl)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldRecordKind { SourceManifestation, PnfCandidate, WorldAtom, Gap, Obligation, RouteAction, Iteration }
impl WorldRecordKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::SourceManifestation => "source_manifestation", Self::PnfCandidate => "pnf_candidate",
            Self::WorldAtom => "world_atom", Self::Gap => "gap", Self::Obligation => "obligation",
            Self::RouteAction => "route_action", Self::Iteration => "iteration",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldRecord {
    pub kind: WorldRecordKind,
    pub id: String,
    #[serde(default)] pub iteration_index: Option<i64>,
    #[serde(default)] pub source_manifestation_id: Option<String>,
    #[serde(default)] pub surface_id: Option<String>,
    #[serde(default)] pub obligation_kind: Option<String>,
    pub payload: Value,
}

pub struct CopyTarget { pub staging_table: &'static str, pub final_table: &'static str, pub merge_sql: &'static str }
const STAGE: &str = "slr_world_stage_record";
const MERGE_SOURCE: &str = "INSERT INTO slr_world_source_manifestation (source_manifestation_id,source_kind,qid,language,revision_ref,source_text_sha256,payload) SELECT record_id,COALESCE(payload->>'manifestation_kind',payload->>'source_kind',''),COALESCE(payload->>'qid',''),COALESCE(payload->>'language',''),COALESCE(payload->>'revision_id',payload->>'revision_ref',''),COALESCE(payload->>'source_text_sha256',''),payload FROM slr_world_stage_record WHERE kind='source_manifestation' ON CONFLICT (source_manifestation_id) DO NOTHING";
const MERGE_PNF: &str = "INSERT INTO slr_world_pnf_candidate (claim_candidate_id,source_manifestation_id,payload) SELECT record_id,COALESCE(aux1,''),payload FROM slr_world_stage_record WHERE kind='pnf_candidate' ON CONFLICT (claim_candidate_id) DO NOTHING";
const MERGE_ATOM: &str = "INSERT INTO slr_world_atom (atom_id,atom_kind,subject_qid,source_manifestation_id,payload) SELECT record_id,COALESCE(payload->>'kind',''),COALESCE(payload->>'subject_qid',''),COALESCE(aux1,payload->>'document_ref',''),payload FROM slr_world_stage_record WHERE kind='world_atom' ON CONFLICT (atom_id) DO NOTHING";
const MERGE_GAP: &str = "INSERT INTO slr_world_gap (gap_id,iteration_index,surface_id,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_stage_record WHERE kind='gap' AND iteration_index IS NOT NULL ON CONFLICT (gap_id,iteration_index) DO NOTHING";
const MERGE_OBLIGATION: &str = "INSERT INTO slr_world_obligation (obligation_id,iteration_index,obligation_kind,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_stage_record WHERE kind='obligation' AND iteration_index IS NOT NULL ON CONFLICT (obligation_id,iteration_index) DO NOTHING";
const MERGE_ROUTE: &str = "INSERT INTO slr_world_route_action (action_id,iteration_index,payload) SELECT record_id,iteration_index,payload FROM slr_world_stage_record WHERE kind='route_action' AND iteration_index IS NOT NULL ON CONFLICT (action_id,iteration_index) DO NOTHING";
const MERGE_ITERATION: &str = "INSERT INTO slr_world_iteration (iteration_index,parent_iteration_index,payload) SELECT iteration_index,CASE WHEN iteration_index>0 THEN iteration_index-1 ELSE NULL END,payload FROM slr_world_stage_record WHERE kind='iteration' AND iteration_index IS NOT NULL ON CONFLICT (iteration_index) DO NOTHING";

pub fn copy_target_for_kind(kind: WorldRecordKind) -> CopyTarget {
    match kind {
        WorldRecordKind::SourceManifestation => CopyTarget{staging_table:STAGE,final_table:"slr_world_source_manifestation",merge_sql:MERGE_SOURCE},
        WorldRecordKind::PnfCandidate => CopyTarget{staging_table:STAGE,final_table:"slr_world_pnf_candidate",merge_sql:MERGE_PNF},
        WorldRecordKind::WorldAtom => CopyTarget{staging_table:STAGE,final_table:"slr_world_atom",merge_sql:MERGE_ATOM},
        WorldRecordKind::Gap => CopyTarget{staging_table:STAGE,final_table:"slr_world_gap",merge_sql:MERGE_GAP},
        WorldRecordKind::Obligation => CopyTarget{staging_table:STAGE,final_table:"slr_world_obligation",merge_sql:MERGE_OBLIGATION},
        WorldRecordKind::RouteAction => CopyTarget{staging_table:STAGE,final_table:"slr_world_route_action",merge_sql:MERGE_ROUTE},
        WorldRecordKind::Iteration => CopyTarget{staging_table:STAGE,final_table:"slr_world_iteration",merge_sql:MERGE_ITERATION},
    }
}

pub fn world_schema_sql() -> &'static str { r#"
CREATE TABLE IF NOT EXISTS slr_world_source_manifestation (source_manifestation_id TEXT PRIMARY KEY, source_kind TEXT NOT NULL, qid TEXT NOT NULL DEFAULT '', language TEXT NOT NULL DEFAULT '', revision_ref TEXT NOT NULL DEFAULT '', source_text_sha256 TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_pnf_candidate (claim_candidate_id TEXT PRIMARY KEY, source_manifestation_id TEXT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_atom (atom_id TEXT PRIMARY KEY, atom_kind TEXT NOT NULL, subject_qid TEXT NOT NULL DEFAULT '', source_manifestation_id TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_gap (gap_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, surface_id TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (gap_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_obligation (obligation_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, obligation_kind TEXT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (obligation_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_route_action (action_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (action_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_iteration (iteration_index BIGINT PRIMARY KEY, parent_iteration_index BIGINT, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
"# }
fn stage_sql() -> &'static str { "CREATE TEMP TABLE slr_world_stage_record (kind TEXT NOT NULL, record_id TEXT NOT NULL, iteration_index BIGINT, aux1 TEXT, payload JSONB NOT NULL) ON COMMIT DROP" }

pub fn latest_frontier_sql() -> &'static str { r#"WITH latest AS (SELECT MAX(iteration_index) AS iteration_index FROM slr_world_iteration)
SELECT 'gap'::text AS kind,g.gap_id AS id,g.iteration_index,g.payload FROM slr_world_gap g,latest l WHERE g.iteration_index=l.iteration_index
UNION ALL SELECT 'obligation'::text,o.obligation_id,o.iteration_index,o.payload FROM slr_world_obligation o,latest l WHERE o.iteration_index=l.iteration_index ORDER BY kind,id"# }

pub fn parse_world_record_line(line:&str)->Result<WorldRecord,WorldStoreError>{ let r:WorldRecord=serde_json::from_str(line)?; if r.id.trim().is_empty(){return Err(WorldStoreError::InvalidRecord("empty id".into()));} Ok(r) }
fn stable_id(prefix:&str,v:&Value)->String{let d=Sha256::digest(serde_json::to_vec(v).expect("json"));format!("{prefix}{d:x}")}
fn s(v:&Value,k:&str)->String{v.get(k).and_then(Value::as_str).unwrap_or("").to_string()}

pub fn records_from_round_values(article:&Value,closure:&Value,plan:&Value,iteration:&Value)->Result<Vec<WorldRecord>,WorldStoreError>{
    let idx=iteration.get("iteration_index").and_then(Value::as_i64).unwrap_or(0); let mut out=Vec::new();
    for m in article.get("article_manifestations").and_then(Value::as_array).into_iter().flatten(){let q=s(m,"qid");let l=s(m,"language");let r=m.get("revision_id").map(|x|x.to_string()).unwrap_or_default();let id={let d=s(m,"document_ref");if d.is_empty(){format!("wiki:{q}:{l}:{r}")}else{d}};out.push(WorldRecord{kind:WorldRecordKind::SourceManifestation,id,iteration_index:Some(idx),source_manifestation_id:None,surface_id:None,obligation_kind:None,payload:m.clone()});}
    for c in article.get("pnf_candidates").and_then(Value::as_array).into_iter().flatten(){let id=s(c,"claim_candidate_id");if !id.is_empty(){out.push(WorldRecord{kind:WorldRecordKind::PnfCandidate,id,iteration_index:Some(idx),source_manifestation_id:Some(s(c,"document_ref")),surface_id:None,obligation_kind:None,payload:c.clone()});}}
    for a in closure.get("canonical_atoms").and_then(Value::as_array).into_iter().flatten(){let id=s(a,"atom_id");if !id.is_empty(){out.push(WorldRecord{kind:WorldRecordKind::WorldAtom,id,iteration_index:Some(idx),source_manifestation_id:Some(s(a,"document_ref")),surface_id:None,obligation_kind:None,payload:a.clone()});}}
    for g in closure.get("gaps").and_then(Value::as_array).into_iter().flatten(){let surface=s(g,"surface_id");if let Some(ms)=g.get("missing_atom_ids").and_then(Value::as_array){for atom in ms.iter().filter_map(Value::as_str){let mut payload=g.clone();if let Some(o)=payload.as_object_mut(){o.insert("missing_atom_id".into(),Value::String(atom.into()));}out.push(WorldRecord{kind:WorldRecordKind::Gap,id:format!("gap:{surface}:{atom}"),iteration_index:Some(idx),source_manifestation_id:None,surface_id:Some(surface.clone()),obligation_kind:None,payload});}}else if s(g,"gap_kind")=="missing-surface"{out.push(WorldRecord{kind:WorldRecordKind::Gap,id:format!("gap:{surface}:missing-surface"),iteration_index:Some(idx),source_manifestation_id:None,surface_id:Some(surface),obligation_kind:None,payload:g.clone()});}}
    for o in closure.get("acquisition_obligations").and_then(Value::as_array).into_iter().flatten(){let mut id=s(o,"obligation_id");if id.is_empty(){id=stable_id("obligation:",o);}out.push(WorldRecord{kind:WorldRecordKind::Obligation,id,iteration_index:Some(idx),source_manifestation_id:None,surface_id:None,obligation_kind:Some(s(o,"obligation_kind")),payload:o.clone()});}
    for a in plan.get("selected_route_actions").and_then(Value::as_array).into_iter().flatten(){let id=s(a,"action_id");if !id.is_empty(){out.push(WorldRecord{kind:WorldRecordKind::RouteAction,id,iteration_index:Some(idx),source_manifestation_id:None,surface_id:None,obligation_kind:None,payload:a.clone()});}}
    out.push(WorldRecord{kind:WorldRecordKind::Iteration,id:format!("iteration:{idx}"),iteration_index:Some(idx),source_manifestation_id:None,surface_id:None,obligation_kind:None,payload:iteration.clone()}); Ok(out)
}

pub fn records_from_round_files(article:&Path,closure:&Path,plan:&Path,iteration:&Path)->Result<Vec<WorldRecord>,WorldStoreError>{fn read(p:&Path)->Result<Value,WorldStoreError>{Ok(serde_json::from_reader(BufReader::new(File::open(p)?))?)} records_from_round_values(&read(article)?,&read(closure)?,&read(plan)?,&read(iteration)?) }
fn copy_text(v:Option<&str>)->String{match v{None=>"\\N".into(),Some(x)=>x.replace('\\',"\\\\").replace('\t',"\\t").replace('\n',"\\n").replace('\r',"\\r")}}
fn aux1(r:&WorldRecord)->Option<&str>{match r.kind{WorldRecordKind::PnfCandidate|WorldRecordKind::WorldAtom=>r.source_manifestation_id.as_deref(),WorldRecordKind::Gap=>r.surface_id.as_deref(),WorldRecordKind::Obligation=>r.obligation_kind.as_deref(),_=>None}}

#[derive(Debug,Serialize)] pub struct IngestReceipt{pub records:u64,pub copy_streams:u64,pub postgres_persistence_is_semantic_authority:bool,pub semantic_promotion:bool}
pub struct WorldStore{client:Client}
impl WorldStore{
 pub fn connect(c:&DatabaseConfig)->Result<Self,WorldStoreError>{Ok(Self{client:Client::connect(c.database_url(),NoTls)?})}
 pub fn ensure_schema(&mut self)->Result<(),WorldStoreError>{self.client.batch_execute(world_schema_sql())?;Ok(())}
 pub fn ingest_records<I:IntoIterator<Item=WorldRecord>>(&mut self,records:I)->Result<IngestReceipt,WorldStoreError>{self.ensure_schema()?;let mut tx=self.client.transaction()?;tx.batch_execute(stage_sql())?;let mut w=tx.copy_in("COPY slr_world_stage_record (kind,record_id,iteration_index,aux1,payload) FROM STDIN WITH (FORMAT text)")?;let mut n=0;for r in records{let it=r.iteration_index.map(|x|x.to_string());let payload=serde_json::to_string(&r.payload)?;writeln!(w,"{}\t{}\t{}\t{}\t{}",copy_text(Some(r.kind.as_str())),copy_text(Some(&r.id)),copy_text(it.as_deref()),copy_text(aux1(&r)),copy_text(Some(&payload)))?;n+=1;}w.finish()?;for sql in [MERGE_SOURCE,MERGE_PNF,MERGE_ATOM,MERGE_GAP,MERGE_OBLIGATION,MERGE_ROUTE,MERGE_ITERATION]{tx.execute(sql,&[])?;}tx.commit()?;Ok(IngestReceipt{records:n,copy_streams:1,postgres_persistence_is_semantic_authority:false,semantic_promotion:false})}
 pub fn ingest_ndjson<R:BufRead>(&mut self,reader:R)->Result<IngestReceipt,WorldStoreError>{let mut rows=Vec::new();for line in reader.lines(){let l=line?;if !l.trim().is_empty(){rows.push(parse_world_record_line(&l)?);}}self.ingest_records(rows)}
 pub fn latest_frontier(&mut self)->Result<Vec<Value>,WorldStoreError>{self.ensure_schema()?;Ok(self.client.query(latest_frontier_sql(),&[])?.into_iter().map(|r|{let k:String=r.get(0);let id:String=r.get(1);let i:i64=r.get(2);let p:Value=r.get(3);json!({"kind":k,"id":id,"iteration_index":i,"payload":p,"candidate_only":true,"semantic_promotion":false})}).collect())}
}
