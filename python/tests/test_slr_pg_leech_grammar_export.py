"""Native SLR Rust-PG source → Leech grammar export source contracts.

Pure fixture tests; a real PostgreSQL parser run/corpus is necessary for
physical source export and is NOT implied by these tests.
"""
import hashlib,json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import Mock
from python.slr_pg_leech_grammar_export import (
    SOURCE_SQL,JOB_SQL,TOKEN_SQL,
    project_native_pg_document,validate_manifest,export_native_pg_manifest)


def fixture(source_text="Éli saw Max.",rev="revision:1",run="parser:1",split="train"):
    raw=source_text.encode("utf-8")
    spec=dict(source_revision_ref=rev,parser_run_ref=run,split=split,
              source_sha256=hashlib.sha256(raw).hexdigest(),
              document_ref="document:1")
    source=dict(parser_run_ref=run,source_revision_ref=rev,
                parser_family="spacy",parser_version="3.8",
                model_ref="en_core_web_sm",config_digest_ref="sha256:cfg",
                status="complete",candidate_only=True,
                creates_semantic_authority=False,
                applicability_promoted=False,claim_truth_promoted=False,
                document_ref="document:1",content_digest_ref="sha256:"+
                hashlib.sha256(raw).hexdigest(),source_candidate_only=True,
                source_authority=False,source_applicability=False,
                source_truth=False,payload=raw)
    jobs=[dict(compilation_key="comp:1",region_ref="region:1",
               source_revision_ref=rev,region_start_char=0,
               region_end_char=len(source_text),status="succeeded",
               candidate_only=True,creates_semantic_authority=False,
               applicability_promoted=False,claim_truth_promoted=False)]
    # All spans are Unicode characters, NOT UTF-8 byte offsets.
    spans=[(0,3,"Éli"),(4,7,"saw"),(8,11,"Max"),(11,12,".")]
    tokens=[]
    for i,(start,end,surface) in enumerate(spans):
        tokens.append(dict(compilation_key="comp:1",token_ordinal=i,
            start_char=start,end_char=end,surface=surface,lemma=surface,
            pos=("PROPN" if i in (0,2) else "VERB" if i==1 else "PUNCT"),
            dependency_ref="ROOT" if i==1 else "nsubj" if i==0 else
                "dobj" if i==2 else "punct",
            morph_json='{"Number":"Sing"}' if i<3 else None,
            head_ordinal=1))
    return spec,source,jobs,tokens


