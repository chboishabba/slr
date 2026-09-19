use sensiblaw_governed_legal_provider::{
    load_oalc_jsonl_snapshot, lookup_oalc_exact_mnc, OalcSnapshotLoadError,
    RECEIPT_AUTHORITY,
};
use std::fs;

#[test]
fn local_oalc_jsonl_loads_pinned_exact_mnc_snapshot() {
    let path = std::env::temp_dir().join(format!(
        "sensiblaw-oalc-snapshot-{}-{}.jsonl",
        std::process::id(),
        "mabo"
    ));
    fs::write(
        &path,
        concat!(
            "{\"version_id\":\"mabo-v1\",\"source\":\"high_court_of_australia\",\"citation\":\"Mabo v Queensland (No 2) [1992] HCA 23\",\"text\":\"Mabo canonical text\"}\n",
            "{\"version_id\":\"other-v1\",\"source\":\"high_court_of_australia\",\"citation\":\"Other [1988] HCA 69\",\"text\":\"Other canonical text\"}\n"
        ),
    )
    .unwrap();

    let loaded = load_oalc_jsonl_snapshot(&path, "oalc:dataset:deadbeef").unwrap();
    assert_eq!(loaded.line_count, 2);
    assert_eq!(loaded.snapshot.records.len(), 2);

    let mabo = lookup_oalc_exact_mnc(&loaded.snapshot, "[1992] HCA 23").unwrap();
    assert_eq!(mabo.network_requests, 0);
    assert_eq!(mabo.receipt_authority, RECEIPT_AUTHORITY);
    assert_eq!(mabo.corpus_revision_ref, "oalc:dataset:deadbeef");
    assert!(mabo.canonical_text_digest.starts_with("sha256:"));

    fs::remove_file(path).unwrap();
}

#[test]
fn mutable_oalc_snapshot_revision_fails_closed() {
    let path = std::env::temp_dir().join(format!(
        "sensiblaw-oalc-snapshot-{}-mutable.jsonl",
        std::process::id()
    ));
    fs::write(
        &path,
        "{\"version_id\":\"mabo-v1\",\"source\":\"hca\",\"citation\":\"Mabo [1992] HCA 23\",\"text\":\"text\"}\n",
    )
    .unwrap();

    assert!(matches!(
        load_oalc_jsonl_snapshot(&path, "latest"),
        Err(OalcSnapshotLoadError::MutableRevisionAlias(_))
    ));
    fs::remove_file(path).unwrap();
}
