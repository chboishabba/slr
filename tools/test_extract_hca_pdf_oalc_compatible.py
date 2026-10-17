import extract_hca_pdf_oalc_compatible as extractor


def test_pdf_runtime_provenance_records_renderer_ocr_and_tesseract_versions():
    versions = extractor.runtime_tool_versions()
    assert versions["package_versions"]["pypdfium2"]
    assert versions["package_versions"]["tesserocr"]
    assert versions["tesseract_version"]
