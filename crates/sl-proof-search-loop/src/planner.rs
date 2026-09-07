use crate::hypothesis::{SearchHypothesis, SearchHypothesisKind};
use crate::local::{CandidatePassage, LocalIndex};
use crate::query::{compile_austlii, QueryExpr};
use crate::world::ResearchWorldSnapshot;
use std::collections::BTreeSet;

/// Explicit consumer/PNF-owned lexical seeds for one proof-search hypothesis.
///
/// The planner may combine these seeds with already-learned world vocabulary,
/// but learned text never changes the target proposition or grants proof value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryLexicalContext {
    pub target_phrases: Vec<String>,
    pub target_terms: Vec<String>,
    pub support_terms: Vec<String>,
    pub defeater_terms: Vec<String>,
    pub comparator_terms: Vec<String>,
    pub contradiction_terms: Vec<String>,
    pub treatment_terms: Vec<String>,
    pub forbidden_terms: Vec<String>,
    pub citation_seeds: Vec<String>,
    pub court_ref: Option<String>,
    pub date_range: Option<(String, String)>,
    pub maximum_world_terms: usize,
    pub maximum_world_authorities: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesizedQueryCandidate {
    pub query_ref: String,
    pub hypothesis_ref: String,
    pub residual_ref: String,
    pub target_proposition_ref: String,
    pub expression: QueryExpr,
    pub lexical_provenance_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalQueryExecution {
    pub candidate: SynthesizedQueryCandidate,
    pub passages: Vec<CandidatePassage>,
    pub network_requests: u64,
    pub execution_authority: &'static str,
}

fn normalized(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_string());
        }
    }
    set.into_iter().collect()
}

fn target_atoms(ctx: &QueryLexicalContext) -> Vec<QueryExpr> {
    let mut atoms: Vec<QueryExpr> = normalized(ctx.target_phrases.clone())
        .into_iter()
        .map(QueryExpr::Phrase)
        .collect();
    atoms.extend(normalized(ctx.target_terms.clone()).into_iter().map(QueryExpr::Term));
    atoms
}

fn kind_terms(kind: SearchHypothesisKind, ctx: &QueryLexicalContext) -> Vec<String> {
    match kind {
        SearchHypothesisKind::Support => normalized(ctx.support_terms.clone()),
        SearchHypothesisKind::Defeater => normalized(ctx.defeater_terms.clone()),
        SearchHypothesisKind::Comparator => normalized(ctx.comparator_terms.clone()),
        SearchHypothesisKind::Contradiction => normalized(ctx.contradiction_terms.clone()),
        SearchHypothesisKind::AuthorityTreatment => normalized(ctx.treatment_terms.clone()),
        SearchHypothesisKind::TerminologyExpansion => Vec::new(),
    }
}

fn world_term_candidates(world: &ResearchWorldSnapshot, limit: usize) -> Vec<String> {
    world.query_vocabulary.iter().take(limit).cloned().collect()
}

fn world_authority_candidates(world: &ResearchWorldSnapshot, limit: usize) -> Vec<String> {
    world.authority_neighbourhood.iter().take(limit).cloned().collect()
}

fn with_filters(mut parts: Vec<QueryExpr>, ctx: &QueryLexicalContext) -> QueryExpr {
    for forbidden in normalized(ctx.forbidden_terms.clone()) {
        parts.push(QueryExpr::Not(Box::new(QueryExpr::Term(forbidden))));
    }
    if let Some(court) = &ctx.court_ref {
        parts.push(QueryExpr::Court(court.clone()));
    }
    if let Some((start, end)) = &ctx.date_range {
        parts.push(QueryExpr::DateRange {
            start: start.clone(),
            end: end.clone(),
        });
    }
    match parts.len() {
        0 => QueryExpr::All(Vec::new()),
        1 => parts.into_iter().next().expect("one query part"),
        _ => QueryExpr::All(parts),
    }
}

fn lexical_query(
    hypothesis: &SearchHypothesis,
    ctx: &QueryLexicalContext,
    world: &ResearchWorldSnapshot,
) -> QueryExpr {
    let mut parts = target_atoms(ctx);
    let typed_terms = kind_terms(hypothesis.kind, ctx);
    if !typed_terms.is_empty() {
        parts.push(QueryExpr::Any(
            typed_terms.into_iter().map(QueryExpr::Term).collect(),
        ));
    }

    if hypothesis.kind == SearchHypothesisKind::TerminologyExpansion {
        let learned = world_term_candidates(world, ctx.maximum_world_terms);
        if !learned.is_empty() {
            parts.push(QueryExpr::Any(
                learned.into_iter().map(QueryExpr::Phrase).collect(),
            ));
        }
    }

    with_filters(parts, ctx)
}

