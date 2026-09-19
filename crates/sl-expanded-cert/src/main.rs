mod relation_attachment;

use relation_attachment::{
    RELATION_ATTACHMENT_KINDS, direct_candidates as direct_relation_candidates,
    kind_from_dependency_label, reference_candidates as reference_relation_candidates,
};
use sensiblaw_core::{
    ActiveTimer, Annotation, Capability, HeadDeclaration, RevisionId, SentenceId, SymbolTable,
    TextSpan, TokenId, TokenObservation,
};
use sensiblaw_semantic_admission::{RESIDUAL_KINDS, ResidualFrontier, residual_kind_name};
use sensiblaw_semantic_expansion::{
    ExpansionResidualKind, ExpansionSignal, ExpandedConsumerObservation, StableResidualObservation,
    compile_expanded_candidates, compile_expanded_direct, expanded_consumer_observation,
};
use std::collections::BTreeMap;
use std::io::{self, BufRead};
use std::time::Instant;

fn unesc(s: &str) -> String {
    s.replace("\\t", "\t")
        .replace("\\n", "\n")
        .replace("\\\\", "\\")
}

fn signal(dep: &str) -> ExpansionSignal {
    match dep {
        "nsubj" | "nsubjpass" => ExpansionSignal::NominalSubject,
        "obj" | "dobj" => ExpansionSignal::DirectObject,
        "neg" => ExpansionSignal::Negation,
        "aux" | "auxpass" => ExpansionSignal::ModalAuxiliary,
        "npadvmod" | "tmod" => ExpansionSignal::TemporalModifier,
        "mark" => ExpansionSignal::ConditionalMarker,
        "advcl" | "ccomp" | "xcomp" => ExpansionSignal::ClausalModifier,
        "relcl" | "acl" => ExpansionSignal::ReferenceAttachment,
        "amod" | "advmod" | "appos" | "nmod" => ExpansionSignal::QualifierAttachment,
        _ => ExpansionSignal::Unsupported,
    }
}

