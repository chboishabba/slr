use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_evidence_payment::{
    apply_review_to_active_frontier, decode_review_spec, encode_review_spec,
    EvidenceCoordinateKind, EvidenceReviewSpec, ReviewDisposition, REVIEW_VERSION,
};

fn arg_value(args:&[String],key:&str)->Option<String>{args.iter().position(|v|v==key).and_then(|i|args.get(i+1)).cloned()}
fn required(args:&[String],key:&str)->String{arg_value(args,key).unwrap_or_else(||panic!("missing {key}"))}
fn path(args:&[String],key:&str)->PathBuf{PathBuf::from(required(args,key))}
fn iteration(args:&[String])->i64{required(args,"--iteration").parse().expect("invalid --iteration")}
fn coordinate(v:&str)->EvidenceCoordinateKind{match v{
    "source-identity"=>EvidenceCoordinateKind::SourceIdentity,"same-object"=>EvidenceCoordinateKind::SameObject,"authority"=>EvidenceCoordinateKind::Authority,
    "mechanism"=>EvidenceCoordinateKind::Mechanism,"quantification"=>EvidenceCoordinateKind::Quantification,"probability"=>EvidenceCoordinateKind::Probability,
    "counterfactual"=>EvidenceCoordinateKind::Counterfactual,"instrument-comparison"=>EvidenceCoordinateKind::InstrumentComparison,"incidence"=>EvidenceCoordinateKind::Incidence,
    "classification"=>EvidenceCoordinateKind::Classification,_=>panic!("unknown coordinate {v}")}}
fn disposition(v:&str)->ReviewDisposition{match v{"pays"=>ReviewDisposition::PaysObligation,"partial"=>ReviewDisposition::PartialEvidenceOnly,"measurement-block"=>ReviewDisposition::MeasurementBlockOnly,"wrong-type"=>ReviewDisposition::RejectedWrongType,_=>panic!("unknown disposition {v}")}}

fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<String>=env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or(""){
        "encode"=>{
            let spec=EvidenceReviewSpec{consumer_id:required(&args,"--consumer-id"),requirement_id:required(&args,"--requirement-id"),coordinate:coordinate(&required(&args,"--coordinate")),evidence_reference:required(&args,"--evidence-reference"),reviewer_reference:required(&args,"--reviewer-reference"),disposition:disposition(&required(&args,"--disposition"))};
            let mut out=BufWriter::new(File::create(path(&args,"--output"))?); encode_review_spec(&mut out,&spec)?; out.flush()?;
            eprintln!("SLR_EVIDENCE_REVIEW_SPEC_RECEIPT review_wire_version={} binary_wire=true json_transport=false regex_parser=false",REVIEW_VERSION);
        }
        "apply"=>{
            let mut frontier=BufReader::new(File::open(path(&args,"--frontier"))?); let mut review=BufReader::new(File::open(path(&args,"--review"))?); let spec=decode_review_spec(&mut review)?;
            let mut out=BufWriter::new(File::create(path(&args,"--output"))?); let receipt=apply_review_to_active_frontier(&mut frontier,&spec,&mut out,iteration(&args))?; out.flush()?;
            eprintln!("SLR_EVIDENCE_PAYMENT_RECEIPT target_obligation_found={} reviews_emitted={} payments_emitted={} claim_truth_promoted={} semantic_authority_created={}",receipt.target_obligation_found,receipt.reviews_emitted,receipt.payments_emitted,receipt.claim_truth_promoted,receipt.semantic_authority_created);
        }
        _=>{eprintln!("usage: sensiblaw-evidence-payment encode --consumer-id ID --requirement-id ID --coordinate KIND --evidence-reference REF --reviewer-reference REF --disposition pays|partial|measurement-block|wrong-type --output review.slre");eprintln!("   or: sensiblaw-evidence-payment apply --frontier active.slrw --review review.slre --output review-payment.slrw --iteration N");std::process::exit(2);}
    }
    Ok(())
}
