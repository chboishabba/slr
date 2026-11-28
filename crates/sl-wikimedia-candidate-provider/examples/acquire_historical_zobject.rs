//! Explicit bounded native provider invocation: one object per call.
use sensiblaw_wikimedia_candidate_provider::fetch_historical_zobject;
use std::{env,fs,path::PathBuf,io::Write};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<_>=env::args().skip(1).collect();
    let [id,as_of,out]=args.as_slice() else{return Err("usage: acquire_historical_zobject ZID AS_OF OUTPUT_DIR".into())};
    let dir=PathBuf::from(out);fs::create_dir_all(&dir)?;
    let mut result=fetch_historical_zobject(id,as_of)?;
    let name=format!("{}.{}.{}.json",result.object_id,result.revision,result.sha256);
    let path=dir.join(&name);
    if path.exists(){if fs::read(&path)?!=result.bytes{return Err("immutable object collision".into());}}
    else{let mut f=fs::OpenOptions::new().create_new(true).write(true).open(&path)?;f.write_all(&result.bytes)?;f.sync_all()?;}
    result.receipt["path"]=serde_json::json!(name);
    println!("{}",serde_json::to_string_pretty(&result.receipt)?);
    Ok(())
}
