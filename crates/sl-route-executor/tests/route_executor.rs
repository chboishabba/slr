use sensiblaw_route_executor::{
    decode_acquired_source, execute_selected_routes_with_fetcher, AcquiredSourceKind,
};
use sensiblaw_world_store::{encode_record, WireRecord, WorldRecordKind};
use std::io::Cursor;

fn write_text(buf: &mut Vec<u8>, value: &str) {
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value.as_bytes());
}

fn selected_article_route() -> WireRecord {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"RTA2");
    payload.push(1); // ArticleSemantic producer
    payload.push(2); // WikipediaArticle route family
    payload.push(1); // candidate-only
    payload.push(0); // no semantic promotion
    write_text(&mut payload, "route-intent:obligation:test");
    write_text(&mut payload, "wiki-article:Q207");
    write_text(&mut payload, "Q207");
    write_text(&mut payload, "https://en.wikipedia.org/wiki/George_W._Bush");
    write_text(&mut payload, "");
    WireRecord {
        kind: WorldRecordKind::RouteAction,
        id: "selected:route-intent:obligation:test:wiki-article:Q207".into(),
        iteration_index: Some(4),
        aux1: Some("https://en.wikipedia.org/wiki/George_W._Bush".into()),
        payload,
    }
}

fn selected_search_route() -> WireRecord {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"RTA2");
    payload.push(6); // MechanismEvidence producer
    payload.push(4); // PrimarySourceSearch route family
    payload.push(1);
    payload.push(0);
    write_text(&mut payload, "route-intent:obligation:mechanism");
    write_text(&mut payload, "search:mechanism:Q207");
    write_text(&mut payload, "Q207");
    write_text(&mut payload, "Q207");
    write_text(&mut payload, "");
    WireRecord {
        kind: WorldRecordKind::RouteAction,
        id: "selected:route-intent:obligation:mechanism:search:mechanism:Q207".into(),
        iteration_index: Some(4),
        aux1: Some("Q207".into()),
        payload,
    }
}

const HTML: &str = r#"<!doctype html><html lang="en"><head><title>George W. Bush</title></head><body><main><h1>George W. Bush</h1><p>George Walker Bush served as president of the United States.</p><p>He previously served as governor of Texas.</p></main><script>ignore_me()</script></body></html>"#;

#[test]
fn wikipedia_article_rta2_emits_versioned_binary_acquired_source() {
    let mut input = Vec::new();
    encode_record(&mut input, &selected_article_route()).unwrap();
    let mut output = Vec::new();
    let receipt = execute_selected_routes_with_fetcher(
        &mut Cursor::new(input),
        &mut output,
        |url| {
            assert_eq!(url, "https://en.wikipedia.org/wiki/George_W._Bush");
            Ok((HTML.as_bytes().to_vec(), Some("W/\"123456789\"".into())))
        },
    )
    .unwrap();
    assert_eq!(receipt.selected_routes_seen, 1);
    assert_eq!(receipt.executable_routes_seen, 1);
    assert_eq!(receipt.sources_emitted, 1);
    assert_eq!(receipt.deferred_routes, 0);
    assert!(!receipt.acquisition_creates_claim_truth);

    let source = decode_acquired_source(&mut Cursor::new(output)).unwrap().unwrap();
    assert_eq!(source.kind, AcquiredSourceKind::WikipediaRenderedHtml);
    assert_eq!(source.source_ref, "Q207");
    assert_eq!(source.canonical_url, "https://en.wikipedia.org/wiki/George_W._Bush");
    assert_eq!(source.language, "en");
    assert_eq!(source.revision_ref, "W/\"123456789\"");
    assert!(source.text.contains("served as president"));
    assert!(source.text.contains("governor of Texas"));
    assert!(!source.text.contains("ignore_me"));
    assert!(source.candidate_only);
    assert!(!source.semantic_promotion);
}

#[test]
fn unsupported_search_family_is_deferred_not_faked_as_source() {
    let mut input = Vec::new();
    encode_record(&mut input, &selected_search_route()).unwrap();
    let mut output = Vec::new();
    let receipt = execute_selected_routes_with_fetcher(
        &mut Cursor::new(input),
        &mut output,
        |_url| panic!("search route must not call article fetcher"),
    )
    .unwrap();
    assert_eq!(receipt.selected_routes_seen, 1);
    assert_eq!(receipt.executable_routes_seen, 0);
    assert_eq!(receipt.sources_emitted, 0);
    assert_eq!(receipt.deferred_routes, 1);
    assert!(output.is_empty());
}

#[test]
fn acquired_source_dependency_surface_has_no_json_or_regex() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
