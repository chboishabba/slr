use sensiblaw_core::{
    ActiveTimer, Annotation, Capability, HeadDeclaration, RevisionId, SentenceId, SymbolTable,
    TextSpan, TokenId, TokenObservation,
};
use sensiblaw_semantic_expansion::{
    ExpansionSignal, ExpandedConsumerObservation, compile_expanded_candidates,
    compile_expanded_direct, expanded_consumer_observation,
};
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

    for line in stdin.lock().lines() {
        let line = line?;
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.is_empty() {
            continue;
        }
        match fields[0] {
            "C" => {
                parity_enabled = fields.get(1).copied() == Some("parity=1");
            }
            "D" => {
                revision = fields.get(1).and_then(|value| value.parse().ok()).unwrap_or(1);
                if pipeline_start.is_none() {
                    pipeline_start = Some(Instant::now());
                }
            }
            "P" => {
                paragraphs = paragraphs.saturating_add(1);
            }
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
                        symbols
                            .get(symbol)
                            .map(signal)
                            .unwrap_or(ExpansionSignal::Unsupported)
                    })
                });
                candidates = candidates.saturating_add(direct.candidates.len() as u64);
                residuals = residuals.saturating_add(direct.residuals.len() as u64);
                alternatives = alternatives.saturating_add(direct.alternative_fibres.len() as u64);
                projection_failures = projection_failures
                    .saturating_add(direct.projection_failures.len() as u64);
                sentences = sentences.saturating_add(1);

                if parity_enabled {
                    let reference = reference_active.measure(|| {
                        compile_expanded_candidates(observations.clone(), |symbol| {
                            symbols
                                .get(symbol)
                                .map(signal)
                                .unwrap_or(ExpansionSignal::Unsupported)
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

    if parity_enabled && parity_failed != 0 {
        return Err(format!("expanded semantic parity failed for {parity_failed} sentence(s)").into());
    }
    Ok(())
}
