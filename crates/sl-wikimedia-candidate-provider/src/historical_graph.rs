//! Native bounded dependency worklist and symbolic composition. No runtime
//! implementation selection, dynamic Wikidata lookup or proof authority.
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{collections::{BTreeMap,BTreeSet},fs,io::{Read,Write},path::{Path,PathBuf}};
use crate::{fetch_historical_zobject,ProviderError};
type R<T>=Result<T,ProviderError>;
fn err(s:&str)->ProviderError{ProviderError::InvalidInput(s.into())}
fn req(b:bool,s:&str)->R<()>{if b{Ok(())}else{Err(err(s))}}
fn text<'a>(v:&'a Value,k:&str)->R<&'a str>{v[k].as_str().filter(|s|!s.is_empty()).ok_or_else(||err(&format!("missing {k}")))}
fn zid(s:&str)->bool{s.strip_prefix('Z').is_some_and(|x|!x.is_empty()&&!x.starts_with('0')&&x.bytes().all(|b|b.is_ascii_digit()))}
fn reference(v:&Value)->Option<&str>{v.as_str().or_else(||match v["Z1K1"].as_str(){
    Some("Z9")=>v["Z9K1"].as_str(),Some("Z6")=>v["Z6K1"].as_str(),_=>None})}
fn digest(b:&[u8])->String{format!("{:x}",Sha256::digest(b))}
fn read(p:&Path)->R<Vec<u8>>{let mut b=vec![];fs::File::open(p)?.take((8<<20)+1).read_to_end(&mut b)?;req(b.len()<=8<<20,"object too large")?;Ok(b)}
fn load(p:&Path)->R<Value>{serde_json::from_slice(&read(p)?).map_err(|_|err("invalid JSON"))}
fn write_new(p:&Path,b:&[u8])->R<()>{
    if p.exists(){return req(read(p)?==b,"immutable cache collision");}
    let mut f=fs::OpenOptions::new().write(true).create_new(true).open(p)?;
    f.write_all(b)?;f.sync_all()?;Ok(())
}
fn pointer(p:&str,k:&str)->String{format!("{p}/{}",k.replace('~',"~0").replace('/',"~1"))}
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
struct Edge{priority:u8,target:String,pointer:String,kind:&'static str}
fn push(out:&mut BTreeSet<Edge>,v:&Value,p:String,kind:&'static str,priority:u8){
    if let Some(z)=reference(v).filter(|s|zid(s)){out.insert(Edge{priority,target:z.into(),pointer:p,kind});}
}
fn walk(v:&Value,p:&str,out:&mut BTreeSet<Edge>){
    if let Some(o)=v.as_object(){
        let typ=reference(&v["Z1K1"]);
        if typ==Some("Z9"){push(out,v,p.into(),"explicit_reference",3);}
        for (k,x) in o{
            let q=pointer(p,k);
            match (typ,k.as_str()){
                (Some("Z8"),"Z8K4")|(Some("Z8"),"Z8K3")=>{
                    if let Some(items)=x.as_array(){
                        let (kind,priority)=if k=="Z8K4"{("implementation",0)}else{("tester",4)};
                        for (i,item) in items.iter().enumerate().skip(1){push(out,item,pointer(&q,&i.to_string()),kind,priority);}
                    }
                },
                (Some("Z7"),"Z7K1")=>push(out,x,q.clone(),"call_target",1),
                (Some("Z14"),"Z14K1")=>push(out,x,q.clone(),"implementation_owner",2),
                (Some("Z20"),"Z20K1")=>push(out,x,q.clone(),"tester_owner",2),
                (Some("Z17"),"Z17K1")|(Some("Z8"),"Z8K2")=>push(out,x,q.clone(),"signature_type",3),
                (_,"Z1K1")=>push(out,x,q.clone(),"object_type",5),
                _=>{},
            }
            if !matches!(k.as_str(),"Z17K3"|"Z14K3"|"Z14K4"){walk(x,&q,out);}
        }
    }else if let Some(a)=v.as_array(){
        if let Some(head)=a.first(){push(out,head,pointer(p,"0"),"typed_list_head",5);}
        for (i,x) in a.iter().enumerate(){if x.is_object()||x.is_array(){walk(x,&pointer(p,&i.to_string()),out);}}
    }
}
fn edges(body:&Value)->BTreeSet<Edge>{let mut e=BTreeSet::new();walk(body,"/Z2K2",&mut e);e}
fn validate_object(root:&Path,row:&Value,as_of:&str)->R<Value>{
    let id=text(row,"function")?;
    req(zid(id)&&row["candidate_only"]==true&&row["as_of"]==as_of&&text(row,"timestamp")?<=as_of&&row["revision"].as_u64().is_some_and(|n|n>0),"foreign historical receipt")?;
    let path=root.join(text(row,"path")?).canonicalize()?;
    req(path.starts_with(root),"object path escapes root")?;
    let b=read(&path)?;
    req(row["byte_length"].as_u64()==Some(b.len() as u64)&&text(row,"sha256")?==digest(&b),"object bytes differ from receipt")?;
    let obj:Value=serde_json::from_slice(&b).map_err(|_|err("invalid Z object"))?;
    req(reference(&obj["Z1K1"])==Some("Z2")&&reference(&obj["Z2K1"])==Some(id),"foreign persistent object")?;
    Ok(obj["Z2K2"].clone())
}
/// Checkpoint is the immutable receipt/object pair. At most one network
/// acquisition per invocation; failed requests remain explicit debt.
pub fn resolve_historical_graph(repo:&Path,manifest:&Path,cache:&Path,new_limit:usize,max_depth:usize,max_nodes:usize)->R<Value>{
    resolve_historical_graph_targeted(repo,manifest,cache,new_limit,max_depth,max_nodes,None)
}
/// An explicit reachable dependency may be selected after an earlier
/// failed request. This does not grant permission to fetch an unrelated ZID.
pub fn resolve_historical_graph_targeted(repo:&Path,manifest:&Path,cache:&Path,new_limit:usize,
    max_depth:usize,max_nodes:usize,target:Option<&str>)->R<Value>{
    req(new_limit<=1&&max_depth<=16&&(3..=512).contains(&max_nodes),"invalid work bounds")?;
    req(target.map_or(true,|z|zid(z)&&new_limit==1),"explicit target requires one bounded request")?;
    let repo=repo.canonicalize()?;fs::create_dir_all(cache)?;let cache=cache.canonicalize()?;
    let m=load(manifest)?;
    req(m["schema"]=="slr.gwb-multiview-native-acquisition.v1"&&m["qid"]=="Q207","wrong seed manifest")?;
    let roots=m["historical_wikifunctions"].as_array().ok_or_else(||err("missing historical roots"))?;
    let as_of="2026-10-15T23:37:24Z";
    let mut rows=BTreeMap::<String,(Value,PathBuf)>::new();
    for row in roots{
        let id=text(row,"function")?.to_owned();validate_object(&repo,row,as_of)?;
        req(rows.insert(id,(row.clone(),repo.clone())).is_none(),"duplicate root receipt")?;
    }
    req(rows.keys().map(String::as_str).collect::<Vec<_>>()==vec!["Z35370","Z38053","Z38757"],"wrong root universe")?;
    let mut queue:BTreeSet<(u8,usize,String)>=rows.keys().map(|z|(0,0,z.clone())).collect();
    for entry in fs::read_dir(&cache)?{
        let path=entry?.path();
        if !path.file_name().and_then(|s|s.to_str()).is_some_and(|s|s.ends_with(".receipt.json")){continue;}
        let row=load(&path)?;validate_object(&cache,&row,as_of)?;
        let id=text(&row,"function")?.to_owned();
        req(!rows.contains_key(&id),"ambiguous cached object receipt")?;
        rows.insert(id,(row,cache.clone()));
    }
    let mut seen=BTreeSet::new();let mut pending=Vec::new();let mut nodes=BTreeMap::new();let mut links=Vec::new();let mut attempts=0;
    while let Some((priority,depth,id))=queue.pop_first(){
        if seen.contains(&id){continue;}
        if seen.len()>=max_nodes||depth>max_depth{pending.push(json!({"target":id,"depth":depth,"priority":priority,"reason":"bound"}));continue;}
        seen.insert(id.clone());
        if !rows.contains_key(&id)&&attempts<new_limit&&target.map_or(true,|z|z==id.as_str()){
            attempts+=1;
            match fetch_historical_zobject(&id,as_of){
                Ok(mut a)=>{
                    let name=format!("{}.{}.{}.json",a.object_id,a.revision,a.sha256);
                    write_new(&cache.join(&name),&a.bytes)?;
                    a.receipt["path"]=json!(name);
                    let b=serde_json::to_vec_pretty(&a.receipt).map_err(|_|err("receipt encoding"))?;
                    write_new(&cache.join(format!("{id}.receipt.json")),&b)?;
                    rows.insert(id.clone(),(a.receipt,cache.clone()));
                },
                Err(e)=>{pending.push(json!({"target":id,"depth":depth,"priority":priority,"reason":"acquisition_held","detail":e.to_string()}));continue;}
            }
        }
        let Some((row,dir))=rows.get(&id)else{pending.push(json!({"target":id,"depth":depth,"priority":priority,"reason":"not_acquired"}));continue;};
        let body=validate_object(dir,row,as_of)?;
        for e in edges(&body){
            links.push(json!({"source":id,"target":e.target,"pointer":e.pointer,"kind":e.kind,"priority":e.priority}));
            if e.target!=id&&!seen.contains(&e.target){queue.insert((e.priority,depth+1,e.target));}
        }
        nodes.insert(id.clone(),json!({"receipt":row,"body":body}));
    }
    pending.retain(|v|!nodes.contains_key(v["target"].as_str().unwrap_or("")));
    // Receipt inventory is separate from the bounded reachable graph. Bind
    // offline replay to the exact validated cache inputs, including unreached
    // receipts, so an older replay cannot masquerade as the current cache.
    let inventory:Vec<_>=rows.iter().map(|(id,(r,_))|json!({"id":id,"revision":r["revision"],"sha256":r["sha256"]})).collect();
    let inventory_bytes=serde_json::to_vec(&inventory).map_err(|_|err("inventory encoding"))?;
    Ok(json!({"schema":"slr.native-historical-zobject-graph.v1","as_of":as_of,
        "roots":["Z35370","Z38053","Z38757"],"nodes":nodes,"edges":links,"pending":pending,
        "cache_inventory":inventory,"cache_inventory_sha256":digest(&inventory_bytes),"network_attempts":attempts,"requested_target":target,"candidate_only":true,"runtime_selection_paid":false,"world_truth_paid":false}))
}
/// Symbolic call-by-value subset. No arbitrary code or assumed builtins.
fn eval(v:&Value,env:&BTreeMap<String,Value>,nodes:&Value,fuel:&mut usize,stack:&mut BTreeSet<String>,trace:&mut Vec<Value>,choices:&BTreeMap<String,String>,frontier:&mut Value)->R<Value>{
    req(*fuel>0,"evaluation fuel exhausted")?;*fuel-=1;
    match reference(&v["Z1K1"]){
        Some("Z18")=>{
            let k=reference(&v["Z18K1"]).ok_or_else(||err("invalid argument reference"))?;
            env.get(k).cloned().ok_or_else(||err("unbound argument reference"))
        },
        Some("Z7")=>{
            let id=reference(&v["Z7K1"]).ok_or_else(||err("invalid call target"))?;
            req(zid(id),"dynamic call target held")?;
            let fun=&nodes[id]["body"];
            if reference(&fun["Z1K1"])!=Some("Z8"){
                *frontier=json!({"kind":if nodes.get(id).is_none(){"missing_function"}else{"foreign_function"},"function":id});
                return Err(err("function definition missing or foreign"));
            }
            let sig=fun["Z8K1"].as_array().ok_or_else(||err("signature held"))?;
            req(sig.first().and_then(reference)==Some("Z17"),"invalid argument list head")?;
            let mut bindings=BTreeMap::new();
            for d in sig.iter().skip(1){
                req(reference(&d["Z1K1"])==Some("Z17"),"argument declaration held")?;
                let key=reference(&d["Z17K2"]).ok_or_else(||err("invalid signature key"))?;
                let arg=v.get(key).ok_or_else(||err("missing call argument"))?;
                req(bindings.insert(key.to_owned(),eval(arg,env,nodes,fuel,stack,trace,choices,frontier)?).is_none(),"duplicate declaration")?;
            }
            req(v.as_object().is_some_and(|o|o.len()==bindings.len()+2),"extra call argument")?;
            let impls=fun["Z8K4"].as_array().ok_or_else(||err("implementation selection held"))?;
            req(impls.len()>=2&&reference(&impls[0])==Some("Z14"),"implementation list held")?;
            let mut listed=BTreeSet::new();
            for item in impls.iter().skip(1){
                let z=reference(item).filter(|z|zid(z)).ok_or_else(||err("invalid implementation reference"))?;
                req(listed.insert(z),"duplicate implementation reference")?;
            }
            let imp=if let Some(chosen)=choices.get(id){
                req(listed.contains(chosen.as_str()),"unlisted implementation choice")?;
                chosen.as_str()
            }else if listed.len()==1{
                *listed.iter().next().ok_or_else(||err("empty implementation list"))?
            }else{
                *frontier=json!({"kind":"implementation_choice","function":id,"listed":listed,"function_receipt":nodes[id]["receipt"]});
                return Err(err("nonunique implementation selection held"));
            };
            let body=&nodes[imp]["body"];
            if reference(&body["Z1K1"])!=Some("Z14")||reference(&body["Z14K1"])!=Some(id){
                *frontier=json!({"kind":if nodes.get(imp).is_none(){"missing_implementation"}else{"foreign_implementation"},"function":id,"implementation":imp});
                return Err(err("implementation missing or foreign"));
            }
            if !(body.get("Z14K2").is_some_and(|x|!x.is_null())&&body.get("Z14K3").map_or(true,Value::is_null)&&body.get("Z14K4").map_or(true,Value::is_null)){
                *frontier=json!({"kind":"unsupported_implementation","function":id,"implementation":imp,"code":!body["Z14K3"].is_null(),"builtin":!body["Z14K4"].is_null(),"implementation_receipt":nodes[imp]["receipt"]});
                return Err(err("code/builtin or unsupported implementation held"));
            }
            req(stack.insert(id.into()),"recursive call held")?;
            let declared_args:Vec<_>=sig.iter().skip(1).map(|d|json!({"key":d["Z17K2"],"declared_type":d["Z17K1"]})).collect();
            trace.push(json!({"function":id,"implementation":imp,"function_receipt":nodes[id]["receipt"],"implementation_receipt":nodes[imp]["receipt"],
                "declared_arguments":declared_args,"declared_return_type":fun["Z8K2"],"bound_arguments":bindings,"type_check_paid":false}));
            let out=eval(&body["Z14K2"],&bindings,nodes,fuel,stack,trace,choices,frontier);
            stack.remove(id);out
        },
        _=>{
            if let Some(a)=v.as_array(){return Ok(Value::Array(a.iter().map(|x|eval(x,env,nodes,fuel,stack,trace,choices,frontier)).collect::<R<Vec<_>>>()?));}
            if let Some(o)=v.as_object(){let mut result=serde_json::Map::new();for(k,x)in o{result.insert(k.clone(),eval(x,env,nodes,fuel,stack,trace,choices,frontier)?);}return Ok(Value::Object(result));}
            Ok(v.clone())
        }
    }
}
pub fn evaluate_composition_candidate(graph:&Value,expression:&Value)->Value{
    evaluate_composition_with_language(graph,expression,None)
}
/// The enclosing Abstract language argument is an explicit input, never
/// silently chosen. A language ZID is still a candidate, not a type proof.
pub fn evaluate_composition_with_language(graph:&Value,expression:&Value,language:Option<&str>)->Value{
    evaluate_selected(graph,expression,language,&BTreeMap::new())
}
fn evaluate_selected(graph:&Value,expression:&Value,language:Option<&str>,choices:&BTreeMap<String,String>)->Value{
    let mut fuel=4096;let mut trace=Vec::new();let mut frontier=Value::Null;
    let mut env=BTreeMap::new();
    let result=if language.map_or(true,zid){
        if let Some(z)=language{env.insert("Z825K2".into(),json!({"Z1K1":"Z9","Z9K1":z}));}
        eval(expression,&env,&graph["nodes"],&mut fuel,&mut BTreeSet::new(),&mut trace,choices,&mut frontier)
    }else{Err(err("invalid explicit language reference"))};
    let (value,held)=match result{Ok(v)=>(v,Value::Null),Err(e)=>(Value::Null,json!(e.to_string()))};
    json!({"schema":"slr.native-abstract-composition-candidate.v1","value":value,"held":held,
        "trace":trace,"frontier":frontier,"candidate_choices":choices,"explicit_language":language,"fuel_used":4096-fuel,"candidate_only":true,"type_check_paid":false,
        "runtime_selection_paid":false,"tester_acceptance_paid":false,"world_truth_paid":false})
}
/// Inventory exact historical tester ownership, input call shape and
/// comparator expression. A comparator is not an expected value by fiat.
fn call_shape(call:&Value,id:&str,fun:&Value)->bool{
    if reference(&call["Z1K1"])!=Some("Z7")||reference(&call["Z7K1"])!=Some(id){return false}
    let Some(sig)=fun["Z8K1"].as_array()else{return false};
    if sig.first().and_then(reference)!=Some("Z17"){return false}
    let mut keys=BTreeSet::new();
    for d in sig.iter().skip(1){
        if reference(&d["Z1K1"])!=Some("Z17"){return false}
        let Some(k)=reference(&d["Z17K2"])else{return false};
        if !keys.insert(k)||call.get(k).is_none(){return false}
    }
    call.as_object().is_some_and(|o|o.len()==keys.len()+2)
}
pub fn inspect_historical_tester_contracts(graph:&Value)->R<Value>{
    let nodes=graph["nodes"].as_object().ok_or_else(||err("missing graph nodes"))?;
    let mut contracts=Vec::new();
    for (id,node) in nodes{
        let fun=&node["body"];
        if reference(&fun["Z1K1"])!=Some("Z8"){continue}
        let Some(testers)=fun["Z8K3"].as_array()else{
            contracts.push(json!({"function":id,"status":"tester_list_held"}));continue
        };
        if testers.first().and_then(reference)!=Some("Z20"){
            contracts.push(json!({"function":id,"status":"tester_list_held"}));continue
        }
        let mut seen=BTreeSet::new();
        for (i,item) in testers.iter().enumerate().skip(1){
            let Some(t)=reference(item).filter(|x|zid(x))else{
                contracts.push(json!({"function":id,"index":i,"status":"invalid_tester_reference"}));continue
            };
            if !seen.insert(t){
                contracts.push(json!({"function":id,"tester":t,"index":i,"status":"duplicate_tester_reference"}));continue
            }
            let body=&graph["nodes"][t]["body"];
            let status=if !nodes.contains_key(t){"missing_tester"}
                else if reference(&body["Z1K1"])!=Some("Z20")||reference(&body["Z20K1"])!=Some(id.as_str()){"foreign_tester"}
                else if !call_shape(&body["Z20K2"],id,fun){"input_call_shape_held"}
                else if reference(&body["Z20K3"]["Z1K1"])!=Some("Z7"){"comparator_shape_held"}
                else{"recorded_contract"};
            contracts.push(json!({"function":id,"tester":t,"index":i,"status":status,
                "tester_pointer":format!("/Z2K2/Z8K3/{i}"),"function_receipt":node["receipt"],
                "tester_receipt":graph["nodes"][t]["receipt"],"input":body["Z20K2"],
                "comparator":body["Z20K3"],"comparator_target":reference(&body["Z20K3"]["Z7K1"]),
                "tester_acceptance_paid":false}));
        }
    }
    Ok(json!({"schema":"slr.native-historical-tester-contracts.v1","contracts":contracts,
        "candidate_only":true,"type_check_paid":false,"tester_acceptance_paid":false,"world_truth_paid":false}))
}
/// Enumerate the listed implementation alternatives reached by the symbolic
/// evaluator. This is a bounded candidate envelope, not runtime selection,
/// admissibility, type soundness, tester acceptance or a semantic licence.
/// Replay each assignment from the original expression; a hold is retained,
/// never treated as an alternative value. Assignments are global per function
/// for this diagnostic, not a claim about contextual Wikifunctions dispatch.
pub fn inspect_composition_envelope(graph:&Value,expression:&Value,language:Option<&str>,budget:usize)->R<Value>{
    req((1..=64).contains(&budget),"invalid candidate envelope budget")?;
    let mut queue=std::collections::VecDeque::from([BTreeMap::<String,String>::new()]);
    let mut branches=Vec::new();
    let mut explored=0usize;
    while explored<budget{
        let Some(choices)=queue.pop_front()else{break};
        explored+=1;
        let answer=evaluate_selected(graph,expression,language,&choices);
        if answer["frontier"]["kind"]=="implementation_choice"{
            let id=text(&answer["frontier"],"function")?;
            let listed=answer["frontier"]["listed"].as_array().ok_or_else(||err("invalid choice frontier"))?;
            // Keep the parent held if its full child set cannot fit the
            // bounded queue. Never silently truncate the choice fibre.
            if explored+queue.len()+listed.len()>budget{
                branches.push(answer);
                continue;
            }
            for item in listed{
                let mut child=choices.clone();
                child.insert(id.into(),item.as_str().ok_or_else(||err("invalid choice"))?.into());
                queue.push_back(child);
            }
        }else{branches.push(answer);}
    }
    let complete=queue.is_empty()&&branches.iter().all(|b|b["held"].is_null());
    let equal=complete&&!branches.is_empty()&&branches.iter().all(|b|b["value"]==branches[0]["value"]);
    Ok(json!({"schema":"slr.native-abstract-candidate-envelope.v1","branches":branches,
        "explored":explored,"unexplored_assignments":queue,"symbolic_envelope_complete":complete,
        "symbolic_values_equal":equal,"candidate_only":true,"admissibility_paid":false,
        "type_check_paid":false,"tester_acceptance_paid":false,"runtime_selection_paid":false,
        "source_query_licence_paid":false,"world_truth_paid":false}))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn actual_root_dependencies(){
        let root:Value=serde_json::from_str(include_str!("../../../artifacts/gwb/native-zobjects/Z38757.326825.64ee066e2c1cf0f4a924b87b7fc87211d344517509141748da70a0e6cf6babe4.json")).unwrap();
        let e=edges(&root["Z2K2"]);
        for (id,kind) in [("Z38758","implementation"),("Z38759","tester"),("Z39656","tester"),("Z881","call_target")]{assert!(e.iter().any(|x|x.target==id&&x.kind==kind));}
    }
    #[test]fn historical_tester_contracts_hold_on_foreign_and_missing(){
        let mut g=json!({"nodes":{
            "Z100":{"body":{"Z1K1":"Z8","Z8K1":["Z17",{"Z1K1":"Z17","Z17K2":"Z100K1"}],"Z8K3":["Z20","Z101","Z102"]}},
            "Z101":{"body":{"Z1K1":"Z20","Z20K1":"Z100","Z20K2":{"Z1K1":"Z7","Z7K1":"Z100","Z100K1":"arg"},"Z20K3":{"Z1K1":"Z7","Z7K1":"Z877"}}}
        }});
        let r=inspect_historical_tester_contracts(&g).unwrap();
        assert_eq!(r["contracts"][0]["status"],"recorded_contract");
        assert_eq!(r["contracts"][1]["status"],"missing_tester");
        assert_eq!(r["tester_acceptance_paid"],false);
        g["nodes"]["Z101"]["body"]["Z20K1"]=json!("Z999");
        assert_eq!(inspect_historical_tester_contracts(&g).unwrap()["contracts"][0]["status"],"foreign_tester");
        g["nodes"]["Z101"]["body"]["Z20K1"]=json!("Z100");
        g["nodes"]["Z101"]["body"]["Z20K2"].as_object_mut().unwrap().remove("Z100K1");
        assert_eq!(inspect_historical_tester_contracts(&g).unwrap()["contracts"][0]["status"],"input_call_shape_held");
    }
    #[test]fn bounded_choice_envelope_preserves_alternatives_and_holds(){
        let mut g=json!({"nodes":{
            "Z100":{"body":{"Z1K1":"Z8","Z8K1":["Z17"],"Z8K4":["Z14","Z102","Z101"]}},
            "Z101":{"body":{"Z1K1":"Z14","Z14K1":"Z100","Z14K2":"one"}},
            "Z102":{"body":{"Z1K1":"Z14","Z14K1":"Z100","Z14K2":"two"}}
        }});
        let x=json!({"Z1K1":"Z7","Z7K1":"Z100"});
        let r=inspect_composition_envelope(&g,&x,None,3).unwrap();
        assert_eq!(r["symbolic_envelope_complete"],true);
        assert_eq!(r["symbolic_values_equal"],false);
        assert_eq!(r["branches"][0]["value"],"one");
        assert_eq!(r["branches"][1]["value"],"two");
        assert_eq!(inspect_composition_envelope(&g,&x,None,2).unwrap()["symbolic_envelope_complete"],false);
        g["nodes"]["Z102"]["body"]["Z14K2"]=json!("one");
        assert_eq!(inspect_composition_envelope(&g,&x,None,3).unwrap()["symbolic_values_equal"],true);
        g["nodes"].as_object_mut().unwrap().remove("Z102");
        let r=inspect_composition_envelope(&g,&x,None,3).unwrap();
        assert_eq!(r["symbolic_values_equal"],false);
        assert_eq!(r["branches"][1]["frontier"]["kind"],"missing_implementation");
        assert_eq!(r["branches"][1]["frontier"]["implementation"],"Z102");
        assert_eq!(r["source_query_licence_paid"],false);
    }
    #[test]fn composition_and_foreign_owner(){
        let mut g=json!({"nodes":{"Z100":{"body":{"Z1K1":"Z8","Z8K1":["Z17",{"Z1K1":"Z17","Z17K2":"Z100K1"}],"Z8K4":["Z14","Z101"]}},
            "Z101":{"body":{"Z1K1":"Z14","Z14K1":"Z100","Z14K2":{"Z1K1":"Z18","Z18K1":"Z100K1"}}}}});
        let x=json!({"Z1K1":"Z7","Z7K1":"Z100","Z100K1":"literal"});
        assert_eq!(evaluate_composition_candidate(&g,&x)["value"],"literal");
        g["nodes"]["Z101"]["body"]["Z14K1"]=json!("Z200");
        assert!(!evaluate_composition_candidate(&g,&x)["held"].is_null());
    }
}
