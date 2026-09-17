use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_pg_source_store::load_database_config;
use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{
    mabo_world_expansion_policy, DisambiguationOutcome, ResidualClass, ReviewDecision,
};
use sensiblaw_proof_search_loop::world_expansion_adapters::{
    from_wikidata_route, ExpansionScoring,
};
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    run_recurrent_world_expansion, PreparedWorldExpansionCycle, RecurrentRunBlocker,
    WorldExpansionCycleSource, WorldExpansionRunnerConfig,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_proof_search_loop::world_expansion_step::ResidualRouting;
use sensiblaw_proof_search_loop::world_identity::{
    WorldIdentityResolutionKind, WorldIdentityResolutionReceipt, WorldObjectIdentity,
};
use sensiblaw_proof_search_loop::world_observation::GetterBackend;
use sensiblaw_proof_search_loop::world_observation_adapters::wikidata_property_observation;
use sensiblaw_reviewed_evidence_payment::compile_consumer_residual_stream_review_aware;
use sensiblaw_route_selector::{decode_route_candidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_entity_rdf_revision_receipt,
};
use sensiblaw_world_expansion_runtime::{
    reviewed_observation_payment_stream, PgDiscoveryLineageSink,
};

const MABO_QID: &str = "Q1501525";
const MABO_REVISION: u64 = 2_333_409_615;
const EDDIE_MABO_QID: &str = "Q975866";

struct OnePreparedCycle(Option<PreparedWorldExpansionCycle>);

impl WorldExpansionCycleSource for OnePreparedCycle {
    fn prepare_next_cycle(
        &mut self,
        _session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        self.0
            .take()
            .ok_or_else(|| RecurrentRunBlocker::new("campaign:no-second-reviewed-cycle"))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let residual = ProofResidual {
        residual_ref: "residual:mabo:participant-identity".into(),
        proposition_ref: "mabo:participant:eddie-mabo".into(),
        producer_class_ref: "producer:world-expansion".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status: ResidualStatus::Open,
    };
    let frontier = ProofFrontier {
        consumer_ref: "consumer:mabo-100-identity-classes".into(),
        frontier_ref: "frontier:mabo:participant-identity:0".into(),
        residuals: vec![residual.clone()],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    let acquired = fetch_entity_rdf_revision_receipt(MABO_QID, MABO_REVISION)?;
    let mut encoded = Vec::new();
    emit_candidates_from_rdf(
        MABO_QID,
        std::io::Cursor::new(acquired.rdf_bytes.as_slice()),
        &mut encoded,
    )?;
    let mut cursor = std::io::Cursor::new(encoded);
    let mut route = None;
    while let Some(candidate) = decode_route_candidate(&mut cursor)? {
        if candidate.route_family == RouteFamily::WikidataProperty
            && candidate.source_ref == MABO_QID
            && candidate.property_ref == "P710"
            && candidate.target_ref == EDDIE_MABO_QID
        {
            route = Some(candidate);
            break;
        }
    }
    let route = route.ok_or_else(|| {
        std::io::Error::other("pinned Mabo P710 -> Q975866 route not present")
    })?;
    let observation = wikidata_property_observation(
        "query:mabo:P710:Q975866",
        &acquired,
        &route,
        GetterBackend::SlrNative,
    )
    .map_err(|error| std::io::Error::other(format!("observation adapter: {error:?}")))?;

    let spec = ConsumerSpec {
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        surface_id: "surface:mabo".into(),
        requirements: vec![ConsumerRequirement {
            requirement_id: "participant-identity".into(),
            need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject),
            scope: RequirementScope::SourceManifestation(observation.source_revision_ref.clone()),
        }],
    };
    let payment_stream = reviewed_observation_payment_stream(
        &spec,
        "participant-identity",
        EvidenceCoordinateKind::SameObject,
        "review:mabo:P710:Q975866",
        &observation,
        7,
    )?;
    let mut residual_output = Vec::new();
    let residual_receipt = compile_consumer_residual_stream_review_aware(
        &mut std::io::Cursor::new(payment_stream),
        &spec,
        &mut residual_output,
        8,
    )?;
    if residual_receipt.requirements_paid != 1 || residual_receipt.requirements_unpaid != 0 {
        return Err(std::io::Error::other(
            "reviewed Mabo participant identity requirement did not contract",
        )
        .into());
    }

    let expansion_candidate = from_wikidata_route(
        &residual,
        ResidualClass::Identity,
        &acquired,
        &route,
        ExpansionScoring {
            expected_residual_contraction: 1,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 1,
        },
    )
    .map_err(|error| std::io::Error::other(format!("expansion adapter: {error:?}")))?;
    let identity_resolution = WorldIdentityResolutionReceipt {
        receipt_ref: "identity-resolution:mabo:eddie".into(),
        identity: WorldObjectIdentity::new("world-object:eddie-mabo", EDDIE_MABO_QID)
            .with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo"),
        resolution_kind: WorldIdentityResolutionKind::ExactSameRepresentation,
        evidence_ref: "review:mabo:P710:Q975866".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let prepared = PreparedWorldExpansionCycle {
        next_frontier_ref: "frontier:mabo:participant-identity:1".into(),
        routing: ResidualRouting {
            residual_ref: residual.residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            routing_reason_ref: "consumer:mabo:participant-identity".into(),
        },
        candidates: vec![expansion_candidate],
        review_decision: ReviewDecision::Reviewed,
        disambiguation_outcome: DisambiguationOutcome::NewRelatedObject,
        identity_resolution,
        observation: PostAcquisitionWorldObservation {
            observation_ref: observation.request_ref.clone(),
            source_revision_ref: observation.source_revision_ref.clone(),
            triggering_residual_ref: residual.residual_ref.clone(),
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: residual_receipt.requirements_paid,
            newly_exposed_residuals: vec![],
            pnf_world_disambiguation_ref: "review:mabo:P710:Q975866".into(),
            observation_authority: "experimental_candidate_only",
        },
    };

    let config = load_database_config(None)?;
    let mut sink = PgDiscoveryLineageSink::new(config);
    let mut source = OnePreparedCycle(Some(prepared));
    let mut session = WorldExpansionSession::new(frontier, mabo_world_expansion_policy());
    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 1 },
    )
    .map_err(|error| std::io::Error::other(format!("recurrent runner: {error:?}")))?;

    println!("cycles_committed={}", receipt.cycles_committed);
    println!("novel_identity_classes={}", receipt.final_novel_identity_classes);
    println!("target_identity_classes={}", receipt.target_novel_identity_classes);
    println!("requirements_paid={}", residual_receipt.requirements_paid);
    println!("remaining_open_residuals={:?}", receipt.remaining_open_residual_refs);
    println!("stop_reason={:?}", receipt.stop_reason);
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
