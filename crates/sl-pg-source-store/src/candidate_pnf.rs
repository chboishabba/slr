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
    #[error("parser token stream is not monotonic in source coordinates")]
    NonMonotonicTokenStream,
    #[error("no persisted parser output exists for exact span {0}")]
    MissingPersistedParserRegion(String),
    #[error("persisted parser residual for exact span {span_ref}: {error_ref}")]
    PersistedParserResidual { span_ref: String, error_ref: String },
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


#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexedSpacyToken {
    ordinal: u32,
    start: u32,
    end: u32,
    surface: String,
    lemma: String,
    pos: String,
    dependency: String,
}

/// Book-scale compatibility adapter.
///
/// Unlike SpacyTsvAdapter, this parses the TSV once and binary-searches the
/// monotonic source-coordinate token stream for each exact sentence span.
#[derive(Debug, Clone)]
pub struct IndexedSpacyTsvAdapter {
    tokens: Vec<IndexedSpacyToken>,
}

impl IndexedSpacyTsvAdapter {
    pub fn parse(tsv: &str) -> Result<Self, CandidatePnfError> {
        let mut tokens = Vec::new();
        let mut previous_start = None;
        let mut previous_end = None;

        for (line_index, raw) in tsv.lines().enumerate() {
            if !raw.starts_with("T\t") {
                continue;
            }
            let fields: Vec<&str> = raw.split('\t').collect();
            if fields.len() < 10 {
                return Err(CandidatePnfError::MalformedRow {
                    line: line_index + 1,
                    detail: format!(
                        "expected at least 10 tab-separated fields, got {}",
                        fields.len()
                    ),
                });
            }
            let ordinal: u32 =
                fields[1]
                    .parse()
                    .map_err(|_| CandidatePnfError::MalformedRow {
                        line: line_index + 1,
                        detail: "invalid token ordinal".into(),
                    })?;
            let start: u32 =
                fields[2]
                    .parse()
                    .map_err(|_| CandidatePnfError::MalformedRow {
                        line: line_index + 1,
                        detail: "invalid start character".into(),
                    })?;
            let end: u32 =
                fields[3]
                    .parse()
                    .map_err(|_| CandidatePnfError::MalformedRow {
                        line: line_index + 1,
                        detail: "invalid end character".into(),
                    })?;
            if start >= end {
                return Err(CandidatePnfError::MalformedRow {
                    line: line_index + 1,
                    detail: "token span must have start < end".into(),
                });
            }
            if previous_start.is_some_and(|value| start < value)
                || previous_end.is_some_and(|value| end < value)
            {
                return Err(CandidatePnfError::NonMonotonicTokenStream);
            }
            previous_start = Some(start);
            previous_end = Some(end);

            tokens.push(IndexedSpacyToken {
                ordinal,
                start,
                end,
                surface: fields[5].to_owned(),
                lemma: fields[6].to_owned(),
                pos: fields[7].to_owned(),
                dependency: fields[9].to_owned(),
            });
        }

        Ok(Self { tokens })
    }

    #[must_use]
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    fn first_overlap_index(&self, source_start: u32) -> usize {
        self.tokens.partition_point(|token| token.end <= source_start)
    }
}

impl CandidatePnfProducer for IndexedSpacyTsvAdapter {
    fn produce(&self, source: &ExactSourceSpan) -> Result<CandidatePnfBatch, CandidatePnfError> {
        if source.start_char >= source.end_char {
            return Err(CandidatePnfError::InvalidExactSpan);
        }

        let first = self.first_overlap_index(source.start_char);
        let mut candidates = Vec::new();
        for token in &self.tokens[first..] {
            if token.start >= source.end_char {
                break;
            }
            if token.end <= source.start_char {
                continue;
            }
            candidates.push(CandidatePnfFactor {
                candidate_ref: format!(
                    "candidate-pnf:{}:{}:{}:{}",
                    source.span_ref, token.ordinal, token.start, token.end
                ),
                role: role_for(&token.pos, &token.dependency),
                source_start_char: token.start,
                source_end_char: token.end,
                surface: token.surface.clone(),
                lemma: token.lemma.clone(),
                dependency_ref: token.dependency.clone(),
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

pub(crate) fn role_for(pos: &str, dependency: &str) -> CandidatePnfRole {
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


#[cfg(test)]
mod indexed_tests {
    use super::*;

    fn fixture_tsv() -> &'static str {
        "T\t1\t0\t5\t0\tAlice\talice\tPROPN\t2\tnsubj\n\
         T\t2\t6\t12\t0\tcalled\tcall\tVERB\t2\tROOT\n\
         T\t3\t13\t16\t0\tBob\tbob\tPROPN\t2\tdobj\n\
         T\t4\t18\t23\t1\tCarol\tcarol\tPROPN\t5\tnsubj\n\
         T\t5\t24\t30\t1\twrote\twrite\tVERB\t5\tROOT"
    }

    #[test]
    fn indexed_adapter_matches_legacy_span_projection() {
        let indexed = IndexedSpacyTsvAdapter::parse(fixture_tsv()).unwrap();
        let legacy = SpacyTsvAdapter::new(fixture_tsv());
        let span = ExactSourceSpan {
            span_ref: "span:second".into(),
            start_char: 18,
            end_char: 30,
        };
        assert_eq!(indexed.produce(&span).unwrap(), legacy.produce(&span).unwrap());
        assert_eq!(indexed.token_count(), 5);
    }

    #[test]
    fn indexed_adapter_rejects_non_monotonic_source_stream() {
        let bad = "T\t1\t10\t12\t0\tA\ta\tNOUN\t1\tROOT\n\
                   T\t2\t5\t8\t0\tB\tb\tNOUN\t2\tROOT";
        assert_eq!(
            IndexedSpacyTsvAdapter::parse(bad).unwrap_err(),
            CandidatePnfError::NonMonotonicTokenStream
        );
    }
}