fn treatment_queries(
    hypothesis: &SearchHypothesis,
    ctx: &QueryLexicalContext,
    world: &ResearchWorldSnapshot,
) -> Vec<QueryExpr> {
    let mut authorities = normalized(ctx.citation_seeds.clone());
    authorities.extend(world_authority_candidates(
        world,
        ctx.maximum_world_authorities,
    ));
    authorities = normalized(authorities);

    if authorities.is_empty() {
        return vec![lexical_query(hypothesis, ctx, world)];
    }

    authorities
        .into_iter()
        .map(|authority| {
            let mut parts = target_atoms(ctx);
            parts.push(QueryExpr::Citation(authority));
            let treatment = normalized(ctx.treatment_terms.clone());
            if !treatment.is_empty() {
                parts.push(QueryExpr::Any(
                    treatment.into_iter().map(QueryExpr::Term).collect(),
                ));
            }
            with_filters(parts, ctx)
        })
        .collect()
}

pub fn synthesize_queries(
    hypothesis: &SearchHypothesis,
    ctx: &QueryLexicalContext,
    world: &ResearchWorldSnapshot,
) -> Vec<SynthesizedQueryCandidate> {
    let expressions = if hypothesis.kind == SearchHypothesisKind::AuthorityTreatment {
        treatment_queries(hypothesis, ctx, world)
    } else {
        vec![lexical_query(hypothesis, ctx, world)]
    };

    expressions
        .into_iter()
        .enumerate()
        .map(|(index, expression)| SynthesizedQueryCandidate {
            query_ref: format!("query:{}:{index}", hypothesis.hypothesis_ref),
            hypothesis_ref: hypothesis.hypothesis_ref.clone(),
            residual_ref: hypothesis.residual_ref.clone(),
            target_proposition_ref: hypothesis.target_proposition_ref.clone(),
            expression,
            lexical_provenance_refs: vec![
                "pnf-or-consumer-explicit-lexical-context".into(),
                world.snapshot_ref.clone(),
            ],
            candidate_only: true,
        })
        .collect()
}

pub fn execute_local_candidates(
    index: &LocalIndex,
    candidates: &[SynthesizedQueryCandidate],
) -> Vec<LocalQueryExecution> {
    candidates
        .iter()
        .cloned()
        .map(|candidate| LocalQueryExecution {
            passages: index.search(&candidate.expression),
            candidate,
            network_requests: 0,
            execution_authority: "candidate_retrieval_only",
        })
        .collect()
}

/// Stable, provider-rendered key used only for deterministic query de-duplication.
/// It does not make the provider string the semantic query object.
pub fn deterministic_query_key(candidate: &SynthesizedQueryCandidate) -> String {
    compile_austlii(&candidate.expression).query_string
}

pub fn deduplicate_queries(
    candidates: impl IntoIterator<Item = SynthesizedQueryCandidate>,
) -> Vec<SynthesizedQueryCandidate> {
    let mut seen = BTreeSet::new();
    candidates
        .into_iter()
        .filter(|candidate| seen.insert(deterministic_query_key(candidate)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::SearchHypothesisKind;

    fn hypothesis(kind: SearchHypothesisKind) -> SearchHypothesis {
        SearchHypothesis {
            hypothesis_ref: "hyp:r:defeater".into(),
            residual_ref: "r".into(),
            target_proposition_ref: "p".into(),
            kind,
            producer_class_ref: "producer".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("primary-case".into()),
            hypothesis_authority: "experimental_candidate_only",
        }
    }

    #[test]
    fn defeater_query_uses_explicit_seeds_not_implicit_doctrine() {
        let ctx = QueryLexicalContext {
            target_phrases: vec!["duty of care".into()],
            defeater_terms: vec!["policy".into(), "coherence".into()],
            maximum_world_terms: 3,
            ..QueryLexicalContext::default()
        };
        let world = ResearchWorldSnapshot::default();
        let query = synthesize_queries(&hypothesis(SearchHypothesisKind::Defeater), &ctx, &world);
        let rendered = deterministic_query_key(&query[0]);
        assert!(rendered.contains("duty of care"));
        assert!(rendered.contains("policy"));
        assert!(rendered.contains("coherence"));
    }

    #[test]
    fn treatment_search_can_reuse_learned_authority_neighbourhood() {
        let ctx = QueryLexicalContext {
            target_phrases: vec!["duty of care".into()],
            treatment_terms: vec!["distinguished".into(), "followed".into()],
            maximum_world_authorities: 2,
            ..QueryLexicalContext::default()
        };
        let mut world = ResearchWorldSnapshot::default();
        world.authority_neighbourhood.insert("authority:donoghue".into());
        let queries = synthesize_queries(
            &hypothesis(SearchHypothesisKind::AuthorityTreatment),
            &ctx,
            &world,
        );
        assert_eq!(queries.len(), 1);
        assert!(deterministic_query_key(&queries[0]).contains("authority:donoghue"));
    }
}
