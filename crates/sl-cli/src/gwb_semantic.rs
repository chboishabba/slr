//! Native production-side bounded semantic projection and golden parity.
//! No Python semantic dependency, arbitrary function execution or proof issuance.
use serde_json::{json, Value};
use sha2::{Digest,Sha256};
use std::{fs,io::{Read},path::{Path,PathBuf},process::Command,collections::{BTreeSet,BTreeMap}};
type R<T>=Result<T,String>;
fn require(ok:bool,why:&str)->R<()> {if ok{Ok(())}else{Err(why.into())}}
fn raw(path:&Path,limit:u64)->R<Vec<u8>>{
    let f=fs::File::open(path).map_err(|e|e.to_string())?;
    let mut b=Vec::new(); f.take(limit+1).read_to_end(&mut b).map_err(|e|e.to_string())?;
    require(b.len() as u64<=limit,"input exceeds bound")?;Ok(b)
}
fn read_json(path:&Path,limit:u64)->R<Value>{serde_json::from_slice(&raw(path,limit)?).map_err(|e|e.to_string())}
fn sha(b:&[u8])->String{format!("{:x}",Sha256::digest(b))}
fn string<'a>(v:&'a Value,k:&str)->R<&'a str>{v[k].as_str().filter(|s|!s.is_empty()).ok_or_else(||format!("missing string {k}"))}
fn array<'a>(v:&'a Value,k:&str)->R<&'a Vec<Value>>{v[k].as_array().ok_or_else(||format!("missing array {k}"))}
fn zid(s:&str)->bool{s.strip_prefix('Z').is_some_and(|v|!v.is_empty()&&!v.starts_with('0')&&v.bytes().all(|c|c.is_ascii_digit()))}
fn qid(s:&str)->bool{s.strip_prefix('Q').is_some_and(|v|!v.is_empty()&&!v.starts_with('0')&&v.bytes().all(|c|c.is_ascii_digit()))}
fn reference(v:&Value)->Option<&str>{
    v.as_str().or_else(||if v["Z1K1"]=="Z9"{v["Z9K1"].as_str()}else{None})
}
fn entity(v:&Value)->R<&str>{
    require(v["Z1K1"]=="Z6091","not an explicit entity constructor")?;
    let s=string(v,"Z6091K1")?;require(qid(s),"bad QID")?;Ok(s)
}
fn entities(v:&Value)->R<Vec<&str>>{
    let a=v.as_array().ok_or("not a typed entity list")?;
    require(a.first().and_then(Value::as_str)==Some("Z6091"),"wrong entity list tag")?;
    a[1..].iter().map(entity).collect()
}
fn call_edges(v:&Value,p:&str,out:&mut Vec<Value>)->R<()>{
    if let Some(o)=v.as_object(){
        if v["Z1K1"]=="Z7"{
            let f=string(v,"Z7K1")?;require(zid(f),"invalid function target")?;
            let args=o.iter().filter(|(k,_)|*k!="Z1K1"&&*k!="Z7K1").collect::<BTreeMap<_,_>>();
            require(args.keys().all(|k|k.strip_prefix(&format!("{f}K")).is_some_and(|n|!n.is_empty()&&n.bytes().all(|b|b.is_ascii_digit()))),"foreign call argument")?;
            out.push(json!({"pointer":p,"function":f,"arguments":args}));
        }
        for (k,x) in o {call_edges(x,&format!("{p}/{}",k.replace('~',"~0").replace('/',"~1")),out)?;}
    }else if let Some(a)=v.as_array(){for (i,x) in a.iter().enumerate(){call_edges(x,&format!("{p}/{i}"),out)?;}}
    Ok(())
}
/// Only exact historical object payloads are reopened; the manifest cannot
/// license execution. Function signatures are parsed independently in Rust.
fn definition(root:&Path,row:&Value,id:&str,as_of:&str)->R<Value>{
    require(row["function"]==id,"foreign definition")?;
    let p=string(row,"path")?;
    require(Path::new(p).file_name().and_then(|s|s.to_str())==Some(p),"unsafe definition path")?;
    require(string(row,"timestamp")?<=as_of,"future definition")?;
    require(row["revision"].as_u64().is_some_and(|n|n>0),"invalid revision")?;
    let path=root.join(p);
    require(path.canonicalize().map_err(|e|e.to_string())?.parent()==Some(root),"definition escapes root")?;
    let b=raw(&path,8<<20)?; require(sha(&b)==string(row,"sha256")?,"definition digest mismatch")?;
    let obj:Value=serde_json::from_slice(&b).map_err(|e|e.to_string())?;
    require(reference(&obj["Z1K1"])==Some("Z2")&&reference(&obj["Z2K1"])==Some(id),"wrong persistent object")?;
    let body=&obj["Z2K2"];
    require(reference(&body["Z1K1"])==Some("Z8"),"not a function")?;
    let args=array(body,"Z8K1")?;
    let mut keys=BTreeMap::new();
    let start=usize::from(args.first().and_then(reference)==Some("Z17")&&args.first().is_some_and(|v|v.is_string()||v["Z1K1"]=="Z9"));
    for arg in &args[start..]{
        require(reference(&arg["Z1K1"])==Some("Z17"),"unknown signature representation")?;
        let key=arg["Z17K2"].as_str().or_else(||arg["Z17K2"]["Z6K1"].as_str()).ok_or("missing argument key")?;
        require(key.starts_with(&format!("{id}K"))&&!arg["Z17K1"].is_null(),"invalid argument declaration")?;
        require(keys.insert(key.to_owned(),arg["Z17K1"].clone()).is_none(),"duplicate signature key")?;
    }
    require(!body["Z8K2"].is_null(),"return type missing")?;
    Ok(json!({"function":id,"revision":row["revision"],"sha256":row["sha256"],"argument_types":keys,
        "return_type":body["Z8K2"],"tester_refs":body["Z8K3"],"implementation_refs":body["Z8K4"]}))
}
fn abstract_source(source:&Path,receipt:&Path,definitions:Option<&Path>)->R<Value>{
    let b=raw(source,8<<20)?;let rec=read_json(receipt,1<<20)?;
    require(rec["schema"]=="slr.abstract-wikipedia-constructor-acquisition.v1"&&rec["qid"]=="Q207"
        &&rec["sha256"]==sha(&b)&&rec["byte_length"].as_u64()==Some(b.len() as u64),"source receipt mismatch")?;
    let ast:Value=serde_json::from_slice(&b).map_err(|e|e.to_string())?;
    require(ast["qid"]=="Q207","wrong subject")?;
    let mut calls=Vec::new();call_edges(&ast,"",&mut calls)?;
    require(!calls.is_empty()&&calls.len()<=128,"invalid call universe")?;
    let mut coordinate=None;let mut held=Vec::new();let mut checked=Vec::new();
    let bundle=if let Some(p)=definitions{
        let root=p.canonicalize().map_err(|e|e.to_string())?;
        let m=read_json(&root.join("manifest.json"),8<<20)?;
        require(m["schema"]=="slr.abstract-definition-acquisition.v1"&&m["source_sha256"]==rec["sha256"]&&m["as_of"]==rec["timestamp"],"foreign historical manifest")?;
        Some((root,m))
    }else{None};
    for c in &calls{
        let f=string(c,"function")?;
        let args=&c["arguments"];
        if f=="Z38757"{
            require(coordinate.is_none(),"ambiguous occupation projection")?;
            let subject=entity(&args["Z38757K1"])?;
            require(subject=="Q207","foreign occupation subject")?;
            coordinate=Some(json!({"subject":subject,"countries":entities(&args["Z38757K2"])?,
                "occupations":entities(&args["Z38757K3"])?,"source_pointer":c["pointer"]}));
        }
        match &bundle{
            Some((root,m))=>match definition(root,&m["definitions"][f],f,string(&rec,"timestamp")?){
                Ok(sig)=>{
                    let actual=args.as_object().ok_or("call arguments not object")?.keys().collect::<BTreeSet<_>>();
                    let expected=sig["argument_types"].as_object().ok_or("signature not object")?.keys().collect::<BTreeSet<_>>();
                    if actual==expected {checked.push(sig);} else {held.push(json!({"function":f,"reason":"argument_key_mismatch"}));}
                },
                Err(e)=>held.push(json!({"function":f,"reason":e})),
            },
            None=>held.push(json!({"function":f,"reason":"historical_definition_required"})),
        }
    }
    // Explicit coordinates are observed syntax, not a completed runtime
    // interpretation. Code, dynamic entity reads, and type instantiation hold.
    Ok(json!({"schema":"slr.native-abstract-consumer.v1","source_sha256":rec["sha256"],
        "source_revision_ref":rec["source_revision_ref"],"calls":calls,"explicit_occupation_coordinate":coordinate,
        "checked_signature_keys":checked,"dependency_holds":held,
        "office_year_support":null,"office_day_support":null,
        "candidate_only":true,"runtime_interpretation_paid":false,"type_check_paid":false,
        "semantic_authority_paid":false,"world_truth_paid":false}))
}

