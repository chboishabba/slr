use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_route_selector::select_routes;

fn arg_value(args:&[String],key:&str)->Option<String>{args.iter().position(|v|v==key).and_then(|i|args.get(i+1)).cloned()}
fn path(args:&[String],key:&str)->PathBuf{PathBuf::from(arg_value(args,key).unwrap_or_else(||panic!("missing {key}")))}
fn max_actions(args:&[String])->usize{arg_value(args,"--max-per-intent").unwrap_or_else(||"4".into()).parse().expect("invalid --max-per-intent")}

fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<String>=env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or(""){
        "select"=>{
            let mut intents=BufReader::new(File::open(path(&args,"--intents"))?);
            let mut candidates=BufReader::new(File::open(path(&args,"--candidates"))?);
            let mut output=BufWriter::new(File::create(path(&args,"--output"))?);
            let receipt=select_routes(&mut intents,&mut candidates,&mut output,max_actions(&args))?;output.flush()?;
            eprintln!("SLR_ROUTE_SELECTOR_RECEIPT intents_seen={} candidates_seen={} selected_actions={} pareto_dimensions_scalarized={} frontier_rank_is_truth_rank={} candidate_only={} semantic_promotion={} route_action_is_claim_truth={}",receipt.intents_seen,receipt.candidates_seen,receipt.selected_actions,receipt.pareto_dimensions_scalarized,receipt.frontier_rank_is_truth_rank,receipt.candidate_only,receipt.semantic_promotion,receipt.route_action_is_claim_truth);
        }
        _=>{eprintln!("usage: sensiblaw-route-selector select --intents route-intents.slrw --candidates candidates.slrg --output selected-routes.slrw [--max-per-intent N]");std::process::exit(2);}
    }
    Ok(())
}
