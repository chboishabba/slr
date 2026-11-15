"""Pure source-contract checks and opt-in ACTUAL PostgreSQL commit/reconnect tests."""
from __future__ import annotations
from copy import deepcopy
import os
from pathlib import Path
import sys
import unittest
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from s29_source_review_postgres import (
    add_explicit_review, candidate_receipt, candidate_sources, install,
    persist_candidate, readback,
)


def candidate(qid="Q123", graph_revision="a" * 40, bin_hash="1" * 64):
    return {
        "version": "itir.zelph_wikibase_source_bound_candidate.v1",
        "qid": qid, "statement_guid": qid + "$fixture-a",
        "graph_dataset_revision": graph_revision,
        "wikibase_native_entity_revision_id": 101,
        "original_bin_sha256_matched_supplied_pin": bin_hash,
        "wikibase_native_raw_entity_sha256": "2" * 64,
        "selected_root_bidirectional_source_adjacency_equal": True,
        "declared_lr_query_family_complete": True,
        "p854_candidate_urls": [], "source_support_paid": False,
        "creates_world_truth": False, "edit_authority": False,
        "persisted_s29_review_verified": False,
        "p854_primary_documents_independently_verified": False,
        "wikipedia_oldid_first_qualified_link_same_as_zelph_edge_proved": False,
    }


class S29Offline(unittest.TestCase):
    def test_exact_two_provenance_coordinates_not_promoted(self):
        c = candidate()
        sources = candidate_sources(c)
        self.assertEqual(len(sources), 2)
        self.assertEqual(sources[0][1], "a" * 40)
        self.assertEqual(sources[1][1], "101")
        self.assertEqual(len(candidate_receipt(c)), 64)

    def test_refuse_provenance_truth_edit_and_fake_s29(self):
        for bad in ("source_support_paid", "creates_world_truth",
                    "edit_authority", "persisted_s29_review_verified",
                    "p854_primary_documents_independently_verified",
                    "wikipedia_oldid_first_qualified_link_same_as_zelph_edge_proved"):
            c = candidate()
            c[bad] = True
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                candidate_sources(c)

    def test_reject_bad_native_or_physical_coordinates(self):
        for field, value in (("qid", "Q0"),
                             ("graph_dataset_revision", "main"),
                             ("wikibase_native_entity_revision_id", True),
                             ("original_bin_sha256_matched_supplied_pin", "0"),
                             ("declared_lr_query_family_complete", False)):
            c = candidate()
            c[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                candidate_sources(c)

    def test_receipt_order_invariant_but_content_sensitive(self):
        c = candidate()
        d = dict(reversed(list(c.items())))
        self.assertEqual(candidate_receipt(c), candidate_receipt(d))
        d["wikibase_native_entity_revision_id"] = 102
        self.assertNotEqual(candidate_receipt(c), candidate_receipt(d))


class S29RealDatabase(unittest.TestCase):
    @unittest.skipUnless(os.environ.get("SLR_S29_TEST_DATABASE_URL"),
                         "set SLR_S29_TEST_DATABASE_URL to run actual PostgreSQL")
    def test_commit_reload_and_sparse_revision_reopen(self):
        try:
            import psycopg
        except ImportError:
            self.fail("psycopg v3 required for actual S29 acceptance")
        # Unique fixture coordinate, not actual Wikibase or paid P854 evidence.
        qid = "Q1" + str(uuid.uuid4().int)[:14]
        database = os.environ["SLR_S29_TEST_DATABASE_URL"]
        first = candidate(qid=qid)
        query_ref = "test:query:" + uuid.uuid4().hex
        with psycopg.connect(database) as conn:
            install(conn)
            saved = persist_candidate(
                conn, first, consumer_ref="test:consumer",
                query_ref=query_ref, policy_ref="test:policy")
            review_id = add_explicit_review(
                conn, saved["receipt_digest"], reviewer="test:fixture",
                disposition="held", rationale="fixture only; no source paid")
        with psycopg.connect(database) as fresh:
            replay = readback(fresh, saved["receipt_digest"])
            self.assertTrue(replay["db_readback_verified"])
            self.assertEqual(replay["reopen_events"], [])
            with fresh.cursor() as cur:
                cur.execute("SELECT COUNT(*) FROM sl_s29_review_events_v1 WHERE event_id=%s",
                            (review_id,))
                self.assertEqual(cur.fetchone()[0], 1)
        second = candidate(qid=qid, graph_revision="b" * 40, bin_hash="3" * 64)
        with psycopg.connect(database) as conn:
            changed = persist_candidate(conn, second)
            self.assertEqual(len(changed["new_sparse_wake_events"]), 1)
            self.assertTrue(changed["new_sparse_wake_events"][0][0].startswith("zelph:"))
        with psycopg.connect(database) as fresh:
            replay = readback(fresh, changed["receipt_digest"])
            self.assertEqual(len(replay["reopen_events"]), 1)
            self.assertEqual(replay["reopen_events"][0][1], "a" * 40)
            self.assertEqual(replay["reopen_events"][0][2], "b" * 40)
            # Replay the exact same original receipt: no duplicated wakes.
            with psycopg.connect(database) as conn2:
                persist_candidate(conn2, second)
            with psycopg.connect(database) as conn3:
                self.assertEqual(len(readback(conn3, changed["receipt_digest"])["reopen_events"]), 1)
        forged = candidate(qid=qid, graph_revision="b" * 40, bin_hash="f" * 64)
        with self.assertRaises(ValueError):
            with psycopg.connect(database) as conn:
                persist_candidate(conn, forged)


if __name__ == "__main__":
    unittest.main()
