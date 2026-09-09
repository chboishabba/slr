//! Governed execution leaf for NSW point-in-time legislation.
//!
//! This reuses the provider crate's existing governance contract and generic
//! HTTP transport. It performs exactly one official NSW Legislation request;
//! the returned bytes remain a fetch candidate until separately retained,
//! digested, and admitted as an immutable historical artifact.

use crate::execution::{GovernanceError, GovernedExecutionContext};
use crate::official_resource::nsw_legislation::{
    official_point_in_time_xml_request, validate_official_point_in_time_response,
    HistoricalLegislationDemand, HistoricalLegislationError, OfficialPointInTimeFetchCandidate,
};
use crate::transport::HttpTransport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NSWLegislationExecutionError {
    Governance(GovernanceError),
    Historical(HistoricalLegislationError),
    Transport(String),
}

pub fn fetch_official_point_in_time_xml<T: HttpTransport>(
    transport: &mut T,
    context: &GovernedExecutionContext,
    demand: &HistoricalLegislationDemand,
) -> Result<OfficialPointInTimeFetchCandidate, NSWLegislationExecutionError>
where
    T::Error: ToString,
{
    context
        .validate()
        .map_err(NSWLegislationExecutionError::Governance)?;

    let request = official_point_in_time_xml_request(demand)
        .map_err(NSWLegislationExecutionError::Historical)?;
    let response = transport
        .get(&request)
        .map_err(|err| NSWLegislationExecutionError::Transport(err.to_string()))?;

    validate_official_point_in_time_response(demand, &request, response)
        .map_err(NSWLegislationExecutionError::Historical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::official_resource::nsw_legislation::HistoricalLegislationDemand;
    use crate::resolver::LiveGovernanceBounds;
    use crate::transport::{HttpRequest, HttpResponse};

    struct MockTransport {
        response: Option<HttpResponse>,
    }

    impl HttpTransport for MockTransport {
        type Error = String;

        fn get(&mut self, _request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
            self.response.take().ok_or_else(|| "no response".into())
        }
    }

    fn demand() -> HistoricalLegislationDemand {
        HistoricalLegislationDemand {
            demand_ref: "demand:cullen:cla:s5b:2017".into(),
            jurisdiction_ref: "AU-NSW".into(),
            act_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            official_document_id: "act-2002-022".into(),
            requested_locator: "s 5B".into(),
            in_force_on: "2017-01-26".into(),
            proposition_ref: "prop:NSW:CLA:s5B:definition".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
        }
    }

    fn context() -> GovernedExecutionContext {
        GovernedExecutionContext {
            operator_opt_in: true,
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        }
    }

    #[test]
    fn governed_fetch_requires_existing_execution_policy_and_returns_unretained_candidate() {
        let final_url = "https://legislation.nsw.gov.au/view/html/inforce/2017-01-26/act-2002-022/xml";
        let mut transport = MockTransport {
            response: Some(HttpResponse {
                final_url: final_url.into(),
                status_code: 200,
                content_type: Some("application/xml".into()),
                body: b"<legislation/>".to_vec(),
            }),
        };
        let candidate = fetch_official_point_in_time_xml(&mut transport, &context(), &demand())
            .unwrap();
        assert!(!candidate.retained);
        assert_eq!(candidate.network_requests, 1);
    }

    #[test]
    fn governed_fetch_fails_closed_without_operator_opt_in() {
        let mut blocked = context();
        blocked.operator_opt_in = false;
        let mut transport = MockTransport { response: None };
        assert!(matches!(
            fetch_official_point_in_time_xml(&mut transport, &blocked, &demand()),
            Err(NSWLegislationExecutionError::Governance(
                GovernanceError::OperatorOptInRequired
            ))
        ));
    }
}