/// Preserve a single original statement and its complete qualifiers. Never
/// join dates/ordinal across different statements or infer negative facts.
fn exact_qualifier<'a>(row:&'a Value,property:&str,datatype:&str)->Option<&'a Value>{
    let values=row["qualifiers"][property].as_array()?;
    if values.len()!=1{return None;}
    let s=&values[0];
    if s["property"]!=property||s["snaktype"]!="value"||s["datatype"]!=datatype{return None;}
    Some(&s["datavalue"])
}
fn exact_date(row:&Value,property:&str,time:&str)->bool{
    let Some(d)=exact_qualifier(row,property,"time") else{return false;};
    let v=&d["value"];
    d["type"]=="time"&&v["time"]==time&&v["precision"]==11&&v["timezone"]==0&&
        v["before"]==0&&v["after"]==0&&v["calendarmodel"]=="http://www.wikidata.org/entity/Q1985727"
}
fn office_row(row:&Value)->bool{
    let s=&row["mainsnak"];
    matches!(row["rank"].as_str(),Some("normal"|"preferred"))&&
        row["id"].as_str().is_some_and(|id|id.to_ascii_lowercase().starts_with("q207$")&&id.len()>5)&&
        s["property"]=="P39"&&s["snaktype"]=="value"&&s["datatype"]=="wikibase-item"&&
        s["datavalue"]["type"]=="wikibase-entityid"&&s["datavalue"]["value"]["id"]=="Q11696"&&
        s["datavalue"]["value"]["entity-type"]=="item"&&s["datavalue"]["value"]["numeric-id"]==11696&&
        exact_qualifier(row,"P1545","string").is_some_and(|d|d["type"]=="string"&&d["value"]=="43")&&
        exact_date(row,"P580","+2001-01-20T00:00:00Z")&&exact_date(row,"P582","+2009-01-20T00:00:00Z")
}
fn entity_office(bytes:&[u8],expected_sha:&str,revision:u64)->R<Value>{
    require(expected_sha.len()==64&&expected_sha.bytes().all(|c|c.is_ascii_digit()||(b'a'..=b'f').contains(&c))&&sha(bytes)==expected_sha,"entity source digest mismatch")?;
    let original:Value=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
    let e=&original["entities"]["Q207"];
    require(e["id"]=="Q207"&&e["lastrevid"].as_u64()==Some(revision)&&revision>0,"entity revision mismatch")?;
    let rows=array(&e["claims"],"P39")?;
    let matches:Vec<_>=rows.iter().enumerate().filter(|(_,r)|office_row(r)).collect();
    let witness=if matches.len()==1{
        let (i,row)=matches[0];
        json!({"pointer":format!("/entities/Q207/claims/P39/{i}"),"statement":row,
            "statement_ref":row["id"],"ordinal":43,"start":"+2001-01-20T00:00:00Z","end":"+2009-01-20T00:00:00Z"})
    }else{Value::Null};
    Ok(json!({"schema":"slr.native-entity-office-dependency.v1","qid":"Q207",
        "property":"P39","office":"Q11696","entity_revision":revision,"source_sha256":expected_sha,
        "matching_statement_count":matches.len(),"witness":witness,
        "year_support":if matches.len()==1{json!({"positive":true,"negative":false})}else{Value::Null},
        "day_support":if matches.len()==1{json!({"positive":true,"negative":false})}else{Value::Null},
        "candidate_only":true,"abstract_runtime_lookup_paid":false,"source_independence_paid":false,
        "world_truth_paid":false}))
}
fn dependency_join(source:&Path,receipt:&Path,definitions:&Path,entity_path:&Path,entity_sha:&str,revision:u64)->R<Value>{
    let abstract_view=abstract_source(source,receipt,Some(definitions))?;
    let kb=entity_office(&raw(entity_path,32<<20)?,entity_sha,revision)?;
    require(abstract_view["explicit_occupation_coordinate"]["subject"]==kb["qid"],"foreign dependency subject")?;
    // This join identifies a discriminator candidate only. In particular the
    // later EntityData revision may not be the historical function's lookup.
    Ok(json!({"schema":"slr.native-abstract-office-dependency-join.v1","abstract_view":abstract_view,
        "entity_dependency":kb,"join_scope":"explicit_subject_coordinate_only",
        "abstract_year_support":null,"abstract_day_support":null,
        "required_welds":["historical_function_implementation_selection","exact_dynamic_entity_revision",
            "implementation_to_query_factorisation","independent_golden_execution"],
        "candidate_only":true,"runtime_interpretation_paid":false,"world_truth_paid":false}))
}

