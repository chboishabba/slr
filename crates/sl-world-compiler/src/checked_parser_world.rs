//! Production M12 parser -> candidate PNF -> candidate world algebra handoff.
//! Reuses the sealed PostgreSQL parser evidence, existing M12 parity audit,
//! and the versioned WorldCompiler PNF1/ATM1 binary record owners.
//! Does not review claims, determine source meaning, or pay legal/proof authority.

use std::collections::HashMap;
use std::io::{Cursor, Write};

use sensiblaw_pg_source_store::{
    audit_candidate_pnf_evidence, CandidatePnfError, ParserEvidenceRegion,
    ParserTokenRecord, StatementCandidatePnf, StatementPnfSpineError,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    compile_observation_stream, encode_observation, CompileReceipt, CompilerError,
    DependencyShape, ObservationRecord,
};

#[derive(Debug, Error)]
pub enum CheckedParserWorldError {
    #[error("invalid M12 candidate: {0}")]
    Candidate(#[from] StatementPnfSpineError),
    #[error("parser/PNF parity failed: {0}")]
    Parity(#[from] CandidatePnfError),
    #[error("source, parser run, region, or parser receipt mismatch")]
    SourceMismatch,
    #[error("source text or parser observation coverage mismatch")]
    SourceCoverageMismatch,
    #[error("world compilation failed: {0}")]
    World(#[from] CompilerError),
    #[error("world output write failed: {0}")]
    Write(#[from] std::io::Error),
}

/// Map only stable syntactic evidence to existing world-algebra fragment families.
/// An unrecognized dependency or missing head stays unresolved rather than being
/// identified with a predicate or a semantic truth by lexical guesswork.
#[must_use]
pub fn dependency_shape_for_parser(
    token: &ParserTokenRecord,
    head_present: bool,
) -> DependencyShape {
    if token.dependency_ref == "ROOT" {
        return if matches!(token.pos.as_str(), "VERB" | "AUX") {
            DependencyShape::RootPredicate
        } else {
            DependencyShape::UnresolvedDependency
        };
    }
    if !head_present {
        return DependencyShape::UnresolvedDependency;
    }
    match token.dependency_ref.as_str() {
        "nsubj" | "csubj" => DependencyShape::NominalSubject,
        "nsubjpass" | "csubjpass" => DependencyShape::PassiveSubject,
        "dobj" | "obj" => DependencyShape::DirectObject,
        "amod" => DependencyShape::AdjectivalModifier,
        "nmod" | "compound" => DependencyShape::NominalModifier,
        "conj" | "cc" => DependencyShape::Conjunction,
        "neg" => DependencyShape::Negation,
        "nummod" => DependencyShape::NumericQuantifier,
        "det" => DependencyShape::Determiner,
        "npadvmod" | "tmod" => DependencyShape::TemporalModifier,
        "ccomp" => DependencyShape::ClausalComplement,
        "xcomp" => DependencyShape::OpenClausalComplement,
        "advcl" => DependencyShape::AdverbialClause,
        "acl" => DependencyShape::ClausalModifier,
        "relcl" => DependencyShape::RelativeClause,
        // aux is not automatically modal; advmod is not automatically
        // temporal. prep/pobj require attachment/scope interpretation.
        _ => DependencyShape::UnresolvedDependency,
    }
}

/// Consume an already checked statement and a DB-constructed parser evidence
/// region. Revalidates all source and candidate joins because callers can clone
/// and mutate the public candidate PNF after the earlier M12 compilation.
///
/// The binary transport is statement-scoped and candidate-only. Its language
/// is explicitly und (undetermined), not guessed from Hebrew or Latin text.
/// The manifestation's qid slot names a source-unit ref, NOT a Wikidata item.
/// World atoms are retrieval/proposal objects and do not pay a legal premise.
pub fn compile_checked_statement_world<W: Write>(
    candidate: &StatementCandidatePnf,
    evidence: &ParserEvidenceRegion,
    writer: &mut W,
    iteration_index: i64,
) -> Result<CompileReceipt, CheckedParserWorldError> {
    candidate.validate()?;
    let statement = &candidate.statement;
    let expected_receipt = format!(
        "db-parser:{}:{}", evidence.parser_run_ref(), evidence.region_ref(),
    );
    if !evidence.candidate_only()
        || evidence.source_revision_ref() != statement.source_revision_ref.as_str()
        || evidence.region_ref() != statement.span.span_ref.as_str()
        || evidence.span() != (statement.span.start_char, statement.span.end_char)
        || candidate.parser_receipt_ref != expected_receipt
    {
        return Err(CheckedParserWorldError::SourceMismatch);
    }
    if !evidence.matches_source_literal(&statement.literal_text)
        || !evidence.matches_observation_coverage(&statement.literal_text)
    {
        return Err(CheckedParserWorldError::SourceCoverageMismatch);
    }
    audit_candidate_pnf_evidence(
        &statement.span,
        &statement.source_revision_ref,
        &candidate.pnf,
        evidence,
    )?;

    // Scope the document key to an exact statement and revision; the existing
    // world compiler hashes this key into each candidate/atom identity.
    let document_key = format!(
        "{}#{}#{}", statement.document_ref,
        statement.statement_ref, statement.source_revision_ref,
    );
    let source_digest = Sha256::digest(statement.literal_text.as_bytes());
    let mut source_sha256 = [0u8; 32];
    source_sha256.copy_from_slice(&source_digest);

    let mut observations = Vec::new();
    encode_observation(&mut observations, &ObservationRecord::Manifestation {
        document_ref: document_key.clone(),
        qid: format!("source-unit:{}", statement.statement_ref),
        language: "und".into(),
        revision_ref: statement.source_revision_ref.clone(),
        source_sha256,
    })?;
    let tokens_by_ordinal: HashMap<u32, &ParserTokenRecord> = evidence.tokens()
        .iter()
        .map(|token| (token.token_ordinal, token))
        .collect();
    for token in evidence.tokens() {
        let head = token.head_ordinal
            .and_then(|index| tokens_by_ordinal.get(&index).copied());
        let shape = dependency_shape_for_parser(token, head.is_some());
        let head_token = head.unwrap_or(token);
        encode_observation(&mut observations, &ObservationRecord::Token {
            document_ref: document_key.clone(),
            sentence_id: 0, // Local statement fibre; no invented global sentence ordinal.
            local_ordinal: token.token_ordinal,
            start_char: token.start_char,
            end_char: token.end_char,
            head_ordinal: head_token.token_ordinal,
            shape,
            orth: token.surface.clone(),
            lemma: token.lemma.clone(),
            head_orth: head_token.surface.clone(),
            head_lemma: head_token.lemma.clone(),
        })?;
    }
    // Atomic at the statement boundary: errors do not leave a half-written
    // manifestation/PNF/atom stream in the downstream WorldStore.
    let mut pending = Vec::new();
    let receipt = compile_observation_stream(
        &mut Cursor::new(observations), &mut pending, iteration_index,
    )?;
    writer.write_all(&pending)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(dep: &str, pos: &str) -> ParserTokenRecord {
        ParserTokenRecord {
            token_ordinal: 0,
            start_char: 0,
            end_char: 5,
            surface: "Alice".into(),
            lemma: "Alice".into(),
            pos: pos.into(),
            morph_json: None,
            head_ordinal: Some(0),
            dependency_ref: dep.into(),
        }
    }

    #[test]
    fn root_predicate_enters_relation_fragment_without_truth_promotion() {
        assert_eq!(
            dependency_shape_for_parser(&token("ROOT", "VERB"), true),
            DependencyShape::RootPredicate,
        );
        assert_eq!(
            dependency_shape_for_parser(&token("ROOT", "NOUN"), true),
            DependencyShape::UnresolvedDependency,
        );
    }

    #[test]
    fn numeric_quantifier_and_negative_scope_do_not_collapse() {
        assert_eq!(
            dependency_shape_for_parser(&token("nummod", "NUM"), true),
            DependencyShape::NumericQuantifier,
        );
        assert_eq!(
            dependency_shape_for_parser(&token("neg", "PART"), true),
            DependencyShape::Negation,
        );
    }

    #[test]
    fn unlicensed_meaning_and_missing_heads_remain_unresolved() {
        for dep in ["aux", "advmod", "prep", "pobj", "unknown"] {
            assert_eq!(
                dependency_shape_for_parser(&token(dep, "AUX"), true),
                DependencyShape::UnresolvedDependency,
            );
        }
        assert_eq!(
            dependency_shape_for_parser(&token("nsubj", "NOUN"), false),
            DependencyShape::UnresolvedDependency,
        );
    }
}
