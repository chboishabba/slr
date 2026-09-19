use std::collections::BTreeMap;
use std::fs;

use sensiblaw_route_selector::ProducerFamily;
use sensiblaw_world_expansion_runtime::sprint1_acquisition_machine::{
    execute_physical_acquisition, plan_physical_acquisition, AcquisitionPath,
    LogicalAcquisitionRequest, PhysicalAcquisitionPlan, PlannedPhysicalObject,
};
use sensiblaw_world_expansion_runtime::zelph_hf_physical::{
    CurlCachedPhysicalTransport, ZelphHfPhysicalPlanner,
};

const MANIFEST: &str = r#"{
  "source": {
    "binPath": "/producer/full.bin",
    "headerLengthBytes": 64
  },
  "sections": {
    "left": {
      "chunks": [
        {"chunkIndex": 0, "objectPath": "hf://datasets/acrion/zelph/demo/shards/left/chunk-000000.capnp-packed", "length": 10}
      ]
    },
    "right": {
      "chunks": [
        {"chunkIndex": 1, "objectPath": "hf://datasets/acrion/zelph/demo/shards/right/chunk-000001.capnp-packed", "length": 11}
      ]
    },
    "nameOfNode": {
      "chunks": [
        {"chunkIndex": 2, "objectPath": "hf://datasets/acrion/zelph/demo/shards/nameOfNode/chunk-000002-wikidata.capnp-packed", "length": 12}
      ]
    },
    "nodeOfName": {
      "chunks": [
        {"chunkIndex": 3, "objectPath": "hf://datasets/acrion/zelph/demo/shards/nodeOfName/chunk-000003-wikidata.capnp-packed", "length": 13}
      ]
    }
  }
}"#;

const ROUTE: &str = r#"{
  "routing": {
    "left": [{"chunkIndex": 0, "nodes": [42]}],
    "right": [{"chunkIndex": 1, "nodes": [42]}],
    "nameOfNode": [{"chunkIndex": 2, "nodes": [42]}],
    "nodeOfName": [{"chunkIndex": 3, "lang": "wikidata", "names": ["Q1", "Q2"]}]
  }
}"#;

fn request(id: &str, target: &str) -> LogicalAcquisitionRequest {
    LogicalAcquisitionRequest {
        request_ref: id.into(),
        semantic_target_ref: target.into(),
        producer: ProducerFamily::ClassificationEvidence,
    }
}

#[test]
fn node_route_manifest_plans_true_physical_objects_and_coalesces_them() {
    let resolved = BTreeMap::from([("Q1".into(), 42), ("Q2".into(), 42)]);
    let mut planner = ZelphHfPhysicalPlanner::from_json(
        MANIFEST,
        ROUTE,
        resolved,
        Some("hf://datasets/acrion/zelph/demo/full.bin".into()),
        AcquisitionPath::RouteAwareGeneralSnapshot,
    )
    .unwrap();

    let plan = plan_physical_acquisition(
        &[request("r1", "Q1"), request("r2", "Q2")],
        &mut planner,
    )
    .unwrap();

    assert_eq!(plan.planned_object_references, 10);
    assert_eq!(plan.unique_objects.len(), 5);
    assert_eq!(plan.coalesced_object_references, 5);
    assert_eq!(plan.route_aware_general_objects, 5);
    assert_eq!(plan.max_cold_remote_objects, 5);

    let refs = plan
        .unique_objects
        .iter()
        .map(|object| object.route_ref.as_str())
        .collect::<Vec<_>>();
    assert!(refs.contains(&"zelph-hf:header"));
    assert!(refs.contains(&"zelph-hf:left:0"));
    assert!(refs.contains(&"zelph-hf:right:1"));
    assert!(refs.contains(&"zelph-hf:nameOfNode:2"));
    assert!(refs.contains(&"zelph-hf:nodeOfName:3"));
}

#[test]
fn cached_transport_reads_exact_range_then_replays_with_zero_cold_gets() {
    let root = std::env::temp_dir().join(format!(
        "slr-sprint1-physical-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let source = root.join("source.bin");
    fs::write(&source, b"abcdefgh").unwrap();

    let object = PlannedPhysicalObject {
        physical_object_ref: "object:range-2-5".into(),
        cache_key: "object:range-2-5".into(),
        route_ref: "route:test".into(),
        source_ref: source.to_string_lossy().into_owned(),
        source_range: Some((2, 5)),
        expected_bytes: Some(4),
        acquisition_path: AcquisitionPath::RouteAwareGeneralSnapshot,
    };
    let plan = PhysicalAcquisitionPlan {
        logical_requests: vec![request("r1", "Q1")],
        bindings: vec![],
        unique_objects: vec![object],
        planned_object_references: 1,
        coalesced_object_references: 0,
        specialised_slice_objects: 0,
        route_aware_general_objects: 1,
        max_cold_remote_objects: 5,
    };

    let cache = root.join("cache");
    let mut first = CurlCachedPhysicalTransport::new(&cache).unwrap();
    let (first_rows, first_receipt) = execute_physical_acquisition(&plan, &mut first).unwrap();
    assert_eq!(first_rows.len(), 1);
    assert_eq!(first_rows[0].bytes, 4);
    assert_eq!(first_receipt.cache_hits, 0);
    assert_eq!(first_receipt.remote_gets, 1);
    assert_eq!(first_receipt.bytes_fetched, 4);

    let mut replay = CurlCachedPhysicalTransport::new(&cache).unwrap();
    let (replay_rows, replay_receipt) =
        execute_physical_acquisition(&plan, &mut replay).unwrap();
    assert_eq!(replay_rows.len(), 1);
    assert!(replay_rows[0].from_cache);
    assert_eq!(replay_receipt.cache_hits, 1);
    assert_eq!(replay_receipt.remote_gets, 0);
    assert_eq!(replay_receipt.bytes_fetched, 0);
    assert_eq!(
        first_rows[0].evidence_digest_ref,
        replay_rows[0].evidence_digest_ref
    );

    fs::remove_dir_all(&root).unwrap();
}
