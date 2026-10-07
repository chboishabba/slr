use sensiblaw_pg_source_store::{
    load_database_config, materialize_real_matter_controversy, DisagreementKind,
    ProceduralGoal, RealMatterControversyDraft, ResponseMode,
};

fn usage() -> ! {
    eprintln!(
        "usage: itir_real_matter_controversy \
<matter-ref> \
<applicant-claim-ref> <applicant-reviewed-evidence-ref> <applicant-normative-order-ref> \
<respondent-claim-ref> <respondent-reviewed-evidence-ref> <respondent-normative-order-ref> \
<response-mode> <disagreement-kind> \
<unresolved-question> <requested-discriminator> <target-evidence-query> [procedural-goal]\n\n\
response-mode: deny-occurrence | admit-occurrence-dispute-characterisation | admit-conduct-add-context | dispute-causation | challenge-evidence-reliability | offer-alternative-event | admit-proposition\n\
disagreement-kind: node | relation | evidence | characterisation | causal | legal-consequence | normative-order-mismatch\n\
procedural-goal: common-ground | isolate-residual | decide-evidence | prepare-adjudication (default: decide-evidence)\n\n\
All source/review coordinates must already exist in PostgreSQL. This runner never imports JSON and never creates a human review decision."
    );
    std::process::exit(2)
}

fn parse_response(value: &str) -> Option<ResponseMode> {
    Some(match value {
        "deny-occurrence" => ResponseMode::DenyOccurrence,
        "admit-occurrence-dispute-characterisation" => {
            ResponseMode::AdmitOccurrenceDisputeCharacterisation
        }
        "admit-conduct-add-context" => ResponseMode::AdmitConductAddContext,
        "dispute-causation" => ResponseMode::DisputeCausation,
        "challenge-evidence-reliability" => ResponseMode::ChallengeEvidenceReliability,
        "offer-alternative-event" => ResponseMode::OfferAlternativeEvent,
        "admit-proposition" => ResponseMode::AdmitProposition,
        _ => return None,
    })
}

fn parse_disagreement(value: &str) -> Option<DisagreementKind> {
    Some(match value {
        "node" => DisagreementKind::Node,
        "relation" => DisagreementKind::Relation,
        "evidence" => DisagreementKind::Evidence,
        "characterisation" => DisagreementKind::Characterisation,
        "causal" => DisagreementKind::Causal,
        "legal-consequence" => DisagreementKind::LegalConsequence,
        "normative-order-mismatch" => DisagreementKind::NormativeOrderMismatch,
        _ => return None,
    })
}

fn parse_goal(value: &str) -> Option<ProceduralGoal> {
    Some(match value {
        "common-ground" => ProceduralGoal::IdentifyCommonGround,
        "isolate-residual" => ProceduralGoal::IsolateResidualControversy,
        "decide-evidence" => ProceduralGoal::DecideEvidenceNeeded,
        "prepare-adjudication" => ProceduralGoal::PrepareForAdjudication,
        _ => return None,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(12..=13).contains(&args.len()) {
        usage();
    }
    let response_mode = parse_response(&args[7]).unwrap_or_else(|| usage());
    let disagreement_kind = parse_disagreement(&args[8]).unwrap_or_else(|| usage());
    let procedural_goal = args
        .get(12)
        .map(String::as_str)
        .map(parse_goal)
        .transpose()
        .unwrap_or_else(|| usage())
        .unwrap_or(ProceduralGoal::DecideEvidenceNeeded);

    let config = load_database_config(None)?;
    let receipt = materialize_real_matter_controversy(
        &config,
        &RealMatterControversyDraft {
            matter_ref: args[0].clone(),
            applicant_claim_ref: args[1].clone(),
            applicant_reviewed_evidence_ref: args[2].clone(),
            applicant_normative_order_ref: args[3].clone(),
            respondent_claim_ref: args[4].clone(),
            respondent_reviewed_evidence_ref: args[5].clone(),
            respondent_normative_order_ref: args[6].clone(),
            response_mode,
            disagreement_kind,
            unresolved_question: args[9].clone(),
            requested_discriminator: args[10].clone(),
            target_evidence_query: args[11].clone(),
            procedural_goal,
        },
    )?;

    println!("matter_ref={}", receipt.matter_ref);
    println!("controversy_ref={}", receipt.controversy_ref);
    println!("applicant_fibre_ref={}", receipt.applicant_fibre_ref);
    println!("respondent_fibre_ref={}", receipt.respondent_fibre_ref);
    println!("response_ref={}", receipt.response_ref);
    println!("response_mode={:?}", receipt.response_mode);
    println!("residual_ref={}", receipt.residual_ref);
    println!("disagreement_kind={:?}", receipt.disagreement_kind);
    println!("obligation_ref={}", receipt.obligation_ref);
    println!("reverse_ref={}", receipt.reverse_ref);
    println!("applicant_normative_order_ref={}", receipt.applicant_normative_order_ref);
    println!("respondent_normative_order_ref={}", receipt.respondent_normative_order_ref);
    println!("candidate_only={}", receipt.candidate_only);
    println!("creates_semantic_authority={}", receipt.creates_semantic_authority);
    println!("creates_legal_authority={}", receipt.creates_legal_authority);
    println!("applicability_promoted={}", receipt.applicability_promoted);
    println!("claim_truth_promoted={}", receipt.claim_truth_promoted);
    Ok(())
}
