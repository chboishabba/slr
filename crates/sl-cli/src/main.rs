mod contract_identity;
mod contracts;
mod cullen;
mod oalc;
mod case_follow;
mod waltons;

use std::env;
use std::path::PathBuf;
use waltons::WaltonsPaths;

fn usage() {
    eprintln!(
        "SensibLaw native CLI

USAGE:
  sensiblaw legal-follow case acquire --citation '[YYYY] COURT N' [options]
  sensiblaw legal-follow cullen pnf --operator-opt-in [--output-dir PATH] [--spacy-model MODEL]
  sensiblaw legal-follow contracts landscape <plan|status|expand|acquire> [--as-at YYYY-MM-DD] [--jurisdiction AU-QLD]
  sensiblaw legal-follow oalc stream-pinned --revision SHA --citation TEXT [options]
  sensiblaw legal-follow waltons [--base PATH] status
  sensiblaw legal-follow waltons [--base PATH] acquire
  sensiblaw legal-follow waltons [--base PATH] materialise
  sensiblaw legal-follow waltons [--base PATH] review prepare
  sensiblaw legal-follow waltons [--base PATH] review finalize
  sensiblaw legal-follow waltons [--base PATH] review compile
  sensiblaw legal-follow waltons [--base PATH] frontier
  sensiblaw legal-follow waltons [--base PATH] cited-by plan
  sensiblaw legal-follow waltons [--base PATH] cited-by import PROVIDER_RESULTS.json
  sensiblaw legal-follow waltons [--base PATH] cited-by worklist
  sensiblaw legal-follow waltons [--base PATH] cited-by acquire
  sensiblaw legal-follow waltons [--base PATH] identity prepare
  sensiblaw legal-follow waltons [--base PATH] identity finalize
  sensiblaw legal-follow waltons [--base PATH] identity compile
  sensiblaw legal-follow waltons [--base PATH] treatment queue
  sensiblaw legal-follow waltons [--base PATH] treatment merge
  sensiblaw legal-follow waltons [--base PATH] treatment prepare
  sensiblaw legal-follow waltons [--base PATH] treatment finalize
  sensiblaw legal-follow waltons [--base PATH] genealogy
  sensiblaw legal-follow waltons [--base PATH] s14-sync

NOTES:
  * acquire/cited-by acquire require --features live-network at build time.
  * review prepare/finalize are human-review file surfaces; they do not make
    legal decisions automatically.
  * Waltons s14-sync is the canonical typed Rust reviewed-hop path.
  * contracts landscape expand --delta is a compatibility/import surface for
    reviewed artifacts, not the semantic command ABI.
  * CitedBy provider results are discovery candidates only and must be
    re-acquired through OALC before treatment review.
"
    );
}

fn parse_base(args: &mut Vec<String>) -> Result<WaltonsPaths, String> {
    if let Some(index) = args.iter().position(|arg| arg == "--base") {
        let Some(value) = args.get(index + 1).cloned() else {
            return Err("--base requires a path".into());
        };
        args.drain(index..=index + 1);
        Ok(WaltonsPaths::from_base(PathBuf::from(value)))
    } else {
        Ok(WaltonsPaths::default())
    }
}

fn waltons_command(mut args: Vec<String>) -> Result<(), String> {
    let paths = parse_base(&mut args)?;
    if args.is_empty() {
        usage();
        return Err("missing Waltons command".into());
    }

    match args.as_slice() {
        [command] if command == "status" => {
            waltons::status(&paths);
            Ok(())
        }
        [command] if command == "acquire" => waltons::acquire(&paths),
        [command] if command == "materialise" || command == "materialize" => {
            waltons::materialise(&paths)
        }
        [command] if command == "frontier" => waltons::frontier(&paths),
        [group, command] if group == "review" && command == "prepare" => {
            waltons::review_prepare(&paths)
        }
        [group, command]
            if group == "review" && (command == "finalize" || command == "finalise") =>
        {
            waltons::review_finalize(&paths)
        }
        [group, command] if group == "review" && command == "compile" => {
            waltons::review_compile(&paths)
        }
        [group, command] if group == "cited-by" && command == "plan" => {
            waltons::cited_by_plan(&paths)
        }
        [group, command, provider_results]
            if group == "cited-by" && command == "import" =>
        {
            waltons::cited_by_import(&paths, &PathBuf::from(provider_results))
        }
        [group, command] if group == "cited-by" && command == "worklist" => {
            waltons::cited_by_worklist(&paths)
        }
        [group, command] if group == "cited-by" && command == "acquire" => {
            waltons::cited_by_acquire(&paths)
        }
        [group, command] if group == "identity" && command == "prepare" => {
            waltons::identity_prepare(&paths)
        }
        [group, command]
            if group == "identity" && (command == "finalize" || command == "finalise") =>
        {
            waltons::identity_finalize(&paths)
        }
        [group, command] if group == "identity" && command == "compile" => {
            waltons::identity_compile(&paths)
        }
        [group, command] if group == "treatment" && command == "queue" => {
            waltons::treatment_queue(&paths)
        }
        [group, command] if group == "treatment" && command == "merge" => {
            waltons::treatment_merge(&paths)
        }
        [group, command] if group == "treatment" && command == "prepare" => {
            waltons::treatment_prepare(&paths)
        }
        [group, command]
            if group == "treatment" && (command == "finalize" || command == "finalise") =>
        {
            waltons::treatment_finalize(&paths)
        }
        [command] if command == "genealogy" => waltons::genealogy(&paths),
        [command] if command == "s14-sync" => waltons::s14_sync(&paths),
        _ => {
            usage();
            Err(format!("unsupported Waltons command: {}", args.join(" ")))
        }
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [domain, matter, rest @ ..] if domain == "legal-follow" && matter == "waltons" => {
            waltons_command(rest.to_vec())
        }
        [domain, matter, rest @ ..] if domain == "legal-follow" && matter == "case" => {
            case_follow::run(rest.to_vec())
        }
        [domain, matter, rest @ ..] if domain == "legal-follow" && matter == "oalc" => {
            oalc::run(rest.to_vec())
        }
        [domain, matter, rest @ ..] if domain == "legal-follow" && matter == "cullen" => {
            cullen::run(rest.to_vec())
        }
        [domain, matter, rest @ ..] if domain == "legal-follow" && matter == "contracts" => {
            contracts::run(rest.to_vec())
        }
        _ => {
            usage();
            Err("expected: sensiblaw legal-follow <case|oalc|waltons|cullen|contracts> ...".into())
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("sensiblaw: {error}");
        std::process::exit(2);
    }
}
