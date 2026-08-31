#!/usr/bin/env python3
"""Full GWB v0.1 tranche preparation + direct Rust semantic run.

Two explicit phases:
  prepare: source inventory -> canonical projected UTF-8 text + hash manifest
  run:     projected text -> spaCy observations -> Rust direct-delta compiler

Source projection is intentionally outside the parser-relative performance gate.
Derived JSON/timeline artifacts in the broad historical directory are not recursively
re-ingested as source narrative. The v0.1 raw payload is six biography HTML files
plus four books (EPUB/PDF), deduplicated by resolved path.
"""
from __future__ import annotations

import argparse
import hashlib
import html.parser
import json
from pathlib import Path
import re
import subprocess
import sys
import time
import zipfile
import xml.etree.ElementTree as ET

import spacy

PROFILE_REF = "tranche-profile:gwb:v0_1"
PUBLIC_BIOS = "source-family:gwb-public-bios:v1"
BOOKS = "source-family:gwb-books:v1"
PROJECTION_SCHEMA = "sensiblaw.gwb-source-projection.v0_1"
RUN_SCHEMA = "sensiblaw.gwb-direct-tranche-receipt.v0_1"
RAW_SUFFIXES = {".html", ".htm", ".epub", ".pdf", ".txt"}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8"))


class TextHTMLParser(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []
        self._suppressed = 0

    def handle_starttag(self, tag: str, attrs) -> None:
        if tag in {"script", "style", "noscript"}:
            self._suppressed += 1
        elif tag in {"p", "div", "section", "article", "header", "footer", "li", "br", "h1", "h2", "h3", "h4", "h5", "h6"}:
            self.parts.append("\n")

    def handle_endtag(self, tag: str) -> None:
        if tag in {"script", "style", "noscript"}:
            self._suppressed = max(0, self._suppressed - 1)
        elif tag in {"p", "div", "section", "article", "li", "h1", "h2", "h3", "h4", "h5", "h6"}:
            self.parts.append("\n")

    def handle_data(self, data: str) -> None:
        if not self._suppressed:
            self.parts.append(data)

    def text(self) -> str:
        joined = "".join(self.parts)
        joined = re.sub(r"[ \t\f\v]+", " ", joined)
        joined = re.sub(r" *\n *", "\n", joined)
        joined = re.sub(r"\n{3,}", "\n\n", joined)
        return joined.strip() + "\n"


def html_to_text(data: bytes) -> str:
    parser = TextHTMLParser()
    parser.feed(data.decode("utf-8", errors="replace"))
    return parser.text()


def epub_to_text(path: Path) -> str:
    with zipfile.ZipFile(path) as zf:
        container = ET.fromstring(zf.read("META-INF/container.xml"))
        rootfile = None
        for elem in container.iter():
            if elem.tag.endswith("rootfile"):
                rootfile = elem.attrib.get("full-path")
                break
        if not rootfile:
            raise ValueError("EPUB has no rootfile")
        opf = ET.fromstring(zf.read(rootfile))
        base = Path(rootfile).parent
        manifest: dict[str, str] = {}
        spine: list[str] = []
        for elem in opf.iter():
            if elem.tag.endswith("item") and elem.attrib.get("id") and elem.attrib.get("href"):
                manifest[elem.attrib["id"]] = elem.attrib["href"]
            elif elem.tag.endswith("itemref") and elem.attrib.get("idref"):
                spine.append(elem.attrib["idref"])
        parts: list[str] = []
        for item_id in spine:
            href = manifest.get(item_id)
            if not href:
                continue
            member = (base / href).as_posix()
            try:
                data = zf.read(member)
            except KeyError:
                continue
            parts.append(html_to_text(data).strip())
        return "\n\n".join(p for p in parts if p).strip() + "\n"


def pdf_to_text(path: Path) -> tuple[str, str]:
    try:
        from pypdf import PdfReader  # type: ignore
        reader = PdfReader(str(path))
        text = "\n\n".join((page.extract_text() or "") for page in reader.pages)
        return text.strip() + "\n", "pypdf"
    except ImportError:
        pass
    try:
        from pdfminer.high_level import extract_text  # type: ignore
        return extract_text(str(path)).strip() + "\n", "pdfminer.six"
    except ImportError as exc:
        raise RuntimeError("PDF projection requires pypdf or pdfminer.six") from exc


def project_source(path: Path) -> tuple[str, str]:
    suffix = path.suffix.lower()
    if suffix in {".html", ".htm"}:
        return html_to_text(path.read_bytes()), "html.parser:v1"
    if suffix == ".epub":
        return epub_to_text(path), "epub-spine-html.parser:v1"
    if suffix == ".pdf":
        return pdf_to_text(path)
    if suffix == ".txt":
        return path.read_text(encoding="utf-8", errors="replace"), "utf8-text:v1"
    raise ValueError(f"unsupported raw source suffix: {suffix}")


def load_inventory(path: Path) -> dict:
    obj = json.loads(path.read_text(encoding="utf-8"))
    profile = obj.get("profile", {})
    if profile.get("profile_ref") != PROFILE_REF or profile.get("tranche") != "GWB":
        raise ValueError(f"inventory is not {PROFILE_REF}")
    by_ref = {x.get("family_ref"): x for x in obj.get("source_families", [])}
    for required in (PUBLIC_BIOS, BOOKS):
        family = by_ref.get(required)
        if not family or not family.get("exists") or not family.get("required"):
            raise ValueError(f"required GWB source family unavailable: {required}")
    return obj


def raw_source_paths(inventory: dict, source_root: Path) -> list[tuple[str, Path]]:
    seen: set[Path] = set()
    out: list[tuple[str, Path]] = []
    for family in inventory.get("source_families", []):
        family_ref = family.get("family_ref", "")
        for rel in family.get("files", []):
            p = (source_root / rel).resolve()
            if p.suffix.lower() not in RAW_SUFFIXES or p in seen:
                continue
            seen.add(p)
            out.append((family_ref, p))
    return out


def cmd_prepare(ns: argparse.Namespace) -> int:
    inventory_path = Path(ns.inventory).resolve()
    source_root = Path(ns.source_root).resolve()
    output = Path(ns.output).resolve()
    text_dir = output / "text"
    text_dir.mkdir(parents=True, exist_ok=True)
    inventory = load_inventory(inventory_path)
    sources = raw_source_paths(inventory, source_root)
    if ns.strict_v01:
        bio_count = sum(1 for family_ref, path in sources if family_ref == PUBLIC_BIOS and path.suffix.lower() in {".html", ".htm"})
        book_count = sum(1 for family_ref, path in sources if family_ref == BOOKS and path.suffix.lower() in {".epub", ".pdf"})
        if len(sources) != 10 or bio_count != 6 or book_count != 4:
            raise SystemExit(
                f"GWB v0.1 strict raw payload is 6 public-bio HTML + 4 books; "
                f"found total={len(sources)} bios={bio_count} books={book_count}"
            )

    documents = []
    for index, (family_ref, path) in enumerate(sources, 1):
        if not path.exists():
            raise FileNotFoundError(path)
        raw = path.read_bytes()
        t0 = time.perf_counter_ns()
        text, projector = project_source(path)
        projection_ns = time.perf_counter_ns() - t0
        out_name = f"{index:04d}_{sha256_bytes(str(path).encode())[:12]}.txt"
        out_path = text_dir / out_name
        out_path.write_text(text, encoding="utf-8")
        documents.append({
            "document_ordinal": index,
            "family_ref": family_ref,
            "source_path": str(path),
            "source_sha256": sha256_bytes(raw),
            "source_bytes": len(raw),
            "projector": projector,
            "projection_ns": projection_ns,
            "projected_path": str(out_path),
            "projected_sha256": sha256_text(text),
            "projected_bytes": len(text.encode("utf-8")),
        })

    manifest = {
        "schema_version": PROJECTION_SCHEMA,
        "authority": "source_projection_only",
        "profile_ref": PROFILE_REF,
        "source_inventory": str(inventory_path),
        "source_inventory_sha256": sha256_bytes(inventory_path.read_bytes()),
        "document_count": len(documents),
        "documents": documents,
        "derived_inventory_artifacts_reingested": False,
    }
    manifest_path = output / "source_projection.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(manifest_path)
    print(f"GWB_PROJECTED documents={len(documents)} bytes={sum(d['projected_bytes'] for d in documents)}")
    return 0


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def paragraph_blocks(text: str):
    pos = 0
    for m in re.finditer(r"(?:\r?\n){2,}", text):
        block = text[pos:m.start()]
        if block.strip():
            lead = len(block) - len(block.lstrip())
            trail_end = len(block.rstrip())
            yield pos + lead, block[lead:trail_end]
        pos = m.end()
    block = text[pos:]
    if block.strip():
        lead = len(block) - len(block.lstrip())
        trail_end = len(block.rstrip())
        yield pos + lead, block[lead:trail_end]


def emit_document(nlp, sink, text: str, revision_id: int, sentence_id: int, paragraph_id: int):
    sink.write(f"D\t{revision_id}\n"); sink.flush()
    parse_ns = 0
    sentences = 0
    paragraphs = 0
    for block_start, block in paragraph_blocks(text):
        sink.write(f"P\t{paragraph_id}\n"); sink.flush()
        t0 = time.perf_counter_ns()
        doc = nlp(block)
        parse_ns += time.perf_counter_ns() - t0
        for sent in doc.sents:
            if not sent.text.strip():
                continue
            abs_start = block_start + sent.start_char
            abs_end = block_start + sent.end_char
            sink.write(f"S\t{sentence_id}\t{abs_start}\t{abs_end}\n")
            for local, tok in enumerate(sent):
                if tok.dep_:
                    head_local = tok.head.i - sent.start
                    dep = tok.dep_
                else:
                    head_local = local
                    dep = "-"
                lemma = tok.lemma_ if tok.lemma_ else tok.text
                pos = tok.pos_ if tok.pos_ else "-"
                tag = tok.tag_ if tok.tag_ else "-"
                a = block_start + tok.idx
                b = a + len(tok.text)
                sink.write("\t".join(("T", str(local), str(a), str(b), str(head_local), esc(tok.text), esc(lemma), pos, tag, dep)) + "\n")
            sink.write(f"E\t{sentence_id}\n"); sink.flush()
            sentence_id += 1
            sentences += 1
        sink.write(f"Q\t{paragraph_id}\n"); sink.flush()
        paragraph_id += 1
        paragraphs += 1
    sink.write(f"M\tspacy_parse_ns={parse_ns}\n"); sink.flush()
    return sentence_id, paragraph_id, parse_ns, sentences, paragraphs


SL_METRIC_RE = re.compile(
    r"SL_METRIC active_ns=(\d+) pipeline_wall_ns=(\d+) sentences=(\d+) paragraphs=(\d+) "
    r"candidates=(\d+) residuals=(\d+) symbols=(\d+) published=(\d+) parity_checked=(\d+) parity_failed=(\d+)"
)


def load_spacy(model: str):
    return spacy.load(model)


def performance_tier(ratio: float) -> str:
    if ratio <= 1.2:
        return "production_1_2x"
    if ratio <= 1.5:
        return "production_1_5x"
    if ratio <= 2.0:
        return "architectural_2_0x"
    return "fail_over_2_0x"


def cmd_run(ns: argparse.Namespace) -> int:
    manifest_path = Path(ns.manifest).resolve()
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != PROJECTION_SCHEMA or manifest.get("profile_ref") != PROFILE_REF:
        raise SystemExit("not a GWB v0.1 source projection manifest")
    docs = manifest.get("documents", [])
    if ns.limit is not None:
        docs = docs[: ns.limit]
    rust_bin = Path(ns.rust_bin).resolve()
    if not rust_bin.exists():
        raise SystemExit(f"missing Rust stream binary: {rust_bin}")

    nlp = load_spacy(ns.model)
    proc = subprocess.Popen([str(rust_bin)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
    assert proc.stdin is not None
    proc.stdin.write("C\tparity=1\n"); proc.stdin.flush()
    pipeline_external_start = time.perf_counter_ns()
    parse_ns = 0
    sid = 0
    pid = 0
    per_document = []
    for revision, doc in enumerate(docs, 1):
        text = Path(doc["projected_path"]).read_text(encoding="utf-8")
        before_sid, before_pid = sid, pid
        sid, pid, d_parse, _, _ = emit_document(nlp, proc.stdin, text, revision, sid, pid)
        parse_ns += d_parse
        per_document.append({
            "document_ordinal": doc["document_ordinal"],
            "projected_sha256": doc["projected_sha256"],
            "spacy_parse_ns": d_parse,
            "sentence_id_start": before_sid,
            "sentence_id_end_exclusive": sid,
            "paragraph_id_start": before_pid,
            "paragraph_id_end_exclusive": pid,
        })
    last_parser_emit_ns = time.perf_counter_ns()
    proc.stdin.close()
    stdout = proc.stdout.read() if proc.stdout else ""
    stderr = proc.stderr.read() if proc.stderr else ""
    rc = proc.wait()
    pipeline_external_ns = time.perf_counter_ns() - pipeline_external_start
    post_parser_tail_ns = time.perf_counter_ns() - last_parser_emit_ns
    match = SL_METRIC_RE.search(stderr)
    if not match:
        print(stdout, file=sys.stderr)
        print(stderr, file=sys.stderr)
        raise SystemExit("Rust stream emitted no parseable SL_METRIC receipt")
    active_ns, pipeline_wall_ns, sentences, paragraphs, candidates, residuals, symbols, published, parity_checked, parity_failed = map(int, match.groups())
    ratio = pipeline_wall_ns / max(parse_ns, 1)
    gate_pass = rc == 0 and parity_failed == 0 and pipeline_wall_ns <= 2 * parse_ns
    receipt = {
        "schema_version": RUN_SCHEMA,
        "authority": "execution_and_parity_receipt_only",
        "profile_ref": PROFILE_REF,
        "projection_manifest": str(manifest_path),
        "projection_manifest_sha256": sha256_bytes(manifest_path.read_bytes()),
        "document_count": len(docs),
        "projected_bytes": sum(int(d["projected_bytes"]) for d in docs),
        "spacy_model": ns.model,
        "spacy_version": spacy.__version__,
        "rust_return_code": rc,
        "metrics": {
            "spacy_parser_wall_occupancy_ns": parse_ns,
            "sensiblaw_active_ns": active_ns,
            "pipeline_wall_ns": pipeline_wall_ns,
            "external_controller_wall_ns": pipeline_external_ns,
            "post_parser_tail_ns": post_parser_tail_ns,
            "parser_relative_ratio": ratio,
            "performance_tier": performance_tier(ratio),
            "architectural_2x_gate_pass": pipeline_wall_ns <= 2 * parse_ns,
            "sentences": sentences,
            "paragraphs": paragraphs,
            "candidate_deltas": candidates,
            "residuals": residuals,
            "symbols": symbols,
            "published": bool(published),
            "parity_checked": parity_checked,
            "parity_failed": parity_failed,
        },
        "invariants": {
            "direct_reference_parity": parity_failed == 0 and parity_checked == sentences,
            "parser_did_not_publish": published == 0,
            "full_gate_pass": gate_pass,
            "source_projection_timing_excluded_from_parser_relative_gate": True,
        },
        "per_document": per_document,
    }
    output = Path(ns.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"GWB_RUN documents={len(docs)} sentences={sentences} candidates={candidates} residuals={residuals}")
    print(f"GWB_PARITY checked={parity_checked} failed={parity_failed}")
    print(f"GWB_PERF total/spacy={ratio:.4f}x tier={performance_tier(ratio)} gate={'PASS' if gate_pass else 'FAIL'}")
    print(output)
    if stderr.strip():
        print(stderr.strip(), file=sys.stderr)
    return 0 if gate_pass else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="command", required=True)
    prep = sub.add_parser("prepare")
    prep.add_argument("--inventory", required=True)
    prep.add_argument("--source-root", required=True)
    prep.add_argument("--output", required=True)
    prep.add_argument("--strict-v01", action=argparse.BooleanOptionalAction, default=True)
    prep.set_defaults(func=cmd_prepare)

    run = sub.add_parser("run")
    run.add_argument("--manifest", required=True)
    run.add_argument("--rust-bin", default="target/release/sensiblaw-stream")
    run.add_argument("--model", default="en_core_web_sm")
    run.add_argument("--output", required=True)
    run.add_argument("--limit", type=int)
    run.set_defaults(func=cmd_run)
    ns = ap.parse_args()
    return ns.func(ns)


if __name__ == "__main__":
    raise SystemExit(main())
