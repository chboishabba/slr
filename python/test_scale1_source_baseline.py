#!/usr/bin/env python3
"""Pure regressions for generic SCALE-1 canonical source ingress."""
import importlib.util
from pathlib import Path
import hashlib
import unittest
from unittest.mock import patch
import json
import subprocess

spec = importlib.util.spec_from_file_location(
    "source_baseline", Path(__file__).with_name("scale1_source_baseline.py")
)
baseline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(baseline)


def example_receipt(family="chat", body=b"hello"):
    return {
        "source_family": family,
        "input_transport": "stdin",
        "source": {
            "content_digest_ref": "sha256:" + hashlib.sha256(body).hexdigest(),
            "canonical_bytes": len(body),
            "canonical_bytes_reload_identically": True,
        },
        "integrity": {
            "source_region_loss_count": 0,
            "unattempted_semantic_regions": 0,
            "every_region_reloaded": True,
            "candidate_pnf_reopen_complete": True,
            "creates_semantic_authority": False,
            "applicability_promoted": False,
            "claim_truth_promoted": False,
            "semantic_eligible_regions": 2,
            "parser_success_regions": 1,
            "parser_residual_regions": 1,
        },
        "parser": {"deferred_retry_this_run": 0},
    }


class GenericBaselineTests(unittest.TestCase):
    def test_chat_and_mail_are_not_coerced_to_document(self):
        for family in ("chat", "mail", "audio", "web", "document"):
            with self.subTest(family=family):
                baseline.validate_compiler_receipt(example_receipt(family), family, b"hello")

    def test_wrong_family_rejected(self):
        with self.assertRaisesRegex(ValueError, "family"):
            baseline.validate_compiler_receipt(example_receipt("document"), "chat", b"hello")

    def test_changed_canonical_bytes_rejected(self):
        with self.assertRaisesRegex(ValueError, "digest"):
            baseline.validate_compiler_receipt(example_receipt(), "chat", b"changed")

    def test_fail_closed_integrity_gates(self):
        modifications = [
            ("source", "canonical_bytes_reload_identically", False),
            ("integrity", "source_region_loss_count", 1),
            ("integrity", "unattempted_semantic_regions", 1),
            ("integrity", "every_region_reloaded", False),
            ("integrity", "candidate_pnf_reopen_complete", False),
            ("integrity", "creates_semantic_authority", True),
            ("integrity", "applicability_promoted", True),
            ("integrity", "claim_truth_promoted", True),
            ("integrity", "parser_residual_regions", 2),
            ("parser", "deferred_retry_this_run", 1),
        ]
        for section, name, value in modifications:
            with self.subTest(section=section, name=name):
                receipt = example_receipt()
                receipt[section][name] = value
                with self.assertRaises(ValueError):
                    baseline.validate_compiler_receipt(receipt, "chat", b"hello")

    def test_source_family_is_passed_to_existing_compiler(self):
        receipt = example_receipt("mail")
        process = subprocess.CompletedProcess([], 0, json.dumps(receipt).encode(), b"")
        with patch.object(baseline.subprocess, "run", return_value=process) as run:
            output = baseline.compile_canonical(
                binary="compiler", family="mail", source_ref="s", provider_ref="p",
                acquisition_ref="a", label="message", model="m", config="{}",
                parser_script="parser", batch_size=4, canonical=b"hello"
            )
        self.assertEqual(run.call_args.args[0][1:3], ["compile-source-stdin", "mail"])
        self.assertEqual(run.call_args.kwargs["input"], b"hello")
        self.assertEqual(output["source_family"], "mail")
        self.assertEqual(
            output["authority"], "canonical_text_execution_only_not_source_adapter_parity"
        )

    def test_invalid_utf8_rejected_before_subprocess(self):
        with patch.object(baseline.subprocess, "run") as run:
            with self.assertRaisesRegex(ValueError, "UTF-8"):
                baseline.compile_canonical(
                    binary="compiler", family="mail", source_ref="s", provider_ref="p",
                    acquisition_ref="a", label="m", model="m", config="{}",
                    parser_script="parser", batch_size=4, canonical=b"\xff"
                )
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
