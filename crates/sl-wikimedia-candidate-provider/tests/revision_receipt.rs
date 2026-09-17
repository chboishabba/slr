use sensiblaw_wikimedia_candidate_provider::entity_revision_receipt_from_rdf;

#[test]
fn revision_receipt_binds_qid_revision_digest_and_candidate_only_state() {
    let receipt = entity_revision_receipt_from_rdf(
        "Q1501525",
        2333409615,
        b"<rdf:RDF>mabo</rdf:RDF>".to_vec(),
    ).unwrap();
    assert_eq!(receipt.qid, "Q1501525");
    assert_eq!(receipt.source_revision_ref, "wikidata:Q1501525:oldid:2333409615");
    assert!(receipt.content_digest_ref.starts_with("sha256:"));
    assert_eq!(receipt.rdf_bytes, b"<rdf:RDF>mabo</rdf:RDF>");
    assert!(receipt.candidate_only);
    assert!(!receipt.semantic_promotion);
}
