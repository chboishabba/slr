use sensiblaw_wikimedia_candidate_provider::{resolve_historical_graph_targeted,evaluate_composition_with_language,inspect_composition_envelope};
use std::{env,fs,path::Path};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<_>=env::args().skip(1).collect();
    let [repo,manifest,cache,new_limit,rest @ ..]=args.as_slice() else{return Err("usage: resolve_historical_graph REPO MANIFEST CACHE NEW_LIMIT(0|1) [TARGET_OR_-] [LANGUAGE_ZID]".into())};
    if rest.len()>2{return Err("too many arguments".into());}
    let target=rest.first().filter(|z|z.as_str()!="-").map(String::as_str);
    let language=rest.get(1).map(String::as_str);
    let graph=resolve_historical_graph_targeted(Path::new(repo),Path::new(manifest),Path::new(cache),new_limit.parse()?,8,256,target)?;
    let ast_bytes=fs::read(Path::new(repo).join("artifacts/gwb/multiview-native/acquired/Q207-abstract-constructor.json"))?;
    use sha2::{Digest,Sha256};
    if format!("{:x}",Sha256::digest(&ast_bytes))!="7cf3c6bbe5a0a90dd13a067e5c99ed276d50d75fdc3ade2532a5e5d0b599f1e2"{return Err("original AST digest mismatch".into());}
    let ast:serde_json::Value=serde_json::from_slice(&ast_bytes)?;
    let calls=ast["sections"]["Q8776414"]["fragments"].as_array().ok_or("invalid original fragments")?;
    let evaluations:Vec<_>=calls.iter().filter(|v|v["Z1K1"]=="Z7").map(|v|evaluate_composition_with_language(&graph,v,language)).collect();
    let envelopes=calls.iter().filter(|v|v["Z1K1"]=="Z7").map(|v|inspect_composition_envelope(&graph,v,language,32)).collect::<Result<Vec<_>,_>>()?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"graph":graph,"composition_candidates":evaluations,"composition_envelopes":envelopes}))?);
    Ok(())
}
