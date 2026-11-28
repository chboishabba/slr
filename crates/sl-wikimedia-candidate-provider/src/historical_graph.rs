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
    Ok(json!({"schema":"slr.native-historical-zobject-graph.v1","as_of":as_of,
        "roots":["Z35370","Z38053","Z38757"],"nodes":nodes,"edges":links,"pending":pending,
        "network_attempts":attempts,"requested_target":target,"candidate_only":true,"runtime_selection_paid":false,"world_truth_paid":false}))
}
/// Symbolic call-by-value subset. No arbitrary code or assumed builtins.
fn eval(v:&Value,env:&BTreeMap<String,Value>,nodes:&Value,fuel:&mut usize,stack:&mut BTreeSet<String>,trace:&mut Vec<Value>)->R<Value>{
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
            req(reference(&fun["Z1K1"])==Some("Z8"),"function definition not acquired")?;
            let sig=fun["Z8K1"].as_array().ok_or_else(||err("signature held"))?;
            let mut bindings=BTreeMap::new();
            for d in sig.iter().skip(1){
                req(reference(&d["Z1K1"])==Some("Z17"),"argument declaration held")?;
                let key=reference(&d["Z17K2"]).ok_or_else(||err("invalid signature key"))?;
                let arg=v.get(key).ok_or_else(||err("missing call argument"))?;
                req(bindings.insert(key.to_owned(),eval(arg,env,nodes,fuel,stack,trace)?).is_none(),"duplicate declaration")?;
            }
            req(v.as_object().is_some_and(|o|o.len()==bindings.len()+2),"extra call argument")?;
            let impls=fun["Z8K4"].as_array().ok_or_else(||err("implementation selection held"))?;
            req(impls.len()==2&&reference(&impls[0])==Some("Z14"),"nonunique implementation selection held")?;
            let imp=reference(&impls[1]).ok_or_else(||err("invalid implementation reference"))?;
            let body=&nodes[imp]["body"];
            req(reference(&body["Z1K1"])==Some("Z14")&&reference(&body["Z14K1"])==Some(id),"implementation missing or foreign")?;
            req(body.get("Z14K2").is_some_and(|x|!x.is_null())&&body.get("Z14K3").map_or(true,Value::is_null)&&body.get("Z14K4").map_or(true,Value::is_null),"code/builtin or unsupported implementation held")?;
            req(stack.insert(id.into()),"recursive call held")?;
            trace.push(json!({"function":id,"implementation":imp,"function_receipt":nodes[id]["receipt"],"implementation_receipt":nodes[imp]["receipt"]}));
            let out=eval(&body["Z14K2"],&bindings,nodes,fuel,stack,trace);
            stack.remove(id);out
        },
        _=>{
            if let Some(a)=v.as_array(){return Ok(Value::Array(a.iter().map(|x|eval(x,env,nodes,fuel,stack,trace)).collect::<R<Vec<_>>>()?));}
            if let Some(o)=v.as_object(){let mut result=serde_json::Map::new();for(k,x)in o{result.insert(k.clone(),eval(x,env,nodes,fuel,stack,trace)?);}return Ok(Value::Object(result));}
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
    let mut fuel=4096;let mut trace=Vec::new();
    let mut env=BTreeMap::new();
    let result=if language.map_or(true,zid){
        if let Some(z)=language{env.insert("Z825K2".into(),json!({"Z1K1":"Z9","Z9K1":z}));}
        eval(expression,&env,&graph["nodes"],&mut fuel,&mut BTreeSet::new(),&mut trace)
    }else{Err(err("invalid explicit language reference"))};
    let (value,held)=match result{Ok(v)=>(v,Value::Null),Err(e)=>(Value::Null,json!(e.to_string()))};
    json!({"schema":"slr.native-abstract-composition-candidate.v1","value":value,"held":held,
        "trace":trace,"explicit_language":language,"fuel_used":4096-fuel,"candidate_only":true,"type_check_paid":false,
        "runtime_selection_paid":false,"tester_acceptance_paid":false,"world_truth_paid":false})
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn actual_root_dependencies(){
        let root:Value=serde_json::from_str(include_str!("../../../artifacts/gwb/native-zobjects/Z38757.326825.64ee066e2c1cf0f4a924b87b7fc87211d344517509141748da70a0e6cf6babe4.json")).unwrap();
        let e=edges(&root["Z2K2"]);
        for (id,kind) in [("Z38758","implementation"),("Z38759","tester"),("Z39656","tester"),("Z881","call_target")]{assert!(e.iter().any(|x|x.target==id&&x.kind==kind));}
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
