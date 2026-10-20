use sensiblaw_pg_source_store::{
    assess_document_pnf_pair, CandidatePnfFactor, CandidatePnfRole, DocumentPnfSide,
};
fn factor(role:CandidatePnfRole,lemma:&str)->CandidatePnfFactor {
    CandidatePnfFactor {
        candidate_ref:format!("factor:{lemma}"),role,source_start_char:0,
        source_end_char:5,surface:lemma.into(),lemma:lemma.into(),
        dependency_ref:"dep".into(),candidate_only:true,
    }
}
fn side(name:&str,factors:Vec<CandidatePnfFactor>)->DocumentPnfSide {
    DocumentPnfSide {
        source_revision_ref:format!("revision:{name}"),
        extraction_ref:format!("extraction:{name}"),
        digest:"a".repeat(64),parser_run_ref:format!("parser:{name}"),
        batch_refs:vec![format!("batch:{name}")],
        inventory_sha256:format!("inventory:{name}"),
        parser_residual_count:0,factors,
    }
}
#[test]
fn full_document_exact_is_candidate_only_and_replays() {
    let a=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let b=side("docx",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let first=assess_document_pnf_pair("comparison",a.clone(),b.clone(),vec![]).unwrap();
    assert!(first.transformation_closed);
    assert!(first.coverage.semantic_factor_multiset_exact);
    assert_eq!(first,assess_document_pnf_pair("comparison",a,b,vec![]).unwrap());
    assert!(first.candidate_only);
    assert!(!first.creates_legal_controversy);
    assert!(!first.creates_semantic_authority);
    assert!(!first.creates_legal_authority);
    assert!(!first.applicability_promoted);
    assert!(!first.claim_truth_promoted);
    assert!(!first.consumer_informativeness_paid);
    assert_eq!(first.independent_source_count_increment,0);
}
#[test]
fn changed_digest_changes_receipt_identity() {
    let a=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let b=side("docx",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let first=assess_document_pnf_pair("comparison",a.clone(),b.clone(),vec![]).unwrap();
    let mut changed=a;
    changed.digest="b".repeat(64);
    let next=assess_document_pnf_pair("comparison",changed,b,vec![]).unwrap();
    assert_ne!(first.receipt_ref,next.receipt_ref);
}
#[test]
fn parser_residual_prevents_full_closure() {
    let mut a=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    a.parser_residual_count=1;
    let result=assess_document_pnf_pair("comparison",a,side("docx",vec![factor(CandidatePnfRole::Actor,"smith")]),vec![]).unwrap();
    assert!(!result.transformation_closed);
}
#[test]
fn unmatched_semantic_factor_prevents_full_closure() {
    let result=assess_document_pnf_pair("comparison",
        side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]),
        side("docx",vec![factor(CandidatePnfRole::Actor,"jones")]),vec![]).unwrap();
    assert!(!result.coverage.semantic_factor_multiset_exact);
    assert!(!result.transformation_closed);
}
#[test]
fn other_requires_independent_review_and_does_not_auto_close() {
    let mut a=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    a.factors.push(factor(CandidatePnfRole::Other,"layout"));
    let b=side("docx",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let result=assess_document_pnf_pair("comparison",a,b,vec!["review:pending".into()]).unwrap();
    assert!(result.coverage.semantic_factor_multiset_exact);
    assert!(result.semantic_pnf_overlap_closed);
    assert!(!result.transformation_closed);
}
#[test]
fn differing_other_counts_preserve_v3_semantic_overlap_and_replay() {
    let mut pdf=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    pdf.factors.push(factor(CandidatePnfRole::Other,"punctuation"));
    pdf.factors.push(factor(CandidatePnfRole::Other,"determiner"));
    let docx=side("docx",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let receipt=assess_document_pnf_pair("comparison",pdf.clone(),docx.clone(),vec![]).unwrap();
    assert!(receipt.coverage.semantic_factor_multiset_exact);
    assert!(receipt.semantic_pnf_overlap_closed);
    assert!(!receipt.transformation_closed); // materiality remains unclassified
    assert_eq!(receipt.coverage.ignored_pdf_nonsemantic_count,2);
    assert_eq!(receipt.coverage.ignored_docx_nonsemantic_count,0);
    assert_eq!(receipt,assess_document_pnf_pair("comparison",pdf,docx,vec![]).unwrap());
}
#[test]
fn unverified_review_coordinate_does_not_authorize_materiality() {
    let mut pdf=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    pdf.factors.push(factor(CandidatePnfRole::Other,"layout"));
    let docx=side("docx",vec![factor(CandidatePnfRole::Actor,"smith")]);
    let pending=assess_document_pnf_pair("comparison",pdf.clone(),docx.clone(),vec![]).unwrap();
    let unverified=assess_document_pnf_pair("comparison",pdf,docx,vec!["review:unverified".into()]).unwrap();
    assert!(pending.semantic_pnf_overlap_closed);
    assert!(unverified.semantic_pnf_overlap_closed);
    assert!(!pending.transformation_closed);
    assert!(!unverified.transformation_closed);
    assert_ne!(pending.receipt_ref,unverified.receipt_ref);
}
#[test]
fn same_revision_is_rejected() {
    let a=side("pdf",vec![factor(CandidatePnfRole::Actor,"smith")]);
    assert!(assess_document_pnf_pair("comparison",a.clone(),a,vec![]).is_err());
}
#[test]
#[ignore = "requires real official Smith PDF/DOCX PG extraction comparison and full DB-native compilation receipts"]
fn live_smith_document_pair_pnf_persists_and_reopens() {
    use sensiblaw_pg_source_store::{
        compare_and_persist_document_pnf_pair, load_document_pnf_pair, load_database_config,
    };
    fn required(key:&str)->String {
        std::env::var(key).unwrap_or_else(|_|panic!("{key} must be an already persisted Smith coordinate"))
    }
    let config=load_database_config(None).expect("DATABASE_URL required");
    let comparison=required("ITIR_SMITH_HCA37_MANIFESTATION_COMPARISON_REF");
    let pdf=required("ITIR_SMITH_HCA37_PDF_SOURCE_REVISION_REF");
    let docx=required("ITIR_SMITH_HCA37_DOCX_SOURCE_REVISION_REF");
    let first=compare_and_persist_document_pnf_pair(&config,&comparison,&pdf,&docx).unwrap();
    let second=compare_and_persist_document_pnf_pair(&config,&comparison,&pdf,&docx).unwrap();
    assert_eq!(first,second);
    assert_eq!(first,load_document_pnf_pair(&config,&first.receipt_ref).unwrap());
}
