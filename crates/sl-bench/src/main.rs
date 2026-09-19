use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: sensiblaw-bench <spacy_parse_ns> <total_pipeline_ns>");
        std::process::exit(2);
    }
    let spacy: u128 = args[1].parse().expect("spacy_parse_ns");
    let total: u128 = args[2].parse().expect("total_pipeline_ns");
    let limit = spacy.saturating_mul(2);
    println!("spacy_parse_ns={spacy} total_pipeline_ns={total} limit_ns={limit} ratio={:.4}", total as f64 / spacy.max(1) as f64);
    if total > limit {
        eprintln!("PERFORMANCE_GATE_FAIL: T_total exceeded 2x spaCy parse walltime");
        std::process::exit(1);
    }
    println!("PERFORMANCE_GATE_PASS");
}
