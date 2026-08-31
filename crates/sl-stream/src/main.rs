use sensiblaw_core::*;
use std::io::{self, BufRead};
use std::time::Instant;

fn unesc(s: &str) -> String { s.replace("\\t", "\t").replace("\\n", "\n").replace("\\\\", "\\") }

fn shape(dep: &str) -> DependencyShape {
    match dep {
        "nsubj" => DependencyShape::NominalSubject,
        "obj" | "dobj" => DependencyShape::DirectObject,
        "nsubjpass" => DependencyShape::PassiveSubject,
        "amod" => DependencyShape::AdjectivalModifier,
        "nmod" => DependencyShape::NominalModifier,
        "conj" => DependencyShape::Conjunction,
        "neg" => DependencyShape::Negation,
        "aux" | "auxpass" => DependencyShape::ModalAuxiliary,
        "npadvmod" | "tmod" => DependencyShape::TemporalModifier,
        _ => DependencyShape::Unresolved,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut symbols = SymbolTable::default();
    let mut active = ActiveTimer::default();
    let mut revision: RevisionId = 1;
    let mut sentence_id: SentenceId = 0;
    let mut pending = Vec::<TokenObservation>::new();
    let mut next_token: TokenId = 1;
    let mut sentences = 0u64;
    let mut candidates = 0u64;
    let mut residuals = 0u64;
    let mut pipeline_start: Option<Instant> = None;
    let mut paragraph: Option<ParagraphAccumulator> = None;
    let mut paragraph_count = 0u64;
    let mut publisher = GenerationPublisher::default();

    for line in stdin.lock().lines() {
        let line = line?;
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.is_empty() { continue; }
        match fields[0] {
            "D" => {
                revision = fields.get(1).and_then(|x| x.parse().ok()).unwrap_or(1);
                pipeline_start = Some(Instant::now());
            }
            "P" => {
                let paragraph_id = fields.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
                paragraph = Some(ParagraphAccumulator::new(paragraph_id));
            }
            "S" => {
                sentence_id = fields.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
                pending.clear();
            }
            "T" => {
                active.measure(|| {
                    if fields.len() < 10 { return; }
                    let local: u32 = fields[1].parse().unwrap_or(0);
                    let start: u32 = fields[2].parse().unwrap_or(0);
                    let end: u32 = fields[3].parse().unwrap_or(start);
                    let head_local: i64 = fields[4].parse().unwrap_or(-1);
                    let orth = symbols.intern(&unesc(fields[5]));
                    let lemma_text = unesc(fields[6]);
                    let lemma = if fields[6] == "-" { Annotation::Unavailable(Capability::Lemma) } else { Annotation::Present(symbols.intern(&lemma_text)) };
                    let pos = if fields[7] == "-" { Annotation::Unavailable(Capability::Pos) } else { Annotation::Present(symbols.intern(fields[7])) };
                    let tag = if fields[8] == "-" { Annotation::Unavailable(Capability::Tag) } else { Annotation::Present(symbols.intern(fields[8])) };
                    let dep_text = fields[9];
                    let dep = if dep_text == "-" { Annotation::Unavailable(Capability::Dependency) } else { Annotation::Present(symbols.intern(dep_text)) };
                    let declared_head = if head_local < 0 || head_local as u32 == local {
                        HeadDeclaration::SelfHead
                    } else { HeadDeclaration::LocalOrdinal(head_local as u32) };
                    if let Ok(span) = TextSpan::new(revision, start, end) {
                        pending.push(TokenObservation {
                            token_id: next_token, sentence_id, local_ordinal: local, span, orth,
                            lemma, pos, tag, dependency: dep, declared_head,
                        });
                        next_token += 1;
                    }
                });
            }
            "E" => {
                let compilation = active.measure(|| {
                    let packed = PackedSentence::from_observations(std::mem::take(&mut pending));
                    compile_packed_sentence(packed, |symbol| {
                        symbols.get(symbol).map(shape).unwrap_or(DependencyShape::Unresolved)
                    })
                });
                candidates = candidates.saturating_add(compilation.interior.deltas.len() as u64);
                residuals = residuals.saturating_add(compilation.interior.residuals.len() as u64);
                if let Some(p) = paragraph.as_mut() {
                    active.measure(|| p.absorb(compilation.outward));
                }
                sentences += 1;
                println!("R\t{}\t{}\t{}\t{}", compilation.interior.sentence_id, compilation.interior.deltas.len(), compilation.projection_failures.len(), compilation.interior.residuals.len());
            }
            "Q" => {
                if let Some(p) = paragraph.take() {
                    paragraph_count = paragraph_count.saturating_add(1);
                    println!("PR\t{}\t{}\t{}", p.paragraph_id, p.accepted_children, p.residuals);
                }
            }
            "M" => {
                let generation = publisher.stage(candidates, residuals);
                println!("G\t{}\tstaged\tvisible=0", generation);
                println!("M\t{}", fields.get(1).unwrap_or(&""));
            }
            _ => {}
        }
    }

    let elapsed = pipeline_start.map(|t| t.elapsed()).unwrap_or_default();
    eprintln!("SL_METRIC active_ns={} pipeline_wall_ns={} sentences={} paragraphs={} candidates={} residuals={} symbols={} published={}",
        active.active.as_nanos(), elapsed.as_nanos(), sentences, paragraph_count, candidates, residuals, symbols.len(), publisher.current().is_some() as u8);
    Ok(())
}
