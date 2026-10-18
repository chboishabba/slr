# Official HCA PDF extraction with pinned OALC-compatible semantics

This lane exists for an official High Court judgment that is newer than the currently pinned Open Australian Legal Corpus Parquet snapshot.

It does **not** substitute another legal source and does **not** claim that the judgment is present in that OALC corpus revision.

```text
official hcourt.gov.au PDF
  -> exact PDF SHA-256
  -> pinned extraction implementation
  -> UTF-8 text + exact output SHA-256
  -> source_provenance.official_primary_extraction
  -> existing provider materialisation
  -> existing legal-source registration
  -> exact slice
  -> candidate PNF
  -> review / controversy / proof-obligation pipeline
```

## Extractor provenance

The narrow PDF transformation follows the implementation used by:

- repository: `https://github.com/isaacus-dev/open-australian-legal-corpus-creator`
- pinned revision: `dbdb9e3752394a87a9c966c9a9199f9685db9af2`
- PDF primitive: `src/oalc_creator/ocr.py::pdf2txt`
- HCA caller: `src/oalc_creator/scrapers/high_court_of_australia.py`
- project license: MIT

The HCA scraper uses PDF rendering at scale 2, OCR via `tesserocr`, joins page text with newlines, and applies the corpus creator's paragraph-number cleanup. SensibLaw vendors only this narrow transformation behavior in `tools/extract_hca_pdf_oalc_compatible.py`; it does not vendor the crawler, corpus indices, checkpoints, or corpus builder.

## Reproduction

```bash
python tools/extract_hca_pdf_oalc_compatible.py judgment.pdf \
  --source-url 'https://www.hcourt.gov.au/...' \
  --citation 'Smith v Chief Executive, Queensland Corrective Services [2026] HCA 37' \
  --expected-source-sha256 '<sha256 of official PDF>' \
  --text-out smith.txt \
  --receipt-out smith-extraction.json

cargo run -p sensiblaw-cli --example materialize_hca_official_pdf -- \
  demand:smith \
  'Smith v Chief Executive, Queensland Corrective Services [2026] HCA 37' \
  'B58-2026:[2026] HCA 37' \
  'https://www.hcourt.gov.au/...' \
  smith.txt \
  smith-extraction.json \
  primary_case_law \
  high_court \
  australian-common-law \
  primary-authority
```

The materialisation uses the official PDF digest as the immutable upstream revision coordinate (`sha256:<pdf digest>`). The canonical text digest is distinct and is welded to the extraction receipt.

## Authority boundary

```text
official PDF identity != extracted text identity
OALC creator implementation != OALC corpus membership
successful OCR != semantic correctness
extracted text != reviewed evidence
candidate PNF != legal authority
court source authority != automatic applicability
formal/runtime receipt != claim truth
```

The extraction receipt is source-observation provenance only. It cannot create semantic authority, legal authority, applicability, or claim truth.