fn lead(e:&str)->R<(&'static str,usize,usize,&'static str)>{
    match e{
        "en"=>Ok(("George Walker Bush (born July 6, 1946) is an American retired politician and businessman who served as the 43rd president of the United States from 2001 to 2009.",93,99,"served")),
        "simple"=>Ok(("George Walker Bush (born July 6, 1946) is an American politician and businessman. He was the 43rd president of the United States from 2001 to 2009.",85,88,"was")),
        "es"=>Ok(("George Walker Bush (pronunciación en inglés: /bʊʃ/ (); New Haven, Connecticut, 6 de julio de 1946), es un político y empresario estadounidense que ejerció como el 43.er presidente de los Estados Unidos entre el 20 de enero de 2001 y el 20 de enero de 2009.",147,154,"ejerció")),
        _=>Err("unsupported edition".into())
    }
}
fn evaluated(e:&str,bytes:&[u8],query:&str)->R<(Value,Value)>{
    let (prefix,a,b,p)=lead(e)?;
    let text=std::str::from_utf8(bytes).map_err(|e|e.to_string())?;
    require(text.starts_with(prefix)&&text.chars().skip(a).take(b-a).collect::<String>()==p,"bounded lead or predicate mismatch")?;
    let sign=match query{
        "query:gwb:office-years:v1"=>json!({"positive":true,"negative":false}),
        "query:gwb:office-days:v1" if e=="es"=>json!({"positive":true,"negative":false}),
        "query:gwb:office-days:v1"=>Value::Null,
        _=>return Err("unknown typed consumer".into())
    };
    Ok((json!({"edition":e,"source_sha256":sha(bytes),"predicate_span":[a,b],"predicate":p}),sign))
}
fn verify_observation(family:&Value,selector:&str,coordinate:&Value)->R<Value>{
    let sources=array(&family["projection"],"sources")?;
    let matches=sources.iter().filter(|x|x["selector_ref"]==selector).collect::<Vec<_>>();
    require(matches.len()==1,"missing or ambiguous native selector")?;
    let o=&matches[0]["observation"];
    let digest=format!("sha256:{}",string(coordinate,"source_sha256")?);
    let span=array(coordinate,"predicate_span")?;
    let marker=format!("syntax:predicate-span:{}:{}",span[0],span[1]);
    let parser=string(o,"parser_or_producer_ref")?;
    require(o["candidate_only"]==true&&o["creates_semantic_authority"]==false&&o["claim_truth_promoted"]==false
        &&parser.starts_with("db-parser:")&&array(o,"provenance_refs")?.iter().any(|v|v.as_str()==Some(digest.as_str()))
        &&array(o,"native_metadata_refs")?.iter().any(|v|v.as_str()==Some(marker.as_str())),"native observation not attached to exact source occurrence")?;
    Ok(o.clone())
}
fn exec(cmd:&mut Command)->R<Vec<u8>>{
    let out=cmd.output().map_err(|e|e.to_string())?;
    require(out.status.success(),&format!("subprocess held: {}",String::from_utf8_lossy(&out.stderr)))?;Ok(out.stdout)
}
fn check_checkout(root:&Path,commit:&str)->R<()>{
    require(commit.len()==40&&commit.bytes().all(|x|x.is_ascii_hexdigit()&&!x.is_ascii_uppercase()),"exact golden commit required")?;
    let got=exec(Command::new("git").arg("-C").arg(root).args(["rev-parse","HEAD"]))?;
    require(String::from_utf8_lossy(&got).trim()==commit,"golden commit mismatch")?;
    let dirty=exec(Command::new("git").arg("-C").arg(root).args(["status","--porcelain","--untracked-files=no"]))?;
    require(dirty.is_empty(),"tracked golden checkout modified")
}
fn verify_pair(args:&[String])->R<Value>{
    let [family_ref,left,right,query,statement,evidence,le,lp,re,rp,lean_root,commit]=args else{
        return Err("verify-pair FAMILY LEFT RIGHT QUERY STATEMENT_DIGEST EVIDENCE_REF LEFT_EDITION LEFT_SOURCE RIGHT_EDITION RIGHT_SOURCE LEAN_ROOT LEAN_COMMIT".into())};
    require(left!=right&&le!=re,"distinct sources required")?;
    let lp=PathBuf::from(lp).canonicalize().map_err(|e|e.to_string())?;
    let rp=PathBuf::from(rp).canonicalize().map_err(|e|e.to_string())?;
    let root=PathBuf::from(lean_root).canonicalize().map_err(|e|e.to_string())?;
    let a=raw(&lp,32<<20)?;let b=raw(&rp,32<<20)?;
    let (lc,ls)=evaluated(le,&a,query)?;let (rc,rs)=evaluated(re,&b,query)?;
    let config=sensiblaw_pg_source_store::load_database_config(None).map_err(|e|e.to_string())?;
    let get_family=||->R<Value>{let f=sensiblaw_pg_source_store::inspect_persisted_relational_family(&config,family_ref).map_err(|e|e.to_string())?;
        serde_json::to_value(f).map_err(|e|e.to_string())};
    let family=get_family()?;require(family["superseded_by"].is_null(),"superseded family")?;
    let lo=verify_observation(&family,left,&lc)?;let ro=verify_observation(&family,right,&rc)?;
    require(lo["source_revision_ref"]!=ro["source_revision_ref"],"same source revision")?;
    let answer=json!({"schema":"slr.signed-view-query-answer.v1","query_ref":query,"left_support":ls,"right_support":rs});
    check_checkout(&root,commit)?;
    let runner=root.join("tools/GwbGoldenPairRunner.lean");
    let theorem=root.join("DASHI/output-final_aristotle/RequestProject/Slr/GwbMultilingualOffice.lean");
    let runner_bytes=raw(&runner,1<<20)?;let theorem_bytes=raw(&theorem,1<<20)?;
    let out=exec(Command::new("timeout").args(["--kill-after=5s","90s","lake","env","lean","--run"])
        .arg(&runner).arg(le).arg(&lp).arg(re).arg(&rp).arg(query).current_dir(&root))?;
    let golden:Value=serde_json::from_slice(&out).map_err(|e|e.to_string())?;
    require(golden["schema"]=="slr.gwb-golden-pair-evaluation.v1"&&golden["theorem"]=="Wikidata.Slr.GwbMultilingualOffice.exactPairAnswer"
        &&golden["query_ref"].as_str()==Some(query.as_str())&&golden["left"]==lc&&golden["right"]==rc&&golden["answer"]==answer
        &&golden["bounded_rule_only"]==true&&golden["world_truth_paid"]==false&&golden["referee_authority_paid"]==false,"golden/native answer mismatch")?;
    check_checkout(&root,commit)?;
    require(raw(&lp,32<<20)?==a&&raw(&rp,32<<20)?==b&&raw(&runner,1<<20)?==runner_bytes&&raw(&theorem,1<<20)?==theorem_bytes,"input changed during verification")?;
    let binding=sensiblaw_pg_source_store::prepare_reciprocal_referee_binding(&config,family_ref,left,right,query,statement,evidence,&answer).map_err(|e|e.to_string())?;
    let fresh=get_family()?;require(fresh==family&&verify_observation(&fresh,left,&lc)?==lo&&verify_observation(&fresh,right,&rc)?==ro,"family changed during parity check")?;
    Ok(json!({"schema":"slr.native-golden-pair-parity.v1","family_ref":family_ref,
        "golden_commit":commit,"runner_sha256":sha(&runner_bytes),"theorem_source_sha256":sha(&theorem_bytes),
        "golden_evaluation":golden,"native_binding_candidate":binding,
        "native_golden_bounded_answer_equal":true,"candidate_only":true,
        "isolated_kernel_certification_paid":false,"statement_digest_verified":false,
        "referee_authority_paid":false,"world_truth_paid":false,"writes_database":false}))
}
pub fn run(args:&[String])->R<()>{
    let result=match args{
        [mode,source,receipt] if mode=="inspect-abstract"=>abstract_source(Path::new(source),Path::new(receipt),None)?,
        [mode,source,receipt,definitions] if mode=="inspect-abstract"=>abstract_source(Path::new(source),Path::new(receipt),Some(Path::new(definitions)))?,
        [mode,entity,digest,revision] if mode=="inspect-entity-office"=>
            entity_office(&raw(Path::new(entity),32<<20)?,digest,revision.parse::<u64>().map_err(|e|e.to_string())?)?,
        [mode,source,receipt,definitions,entity,digest,revision] if mode=="inspect-dependency-join"=>
            dependency_join(Path::new(source),Path::new(receipt),Path::new(definitions),Path::new(entity),digest,revision.parse::<u64>().map_err(|e|e.to_string())?)?,
        [mode,rest @ ..] if mode=="verify-pair"=>verify_pair(rest)?,
        _=>return Err("gwb-semantic inspect-abstract SOURCE RECEIPT [DEFINITIONS_DIR] | verify-pair FAMILY LEFT RIGHT QUERY STATEMENT_DIGEST EVIDENCE_REF LEFT_EDITION LEFT_SOURCE RIGHT_EDITION RIGHT_SOURCE LEAN_ROOT LEAN_COMMIT".into())
    };
    println!("{}",serde_json::to_string_pretty(&result).map_err(|e|e.to_string())?);Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn bounded_pair_and_day_loss(){
        for e in ["en","simple","es"]{
            let (s,_,_,_)=lead(e).unwrap();
            let (_,y)=evaluated(e,s.as_bytes(),"query:gwb:office-years:v1").unwrap();
            let (_,d)=evaluated(e,s.as_bytes(),"query:gwb:office-days:v1").unwrap();
            assert_eq!(y["positive"],true);assert_eq!(d.is_null(),e!="es");
            assert!(evaluated(e,b"unrelated","query:gwb:office-years:v1").is_err());
        }
    }
    #[test] fn original_entity_office_exact_and_mutations(){
        let bytes=include_bytes!("../../../artifacts/gwb/multiview-native/acquired/Q207-EntityData.json");
        let found=entity_office(bytes,&sha(bytes),2575571605).unwrap();
        assert_eq!(found["matching_statement_count"],1);
        assert_eq!(found["witness"]["statement_ref"],"q207$B2345BA9-CE6F-4636-A7B1-38210CEBDFD7");
        let row=found["witness"]["statement"].clone();
        assert!(office_row(&row));
        for p in ["P580","P582","P1545"]{
            let mut changed=row.clone();changed["qualifiers"][p]=json!([]);
            assert!(!office_row(&changed));
        }
        let mut bad=row.clone();bad["rank"]=json!("deprecated");assert!(!office_row(&bad));
        let mut duplicate=row.clone();let extra=duplicate["qualifiers"]["P580"][0].clone();
        duplicate["qualifiers"]["P580"].as_array_mut().unwrap().push(extra);
        assert!(!office_row(&duplicate));
        assert!(entity_office(bytes,&"0".repeat(64),2575571605).is_err());
        assert!(entity_office(bytes,&sha(bytes),2575571606).is_err());
    }
    #[test] fn typed_entity_list_rejects_wrong_tag(){
        assert!(entities(&json!(["Z6",{"Z1K1":"Z6091","Z6091K1":"Q30"}])).is_err());
        assert_eq!(entities(&json!(["Z6091",{"Z1K1":"Z6091","Z6091K1":"Q30"}])).unwrap(),vec!["Q30"]);
    }
    #[test] fn rejects_foreign_function_argument(){
        assert!(call_edges(&json!({"Z1K1":"Z7","Z7K1":"Z38757","Z35370K1":"Q207"}),"",&mut Vec::new()).is_err());
    }
}
