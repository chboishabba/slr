//! Pure Rust strict numeric-PNF sentence composer.
//! Original semantic owner: SensibLaw numeric_operator_composition.py.
//! This emits candidate objects/factors/demands, never actor identity admission.

use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::collections::{BTreeMap,BTreeSet,HashMap};
use thiserror::Error;

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NumericToken {
    pub token_id:i64, pub orth_id:i64, pub lemma_id:i64,
    pub pos_id:i64, pub tag_id:i64, pub dependency_id:i64,
    pub head_token_id:i64, pub morph_set_id:Option<i64>,
    pub start_char:i64, pub end_char:i64,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NumericOperatorLexicon {
    pub lemma_ids:BTreeMap<String,i64>,
    pub dependency_ids:BTreeMap<String,i64>,
    pub pos_ids:BTreeMap<String,i64>,
    pub factor_type_ids:BTreeMap<String,i64>,
    pub predicate_ids:BTreeMap<String,i64>,
    pub role_ids:BTreeMap<String,i64>,
    pub residual_ids:BTreeMap<String,i64>,
    pub object_kind_ids:BTreeMap<String,i64>,
}
#[derive(Debug,Error)]
pub enum NumericOperatorError {
    #[error("missing original numeric symbol: {0}/{1}")]
    MissingSymbol(&'static str,String),
    #[error("missing original dependency head for token {0}")]
    MissingHead(i64),
    #[error("duplicate conflicting numeric token {0}")]
    ConflictingToken(i64),
    #[error("invalid numeric source token or region")]
    InvalidSpan,
    #[error("numeric PNF sentence closure digest/support/measure mismatch")]
    InconsistentClosure,
}
impl NumericOperatorLexicon {
    pub fn symbol(&self,kind:&'static str,label:&str)->Result<i64,NumericOperatorError> {
        let map=match kind {
            "lemma"=>&self.lemma_ids,"dep"=>&self.dependency_ids,
            "pos"=>&self.pos_ids,"factor"=>&self.factor_type_ids,
            "predicate"=>&self.predicate_ids,"role"=>&self.role_ids,
            "residual"=>&self.residual_ids,"object"=>&self.object_kind_ids,
            _=>return Err(NumericOperatorError::MissingSymbol(kind,label.into()))
        };
        map.get(label).copied().ok_or_else(||NumericOperatorError::MissingSymbol(kind,label.into()))
    }
    pub fn validate(&self)->Result<(),NumericOperatorError>{
        for l in ["must","shall","may","not","never","if","when","provided",
          "providing","unless","except","excluding","commence","begin","repeal",
          "amend","cease"] {self.symbol("lemma",l)?;}
        for d in ["aux","auxpass","nsubj","nsubjpass","csubj","obj","dobj",
          "pobj","attr","oprd","mark","prep","advmod"] {self.symbol("dep",d)?;}
        for p in ["VERB","AUX"]{self.symbol("pos",p)?;}
        for n in ["semantic.normative_relation","semantic.legal_condition",
          "semantic.legal_exception","semantic.legal_transition"]{self.symbol("factor",n)?;}
        for n in ["normative.obligation","normative.permission_candidate",
          "normative.prohibition","legal.activation_condition_candidate",
          "legal.exception_candidate","legal.commencement",
          "legal.commencement_candidate","legal.repeal",
          "legal.amendment","legal.cessation"]{self.symbol("predicate",n)?;}
        for n in ["conduct","bearer","object","condition","exception",
          "host","transition","legal_object"]{self.symbol("role",n)?;}
        for n in ["jurisdiction_unresolved","legal_time_unresolved",
          "normative_scope_unresolved","modal_sense_unresolved",
          "norm_bearer_unresolved","exception_attachment_unresolved",
          "exception_burden_unresolved","condition_attachment_unresolved",
          "legal_object_identity_unresolved","effective_time_unresolved"] {
            self.symbol("residual",n)?;
        }
        self.symbol("object","parser.role_participant")?;
        Ok(())
    }
}
#[derive(Debug,Clone)]
pub enum NumericDigestValue {
    Nil,Bool(bool),Int(i128),Float(f64),Bytes(Vec<u8>),Seq(Vec<NumericDigestValue>),
}
impl NumericDigestValue {
    fn encode(&self,dst:&mut Vec<u8>){
        match self {
            Self::Nil=>dst.push(0),
            Self::Bool(false)=>dst.push(1),Self::Bool(true)=>dst.push(2),
            Self::Int(n)=>{dst.push(3);dst.extend_from_slice(&n.to_be_bytes());},
            Self::Float(n)=>{dst.push(4);dst.extend_from_slice(&n.to_be_bytes());},
            Self::Bytes(b)=>{dst.push(5);dst.extend_from_slice(&(b.len() as u64).to_be_bytes());dst.extend_from_slice(b);},
            Self::Seq(xs)=>{
                dst.push(6);dst.extend_from_slice(&(xs.len() as u64).to_be_bytes());
                for x in xs {
                    let mut tmp=Vec::new();x.encode(&mut tmp);
                    dst.extend_from_slice(&(tmp.len() as u64).to_be_bytes());
                    dst.extend_from_slice(&tmp);
                }
            }
        }
    }
}
fn num(n:i64)->NumericDigestValue{NumericDigestValue::Int(n as i128)}
fn nums(xs:&[i64])->NumericDigestValue{NumericDigestValue::Seq(xs.iter().map(|&n|num(n)).collect())}
pub fn strict_numeric_digest(xs:Vec<NumericDigestValue>)->[u8;32]{
    let mut raw=Vec::new();NumericDigestValue::Seq(xs).encode(&mut raw);
    Sha256::digest(raw).into()
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub struct NativeObject {
    pub object_digest:[u8;32],pub source_token_id:i64,pub object_kind_symbol_id:i64,
    pub head_symbol_id:i64,pub information_gain:f64,pub representation_cost:f64,
    pub ambiguity_cost:f64,pub factor_participation:i32,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NativeSlot {pub role_symbol_id:i64,pub source_token_id:i64,
    pub resolution_state:i16,pub required:bool}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub struct NativeFactor {
    pub factor_digest:[u8;32],pub factor_type_symbol_id:i64,
    pub predicate_symbol_id:i64,pub modal_state:i16,pub temporal_state:i16,
    pub slots:Vec<NativeSlot>,pub support_token_ids:Vec<i64>,
    pub residual_symbol_ids:Vec<i64>,pub support_score:f64,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NativeDemand {
    pub demand_digest:[u8;32],pub expected_target_kind:i16,
    pub expected_factor_type_symbol_id:Option<i64>,
    pub expected_object_kind_symbol_id:Option<i64>,pub lexical_symbol_id:Option<i64>,
    pub role_symbol_id:Option<i64>,pub residual_type_symbol_id:i64,
    pub recency_class:i16,pub max_candidates:i32,
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub struct NativeSentenceMeasure {
    pub node_count:usize,pub edge_count:usize,pub unresolved_count:usize,
    pub boundary_demand_weight:f64,pub encoded_byte_count:usize,
    pub rule_count:usize,pub closure_rounds:usize,pub promoted_object_count:usize,
    pub interface_cardinality:usize,
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub struct NativeSentenceClosure {
    pub objects:Vec<NativeObject>,pub factors:Vec<NativeFactor>,
    pub demands:Vec<NativeDemand>,pub measure:NativeSentenceMeasure,
}
fn object(region:i64,token:&NumericToken,lex:&NumericOperatorLexicon)
    ->Result<NativeObject,NumericOperatorError>{
    let kind=lex.symbol("object","parser.role_participant")?;
    Ok(NativeObject{
        object_digest:strict_numeric_digest(vec![num(region),num(token.token_id),num(kind)]),
        source_token_id:token.token_id,object_kind_symbol_id:kind,head_symbol_id:token.lemma_id,
        information_gain:2.0,representation_cost:1.0,ambiguity_cost:0.5,
        factor_participation:1,
    })
}
fn slot(role:i64,token:i64)->NativeSlot{
    NativeSlot{role_symbol_id:role,source_token_id:token,resolution_state:2,required:true}
}
fn factor(region:i64,typ:i64,predicate:i64,modal:i16,temporal:i16,
  mut slots:Vec<NativeSlot>,support:Vec<i64>,residuals:Vec<i64>)->NativeFactor{
    slots.sort_by_key(|s|(s.role_symbol_id,s.source_token_id));
    let support=BTreeSet::<_>::from_iter(support).into_iter().collect::<Vec<_>>();
    let residuals=BTreeSet::<_>::from_iter(residuals).into_iter().collect::<Vec<_>>();
    let slot_bytes=NumericDigestValue::Seq(slots.iter().map(|s|
        NumericDigestValue::Seq(vec![num(s.role_symbol_id),num(s.source_token_id),
            num(i64::from(s.resolution_state)),NumericDigestValue::Bool(s.required)])).collect());
    let factor_digest=strict_numeric_digest(vec![
        num(region),num(typ),num(predicate),num(i64::from(modal)),
        num(i64::from(temporal)),slot_bytes,nums(&support),nums(&residuals)
    ]);
    NativeFactor{factor_digest,factor_type_symbol_id:typ,
        predicate_symbol_id:predicate,modal_state:modal,temporal_state:temporal,
        slots,support_token_ids:support,residual_symbol_ids:residuals,support_score:1.0}
}
fn demands(region:i64,f:&NativeFactor,lemma:i64)->Vec<NativeDemand>{
    f.residual_symbol_ids.iter().map(|r|NativeDemand{
        demand_digest:strict_numeric_digest(vec![
            num(region),NumericDigestValue::Bytes(f.factor_digest.to_vec()),num(*r),num(lemma)]),
        expected_target_kind:2,expected_factor_type_symbol_id:Some(f.factor_type_symbol_id),
        expected_object_kind_symbol_id:None,lexical_symbol_id:Some(lemma),
        role_symbol_id:None,residual_type_symbol_id:*r,
        recency_class:3,max_candidates:16,
    }).collect()
}
fn matching_child(children:&[&NumericToken],ids:&BTreeSet<i64>)->Option<i64>{
    children.iter().find(|t|ids.contains(&t.dependency_id)).map(|t|t.token_id)
}
fn participants(children:&[&NumericToken],lex:&NumericOperatorLexicon)
  ->Result<(Option<i64>,Option<i64>),NumericOperatorError>{
    let subj=["nsubj","nsubjpass","csubj"].into_iter()
        .map(|n|lex.symbol("dep",n)).collect::<Result<BTreeSet<_>,_>>()?;
    let obj=["obj","dobj","pobj","attr","oprd"].into_iter()
        .map(|n|lex.symbol("dep",n)).collect::<Result<BTreeSet<_>,_>>()?;
    Ok((matching_child(children,&subj),matching_child(children,&obj)))
}
pub fn compose_numeric_sentence_native(
    region:i64,tokens:&[NumericToken],lex:&NumericOperatorLexicon,
)->Result<NativeSentenceClosure,NumericOperatorError>{
    if region<=0 {return Err(NumericOperatorError::InvalidSpan);}
    lex.validate()?;
    let mut tokens_by_id=HashMap::<i64,&NumericToken>::new();
    let mut children=HashMap::<i64,Vec<&NumericToken>>::new();
    for token in tokens {
        if token.token_id<=0||token.start_char<0||token.end_char<token.start_char{
            return Err(NumericOperatorError::InvalidSpan);
        }
        if let Some(old)=tokens_by_id.insert(token.token_id,token){
            if old!=token{return Err(NumericOperatorError::ConflictingToken(token.token_id));}
        }
        children.entry(token.head_token_id).or_default().push(token);
    }
    for t in tokens {
        if !tokens_by_id.contains_key(&t.head_token_id){
            return Err(NumericOperatorError::MissingHead(t.token_id));
        }
    }
    let mut objects=HashMap::<i64,NativeObject>::new();
    let mut factors=Vec::new();
    let mut all_demands=Vec::new();
    let aux=BTreeSet::from([lex.symbol("dep","aux")?,lex.symbol("dep","auxpass")?]);
    let neg=BTreeSet::from([lex.symbol("lemma","not")?,lex.symbol("lemma","never")?]);
    let modal_rules=[("must",1i16,"normative.obligation"),("shall",1,"normative.obligation"),
                     ("may",2,"normative.permission_candidate")];
    for modal in tokens {
        if !aux.contains(&modal.dependency_id){continue;}
        let mut hit=None;
        for (name,state,pred) in modal_rules {
            if modal.lemma_id==lex.symbol("lemma",name)? {hit=Some((state,pred));break;}
        }
        let Some((modality,predicate_name))=hit else{continue};
        let head=tokens_by_id[&modal.head_token_id];
        let c=children.get(&head.token_id).map(Vec::as_slice).unwrap_or(&[]);
        let (subject,object_id)=participants(c,lex)?;
        let mut negation=None;
        for id in [head.token_id,modal.token_id]{
            if let Some(rows)=children.get(&id){
                if let Some(t)=rows.iter().find(|t|neg.contains(&t.lemma_id)){
                    negation=Some(t.token_id);break;
                }
            }
        }
        let state=if negation.is_some()&&modality==1{3}else{modality};
        let pred=lex.symbol("predicate",if state==3{"normative.prohibition"}else{predicate_name})?;
        let mut slots=vec![slot(lex.symbol("role","conduct")?,head.token_id)];
        for id in [Some(head.token_id),subject,object_id].into_iter().flatten(){
            if !objects.contains_key(&id){
                objects.insert(id,object(region,tokens_by_id[&id],lex)?);
            }
        }
        if let Some(id)=subject{slots.push(slot(lex.symbol("role","bearer")?,id));}
        if let Some(id)=object_id{slots.push(slot(lex.symbol("role","object")?,id));}
        let mut res=vec!["jurisdiction_unresolved","legal_time_unresolved",
                         "normative_scope_unresolved"];
        if modality==2{res.push("modal_sense_unresolved");}
        if subject.is_none(){res.push("norm_bearer_unresolved");}
        let residual=res.into_iter().map(|n|lex.symbol("residual",n))
            .collect::<Result<Vec<_>,_>>()?;
        let mut support=vec![modal.token_id,head.token_id];
        if let Some(id)=negation{support.push(id);}
        let f=factor(region,lex.symbol("factor","semantic.normative_relation")?,
            pred,state,0,slots,support,residual);
        all_demands.extend(demands(region,&f,head.lemma_id));factors.push(f);
    }
    let markers=BTreeSet::from([lex.symbol("dep","mark")?,lex.symbol("dep","prep")?,
                                lex.symbol("dep","advmod")?]);
    for marker in tokens {
        if !markers.contains(&marker.dependency_id){continue;}
        let exc=["unless","except","excluding"].iter()
            .any(|n|lex.symbol("lemma",n).ok()==Some(marker.lemma_id));
        let cond=["if","when","provided","providing"].iter()
            .any(|n|lex.symbol("lemma",n).ok()==Some(marker.lemma_id));
        if !cond&&!exc{continue;}
        let clause=tokens_by_id[&marker.head_token_id];
        let host=tokens_by_id.get(&clause.head_token_id).copied();
        let role=if exc{"exception"}else{"condition"};
        let typ=if exc{"semantic.legal_exception"}else{"semantic.legal_condition"};
        let pred=if exc{"legal.exception_candidate"}else{"legal.activation_condition_candidate"};
        let res=if exc{vec!["exception_attachment_unresolved","exception_burden_unresolved"]}
                else{vec!["condition_attachment_unresolved"]};
        let mut slots=vec![slot(lex.symbol("role",role)?,clause.token_id)];
        if !objects.contains_key(&clause.token_id){
            objects.insert(clause.token_id,object(region,clause,lex)?);
        }
        if let Some(t)=host{
            if !objects.contains_key(&t.token_id){
                objects.insert(t.token_id,object(region,t,lex)?);
            }
            slots.push(slot(lex.symbol("role","host")?,t.token_id));
        }
        let residual=res.into_iter().map(|n|lex.symbol("residual",n))
            .collect::<Result<Vec<_>,_>>()?;
        let f=factor(region,lex.symbol("factor",typ)?,lex.symbol("predicate",pred)?,
            0,0,slots,vec![marker.token_id,clause.token_id],residual);
        all_demands.extend(demands(region,&f,clause.lemma_id));factors.push(f);
    }
    let transitions=[("commence",1i16,2i16,"legal.commencement"),
      ("begin",1,2,"legal.commencement_candidate"),("repeal",2,3,"legal.repeal"),
      ("amend",4,5,"legal.amendment"),("cease",2,1,"legal.cessation")];
    let verbs=BTreeSet::from([lex.symbol("pos","VERB")?,lex.symbol("pos","AUX")?]);
    for pred in tokens{
        if !verbs.contains(&pred.pos_id){continue;}
        let mut hit=None;
        for (name,prior,next,predicate) in transitions{
            if pred.lemma_id==lex.symbol("lemma",name)?{hit=Some((prior,next,predicate));break;}
        }
        let Some((prior,next,predicate))=hit else{continue};
        let c=children.get(&pred.token_id).map(Vec::as_slice).unwrap_or(&[]);
        let (subject,object_id)=participants(c,lex)?;
        let legal_object=subject.or(object_id);
        let mut slots=vec![slot(lex.symbol("role","transition")?,pred.token_id)];
        if !objects.contains_key(&pred.token_id){
            objects.insert(pred.token_id,object(region,pred,lex)?);
        }
        if let Some(id)=legal_object{
            if !objects.contains_key(&id){
                objects.insert(id,object(region,tokens_by_id[&id],lex)?);
            }
            slots.push(slot(lex.symbol("role","legal_object")?,id));
        }
        let residual=["legal_object_identity_unresolved","effective_time_unresolved",
                      "jurisdiction_unresolved"].iter()
            .map(|n|lex.symbol("residual",n)).collect::<Result<Vec<_>,_>>()?;
        let f=factor(region,lex.symbol("factor","semantic.legal_transition")?,
            lex.symbol("predicate",predicate)?,0,(prior<<8)|next,
            slots,vec![pred.token_id],residual);
        all_demands.extend(demands(region,&f,pred.lemma_id));factors.push(f);
    }
    let factor_map=factors.into_iter().map(|f|(f.factor_digest,f))
        .collect::<BTreeMap<_,_>>();
    let demand_map=all_demands.into_iter().map(|d|(d.demand_digest,d))
        .collect::<BTreeMap<_,_>>();
    let edge_count=factor_map.values().map(|f|f.slots.len()).sum();
    let measure=NativeSentenceMeasure{
        node_count:objects.len()+factor_map.len(),edge_count,
        unresolved_count:demand_map.len(),boundary_demand_weight:demand_map.len() as f64,
        encoded_byte_count:tokens.len()*80+factor_map.len()*32+demand_map.len()*32,
        rule_count:3,closure_rounds:1,promoted_object_count:objects.len(),
        interface_cardinality:objects.len()+factor_map.len()+demand_map.len(),
    };
    let mut object_rows=objects.into_values().collect::<Vec<_>>();
    object_rows.sort_by_key(|o|o.object_digest);
    Ok(NativeSentenceClosure{
        objects:object_rows,factors:factor_map.into_values().collect(),
        demands:demand_map.into_values().collect(),measure,
    })
}

/// Fail-closed semantic integrity fence before *any* original SQL write.
pub fn validate_native_numeric_sentence_closure(
    region:i64, closure:&NativeSentenceClosure,
)->Result<(),NumericOperatorError>{
    if region<=0{return Err(NumericOperatorError::InvalidRegion);}
    let mut token_objects=BTreeSet::new();
    let mut object_digests=BTreeSet::new();
    for obj in &closure.objects{
        if obj.source_token_id<=0
            ||obj.object_kind_symbol_id<=0||obj.head_symbol_id<=0
            ||obj.object_digest!=strict_numeric_digest(vec![
                num(region),num(obj.source_token_id),num(obj.object_kind_symbol_id)
            ])
            ||obj.information_gain!=2.0
            ||obj.representation_cost!=1.0
            ||obj.ambiguity_cost!=0.5
            ||obj.factor_participation!=1
            ||!token_objects.insert(obj.source_token_id)
            ||!object_digests.insert(obj.object_digest){
            return Err(NumericOperatorError::InconsistentClosure);
        }
    }
    let mut factor_digests=BTreeSet::new();
    let mut factors_by_digest=BTreeMap::new();
    for f in &closure.factors{
        let slots=f.slots.iter().map(|s|NumericDigestValue::Seq(vec![
            num(s.role_symbol_id),num(s.source_token_id),
            num(i64::from(s.resolution_state)),NumericDigestValue::Bool(s.required)
        ])).collect::<Vec<_>>();
        let mut sorted=f.slots.clone();
        sorted.sort_by_key(|s|(s.role_symbol_id,s.source_token_id));
        if sorted!=f.slots
            ||f.support_token_ids.iter().any(|id|*id<=0)
            ||f.slots.iter().any(|s|!token_objects.contains(&s.source_token_id)
                  ||s.role_symbol_id<=0||s.resolution_state!=2||!s.required)
            ||f.support_token_ids.windows(2).any(|w|w[0]>=w[1])
            ||f.residual_symbol_ids.windows(2).any(|w|w[0]>=w[1])
            ||f.residual_symbol_ids.iter().any(|id|*id<=0)
            ||f.support_score!=1.0
            ||f.factor_digest!=strict_numeric_digest(vec![
                num(region),num(f.factor_type_symbol_id),
                num(f.predicate_symbol_id),num(i64::from(f.modal_state)),
                num(i64::from(f.temporal_state)),
                NumericDigestValue::Seq(slots),nums(&f.support_token_ids),
                nums(&f.residual_symbol_ids)
            ])
            ||!factor_digests.insert(f.factor_digest){
            return Err(NumericOperatorError::InconsistentClosure);
        }
        factors_by_digest.insert(f.factor_digest,f);
    }
    let mut demand_digests=BTreeSet::new();
    for d in &closure.demands{
        if d.expected_target_kind!=2||d.recency_class!=3||d.max_candidates!=16
            ||d.expected_object_kind_symbol_id.is_some()
            ||d.role_symbol_id.is_some(){
            return Err(NumericOperatorError::InconsistentClosure);
        }
        let matched=factors_by_digest.iter().any(|(factor_digest,f)|
            d.expected_factor_type_symbol_id==Some(f.factor_type_symbol_id)
            &&f.residual_symbol_ids.contains(&d.residual_type_symbol_id)
            &&d.lexical_symbol_id.is_some_and(|lemma|
                d.demand_digest==strict_numeric_digest(vec![
                    num(region),NumericDigestValue::Bytes(factor_digest.to_vec()),
                    num(d.residual_type_symbol_id),num(lemma)
                ])));
        if !matched||!demand_digests.insert(d.demand_digest){
            return Err(NumericOperatorError::InconsistentClosure);
        }
    }
    let m=&closure.measure;
    if m.node_count!=closure.objects.len()+closure.factors.len()
        ||m.edge_count!=closure.factors.iter().map(|f|f.slots.len()).sum::<usize>()
        ||m.unresolved_count!=closure.demands.len()
        ||m.boundary_demand_weight!=closure.demands.len() as f64
        ||m.rule_count!=3||m.closure_rounds!=1
        ||m.promoted_object_count!=closure.objects.len()
        ||m.interface_cardinality!=
            closure.objects.len()+closure.factors.len()+closure.demands.len(){
        return Err(NumericOperatorError::InconsistentClosure);
    }
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(bytes:&[u8])->String{bytes.iter().map(|x|format!("{x:02x}")).collect()}
    #[test]
    fn strict_python_numeric_digest_goldens(){
        assert_eq!(hex(&strict_numeric_digest(vec![num(1),num(2),num(3)])),
          "af3b9c4c815b3c691e990fe46233ef5f112c301c22a18dc2f147fdc1fd3873a6");
        assert_eq!(hex(&strict_numeric_digest(vec![num(1),
            NumericDigestValue::Bytes(b"abc".to_vec()),num(-4)])),
          "51a1d25a79349115337a3e32247922d1c75bfbb362a393f642b43659fa7bbcf9");
        assert_eq!(hex(&strict_numeric_digest(vec![num(1),num(2),num(3),
            NumericDigestValue::Seq(vec![num(1),NumericDigestValue::Bool(true)])])),
          "677195cdcbaa5cf30abd011118b77292437e4007930b9716446962dceee71eb5");
    }
}