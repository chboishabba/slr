use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactSourceSpan {
    pub span_ref: String,
    pub start_char: u32,
    pub end_char: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidatePnfRole {
    Actor,
    Predicate,
    Patient,
    Qualifier,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePnfFactor {
    pub candidate_ref: String,
    pub role: CandidatePnfRole,
    pub source_start_char: u32,
    pub source_end_char: u32,
    pub surface: String,
    pub lemma: String,
    pub dependency_ref: String,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePnfBatch {
    pub exact_span_ref: String,
    pub candidates: Vec<CandidatePnfFactor>,
    pub proposition_support_paid: bool,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CandidatePnfError {
    #[error("exact source span must have start < end")]
    InvalidExactSpan,
    #[error("malformed parser row at line {line}: {detail}")]
    MalformedRow { line: usize, detail: String },
}

pub trait CandidatePnfProducer {
    fn produce(&self, source: &ExactSourceSpan) -> Result<CandidatePnfBatch, CandidatePnfError>;
}

/// Temporary compatibility adapter for the historical spaCy TSV observation
/// stream. The stable contract is `CandidatePnfProducer`; TSV is not semantic
/// identity and downstream code must not depend on this representation.
pub struct SpacyTsvAdapter<'a> {
    tsv: &'a str,
}

impl<'a> SpacyTsvAdapter<'a> {
    #[must_use]
    pub const fn new(tsv: &'a str) -> Self {
        Self { tsv }
    }
}

fn role_for(pos: &str, dependency: &str) -> CandidatePnfRole {
    match dependency {
        "nsubj" | "nsubjpass" | "csubj" | "csubjpass" | "agent" => CandidatePnfRole::Actor,
        "dobj" | "obj" | "pobj" | "attr" | "oprd" => CandidatePnfRole::Patient,
        "amod" | "advmod" | "nmod" | "npadvmod" | "prep" | "acl" | "relcl" => {
            CandidatePnfRole::Qualifier
        }
        "ROOT" if pos == "VERB" || pos == "AUX" => CandidatePnfRole::Predicate,
        _ if pos == "VERB" || pos == "AUX" => CandidatePnfRole::Predicate,
        _ => CandidatePnfRole::Other,
    }
}

fn overlaps(start: u32, end: u32, source: &ExactSourceSpan) -> bool {
    start < source.end_char && end > source.start_char
}

impl CandidatePnfProducer for SpacyTsvAdapter<'_> {
    fn produce(&self, source: &ExactSourceSpan) -> Result<CandidatePnfBatch, CandidatePnfError> {
        if source.start_char >= source.end_char {
            return Err(CandidatePnfError::InvalidExactSpan);
        }

        let mut candidates = Vec::new();
        for (line_index, raw) in self.tsv.lines().enumerate() {
            if !raw.starts_with("T\t") {
                continue;
            }
            let fields: Vec<&str> = raw.split('\t').collect();
            if fields.len() < 10 {
                return Err(CandidatePnfError::MalformedRow {
                    line: line_index + 1,
                    detail: format!("expected at least 10 tab-separated fields, got {}", fields.len()),
                });
            }
            let ordinal: u32 = fields[1].parse().map_err(|_| CandidatePnfError::MalformedRow {
                line: line_index + 1,
                detail: "invalid token ordinal".into(),
            })?;
            let start: u32 = fields[2].parse().map_err(|_| CandidatePnfError::MalformedRow {
                line: line_index + 1,
                detail: "invalid start character".into(),
            })?;
            let end: u32 = fields[3].parse().map_err(|_| CandidatePnfError::MalformedRow {
                line: line_index + 1,
                detail: "invalid end character".into(),
            })?;
            if start >= end {
                return Err(CandidatePnfError::MalformedRow {
                    line: line_index + 1,
                    detail: "token span must have start < end".into(),
                });
            }
            if !overlaps(start, end, source) {
                continue;
            }

            let surface = fields[5].to_owned();
            let lemma = fields[6].to_owned();
            let pos = fields[7];
            let dependency = fields[9].to_owned();
            candidates.push(CandidatePnfFactor {
                candidate_ref: format!(
                    "candidate-pnf:{}:{}:{}:{}",
                    source.span_ref, ordinal, start, end
                ),
                role: role_for(pos, &dependency),
                source_start_char: start,
                source_end_char: end,
                surface,
                lemma,
                dependency_ref: dependency,
                candidate_only: true,
            });
        }

        Ok(CandidatePnfBatch {
            exact_span_ref: source.span_ref.clone(),
            candidates,
            proposition_support_paid: false,
            applicability_paid: false,
            claim_truth_paid: false,
        })
    }
}
