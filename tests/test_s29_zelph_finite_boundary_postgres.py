"""SLR S29 source-reachable finite boundary acceptance and real-PG opt-in."""
from __future__ import annotations

from copy import deepcopy
import os
from pathlib import Path
import sys
import unittest
import uuid

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from s29_zelph_finite_boundary_postgres import (
    check_boundary_certificate, install, persist_boundary, readback_boundary,
)


def boundary(complete=False):
    source = "f"*64
    entry = {
        "query_id":"Q56404383:right", "qid":"Q56404383",
        "original_numeric_root":741397, "relation_word":["right"],
        "source_reachable_steps":[],
        "status":"blocked-by-unknown-original-source-direction",
        "missing_source_boundaries":[{"step":0,"node_id":741397,"direction":"right"}],
        "end_nodes":None,
    }
    if complete:
        entry.update(
            source_reachable_steps=[{
                "step":0,"direction":"right",
                "source_reachable_input_nodes":[741397],
                "original_and_selected_next_nodes":[7,8],
                "per_node_direction_witnesses":[{
                    "node_id":741397,"direction":"right",
                    "native_original_neighbors_sorted_unique":[7,8],
                    "native_original_values_seen":3,
                    "exact_native_audit_shas":[source],
                }],
                "source_equal_native_scan_receipt_shas":[source],
            }],
            status="conditional-source-reachable-steps-complete",
            missing_source_boundaries=[],end_nodes=[7,8],
        )
    return {
        "version":"itir.zelph.source_reachable_lr_finite_word_certificate.v1",
        "dataset_revision":"a"*40,"manifest_sha256":"b"*64,
        "original_index_sha256":"c"*64,
        "native_positive_roots_receipt_sha256":"d"*64,
        "native_decoder_sha256":"e"*64,
        "query_family_raw_sha256":"1"*64,
        "finite_result_raw_sha256":"2"*64,
        "accepted_audit_dependencies":[{"sha256":source,
                "used_for_direction_equality":complete}],
        "query_count":1,"query_certificate_entries":[entry],
        "all_declared_words_conditional_boundary_complete":complete,
        "all_twenty_distinct_words_conditional_boundary_complete":False,
        "physical_http206_bytes_independently_replayed_here":False,
        "native_capnp_cpp_semantics_formally_refined_here":False,
        "agda_source_reachable_boundary_term_generated_here":False,
        "evidence_only":True,"s29_persistence_paid":False,
        "p854_source_support_paid":False,"nora_review_authority":False,
        "edit_authority":False,
    }


class FiniteBoundaryOffline(unittest.TestCase):
    def test_incomplete_five_shard_positive_cannot_become_global_negative(self):
        cert=boundary()
        projection=check_boundary_certificate(cert)
        self.assertTrue(projection["unknown_boundaries_visible"])
        self.assertFalse(projection["conditional_boundary_complete"])
        self.assertFalse(projection["source_support_paid"])
        self.assertFalse(projection["edit_authority"])

    def test_source_complete_candidate_preserves_values_seen_multiplicity(self):
        cert=boundary(complete=True)
        projection=check_boundary_certificate(cert)
        self.assertTrue(projection["conditional_boundary_complete"])
        self.assertEqual(cert["query_certificate_entries"][0]["source_reachable_steps"][0][
            "per_node_direction_witnesses"][0]["native_original_values_seen"],3)
        self.assertFalse(projection["source_support_paid"])

    def test_tampered_frontier_or_unknown_state_refused(self):
        for mutation in (
            lambda c: c["query_certificate_entries"][0][
                "source_reachable_steps"][0].update(original_and_selected_next_nodes=[999]),
            lambda c: c["query_certificate_entries"][0].update(end_nodes=[999]),
            lambda c: c["query_certificate_entries"][0][
                "source_reachable_steps"][0]["per_node_direction_witnesses"][0].update(
                    native_original_values_seen=1),
            lambda c: c.update(agda_source_reachable_boundary_term_generated_here=True),
            lambda c: c.update(nora_review_authority=True),
            lambda c: c["accepted_audit_dependencies"][0].update(
                used_for_direction_equality=False),
        ):
            with self.subTest(mutation=mutation):
                forged=boundary(complete=True)
                mutation(forged)
                with self.assertRaises(ValueError):
                    check_boundary_certificate(forged)
        forged=boundary()
        forged["all_declared_words_conditional_boundary_complete"]=True
        with self.assertRaises(ValueError):
            check_boundary_certificate(forged)

    def test_duplicate_qid_word_relabel_does_not_count_as_two_queries(self):
        forged=boundary(complete=True)
        row=deepcopy(forged["query_certificate_entries"][0])
        row["query_id"]="other-label"
        forged["query_certificate_entries"].append(row)
        forged["query_count"]=2
        with self.assertRaises(ValueError):
            check_boundary_certificate(forged)

    def test_source_revision_and_raw_digest_are_exact(self):
        forged=boundary()
        forged["native_decoder_sha256"]="not-a-digest"
        with self.assertRaises(ValueError):
            check_boundary_certificate(forged)
        forged=boundary()
        forged["dataset_revision"]="main"
        with self.assertRaises(ValueError):
            check_boundary_certificate(forged)


class FiniteBoundaryRealPostgres(unittest.TestCase):
    @unittest.skipUnless(os.getenv("SLR_S29_TEST_DATABASE_URL"),
                         "set SLR_S29_TEST_DATABASE_URL for actual PG")
    def test_commit_reconnect_readback_without_source_promotion(self):
        try:
            import psycopg
        except ImportError:
            self.fail("psycopg v3 required for real S29 acceptance")
        # A fresh synthetic revision avoids silently reusing a prior test key.
        cert=boundary()
        cert["dataset_revision"]=uuid.uuid4().hex + "f"*8
        database=os.environ["SLR_S29_TEST_DATABASE_URL"]
        with psycopg.connect(database) as conn:
            install(conn)
            result=persist_boundary(conn,cert)
            self.assertFalse(result["conditional_boundary_complete"])
        with psycopg.connect(database) as reopened:
            result2=readback_boundary(reopened,result["receipt_digest"])
            self.assertTrue(result2["db_readback_verified"])
            self.assertTrue(result2["unknown_boundaries_visible"])
            self.assertFalse(result2["physical_original_bin_provenance_verified"])
            self.assertFalse(result2["agda_kernel_witness_instantiated"])
            self.assertFalse(result2["edit_authority"])


if __name__=="__main__":
    unittest.main()
