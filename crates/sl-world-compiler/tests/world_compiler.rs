use sensiblaw_world_compiler::{
    compile_observation_stream, decode_observation, encode_observation, DependencyShape,
    FragmentKind, ObservationRecord, OBS_MAGIC, OBS_VERSION,
};
use sensiblaw_world_store::{decode_record, WorldRecordKind};
use std::io::Cursor;

fn manifestation() -> ObservationRecord {
    ObservationRecord::Manifestation {
        document_ref: "wiki:Q207:en:456".into(),
        qid: "Q207".into(),
        language: "en".into(),
        revision_ref: "456".into(),
        source_sha256: [7u8; 32],
    }
}

fn token(shape: DependencyShape, ordinal: u32, head: u32, orth: &str, lemma: &str) -> ObservationRecord {
    ObservationRecord::Token {
        document_ref: "wiki:Q207:en:456".into(),
        sentence_id: 3,
        local_ordinal: ordinal,
        start_char: ordinal * 4,
        end_char: ordinal * 4 + orth.len() as u32,
        head_ordinal: head,
        shape,
        orth: orth.into(),
        lemma: lemma.into(),
    }
}

#[test]
fn observation_wire_round_trip_is_versioned_binary() {
    let record = token(DependencyShape::NominalSubject, 0, 1, "Bush", "Bush");
    let mut bytes = Vec::new();
    encode_observation(&mut bytes, &record).expect("encode observation");
    assert_eq!(&bytes[..4], &OBS_MAGIC);
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), OBS_VERSION);
    let decoded = decode_observation(&mut Cursor::new(bytes)).expect("decode").expect("record");
    assert_eq!(decoded, record);
}

#[test]
fn nominal_subject_compiles_to_actor_candidate_and_world_atom() {
    let records = [
        manifestation(),
        token(DependencyShape::NominalSubject, 0, 1, "Bush", "Bush"),
        token(DependencyShape::UnresolvedDependency, 1, 1, "signed", "sign"),
    ];
    let mut input = Vec::new();
    for r in &records { encode_observation(&mut input, r).unwrap(); }
    let mut output = Vec::new();
    let receipt = compile_observation_stream(&mut Cursor::new(input), &mut output, 5).expect("compile");
    assert_eq!(receipt.manifestations, 1);
    assert_eq!(receipt.pnf_candidates, 1);
    assert_eq!(receipt.world_atoms, 1);
    assert!(!receipt.semantic_promotion);

    let mut cursor = Cursor::new(output);
    let m = decode_record(&mut cursor).unwrap().unwrap();
    let pnf = decode_record(&mut cursor).unwrap().unwrap();
    let atom = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(m.kind, WorldRecordKind::SourceManifestation);
    assert_eq!(pnf.kind, WorldRecordKind::PnfCandidate);
    assert_eq!(atom.kind, WorldRecordKind::WorldAtom);
    assert_eq!(pnf.aux1.as_deref(), Some("wiki:Q207:en:456"));
    assert_eq!(pnf.payload[4], FragmentKind::Actor as u8);
    assert_eq!(pnf.payload[5], DependencyShape::NominalSubject as u8);
}

#[test]
fn object_negation_and_clause_shapes_preserve_agda_fragment_families() {
    let cases = [
        (DependencyShape::DirectObject, FragmentKind::Patient),
        (DependencyShape::Negation, FragmentKind::Negation),
        (DependencyShape::ClausalComplement, FragmentKind::ContentClause),
        (DependencyShape::AdverbialClause, FragmentKind::ClauseAttachment),
    ];
    for (shape, expected) in cases {
        let records = [manifestation(), token(shape, 0, 1, "x", "x")];
        let mut input = Vec::new();
        for r in &records { encode_observation(&mut input, r).unwrap(); }
        let mut output = Vec::new();
        compile_observation_stream(&mut Cursor::new(input), &mut output, 7).unwrap();
        let mut cursor = Cursor::new(output);
        let _manifestation = decode_record(&mut cursor).unwrap().unwrap();
        let pnf = decode_record(&mut cursor).unwrap().unwrap();
        assert_eq!(pnf.payload[4], expected as u8, "{shape:?}");
        assert_eq!(pnf.payload[5], shape as u8, "{shape:?}");
    }
}

#[test]
fn unresolved_dependency_does_not_silently_promote_semantics() {
    let records = [manifestation(), token(DependencyShape::UnresolvedDependency, 0, 0, "x", "x")];
    let mut input = Vec::new();
    for r in &records { encode_observation(&mut input, r).unwrap(); }
    let mut output = Vec::new();
    let receipt = compile_observation_stream(&mut Cursor::new(input), &mut output, 9).unwrap();
    assert_eq!(receipt.pnf_candidates, 0);
    assert_eq!(receipt.world_atoms, 0);
    assert_eq!(receipt.unresolved_dependencies, 1);
}

#[test]
fn compiler_source_has_no_json_or_regex_dependency() {
    let cargo = std::fs::read_to_string("crates/sl-world-compiler/Cargo.toml").unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