class TestNativePgLeechBoundary(unittest.TestCase):
    def test_exact_native_pg_rows_export_without_promotion(self):
        spec,source,jobs,tokens=fixture()
        a=project_native_pg_document(spec,source,jobs,tokens)
        b=project_native_pg_document(spec,source,jobs,tokens)
        self.assertEqual(a,b)
        self.assertEqual(a["schema_version"],"slr.numeric_spacy.leech_grammar.v1")
        self.assertEqual(a["source_coordinate_unit"],"unicode_codepoints")
        self.assertEqual(a["source_text"],"Éli saw Max.")
        self.assertEqual(a["source_sha256"],spec["source_sha256"])
        self.assertEqual([t["head_token_id"] for t in a["slr_tokens"]],[2]*4)
        self.assertEqual(a["slr_tokens"][0]["source_occurrence_ref"],"comp:1:0")
        self.assertEqual(a["slr_tokens"][0]["surface"],"Éli")
        self.assertEqual(a["slr_tokens"][0]["end_char"],3)
        self.assertEqual(a["slr_tokens"][0]["pos_symbol_digest"],
                         a["slr_tokens"][2]["pos_symbol_digest"])
        self.assertNotEqual(a["slr_tokens"][0]["pos_symbol_digest"],
                            a["slr_tokens"][1]["pos_symbol_digest"])
        self.assertIsNone(a["slr_tokens"][-1]["morph_set_digest"])
        self.assertTrue(a["world_resolution_deferred"])
        self.assertFalse(a["parser_observation_is_semantic_authority"])
        self.assertEqual(len(a["parser_receipt_sha256"]),64)

    def test_fails_on_source_SHA_or_noncanonical_document(self):
        spec,source,jobs,tokens=fixture()
        bad=dict(spec,source_sha256="f"*64)
        with self.assertRaisesRegex(ValueError,"source bytes"):
            project_native_pg_document(bad,source,jobs,tokens)
        bad=dict(source,content_digest_ref="sha256:"+"f"*64)
        with self.assertRaisesRegex(ValueError,"source bytes"):
            project_native_pg_document(spec,bad,jobs,tokens)
        bad=dict(spec,document_ref="other")
        with self.assertRaisesRegex(ValueError,"document identity"):
            project_native_pg_document(bad,source,jobs,tokens)

    def test_missing_semantics_and_job_residual_fail_closed(self):
        spec,source,jobs,tokens=fixture()
        for state in ("active","missing"):
            with self.assertRaisesRegex(ValueError,"not complete"):
                project_native_pg_document(spec,dict(source,status=state),jobs,tokens)
        with self.assertRaisesRegex(ValueError,"nonpromoting"):
            project_native_pg_document(
                spec,dict(source,claim_truth_promoted=True),jobs,tokens)
        with self.assertRaisesRegex(ValueError,"residual"):
            project_native_pg_document(
                spec,source,[dict(jobs[0],status="residual")],tokens)
        with self.assertRaisesRegex(ValueError,"parser run"):
            project_native_pg_document(dict(spec,parser_run_ref="different"),
                                        source,jobs,tokens)

    def test_invalid_source_surface_and_dependency_region(self):
        spec,source,jobs,tokens=fixture()
        bad=list(tokens)
        bad[0]=dict(bad[0],surface="Eli")
        with self.assertRaisesRegex(ValueError,"surface"):
            project_native_pg_document(spec,source,jobs,bad)
        bad=list(tokens)
        bad[2]=dict(bad[2],head_ordinal=99)
        with self.assertRaisesRegex(ValueError,"dependency head"):
            project_native_pg_document(spec,source,jobs,bad)
        bad=list(tokens)
        bad[1]=dict(bad[1],start_char=2)
        with self.assertRaisesRegex(ValueError,"nonoverlapping"):
            project_native_pg_document(spec,source,jobs,bad)
        bad=list(tokens)
        bad[1]=dict(bad[1],morph_json="{broken")
        with self.assertRaisesRegex(ValueError,"morph JSON"):
            project_native_pg_document(spec,source,jobs,bad)

    def test_F5_and_manifest_scope_rejected_before_database_access(self):
        spec,source,jobs,tokens=fixture()
        for split in ("reserved","test","F5","heldout"):
            with self.assertRaisesRegex(ValueError,"F5"):
                validate_manifest([dict(spec,split=split)])
        with self.assertRaisesRegex(ValueError,"both required"):
            validate_manifest([spec])
        with self.assertRaisesRegex(ValueError,"duplicate"):
            validate_manifest([spec,dict(spec,split="development")])
        with TemporaryDirectory() as root:
            manifest=Path(root)/"manifest.json"
            manifest.write_text(json.dumps([dict(spec,split="reserved")]))
            called=[]
            with self.assertRaisesRegex(ValueError,"F5"):
                export_native_pg_manifest(manifest,Path(root)/"out.jsonl",
                    lambda:called.append(True))
            self.assertFalse(called)

    def test_native_PG_SQL_uses_existing_corpus_and_parser_tables(self):
        self.assertIn("corpus.canonical_content",SOURCE_SQL)
        self.assertIn("ingest.generic_source_revision",SOURCE_SQL)
        self.assertIn("ingest.parser_run",SOURCE_SQL)
        self.assertIn("ingest.parser_job",JOB_SQL)
        self.assertIn("ingest.parser_token",TOKEN_SQL)
        self.assertNotIn("CREATE TABLE",SOURCE_SQL+JOB_SQL+TOKEN_SQL)
        self.assertNotIn("execution.semantic_parser",SOURCE_SQL+JOB_SQL+TOKEN_SQL)


if __name__=="__main__": unittest.main()
