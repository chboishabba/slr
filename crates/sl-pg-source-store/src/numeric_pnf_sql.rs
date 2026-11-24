//! Fenced, set-wise Rust publisher to the ORIGINAL SensibLaw numeric schema.
//! It never creates a source, token, identity witness, QID, or Level-3 claim.
//! All publication is one PostgreSQL transaction; source/lease/digests are
//! checked before the first persistent insert, and failure rolls back.
//! Original authority: numeric_sentence_admission.py and its SQL migrations.

use crate::{NativeManifest,NativeSentenceClosure,
    validate_native_numeric_sentence_closure,NumericOperatorError,
    NumericDigestValue,strict_numeric_digest};
use postgres::{Client,NoTls};
use sha2::{Digest,Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug,Error)]
pub enum NumericSqlError{
    #[error("postgres original numeric authority: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("numeric sentence closure: {0}")]
    Closure(#[from] NumericOperatorError),
    #[error("numeric database has no exact registered source manifest")]
    ForeignSource,
    #[error("numeric sentence lease lost, foreign region or already closed")]
    LostLease,
    #[error("numeric closure references missing/foreign source parser token")]
    MissingToken,
    #[error("numeric sentence exceeds declared bounded admission budget")]
    TooLarge,
    #[error("numeric publication row identity could not be reconstructed")]
    MissingPublication,
}

#[derive(Debug,Clone)]
pub struct NumericMdlWeights{
    pub node:f64,pub edge:f64,pub alternative:f64,pub unresolved:f64,
    pub boundary:f64,pub encoded_byte:f64,pub rule:f64,pub closure:f64,
    pub query_ns:f64,pub promoted:f64,pub interface_member:f64,
    pub hierarchy:f64,pub promotion_alpha:f64,pub promotion_beta:f64,
    pub promotion_threshold:f64,
}
impl Default for NumericMdlWeights{
    fn default()->Self{Self{
        node:1.0,edge:0.75,alternative:2.0,unresolved:3.0,
        boundary:4.0,encoded_byte:1.0/1024.0,rule:0.5,closure:1.0,
        query_ns:1.0/1_000_000.0,promoted:0.25,interface_member:0.5,
        hierarchy:0.25,promotion_alpha:1.0,promotion_beta:1.0,
        promotion_threshold:0.0,
    }}
}
impl NumericMdlWeights{
    pub fn validate(&self)->bool{
        [self.node,self.edge,self.alternative,self.unresolved,self.boundary,
         self.encoded_byte,self.rule,self.closure,self.query_ns,self.promoted,
         self.interface_member,self.hierarchy,self.promotion_alpha,
         self.promotion_beta].iter().all(|n|n.is_finite()&&*n>=0.0)
         &&self.promotion_threshold.is_finite()
    }
    pub fn object_score(&self,info:f64,repr:f64,amb:f64,participation:i32)->f64{
        info+f64::from(participation)-self.promotion_alpha*repr-self.promotion_beta*amb
    }
}

#[derive(Debug,Clone)]
pub struct NumericSentenceLease{
    pub work_id:i64,pub region_id:i64,
    pub lease_token:String,pub lease_epoch:i64,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct NumericPublishedSentence{
    pub region_id:i64,pub graph_revision:i64,pub interface_id:i64,
    pub source_sha256:String,pub object_count:usize,
    pub factor_count:usize,pub demand_count:usize,
    pub original_semantic_identity_paid:bool,
    pub original_level_three_paid:bool,
}
fn num(n:i64)->NumericDigestValue{NumericDigestValue::Int(n as i128)}
fn seq(xs:&[i64])->NumericDigestValue{
    NumericDigestValue::Seq(xs.iter().map(|n|num(*n)).collect())
}

/// Insert only into the ORIGINAL numeric tables using set-wise array stages.
/// The supplied token IDs must already be present in the numeric parser DB
/// and on this exact source+sentence; this function does not forge them.
pub fn publish_original_numeric_sentence(
    database_url:&str,
    manifest:&NativeManifest,
    canonical_source_bytes:&[u8],
    lease:&NumericSentenceLease,
    closure:&NativeSentenceClosure,
    mdl:&NumericMdlWeights,
)->Result<NumericPublishedSentence,NumericSqlError>{
    manifest.verify_source_bytes(canonical_source_bytes)
        .map_err(|_|NumericSqlError::ForeignSource)?;
    if !mdl.validate(){return Err(NumericSqlError::ForeignSource);}
    validate_native_numeric_sentence_closure(lease.region_id,closure)?;
    const MAX_ROWS:usize=2048;
    if closure.objects.len()>MAX_ROWS||closure.factors.len()>MAX_ROWS
        ||closure.demands.len()>MAX_ROWS{
        return Err(NumericSqlError::TooLarge);
    }
    let mut db=Client::connect(database_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET LOCAL statement_timeout='90000ms'")?;
    let source=tx.query(
        "SELECT content_sha256,byte_count,char_count,encoding_ref
         FROM execution.semantic_parser_source
         WHERE run_ref=$1 AND document_ref=$2",
        &[&manifest.parser_run_ref,&manifest.document_ref])?;
    if source.len()!=1{return Err(NumericSqlError::ForeignSource);}
    let digest:Vec<u8>=source[0].get(0);
    let byte_count:i64=source[0].get(1);
    let char_count:i64=source[0].get(2);
    let encoding:String=source[0].get(3);
    if digest!=Sha256::digest(canonical_source_bytes).as_slice()
        ||byte_count!=manifest.source_byte_length as i64
        ||char_count!=manifest.source_codepoint_length as i64
        ||encoding!="utf8-canonical-source:v1"{
        return Err(NumericSqlError::ForeignSource);
    }
    let work=tx.query(
        "SELECT work.state_id,work.lease_token,work.lease_epoch,
                region.run_ref,region.document_ref,
                region.parent_region_id,region.graph_revision,
                region.region_kind,region.start_char,region.end_char
         FROM execution.semantic_pnf_work_item AS work
         JOIN execution.semantic_pnf_region AS region ON region.region_id=work.region_id
         WHERE work.work_id=$1 AND work.region_id=$2
         FOR UPDATE OF work,region",
        &[&lease.work_id,&lease.region_id])?;
    if work.len()!=1{return Err(NumericSqlError::LostLease);}
    let w=&work[0];
    if w.get::<_,i16>(0)!=2
        ||w.get::<_,String>(1)!=lease.lease_token
        ||w.get::<_,i64>(2)!=lease.lease_epoch
        ||w.get::<_,String>(3)!=manifest.parser_run_ref
        ||w.get::<_,String>(4)!=manifest.document_ref
        ||w.get::<_,i16>(7)!=1{
        return Err(NumericSqlError::LostLease);
    }
    let parent:Option<i64>=w.get(5);
    let revision:i64=w.get::<_,i64>(6)+1;
    let start:i64=w.get(8);
    let end:i64=w.get(9);
    // Demand projection is owned by an exact parser sentence link. Closing
    // a region with no original numeric parser sentence would falsely appear
    // to pay the anaphor producer while migration 180 emits nothing.
    let parser_sentences:i64=tx.query_one(
        "SELECT count(DISTINCT sentence.sentence_id)::bigint
         FROM execution.semantic_pnf_sentence_region AS link
         JOIN execution.semantic_parser_sentence AS sentence
           ON sentence.sentence_id=link.sentence_id
         WHERE link.region_id=$1
           AND sentence.run_ref=$2 AND sentence.document_ref=$3
           AND sentence.representation_version=2
           AND sentence.start_char=$4 AND sentence.end_char=$5",
        &[&lease.region_id,&manifest.parser_run_ref,
          &manifest.document_ref,&start,&end])?.get(0);
    if parser_sentences!=1{return Err(NumericSqlError::MissingToken);}
    let mut referenced=BTreeSet::<i64>::new();
    for obj in &closure.objects{referenced.insert(obj.source_token_id);}
    for f in &closure.factors{
        referenced.extend(f.support_token_ids.iter().copied());
        referenced.extend(f.slots.iter().map(|s|s.source_token_id));
    }
    if referenced.len()>MAX_ROWS*8{return Err(NumericSqlError::TooLarge);}
    let ids=referenced.into_iter().collect::<Vec<_>>();
    let valid:i64=tx.query_one(
        "SELECT count(DISTINCT token_id)::bigint
         FROM execution.semantic_parser_token
         WHERE token_id=ANY($1::bigint[])
           AND run_ref=$2 AND document_ref=$3
           AND representation_version=2
           AND start_char >= $4 AND end_char <= $5",
        &[&ids,&manifest.parser_run_ref,&manifest.document_ref,&start,&end])?.get(0);
    if valid!=ids.len() as i64{return Err(NumericSqlError::MissingToken);}
    let sentence_token_count:i64=tx.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_parser_token
         WHERE run_ref=$1 AND document_ref=$2
           AND representation_version=2
           AND start_char >= $3 AND end_char <= $4",
        &[&manifest.parser_run_ref,&manifest.document_ref,&start,&end])?.get(0);
    if sentence_token_count<0 ||closure.measure.encoded_byte_count !=
        sentence_token_count as usize *80
          +closure.factors.len()*32+closure.demands.len()*32{
        return Err(NumericSqlError::MissingPublication);
    }

    // Exactly five bounded stage families, no per-object query loop.
    tx.batch_execute(r#"
      CREATE TEMP TABLE IF NOT EXISTS tmp_native_num_obj(
        ordinal integer PRIMARY KEY,digest bytea NOT NULL,kind bigint NOT NULL,
        head bigint NOT NULL,source_token bigint NOT NULL,
        info double precision NOT NULL,repr double precision NOT NULL,
        ambiguity double precision NOT NULL,score double precision NOT NULL,
        promoted boolean NOT NULL) ON COMMIT DROP;
      CREATE TEMP TABLE IF NOT EXISTS tmp_native_num_factor(
        ordinal integer PRIMARY KEY,digest bytea NOT NULL,typ bigint NOT NULL,
        predicate bigint NOT NULL,temporal smallint NOT NULL,
        modal smallint NOT NULL,score double precision NOT NULL) ON COMMIT DROP;
      CREATE TEMP TABLE IF NOT EXISTS tmp_native_num_support(
        factor_ordinal integer NOT NULL,support_ordinal integer NOT NULL,
        token_id bigint NOT NULL,
        PRIMARY KEY(factor_ordinal,support_ordinal)) ON COMMIT DROP;
      CREATE TEMP TABLE IF NOT EXISTS tmp_native_num_slot(
        factor_ordinal integer NOT NULL,slot_ordinal integer NOT NULL,
        role bigint NOT NULL,source_token bigint NOT NULL,
        state smallint NOT NULL,required boolean NOT NULL,
        PRIMARY KEY(factor_ordinal,slot_ordinal)) ON COMMIT DROP;
      CREATE TEMP TABLE IF NOT EXISTS tmp_native_num_demand(
        ordinal integer PRIMARY KEY,digest bytea NOT NULL,kind smallint NOT NULL,
        factor_typ bigint,object_kind bigint,lexical bigint,role bigint,
        residual bigint NOT NULL,recency smallint NOT NULL,
        max_candidates integer NOT NULL) ON COMMIT DROP;
      TRUNCATE tmp_native_num_obj,tmp_native_num_factor,
               tmp_native_num_support,tmp_native_num_slot,tmp_native_num_demand;
    "#)?;

    let ord=(0..closure.objects.len()).map(|n|n as i32).collect::<Vec<_>>();
    let obj_digest=closure.objects.iter().map(|o|o.object_digest.to_vec())
        .collect::<Vec<_>>();
    let kind=closure.objects.iter().map(|o|o.object_kind_symbol_id).collect::<Vec<_>>();
    let head=closure.objects.iter().map(|o|o.head_symbol_id).collect::<Vec<_>>();
    let src=closure.objects.iter().map(|o|o.source_token_id).collect::<Vec<_>>();
    let info=closure.objects.iter().map(|o|o.information_gain).collect::<Vec<_>>();
    let repr=closure.objects.iter().map(|o|o.representation_cost).collect::<Vec<_>>();
    let ambiguity=closure.objects.iter().map(|o|o.ambiguity_cost).collect::<Vec<_>>();
    let score=closure.objects.iter().map(|o|
        mdl.object_score(o.information_gain,o.representation_cost,
            o.ambiguity_cost,o.factor_participation)).collect::<Vec<_>>();
    let promoted=score.iter().map(|s|*s>mdl.promotion_threshold).collect::<Vec<_>>();
    tx.execute(
        "INSERT INTO tmp_native_num_obj
         SELECT * FROM unnest($1::integer[],$2::bytea[],$3::bigint[],
         $4::bigint[],$5::bigint[],$6::double precision[],
         $7::double precision[],$8::double precision[],
         $9::double precision[],$10::boolean[])",
        &[&ord,&obj_digest,&kind,&head,&src,&info,&repr,
          &ambiguity,&score,&promoted])?;

    let ford=(0..closure.factors.len()).map(|n|n as i32).collect::<Vec<_>>();
    let fdig=closure.factors.iter().map(|f|f.factor_digest.to_vec()).collect::<Vec<_>>();
    let ftyp=closure.factors.iter().map(|f|f.factor_type_symbol_id).collect::<Vec<_>>();
    let fpred=closure.factors.iter().map(|f|f.predicate_symbol_id).collect::<Vec<_>>();
    let ftemporal=closure.factors.iter().map(|f|f.temporal_state).collect::<Vec<_>>();
    let fmodal=closure.factors.iter().map(|f|f.modal_state).collect::<Vec<_>>();
    let fscore=closure.factors.iter().map(|f|f.support_score).collect::<Vec<_>>();
    tx.execute(
        "INSERT INTO tmp_native_num_factor SELECT * FROM unnest(
         $1::integer[],$2::bytea[],$3::bigint[],$4::bigint[],
         $5::smallint[],$6::smallint[],$7::double precision[])",
        &[&ford,&fdig,&ftyp,&fpred,&ftemporal,&fmodal,&fscore])?;

    let mut s_factor=Vec::<i32>::new();let mut s_ord=Vec::<i32>::new();
    let mut s_token=Vec::<i64>::new();
    let mut x_factor=Vec::<i32>::new();let mut x_ord=Vec::<i32>::new();
    let mut x_role=Vec::<i64>::new();let mut x_token=Vec::<i64>::new();
    let mut x_state=Vec::<i16>::new();let mut x_required=Vec::<bool>::new();
    for (i,f) in closure.factors.iter().enumerate(){
        for (j,id) in f.support_token_ids.iter().enumerate(){
            s_factor.push(i as i32);s_ord.push(j as i32);s_token.push(*id);
        }
        for (j,slot) in f.slots.iter().enumerate(){
            x_factor.push(i as i32);x_ord.push(j as i32);
            x_role.push(slot.role_symbol_id);x_token.push(slot.source_token_id);
            x_state.push(slot.resolution_state);x_required.push(slot.required);
        }
    }
    tx.execute(
        "INSERT INTO tmp_native_num_support SELECT * FROM unnest(
         $1::integer[],$2::integer[],$3::bigint[])",
        &[&s_factor,&s_ord,&s_token])?;
    tx.execute(
        "INSERT INTO tmp_native_num_slot SELECT * FROM unnest(
         $1::integer[],$2::integer[],$3::bigint[],$4::bigint[],
         $5::smallint[],$6::boolean[])",
        &[&x_factor,&x_ord,&x_role,&x_token,&x_state,&x_required])?;

