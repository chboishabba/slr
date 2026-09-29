#!/usr/bin/env python3
"""Adversarial regressions for SCALE-1 retained and final scaling receipts."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent


def module(filename, name):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


UPGRADE = module("scale1_revalidate_scale_receipt.py", "scale_upgrade")
CLOSURE = module("scale1_production_scale_closure.py", "scale_closure")


def worker_point(count, duration):
    return {
        "worker_count": count,
        "worker_wall_ns": duration,
        "source_revision_ref": f"source:test:{count}",
        "parser_run_ref": f"parser:test:{count}",
        "aggregate_parser_process_ns": 100,
        "aggregate_parser_persist_ns": 50,
        "aggregate_worker_busy_ns": 150,
        "semantic_regions": 10,
        "new_jobs": 10,
        "reused_jobs": 0,
        "succeeded": 10,
        "residual": 0,
        "deferred_retry": 0,
        "tokens": 200,
        "job_tail": {"count": 10},
        "speedup_vs_one_worker": 100.0,
        "parallel_efficiency": 100.0,
    }


def worker_receipt(second_ns):
    return {
        "schema": "sensiblaw.scale1.worker-scaling-series.v0_1",
        "runtime_head": "head:test",
        "points": [worker_point(1, 1000), worker_point(2, second_ns)],
    }


def archive_point(tokens, work):
    return {
        "runtime_head": "head:test",
        "source_family": "document",
        "parser_model_ref": "model:test",
        "parser_config_digest_ref": "config:test",
        "represented_tokens": tokens,
        "source_revision_ref": f"archive-source:{tokens}",
        "semantic_regions": 10,
        "measured_elapsed_ns": 1000,
        "measured_post_parser_elapsed_ns": 500,
        "measured_post_parser_work_units": work,
    }


def archive_receipt(second_work):
    return {
        "schema": "sensiblaw.scale1.archive-scale-series.v0_1",
        "runtime_head": "head:test",
        "represented_carrier": "parser_tokens",
        "source_family": "document",
        "parser_model_ref": "model:test",
        "parser_config_digest_ref": "config:test",
        "observed_affine_envelope": {
            "slope_work_units_per_token": 0,
            "all_points_within": True,
            "scope": "observed_points_only_not_asymptotic_claim",
        },
        "points": [
            archive_point(100, 200),
            archive_point(300, second_work),
        ],
    }


def economy_receipt():
    return {
        "schema": "sensiblaw.scale1.economy-closure.v0_1",
        "runtime_head": "head:test",
        "scale1_economy_closed": True,
        "same_run_parser_dominance": {"green": True},
        "small_edit_locality": {"green": True},
        "same_domain_new_source": {"green": True},
    }


def final_closure(economy, worker, archive):
    with tempfile.TemporaryDirectory() as folder:
        folder = Path(folder)
        for name, value in (
            ("economy", economy),
            ("worker", worker),
            ("archive", archive),
        ):
            (folder / f"{name}.json").write_text(json.dumps(value))
        import sys
        previous = sys.argv
        sys.argv = [
            "scale1_production_scale_closure.py",
            "--economy", str(folder / "economy.json"),
            "--worker-scaling", str(folder / "worker.json"),
            "--archive-scale", str(folder / "archive.json"),
            "--output", str(folder / "result.json"),
        ]
        try:
            CLOSURE.main()
        finally:
            sys.argv = previous
        return json.loads((folder / "result.json").read_text())


class ScaleEvidenceRevalidationTests(unittest.TestCase):
    def test_worker_forged_parallel_speedup_is_rejected(self):
        with self.assertRaises(SystemExit):
            UPGRADE.upgrade_worker(worker_receipt(2000), 1.05, 0.2)

    def test_worker_uses_measured_wall_time(self):
        upgraded = UPGRADE.upgrade_worker(worker_receipt(500), 1.05, 0.2)
        self.assertEqual(upgraded["acceptance"]["observed_best_parallel_speedup"], 2.0)

    def test_archive_forged_envelope_is_rejected(self):
        with self.assertRaises(SystemExit):
            UPGRADE.upgrade_archive(archive_receipt(9000), 20, 2)

    def test_archive_envelope_is_rederived(self):
        upgraded = UPGRADE.upgrade_archive(archive_receipt(600), 3, 2)
        self.assertEqual(
            upgraded["observed_affine_envelope"]["slope_work_units_per_token"], 2
        )

    def test_final_gate_rejects_forged_worker_acceptance(self):
        worker = worker_receipt(2000)
        worker["schema"] = "sensiblaw.scale1.worker-scaling-series.v0_2"
        worker["acceptance"] = {
            "green": True,
            "min_best_parallel_speedup": 1.05,
            "observed_best_parallel_speedup": 100,
            "min_max_worker_efficiency": 0.2,
            "observed_max_worker_efficiency": 100,
        }
        archive = UPGRADE.upgrade_archive(archive_receipt(600), 3, 2)
        with self.assertRaises(SystemExit):
            final_closure(economy_receipt(), worker, archive)

    def test_final_gate_rejects_forged_archive_acceptance(self):
        worker = UPGRADE.upgrade_worker(worker_receipt(500), 1.05, 0.2)
        archive = archive_receipt(9000)
        archive["schema"] = "sensiblaw.scale1.archive-scale-series.v0_2"
        archive["acceptance"] = {
            "green": True,
            "max_work_units_per_token": 3,
            "observed_slope_work_units_per_token": 0,
            "min_token_span_ratio": 2,
            "observed_token_span_ratio": 3,
        }
        with self.assertRaises(SystemExit):
            final_closure(economy_receipt(), worker, archive)

    def test_worker_recycled_parser_product_is_rejected(self):
        raw = worker_receipt(500)
        raw["points"][1]["parser_run_ref"] = raw["points"][0]["parser_run_ref"]
        with self.assertRaises(SystemExit):
            UPGRADE.upgrade_worker(raw, 1.05, 0.2)

    def test_worker_contradictory_busy_counter_is_rejected(self):
        raw = worker_receipt(500)
        raw["points"][1]["aggregate_worker_busy_ns"] = 1
        with self.assertRaises(SystemExit):
            UPGRADE.upgrade_worker(raw, 1.05, 0.2)

    def test_archive_duplicate_source_is_rejected(self):
        raw = archive_receipt(600)
        raw["points"][1]["source_revision_ref"] = raw["points"][0]["source_revision_ref"]
        with self.assertRaises(SystemExit):
            UPGRADE.upgrade_archive(raw, 3, 2)

    def test_archive_missing_measured_elapsed_is_rejected(self):
        raw = archive_receipt(600)
        raw["points"][1]["measured_post_parser_elapsed_ns"] = 0
        with self.assertRaises(SystemExit):
            UPGRADE.upgrade_archive(raw, 3, 2)

    def test_final_gate_rechecks_identity_independently(self):
        worker = UPGRADE.upgrade_worker(worker_receipt(500), 1.05, 0.2)
        archive = UPGRADE.upgrade_archive(archive_receipt(600), 3, 2)
        worker["points"][1]["source_revision_ref"] = worker["points"][0]["source_revision_ref"]
        with self.assertRaises(SystemExit):
            final_closure(economy_receipt(), worker, archive)

    def test_final_gate_rechecks_archive_metadata_independently(self):
        worker = UPGRADE.upgrade_worker(worker_receipt(500), 1.05, 0.2)
        archive = UPGRADE.upgrade_archive(archive_receipt(600), 3, 2)
        archive["points"][1]["parser_model_ref"] = "different-model"
        with self.assertRaises(SystemExit):
            final_closure(economy_receipt(), worker, archive)

    def test_final_gate_accepts_measured_series(self):
        worker = UPGRADE.upgrade_worker(worker_receipt(500), 1.05, 0.2)
        archive = UPGRADE.upgrade_archive(archive_receipt(600), 3, 2)
        result = final_closure(economy_receipt(), worker, archive)
        self.assertTrue(result["scale1_production_scale_closed"])
        self.assertEqual(result["worker_scaling"]["observed_best_parallel_speedup"], 2)
        self.assertEqual(result["archive_scale"]["observed_work_slope_per_token"], 2)


if __name__ == "__main__":
    unittest.main()