fn observation_summary(observation: &ExpandedConsumerObservation) -> String {
    format!(
        "sentence={} candidates={} residuals={} alternatives={} projection_failures={}",
        observation.sentence_id,
        observation.candidates.len(),
        observation.residuals.len(),
        observation.alternative_fibres.len(),
        observation.projection_failures.len(),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut symbols = SymbolTable::default();
    let mut framing_active = ActiveTimer::default();
    let mut direct_active = ActiveTimer::default();
    let mut reference_active = ActiveTimer::default();
    let mut residual_frontier = ResidualFrontier::default();
    let mut unsupported_dependency_fibre = BTreeMap::<String, u64>::new();
    let mut relation_attachment_frontier = BTreeMap::<&'static str, u64>::new();
    let mut revision: RevisionId = 1;
    let mut sentence_id: SentenceId = 0;
    let mut pending = Vec::<TokenObservation>::new();
    let mut next_token: TokenId = 1;
    let mut pipeline_start: Option<Instant> = None;
    let mut parity_enabled = false;

    let mut sentences = 0u64;
    let mut paragraphs = 0u64;
    let mut candidates = 0u64;
    let mut residuals = 0u64;
    let mut alternatives = 0u64;
    let mut projection_failures = 0u64;
    let mut parity_checked = 0u64;
    let mut parity_failed = 0u64;
    let mut relation_candidates = 0u64;
    let mut relation_parity_checked = 0u64;
    let mut relation_parity_failed = 0u64;

    for line in stdin.lock().lines() {
        let line = line?;
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.is_empty() {
            continue;
        }
        match fields[0] {
            "C" => parity_enabled = fields.get(1).copied() == Some("parity=1"),
            "D" => {
                revision = fields.get(1).and_then(|value| value.parse().ok()).unwrap_or(1);
                if pipeline_start.is_none() {
                    pipeline_start = Some(Instant::now());
                }
            }
            "P" => paragraphs = paragraphs.saturating_add(1),
            "S" => {
                sentence_id = fields.get(1).and_then(|value| value.parse().ok()).unwrap_or(0);
                pending.clear();
            }
            "T" => {
                framing_active.measure(|| {
                    if fields.len() < 10 {
                        return;
                    }
                    let local: u32 = fields[1].parse().unwrap_or(0);
                    let start: u32 = fields[2].parse().unwrap_or(0);
                    let end: u32 = fields[3].parse().unwrap_or(start);
                    let head_local: i64 = fields[4].parse().unwrap_or(-1);
                    let orth = symbols.intern(&unesc(fields[5]));
                    let lemma_text = unesc(fields[6]);
                    let lemma = if fields[6] == "-" {
                        Annotation::Unavailable(Capability::Lemma)
                    } else {
                        Annotation::Present(symbols.intern(&lemma_text))
                    };
                    let pos = if fields[7] == "-" {
                        Annotation::Unavailable(Capability::Pos)
                    } else {
                        Annotation::Present(symbols.intern(fields[7]))
                    };
                    let tag = if fields[8] == "-" {
                        Annotation::Unavailable(Capability::Tag)
                    } else {
                        Annotation::Present(symbols.intern(fields[8]))
                    };
                    let dep_text = fields[9];
                    let dependency = if dep_text == "-" {
                        Annotation::Unavailable(Capability::Dependency)
                    } else {
                        Annotation::Present(symbols.intern(dep_text))
                    };
                    let declared_head = if head_local < 0 || head_local as u32 == local {
                        HeadDeclaration::SelfHead
                    } else {
                        HeadDeclaration::LocalOrdinal(head_local as u32)
                    };
                    if let Ok(span) = TextSpan::new(revision, start, end) {
                        pending.push(TokenObservation {
                            token_id: next_token,
                            sentence_id,
                            local_ordinal: local,
                            span,
                            orth,
                            lemma,
                            pos,
                            tag,
                            dependency,
                            declared_head,
                        });
                        next_token = next_token.saturating_add(1);
                    }
                });
            }
            "E" => {
                let observations = std::mem::take(&mut pending);
                let direct = direct_active.measure(|| {
                    compile_expanded_direct(observations.clone(), |symbol| {
                        symbols.get(symbol).map(signal).unwrap_or(ExpansionSignal::Unsupported)
                    })
                });
                for residual in &direct.residuals {
                    residual_frontier.observe_residual(&StableResidualObservation {
                        kind: residual.kind,
                        address: residual.address,
                    });
                    if residual.kind == ExpansionResidualKind::UnsupportedDependency {
                        let label = observations
                            .iter()
                            .find(|token| token.local_ordinal == residual.address.local_ordinal)
                            .and_then(|token| match token.dependency {
                                Annotation::Present(symbol) => symbols.get(symbol),
                                Annotation::Unavailable(_) => None,
                            })
                            .unwrap_or("<unavailable>");
                        let count = unsupported_dependency_fibre.entry(label.to_owned()).or_default();
                        *count = count.saturating_add(1);
                    }
                }

                let direct_relations = direct_relation_candidates(&observations, |symbol| {
                    symbols.get(symbol).and_then(kind_from_dependency_label)
                });
                relation_candidates =
                    relation_candidates.saturating_add(direct_relations.len() as u64);
                for candidate in &direct_relations {
                    let count = relation_attachment_frontier
                        .entry(candidate.kind.name())
                        .or_default();
                    *count = count.saturating_add(1);
                }

                candidates = candidates.saturating_add(direct.candidates.len() as u64);
                residuals = residuals.saturating_add(direct.residuals.len() as u64);
                alternatives = alternatives.saturating_add(direct.alternative_fibres.len() as u64);
                projection_failures = projection_failures
                    .saturating_add(direct.projection_failures.len() as u64);
                sentences = sentences.saturating_add(1);

                if parity_enabled {
                    let reference = reference_active.measure(|| {
                        compile_expanded_candidates(observations.clone(), |symbol| {
                            symbols.get(symbol).map(signal).unwrap_or(ExpansionSignal::Unsupported)
                        })
                    });
                    let direct_observation = expanded_consumer_observation(&observations, &direct);
                    let reference_observation = expanded_consumer_observation(&observations, &reference);
                    parity_checked = parity_checked.saturating_add(1);
                    if direct_observation != reference_observation {
                        parity_failed = parity_failed.saturating_add(1);
                        eprintln!(
                            "SL_EXPANDED_PARITY_FAIL sentence_id={} direct={} reference={}",
                            direct.sentence_id,
                            observation_summary(&direct_observation),
                            observation_summary(&reference_observation),
                        );
                    }

                    let reference_relations = reference_relation_candidates(
                        observations.clone(),
                        |symbol| symbols.get(symbol).and_then(kind_from_dependency_label),
                    );
                    relation_parity_checked = relation_parity_checked.saturating_add(1);
                    if direct_relations != reference_relations {
                        relation_parity_failed = relation_parity_failed.saturating_add(1);
                        eprintln!(
                            "SL_RELATION_ATTACHMENT_PARITY_FAIL sentence_id={} direct={} reference={}",
                            sentence_id,
                            direct_relations.len(),
                            reference_relations.len(),
                        );
                    }
                }
            }
            "Q" | "M" => {}
            _ => {}
        }
    }

    let pipeline_wall = pipeline_start.map(|start| start.elapsed()).unwrap_or_default();
    eprintln!(
        "SL_EXPANDED_METRIC parity_mode={} framing_active_ns={} direct_active_ns={} reference_active_ns={} pipeline_wall_ns={} sentences={} paragraphs={} candidates={} residuals={} alternatives={} projection_failures={} symbols={} publication_effects=0 parity_checked={} parity_failed={}",
        u8::from(parity_enabled),
        framing_active.active.as_nanos(),
        direct_active.active.as_nanos(),
        reference_active.active.as_nanos(),
        pipeline_wall.as_nanos(),
        sentences,
        paragraphs,
        candidates,
        residuals,
        alternatives,
        projection_failures,
        symbols.len(),
        parity_checked,
        parity_failed,
    );
    for kind in RESIDUAL_KINDS {
        eprintln!(
            "SL_EXPANDED_RESIDUAL kind={} count={}",
            residual_kind_name(kind),
            residual_frontier.count(kind),
        );
    }
    for (label, count) in unsupported_dependency_fibre {
        eprintln!("SL_EXPANDED_UNSUPPORTED_DEP label={label} count={count}");
    }
    eprintln!(
        "SL_RELATION_ATTACHMENT_METRIC parity_mode={} candidates={} parity_checked={} parity_failed={} semantic_authority=0 publication_effects=0",
        u8::from(parity_enabled),
        relation_candidates,
        relation_parity_checked,
        relation_parity_failed,
    );
    for kind in RELATION_ATTACHMENT_KINDS {
        eprintln!(
            "SL_RELATION_ATTACHMENT kind={} count={}",
            kind.name(),
            relation_attachment_frontier.get(kind.name()).copied().unwrap_or(0),
        );
    }

    if parity_enabled && parity_failed != 0 {
        return Err(format!("expanded semantic parity failed for {parity_failed} sentence(s)").into());
    }
    if parity_enabled && relation_parity_failed != 0 {
        return Err(format!(
            "relation attachment parity failed for {relation_parity_failed} sentence(s)"
        )
        .into());
    }
    Ok(())
}