    let dord=(0..closure.demands.len()).map(|n|n as i32).collect::<Vec<_>>();
    let ddig=closure.demands.iter().map(|d|d.demand_digest.to_vec()).collect::<Vec<_>>();
    let dkind=closure.demands.iter().map(|d|d.expected_target_kind).collect::<Vec<_>>();
    let dtyp=closure.demands.iter().map(|d|d.expected_factor_type_symbol_id)
        .collect::<Vec<_>>();
    let dobj=closure.demands.iter().map(|d|d.expected_object_kind_symbol_id)
        .collect::<Vec<_>>();
    let dlex=closure.demands.iter().map(|d|d.lexical_symbol_id).collect::<Vec<_>>();
    let drole=closure.demands.iter().map(|d|d.role_symbol_id).collect::<Vec<_>>();
    let dres=closure.demands.iter().map(|d|d.residual_type_symbol_id).collect::<Vec<_>>();
    let drec=closure.demands.iter().map(|d|d.recency_class).collect::<Vec<_>>();
    let dmax=closure.demands.iter().map(|d|d.max_candidates).collect::<Vec<_>>();
    tx.execute(
        "INSERT INTO tmp_native_num_demand SELECT * FROM unnest(
         $1::integer[],$2::bytea[],$3::smallint[],$4::bigint[],
         $5::bigint[],$6::bigint[],$7::bigint[],$8::bigint[],
         $9::smallint[],$10::integer[])",
        &[&dord,&ddig,&dkind,&dtyp,&dobj,&dlex,&drole,
          &dres,&drec,&dmax])?;

    // Original admitted object/factor/demand schemas and their triggers.
    tx.execute(
        "INSERT INTO execution.semantic_pnf_object
         (object_digest,region_id,object_kind_symbol_id,head_symbol_id,
          scope_region_id,promotion_level,information_gain,
          representation_cost,ambiguity_cost,promotion_score,active)
         SELECT digest,$1,kind,head,$1,0,info,repr,ambiguity,score,TRUE
         FROM tmp_native_num_obj ORDER BY ordinal
         ON CONFLICT(object_digest) DO UPDATE
         SET promotion_score=EXCLUDED.promotion_score,active=TRUE",
        &[&lease.region_id])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_object_token_support
         (object_id,token_id,ordinal)
         SELECT o.object_id,s.source_token,0 FROM tmp_native_num_obj AS s
         JOIN execution.semantic_pnf_object AS o ON o.object_digest=s.digest
         ON CONFLICT DO NOTHING",&[])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_factor
         (factor_digest,region_id,factor_type_symbol_id,predicate_symbol_id,
          scope_region_id,temporal_state,modal_state,promotion_level,
          support_score,active)
         SELECT digest,$1,typ,predicate,$1,temporal,modal,0,score,TRUE
         FROM tmp_native_num_factor ORDER BY ordinal
         ON CONFLICT(factor_digest) DO UPDATE SET active=TRUE",
        &[&lease.region_id])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_factor_token_support
         (factor_id,token_id,ordinal)
         SELECT f.factor_id,s.token_id,s.support_ordinal
         FROM tmp_native_num_support AS s
         JOIN tmp_native_num_factor AS t ON t.ordinal=s.factor_ordinal
         JOIN execution.semantic_pnf_factor AS f ON f.factor_digest=t.digest
         ON CONFLICT DO NOTHING",&[])?;
    tx.execute(
        "WITH choice AS (
           SELECT DISTINCT ON (source_token) source_token,digest
           FROM tmp_native_num_obj ORDER BY source_token,ordinal DESC)
         INSERT INTO execution.semantic_pnf_hyperedge
         (factor_id,slot_ordinal,role_symbol_id,object_id,resolution_state,required)
         SELECT f.factor_id,s.slot_ordinal,s.role,o.object_id,s.state,s.required
         FROM tmp_native_num_slot AS s
         JOIN tmp_native_num_factor AS t ON t.ordinal=s.factor_ordinal
         JOIN execution.semantic_pnf_factor AS f ON f.factor_digest=t.digest
         JOIN choice AS c ON c.source_token=s.source_token
         JOIN execution.semantic_pnf_object AS o ON o.object_digest=c.digest
         ON CONFLICT DO NOTHING",&[])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_demand
         (demand_digest,source_region_id,expected_target_kind,
          expected_factor_type_symbol_id,expected_object_kind_symbol_id,
          lexical_symbol_id,role_symbol_id,residual_type_symbol_id,
          recency_class,state,max_candidates)
         SELECT digest,$1,kind,factor_typ,object_kind,lexical,role,
                residual,recency,1,max_candidates
         FROM tmp_native_num_demand ORDER BY ordinal
         ON CONFLICT(demand_digest) DO UPDATE
         SET state=LEAST(execution.semantic_pnf_demand.state,1)",
        &[&lease.region_id])?;

    let objects=tx.query(
        "SELECT o.object_id,s.head,s.score
         FROM tmp_native_num_obj AS s
         JOIN execution.semantic_pnf_object AS o ON o.object_digest=s.digest
         WHERE s.promoted ORDER BY s.ordinal",&[])?;
    let factors=tx.query(
        "SELECT f.factor_id,s.typ,s.predicate
         FROM tmp_native_num_factor AS s
         JOIN execution.semantic_pnf_factor AS f ON f.factor_digest=s.digest
         ORDER BY s.ordinal",&[])?;
    let demands=tx.query(
        "SELECT d.demand_id,s.residual,s.factor_typ,s.lexical
         FROM tmp_native_num_demand AS s
         JOIN execution.semantic_pnf_demand AS d ON d.demand_digest=s.digest
         ORDER BY s.ordinal",&[])?;
    if factors.len()!=closure.factors.len()||demands.len()!=closure.demands.len(){
        return Err(NumericSqlError::MissingPublication);
    }
    let object_ids=objects.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let factor_ids=factors.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let demand_ids=demands.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let interface_digest=strict_numeric_digest(vec![
        num(lease.region_id),num(revision),seq(&object_ids),
        seq(&factor_ids),seq(&demand_ids)
    ]);
    let parent_interface:Option<i64>=match parent {
        Some(id)=>tx.query_opt(
            "SELECT interface_id FROM execution.semantic_pnf_interface
             WHERE region_id=$1",&[&id])?.map(|r|r.get(0)),
        None=>None,
    };
    let m=&closure.measure;
    let card=object_ids.len()+factor_ids.len()+demand_ids.len();
    let mdl_cost=mdl.node*m.node_count as f64+mdl.edge*m.edge_count as f64
        +mdl.unresolved*m.unresolved_count as f64
        +mdl.boundary*m.boundary_demand_weight
        +mdl.encoded_byte*m.encoded_byte_count as f64
        +mdl.rule*m.rule_count as f64+mdl.closure*m.closure_rounds as f64
        +mdl.promoted*object_ids.len() as f64
        +mdl.interface_member*card as f64;
    let interface_id:i64=tx.query_one(
        "INSERT INTO execution.semantic_pnf_interface
         (interface_digest,region_id,parent_interface_id,closure_state,
          graph_revision,node_count,edge_count,alternative_count,
          unresolved_count,boundary_demand_weight,encoded_byte_count,
          rule_count,closure_rounds,query_cost_ns,promoted_object_count,
          interface_cardinality,hierarchy_cost,mdl_cost)
         VALUES($1,$2,$3,2,$4,$5,$6,0,$7,$8,$9,$10,$11,0,$12,$13,0,$14)
         ON CONFLICT(region_id) DO UPDATE SET
           interface_digest=EXCLUDED.interface_digest,
           closure_state=EXCLUDED.closure_state,
           graph_revision=EXCLUDED.graph_revision,
           node_count=EXCLUDED.node_count,
           edge_count=EXCLUDED.edge_count,
           unresolved_count=EXCLUDED.unresolved_count,
           boundary_demand_weight=EXCLUDED.boundary_demand_weight,
           encoded_byte_count=EXCLUDED.encoded_byte_count,
           rule_count=EXCLUDED.rule_count,
           closure_rounds=EXCLUDED.closure_rounds,
           promoted_object_count=EXCLUDED.promoted_object_count,
           interface_cardinality=EXCLUDED.interface_cardinality,
           mdl_cost=EXCLUDED.mdl_cost
         RETURNING interface_id",
        &[&interface_digest.to_vec(),&lease.region_id,&parent_interface,
          &revision,&(m.node_count as i64),&(m.edge_count as i64),
          &(m.unresolved_count as i64),&m.boundary_demand_weight,
          &(m.encoded_byte_count as i64),&(m.rule_count as i64),
          &(m.closure_rounds as i64),&(object_ids.len() as i64),
          &(card as i64),&mdl_cost])?.get(0);

    // Original interface export and searchable lookup keys, set-wise.
    tx.execute(
        "INSERT INTO execution.semantic_pnf_interface_export
         (interface_id,export_kind,target_kind,target_id,key_symbol_id,
          rank,promotion_score)
         SELECT $1,1,1,o.object_id,s.head,s.ordinal,s.score
         FROM tmp_native_num_obj AS s
         JOIN execution.semantic_pnf_object AS o ON o.object_digest=s.digest
         WHERE s.promoted ON CONFLICT DO NOTHING",
        &[&interface_id])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_interface_export
         (interface_id,export_kind,target_kind,target_id,key_symbol_id,
          rank,promotion_score)
         SELECT $1,2,2,f.factor_id,s.typ,s.ordinal,0
         FROM tmp_native_num_factor AS s
         JOIN execution.semantic_pnf_factor AS f ON f.factor_digest=s.digest
         ON CONFLICT DO NOTHING",&[&interface_id])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_interface_export
         (interface_id,export_kind,target_kind,target_id,key_symbol_id,
          residual_type_symbol_id,rank,promotion_score)
         SELECT $1,5,3,d.demand_id,s.lexical,s.residual,s.ordinal,0
         FROM tmp_native_num_demand AS s
         JOIN execution.semantic_pnf_demand AS d ON d.demand_digest=s.digest
         ON CONFLICT DO NOTHING",&[&interface_id])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_interface_lookup
         (interface_id,key_kind,key_a,key_b,target_kind,target_id,rank)
         SELECT $1,3,s.head,0,1,o.object_id,s.ordinal
         FROM tmp_native_num_obj AS s
         JOIN execution.semantic_pnf_object AS o ON o.object_digest=s.digest
         WHERE s.promoted
         UNION ALL
         SELECT $1,1,s.typ,0,2,f.factor_id,s.ordinal
         FROM tmp_native_num_factor AS s
         JOIN execution.semantic_pnf_factor AS f ON f.factor_digest=s.digest
         UNION ALL
         SELECT $1,3,s.predicate,0,2,f.factor_id,s.ordinal
         FROM tmp_native_num_factor AS s
         JOIN execution.semantic_pnf_factor AS f ON f.factor_digest=s.digest
         UNION ALL
         SELECT $1,5,s.residual,COALESCE(s.factor_typ,0),3,d.demand_id,s.ordinal
         FROM tmp_native_num_demand AS s
         JOIN execution.semantic_pnf_demand AS d ON d.demand_digest=s.digest
         UNION ALL
         SELECT $1,3,s.lexical,s.residual,3,d.demand_id,s.ordinal
         FROM tmp_native_num_demand AS s
         JOIN execution.semantic_pnf_demand AS d ON d.demand_digest=s.digest
         WHERE s.lexical IS NOT NULL
         ON CONFLICT DO NOTHING",&[&interface_id])?;
    tx.query_one("SELECT execution.rebuild_pnf_interface_ancestors($1)",
        &[&interface_id])?;
    tx.execute(
        "UPDATE execution.semantic_pnf_region
         SET closure_state=2,graph_revision=$1,closed_at=CURRENT_TIMESTAMP
         WHERE region_id=$2",
        &[&revision,&lease.region_id])?;
    let done=tx.execute(
        "UPDATE execution.semantic_pnf_work_item
         SET state_id=3,lease_owner=NULL,lease_token=NULL,
             lease_expires_at=NULL,completed_at=CURRENT_TIMESTAMP
         WHERE work_id=$1 AND state_id=2
           AND lease_token=$2 AND lease_epoch=$3",
        &[&lease.work_id,&lease.lease_token,&lease.lease_epoch])?;
    if done!=1{return Err(NumericSqlError::LostLease);}
    // Existing migration-180 relation-valued projector owns the pronoun
    // occurrence / object / unresolved anaphor demand. Verify its effect
    // inside this SAME transaction; do not fabricate a Rust identity.
    let expected_pronouns:i64=tx.query_one(
        "SELECT count(*)::bigint
         FROM execution.semantic_parser_token AS token
         JOIN execution.semantic_pnf_anaphor_projection_constant AS const
           ON const.singleton
          AND token.pos_symbol_id=const.pronoun_pos_symbol_id
         WHERE token.run_ref=$1 AND token.document_ref=$2
           AND token.representation_version=2
           AND token.start_char >= $3 AND token.end_char <= $4",
        &[&manifest.parser_run_ref,&manifest.document_ref,&start,&end])?.get(0);
    let produced_pronouns:i64=tx.query_one(
        "SELECT count(DISTINCT mention.head_token_id)::bigint
         FROM execution.semantic_pnf_mention AS mention
         WHERE mention.region_id=$1 AND mention.mention_kind=4
           AND EXISTS (
             SELECT 1 FROM execution.semantic_pnf_object_token_support AS support
             JOIN execution.semantic_pnf_object AS object
               ON object.object_id=support.object_id
             WHERE support.token_id=mention.head_token_id
               AND object.region_id=mention.region_id
           )",
        &[&lease.region_id])?.get(0);
    if produced_pronouns<expected_pronouns{
        return Err(NumericSqlError::MissingPublication);
    }
    tx.commit()?;
    Ok(NumericPublishedSentence{
        region_id:lease.region_id,graph_revision:revision,interface_id,
        source_sha256:manifest.canonical_content_sha256.clone(),
        object_count:object_ids.len(),factor_count:factor_ids.len(),
        demand_count:demand_ids.len(),
        original_semantic_identity_paid:false,original_level_three_paid:false,
    })
}
