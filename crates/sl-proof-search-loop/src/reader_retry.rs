use crate::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_reader_model::{ReaderDisposition, ResidualRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderRetryError {
    ReaderNotDeferred,
    UnsupportedReaderResidual(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderRetryPlan {
    pub proposition_ref: String,
    pub frontier: ProofFrontier,
    pub acquisition_is_semantic_payment: bool,
}

fn producer_for(residual: &ResidualRef) -> Option<&'static str> {
    match residual.as_str() {
        "reader-residual:exact-authority-span" => Some("producer:exact-primary-authority"),
        "reader-residual:proposition-support" | "reader-residual:source-provenance-weld" => {
            Some("producer:reviewed-pnf-proposition-support")
        }
        "reader-residual:qualifier" => Some("producer:proposition-qualifier"),
        "reader-residual:defeater" => Some("producer:proposition-defeater"),
        "reader-residual:comparator" => Some("producer:proposition-comparator"),
        _ => None,
    }
}

fn salience_for(residual: &ResidualRef) -> u64 {
    match residual.as_str() {
        "reader-residual:exact-authority-span"
        | "reader-residual:proposition-support"
        | "reader-residual:source-provenance-weld" => 10,
        _ => 5,
    }
}

/// Compile reader-facing missing coordinates into the existing proof-search
/// frontier. This plans work only; acquisition and persistence remain owned by
/// the established provider/world path and are not semantic payment.
pub fn plan_reader_retry(
    disposition: &ReaderDisposition,
    proposition_ref: &str,
    jurisdiction_ref: Option<&str>,
) -> Result<ReaderRetryPlan, ReaderRetryError> {
    let ReaderDisposition::Defer(residuals) = disposition else {
        return Err(ReaderRetryError::ReaderNotDeferred);
    };

    let mut proof_residuals = Vec::with_capacity(residuals.len());
    for residual in residuals {
        let producer = producer_for(residual).ok_or_else(|| {
            ReaderRetryError::UnsupportedReaderResidual(residual.as_str().to_owned())
        })?;
        proof_residuals.push(ProofResidual {
            residual_ref: residual.as_str().to_owned(),
            proposition_ref: proposition_ref.to_owned(),
            producer_class_ref: producer.to_owned(),
            jurisdiction_ref: jurisdiction_ref.map(str::to_owned),
            authority_requirement_ref: (producer == "producer:exact-primary-authority")
                .then(|| "official-primary-case".to_owned()),
            salience: salience_for(residual),
            dependency_refs: vec![proposition_ref.to_owned()],
            status: ResidualStatus::Open,
        });
    }

    Ok(ReaderRetryPlan {
        proposition_ref: proposition_ref.to_owned(),
        frontier: ProofFrontier {
            consumer_ref: "consumer:semantic-reader".into(),
            frontier_ref: format!("frontier:semantic-reader:{proposition_ref}"),
            residuals: proof_residuals,
            satisfied_payment_refs: Vec::new(),
            contested_coordinate_refs: Vec::new(),
            authority_blocked_refs: Vec::new(),
            authority: "experimental_candidate_only",
        },
        acquisition_is_semantic_payment: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderRetryOutcome {
    Executed { before: ReaderDisposition, after: ReaderDisposition },
    StillDeferred { before: ReaderDisposition, after: ReaderDisposition },
    Rejected { before: ReaderDisposition, after: ReaderDisposition },
}

impl ReaderRetryOutcome {
    #[must_use]
    pub const fn re_evaluated(&self) -> bool { true }
    #[must_use]
    pub const fn acquisition_receipt_is_payment(&self) -> bool { false }
}

/// Re-evaluate the reader only after the acquisition/persistence/world-extension
/// lane has run. The extension receipt itself never closes the reader.
pub fn retry_after_extension<F>(
    before: ReaderDisposition,
    re_evaluate: F,
) -> Result<ReaderRetryOutcome, ReaderRetryError>
where
    F: FnOnce() -> ReaderDisposition,
{
    if !matches!(&before, ReaderDisposition::Defer(_)) {
        return Err(ReaderRetryError::ReaderNotDeferred);
    }
    let after = re_evaluate();
    Ok(match &after {
        ReaderDisposition::ExecuteSource { .. } | ReaderDisposition::ExecuteBoundedWhy(_) => {
            ReaderRetryOutcome::Executed { before, after }
        }
        ReaderDisposition::Defer(_) => ReaderRetryOutcome::StillDeferred { before, after },
        ReaderDisposition::Reject { .. } => ReaderRetryOutcome::Rejected { before, after },
    })
}
