use sensiblaw_wikimedia_candidate_provider::{
    parse_latest_revision_id, parse_previous_revision_id, ProviderError,
};

#[test]
fn parses_exact_qid_latest_revision_from_mediawiki_response() {
    let json = br#"{
      "batchcomplete": true,
      "query": {
        "pages": [
          {
            "pageid": 123,
            "ns": 0,
            "title": "Q36074",
            "revisions": [{"revid": 246813579}]
          }
        ]
      }
    }"#;

    assert_eq!(parse_latest_revision_id("Q36074", json).unwrap(), 246813579);
}

#[test]
fn rejects_latest_revision_response_for_another_qid() {
    let json = br#"{
      "query": {
        "pages": [
          {"title": "Q408", "revisions": [{"revid": 99}]}
        ]
      }
    }"#;

    let error = parse_latest_revision_id("Q36074", json)
        .expect_err("coordinate lookup must not silently accept another entity");
    assert!(matches!(error, ProviderError::InvalidInput(_)));
}

#[test]
fn rejects_missing_or_zero_revision() {
    let missing = br#"{"query":{"pages":[{"title":"Q36074"}]}}"#;
    assert!(matches!(
        parse_latest_revision_id("Q36074", missing),
        Err(ProviderError::InvalidInput(_))
    ));

    let zero = br#"{"query":{"pages":[{"title":"Q36074","revisions":[{"revid":0}]}]}}"#;
    assert!(matches!(
        parse_latest_revision_id("Q36074", zero),
        Err(ProviderError::InvalidInput(_))
    ));
}

#[test]
fn parses_immediate_predecessor_revision_from_bounded_history() {
    let json = br#"{
      "query": {
        "pages": [
          {
            "title": "Q36074",
            "revisions": [
              {"revid": 246813579},
              {"revid": 246813500}
            ]
          }
        ]
      }
    }"#;
    assert_eq!(
        parse_previous_revision_id("Q36074", 246813579, json).unwrap(),
        246813500
    );
}

#[test]
fn predecessor_lookup_requires_requested_start_revision() {
    let json = br#"{
      "query": {
        "pages": [
          {
            "title": "Q36074",
            "revisions": [
              {"revid": 246813500},
              {"revid": 246813400}
            ]
          }
        ]
      }
    }"#;
    assert!(matches!(
        parse_previous_revision_id("Q36074", 246813579, json),
        Err(ProviderError::InvalidInput(_))
    ));
}
