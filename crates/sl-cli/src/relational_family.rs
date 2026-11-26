//! Native comparison family over existing PG-owned selectors and consumer.
use sensiblaw_pg_source_store::{load_database_config, run_persisted_relational_family};

pub fn run(args:&[String])->Result<(),String>{
    if !(4..=12).contains(&args.len()) {
        return Err("usage: sensiblaw relational-family CONSUMER_REF ACCESS_CONSTRAINT_REF SELECTOR_REF SELECTOR_REF [SELECTOR_REF ...]".into());
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let result=run_persisted_relational_family(&config,&args[0],&args[1],&args[2..])
        .map_err(|e|e.to_string())?;
    println!("{}",serde_json::to_string_pretty(&result).map_err(|e|e.to_string())?);
    Ok(())
}
