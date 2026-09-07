use sensiblaw_governed_legal_provider::*;

struct FixtureTransport {
    response: Option<HttpResponse>,
}

impl HttpTransport for FixtureTransport {
    type Error = String;

    fn get(&mut self, _request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
        self.response.take().ok_or_else(|| "fixture transport exhausted".into())
    }
}

fn main() {
    let source_identity = "case:[2026]-HCA-19";
    let proposition = "prop:cullen-positive-operational-duty";
    let citation = "[2026] HCA 19";
    let official = official_hca_known_reference(citation).expect("known HCA calibration must resolve");

    let demand = KnownAuthorityDemand {
        demand_ref: "duty:cullen:source".into(),
        jurisdiction_ref: "AU".into(),
        source_identity_ref: source_identity.into(),
        medium_neutral_citation: Some(citation.into()),
        explicit_austlii_ref: None,
        proposition_ref: Some(proposition.into()),
        use_intent: PropositionUseIntent::SourceProposition,
        treatment_intent: CitationTreatmentIntent::CitedBy,
    };

    let first_resolution = resolve_known_authority(&demand, &ResolutionContext::default());
    assert!(matches!(
        first_resolution.stage,
        ResolutionStage::OfficialHighCourt { .. }
    ));

    let transport = FixtureTransport {
        response: Some(HttpResponse {
            final_url: official.clone(),
            status_code: 200,
            content_type: Some("text/html".into()),
            body: b"<html><h1>Cullen official HCA fixture</h1></html>".to_vec(),
        }),
    };
    let context = GovernedExecutionContext {
        operator_opt_in: true,
        cache_checked_first: true,
        persisted_receipts_checked_first: true,
        bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
    };
    let mut executor = GovernedExecutor::new(transport, context).expect("governance preflight");
    let fetched = executor.fetch_hca(&official).expect("bounded official fixture fetch");
    assert_eq!(executor.network_requests(), 1);
    assert_eq!(fetched.provider, LegalProvider::HighCourtAustralia);
    assert!(!fetched.locally_ingested);

    let ingested = mark_locally_ingested(
        &fetched,
        source_identity,
        "source:hca:2026:19:rev:fixture-1",
        "sha256:hca-cullen-fixture",
    );
    assert!(ingested.locally_ingested);

    let replay_context = ResolutionContext {
        persisted: vec![PersistedAuthorityReceipt {
            source_identity_ref: source_identity.into(),
            source_revision_ref: ingested.source_revision_ref.clone(),
            jurisdiction_ref: "AU".into(),
            compile_eligible: true,
        }],
        ..ResolutionContext::default()
    };
    let replay = resolve_known_authority(&demand, &replay_context);
    assert!(matches!(replay.stage, ResolutionStage::Persisted { .. }));

    println!(
        "provider=HighCourtAustralia first_network_requests=1 replay_network_requests=0 source={} proposition={} authority={}",
        source_identity, proposition, RECEIPT_AUTHORITY
    );
}
