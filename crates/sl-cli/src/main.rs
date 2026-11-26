mod contract_follow_campaign;
mod contract_identity;
mod contract_treatment;
mod contracts;
mod cullen;
mod oalc;
mod case_follow;
mod s14_adaptive_fixture;
mod waltons;
mod waltons_live;
mod pnf_candidate;
mod pnf_db_checked;
mod relational_family;
mod native_antecedent_review;
mod numeric_pnf_native_cli;
mod timeline_candidates;

use std::env;
use std::path::PathBuf;
use waltons::WaltonsPaths;

fn usage() {
    eprintln!(
        "SensibLaw native CLI

USAGE:
  sensiblaw pnf-candidate TSV_PATH SPAN_REF START_CHAR END_CHAR
  sensiblaw pnf-db-checked PARSER_RUN STATEMENT_REF DOCUMENT_REF SOURCE_REV REGION_REF START_CHAR END_CHAR LITERAL_FILE
  sensiblaw relational-family CONSUMER_REF ACCESS_CONSTRAINT_REF SELECTOR_REF SELECTOR_REF [SELECTOR_REF ...]
  sensiblaw relational-family inspect FAMILY_REF
  sensiblaw relational-family prepare-revision FAMILY_REF OLD_SELECTOR NEW_SELECTOR
  sensiblaw relational-family inspect-revision TRANSITION_REF
  sensiblaw relational-family review-revision TRANSITION_REF COMMAND_REF REVIEWER_REF accept|reject|abstain
  sensiblaw relational-family publish-revision TRANSITION_REF
  sensiblaw native-antecedent-review prepare ORIGINAL_C410_RECEIPT.json
  sensiblaw native-antecedent-review inspect REVIEW_ITEM_REF
  sensiblaw native-antecedent-review runs REVIEW_ITEM_REF
  sensiblaw native-antecedent-review frontier REVIEW_ITEM_REF [PARSER_RUN_REF]
  sensiblaw native-antecedent-review derive REVIEW_ITEM_REF [PARSER_RUN_REF]
  sensiblaw native-antecedent-review answer REVIEW_ITEM_REF [PARSER_RUN_REF]
  sensiblaw native-antecedent-review replay REVIEW_ITEM_REF ANSWER.json [PARSER_RUN_REF]
  sensiblaw numeric-pnf-native relation-query RUN DOC SNAPSHOT PACKET_SHA256 RELATION_TYPE_OR_all
  sensiblaw numeric-pnf-native inspect ORIGINAL_SOURCE_REV ORIGINAL_SLR_PARSER_RUN
  sensiblaw numeric-pnf-native plan ORIGINAL_SOURCE_REV ORIGINAL_SLR_PARSER_RUN
  sensiblaw numeric-pnf-native compile ORIGINAL_SOURCE_REV ORIGINAL_SLR_PARSER_RUN MAX_SENTENCES --commit
  sensiblaw timeline-candidates EVENT_REF
  sensiblaw legal-follow case acquire --citation '[YYYY] COURT N' [options]
  sensiblaw legal-follow cullen pnf --operator-opt-in [--output-dir PATH] [--spacy-model MODEL]
  sensiblaw legal-follow contracts landscape <plan|status|expand|acquire> [--as-at YYYY-MM-DD] [--jurisdiction AU-QLD]
  sensiblaw legal-follow contracts landscape adaptive-fixture [--output PATH]
  sensiblaw legal-follow contracts campaign <discover|acquire-next|identity-prepare|identity-reviewed|treatment-prepare|treatment-reviewed> ...
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
  sensiblaw legal-follow waltons [--base PATH] live status
  sensiblaw legal-follow waltons [--base PATH] live prepare
  sensiblaw legal-follow waltons [--base PATH] live paragraph-reviewed
  sensiblaw legal-follow waltons [--base PATH] live cited-by PROVIDER_RESULTS.json
  sensiblaw legal-follow waltons [--base PATH] live identity-reviewed
  sensiblaw legal-follow waltons [--base PATH] live treatment-reviewed

NOTES:
  * acquire/cited-by acquire require --features live-network at build time.
  * review prepare/finalize are human-review file surfaces; they do not make
    legal decisions automatically.
  * Waltons s14-sync is the canonical typed Rust reviewed-hop path.
  * Waltons live is the resumable live-OALC operator path. It advances every
    deterministic stage and stops only at explicit human/provider gates.
  * contracts landscape adaptive-fixture is an executable controller
    calibration; it does not claim a live human-reviewed campaign.
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
        [group, command] if group == "live" && command == "status" => {
            waltons_live::status(&paths)
        }
        [group, command] if group == "live" && command == "prepare" => {
            waltons_live::prepare(&paths)
        }
        [group, command] if group == "live" && command == "paragraph-reviewed" => {
            waltons_live::paragraph_reviewed(&paths)
        }
        [group, command, provider_results] if group == "live" && command == "cited-by" => {
            waltons_live::cited_by(&paths, &PathBuf::from(provider_results))
        }
        [group, command] if group == "live" && command == "identity-reviewed" => {
            waltons_live::identity_reviewed(&paths)
        }
        [group, command] if group == "live" && command == "treatment-reviewed" => {
            waltons_live::treatment_reviewed(&paths)
        }
        _ => {
            usage();
            Err(format!("unsupported Waltons command: {}", args.join(" ")))
        }
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, rest @ ..] if command == "pnf-candidate" => {
            pnf_candidate::run(rest)
        }
        [command, rest @ ..] if command == "relational-family" => {
            relational_family::run(rest)
        }
        [command, rest @ ..] if command == "pnf-db-checked" => {
            pnf_db_checked::run(rest)
        }
        [command, rest @ ..] if command == "native-antecedent-review" => {
            native_antecedent_review::run(rest)
        }
        [command, rest @ ..] if command == "numeric-pnf-native" => {
            numeric_pnf_native_cli::run(rest)
        }
        [command, rest @ ..] if command == "timeline-candidates" => {
            timeline_candidates::run(rest)
        }
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
        [domain, matter, scope, command, rest @ ..]
            if domain == "legal-follow"
                && matter == "contracts"
                && scope == "landscape"
                && command == "adaptive-fixture" =>
        {
            s14_adaptive_fixture::run(rest.to_vec())
        }
        [domain, matter, scope, rest @ ..]
            if domain == "legal-follow" && matter == "contracts" && scope == "campaign" =>
        {
            contract_follow_campaign::run(rest.to_vec())
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