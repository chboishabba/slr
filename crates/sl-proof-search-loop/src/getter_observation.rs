#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GetterBackend {
    SlrNative,
    ExternalFormalMachine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetrievalStatus {
    Retrieved,
    Missing,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FreshnessStatus {
    Pinned,
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationQuery {
    pub query_ref: String,
    pub object_ref: String,
    pub relation_ref: String,
    pub source_ref: String,
    pub requested_revision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetterObservation {
    pub backend: GetterBackend,
    pub query: ObservationQuery,
    pub observed_value_ref: String,
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub retrieval_status: RetrievalStatus,
    pub freshness_status: FreshnessStatus,
    pub receipt_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl GetterObservation {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn candidate_only(
        backend: GetterBackend,
        query: ObservationQuery,
        observed_value_ref: impl Into<String>,
        source_revision_ref: impl Into<String>,
        content_digest_ref: impl Into<String>,
        retrieval_status: RetrievalStatus,
        freshness_status: FreshnessStatus,
        receipt_ref: impl Into<String>,
    ) -> Self {
        Self {
            backend,
            query,
            observed_value_ref: observed_value_ref.into(),
            source_revision_ref: source_revision_ref.into(),
            content_digest_ref: content_digest_ref.into(),
            retrieval_status,
            freshness_status,
            receipt_ref: receipt_ref.into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GetterMismatchKind {
    ValueMismatch,
    RevisionMismatch,
    DigestMismatch,
    RetrievalMismatch,
    FreshnessMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationParityStatus {
    Agreement,
    Residual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetterParityComparison {
    pub query_ref: String,
    pub left_backend: GetterBackend,
    pub right_backend: GetterBackend,
    pub status: ObservationParityStatus,
    pub mismatches: Vec<GetterMismatchKind>,
    pub residual_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetterParityError {
    QueryMismatch,
    ObservationMayNotPromote,
    EmptyCoordinate(&'static str),
}

fn same_query(left: &ObservationQuery, right: &ObservationQuery) -> bool {
    left.query_ref == right.query_ref
        && left.object_ref == right.object_ref
        && left.relation_ref == right.relation_ref
        && left.source_ref == right.source_ref
        && left.requested_revision_ref == right.requested_revision_ref
}

pub fn compare_normalized_observations(
    left: &GetterObservation,
    right: &GetterObservation,
) -> Result<GetterParityComparison, GetterParityError> {
    if !same_query(&left.query, &right.query) {
        return Err(GetterParityError::QueryMismatch);
    }
    if !left.candidate_only
        || !right.candidate_only
        || left.creates_semantic_authority
        || right.creates_semantic_authority
        || left.creates_claim_truth
        || right.creates_claim_truth
    {
        return Err(GetterParityError::ObservationMayNotPromote);
    }
    for (name, value) in [
        ("query_ref", left.query.query_ref.as_str()),
        ("object_ref", left.query.object_ref.as_str()),
        ("relation_ref", left.query.relation_ref.as_str()),
        ("source_ref", left.query.source_ref.as_str()),
        ("source_revision_ref", left.source_revision_ref.as_str()),
        ("content_digest_ref", left.content_digest_ref.as_str()),
        ("receipt_ref", left.receipt_ref.as_str()),
        ("right_receipt_ref", right.receipt_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(GetterParityError::EmptyCoordinate(name));
        }
    }

    let mut mismatches = Vec::new();
    if left.observed_value_ref != right.observed_value_ref {
        mismatches.push(GetterMismatchKind::ValueMismatch);
    }
    if left.source_revision_ref != right.source_revision_ref {
        mismatches.push(GetterMismatchKind::RevisionMismatch);
    }
    if left.content_digest_ref != right.content_digest_ref {
        mismatches.push(GetterMismatchKind::DigestMismatch);
    }
    if left.retrieval_status != right.retrieval_status {
        mismatches.push(GetterMismatchKind::RetrievalMismatch);
    }
    if left.freshness_status != right.freshness_status {
        mismatches.push(GetterMismatchKind::FreshnessMismatch);
    }
    let status = if mismatches.is_empty() {
        ObservationParityStatus::Agreement
    } else {
        ObservationParityStatus::Residual
    };
    let residual_ref = (status == ObservationParityStatus::Residual)
        .then(|| format!("getter-parity:{}", left.query.query_ref));

    Ok(GetterParityComparison {
        query_ref: left.query.query_ref.clone(),
        left_backend: left.backend,
        right_backend: right.backend,
        status,
        mismatches,
        residual_ref,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> ObservationQuery {
        ObservationQuery {
            query_ref: "query:mabo:P710".into(),
            object_ref: "Q1501525".into(),
            relation_ref: "P710".into(),
            source_ref: "wikidata".into(),
            requested_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        }
    }

    #[test]
    fn native_and_external_backends_normalize_to_same_semantic_observation() {
        let native = GetterObservation::candidate_only(
            GetterBackend::SlrNative,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409615",
            "sha256:mabo-revision",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:slr:mabo:P710",
        );
        let external = GetterObservation::candidate_only(
            GetterBackend::ExternalFormalMachine,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409615",
            "sha256:mabo-revision",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:lean:mabo:P710",
        );

        let comparison = compare_normalized_observations(&native, &external).unwrap();
        assert_eq!(comparison.status, ObservationParityStatus::Agreement);
        assert!(comparison.mismatches.is_empty());
        assert!(!comparison.creates_claim_truth);
        assert!(!comparison.creates_semantic_authority);
    }

    #[test]
    fn revision_mismatch_becomes_residual_not_truth_judgment() {
        let native = GetterObservation::candidate_only(
            GetterBackend::SlrNative,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409615",
            "sha256:a",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:slr",
        );
        let external = GetterObservation::candidate_only(
            GetterBackend::ExternalFormalMachine,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409000",
            "sha256:b",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:external",
        );

        let comparison = compare_normalized_observations(&native, &external).unwrap();
        assert_eq!(comparison.status, ObservationParityStatus::Residual);
        assert!(comparison.mismatches.contains(&GetterMismatchKind::RevisionMismatch));
        assert!(comparison.mismatches.contains(&GetterMismatchKind::DigestMismatch));
        assert_eq!(comparison.residual_ref.as_deref(), Some("getter-parity:query:mabo:P710"));
    }

    #[test]
    fn comparison_requires_same_query_and_candidate_only_receipts() {
        let left = GetterObservation::candidate_only(
            GetterBackend::SlrNative,
            query(),
            "Q975866",
            "rev",
            "digest",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:left",
        );
        let mut other_query = query();
        other_query.relation_ref = "P31".into();
        let right = GetterObservation::candidate_only(
            GetterBackend::ExternalFormalMachine,
            other_query,
            "Q5",
            "rev",
            "digest",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:right",
        );
        assert_eq!(
            compare_normalized_observations(&left, &right),
            Err(GetterParityError::QueryMismatch)
        );
    }
}
