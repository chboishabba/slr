use crate::frontier::{FrontierCandidateMove, ProofFrontier};
use crate::hypothesis::{families_for_frontier, SearchHypothesis};
use crate::local::{CandidatePassage, LocalIndex};
use crate::planner::{
    deduplicate_queries, deterministic_query_key, execute_local_candidates, synthesize_queries,
    QueryLexicalContext, SynthesizedQueryCandidate,
};
use crate::world::ResearchWorldSnapshot;
use sensiblaw_proof_search_scheduler::{
    CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredResearchValue {
    pub expected_residual_reduction: u64,
    pub discriminative_value: u64,
    pub authority_fitness: u64,
    pub novelty: u64,
    pub coverage_gain: u64,
    pub execution_cost: ExecutionCostVector,
    pub calibration_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedLocalResearchMove {
    pub query_key: String,
    pub query: SynthesizedQueryCandidate,
    pub supporting_hypothesis_refs: Vec<String>,
    pub target_residual_refs: Vec<String>,
    pub local_passages: Vec<CandidatePassage>,
    pub frontier_move: FrontierCandidateMove,
    pub planning_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchPlanningError {
    MissingLexicalContext(String),
    MissingDeclaredValue(String),
}

fn componentwise_max_cost(values: &[&DeclaredResearchValue]) -> ExecutionCostVector {
    let mut out = ExecutionCostVector::default();
    for value in values {
        let cost = value.execution_cost;
        out.network_requests = out.network_requests.max(cost.network_requests);
        out.minimum_pacing_seconds = out.minimum_pacing_seconds.max(cost.minimum_pacing_seconds);
        out.citation_depth = out.citation_depth.max(cost.citation_depth);
        out.maximum_new_documents = out.maximum_new_documents.max(cost.maximum_new_documents);
        out.cache_misses = out.cache_misses.max(cost.cache_misses);
        out.local_bytes_read_cost = out.local_bytes_read_cost.max(cost.local_bytes_read_cost);
        out.parser_pnf_cost = out.parser_pnf_cost.max(cost.parser_pnf_cost);
        out.semantic_assessment_cost = out.semantic_assessment_cost.max(cost.semantic_assessment_cost);
        out.operator_review_cost = out.operator_review_cost.max(cost.operator_review_cost);
    }
    out
}

fn conservative_group_value(values: &[&DeclaredResearchValue], target_count: usize) -> ProofValueVector {
    let expected_proof_reduction = values
        .iter()
        .map(|value| value.expected_residual_reduction)
        .max()
        .unwrap_or(0);
    let discriminative_value = values
        .iter()
        .map(|value| value.discriminative_value)
        .min()
        .unwrap_or(0);
    let authority_fitness = values
        .iter()
        .map(|value| value.authority_fitness)
        .min()
        .unwrap_or(0);
    let novelty = values.iter().map(|value| value.novelty).max().unwrap_or(0);
    let declared_coverage = values.iter().map(|value| value.coverage_gain).max().unwrap_or(0);
    ProofValueVector {
        expected_proof_reduction,
        discriminative_value,
        authority_fitness,
        novelty,
        coverage_gain: declared_coverage.max(target_count as u64),
    }
}

fn candidate_move_for_group(
    key: &str,
    values: &[&DeclaredResearchValue],
    target_count: usize,
) -> CandidateMove {
    CandidateMove {
        move_ref: format!("move:local-query:{key}"),
        strategy: ExecutionStrategy::LocalWorldGraph,
        source_ref: None,
        provider_operation_ref: "local-query-execution".into(),
        cost: componentwise_max_cost(values),
        value: conservative_group_value(values, target_count),
        admissible: true,
        calibration_ref: values
            .iter()
            .map(|value| value.calibration_ref.as_str())
            .collect::<Vec<_>>()
            .join("+"),
    }
}

/// Build and execute candidate-only local research moves across the complete open frontier.
///
/// Query hits decide only whether a local query produced candidate passages. Expected proof
/// reduction and authority fitness remain separately declared calibration inputs.
pub fn plan_local_frontier_research(
    frontier: &ProofFrontier,
    world: &ResearchWorldSnapshot,
    index: &LocalIndex,
    lexical_contexts: &BTreeMap<String, QueryLexicalContext>,
    declared_values: &BTreeMap<String, DeclaredResearchValue>,
) -> Result<Vec<PlannedLocalResearchMove>, ResearchPlanningError> {
    let hypotheses = families_for_frontier(frontier);
    let mut grouped: BTreeMap<String, Vec<(SearchHypothesis, SynthesizedQueryCandidate)>> =
        BTreeMap::new();

    for hypothesis in hypotheses {
        let context = lexical_contexts
            .get(&hypothesis.hypothesis_ref)
            .ok_or_else(|| ResearchPlanningError::MissingLexicalContext(hypothesis.hypothesis_ref.clone()))?;
        if !declared_values.contains_key(&hypothesis.hypothesis_ref) {
            return Err(ResearchPlanningError::MissingDeclaredValue(
                hypothesis.hypothesis_ref.clone(),
            ));
        }
        let queries = deduplicate_queries(synthesize_queries(&hypothesis, context, world));
        for query in queries {
            grouped
                .entry(deterministic_query_key(&query))
                .or_default()
                .push((hypothesis.clone(), query));
        }
    }

    let mut moves = Vec::new();
    for (key, group) in grouped {
        let representative = group[0].1.clone();
        let executions = execute_local_candidates(index, std::slice::from_ref(&representative));
        let passages = executions
            .into_iter()
            .next()
            .expect("one synthesized query has one local execution")
            .passages;
        if passages.is_empty() {
            continue;
        }

        let supporting_hypothesis_refs: Vec<String> = group
            .iter()
            .map(|(hypothesis, _)| hypothesis.hypothesis_ref.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let target_residual_refs: Vec<String> = group
            .iter()
            .map(|(hypothesis, _)| hypothesis.residual_ref.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let values: Vec<&DeclaredResearchValue> = supporting_hypothesis_refs
            .iter()
            .map(|hypothesis_ref| {
                declared_values
                    .get(hypothesis_ref)
                    .expect("declared value checked during synthesis")
            })
            .collect();
        let move_ = candidate_move_for_group(&key, &values, target_residual_refs.len());
        let frontier_move = FrontierCandidateMove {
            target_residual_refs: target_residual_refs.clone(),
            expected_whole_frontier_reduction: move_.value.expected_proof_reduction,
            shared_dependency_gain: target_residual_refs.len().saturating_sub(1) as u64,
            move_,
        };
        moves.push(PlannedLocalResearchMove {
            query_key: key,
            query: representative,
            supporting_hypothesis_refs,
            target_residual_refs,
            local_passages: passages,
            frontier_move,
            planning_authority: "experimental_candidate_only",
        });
    }

    moves.sort_by(|left, right| left.query_key.cmp(&right.query_key));
    Ok(moves)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofResidual, ResidualStatus};
    use crate::local::LocalDocument;
    use crate::hypothesis::family_for_residual;

    fn frontier() -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:pabai".into(),
            frontier_ref: "frontier:pabai".into(),
            residuals: vec![ProofResidual {
                residual_ref: "residual:comparator".into(),
                proposition_ref: "prop:positive-operational-act".into(),
                producer_class_ref: "comparator-producer".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("primary-case".into()),
                salience: 5,
                dependency_refs: vec![],
                status: ResidualStatus::Open,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    #[test]
    fn local_hits_do_not_supply_declared_proof_value() {
        let frontier = frontier();
        let hypotheses = family_for_residual(&frontier.residuals[0]);
        let mut lexical = BTreeMap::new();
        let mut values = BTreeMap::new();
        for hypothesis in hypotheses {
            lexical.insert(
                hypothesis.hypothesis_ref.clone(),
                QueryLexicalContext {
                    target_phrases: vec!["positive operational act".into()],
                    maximum_world_terms: 3,
                    maximum_world_authorities: 3,
                    ..QueryLexicalContext::default()
                },
            );
            values.insert(
                hypothesis.hypothesis_ref,
                DeclaredResearchValue {
                    expected_residual_reduction: 2,
                    discriminative_value: 3,
                    authority_fitness: 4,
                    novelty: 1,
                    coverage_gain: 1,
                    execution_cost: ExecutionCostVector {
                        local_bytes_read_cost: 1,
                        ..ExecutionCostVector::default()
                    },
                    calibration_ref: "fixture:declared".into(),
                },
            );
        }
        let mut index = LocalIndex::default();
        index.insert(LocalDocument {
            document_ref: "doc:cullen".into(),
            source_revision_ref: "source:cullen:rev:1".into(),
            jurisdiction_ref: Some("AU".into()),
            court_ref: Some("HCA".into()),
            date_ref: Some("2015".into()),
            canonical_text: "The positive operational act was considered.".into(),
            citation_refs: vec![],
        });
        let planned = plan_local_frontier_research(
            &frontier,
            &ResearchWorldSnapshot::default(),
            &index,
            &lexical,
            &values,
        )
        .unwrap();
        assert!(!planned.is_empty());
        assert!(planned.iter().all(|planned| {
            planned.frontier_move.move_.value.expected_proof_reduction == 2
        }));
    }
}
