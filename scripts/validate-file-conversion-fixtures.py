#!/usr/bin/env python3
"""Validate the committed, non-sensitive Zero File conversion corpus."""

from __future__ import annotations

import hashlib
import json
import logging
import sys
import zipfile
from pathlib import Path

from lxml import etree
from pypdf import PdfReader
from pypdf.errors import FileNotDecryptedError, PdfReadError


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures" / "fileConversion"
MANIFEST = FIXTURES / "expected.json"
RELATIONSHIPS_NS = "http://schemas.openxmlformats.org/package/2006/relationships"
FONT_RELATIONSHIP_TYPE = (
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/font"
)
REQUIRED_DOCX_ENTRIES = {
    "[Content_Types].xml",
    "word/document.xml",
    "word/fontTable.xml",
    "word/_rels/fontTable.xml.rels",
    "word/fonts/Noto Sans SC.odttf",
}

logging.getLogger("pypdf").setLevel(logging.ERROR)


def fail(message: str) -> None:
    raise AssertionError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def assert_valid_docx(path: Path) -> None:
    if not zipfile.is_zipfile(path):
        fail(f"{path.name}: expected a ZIP-based DOCX container")

    with zipfile.ZipFile(path) as package:
        names = set(package.namelist())
        missing = REQUIRED_DOCX_ENTRIES - names
        if missing:
            fail(f"{path.name}: missing DOCX entries {sorted(missing)}")

        relationships_xml = package.read("word/_rels/fontTable.xml.rels")
        if not relationships_xml.lstrip().startswith(b"<?xml"):
            fail(f"{path.name}: font relationships must be serialized as XML")
        if b'<Relationships xmlns="' + RELATIONSHIPS_NS.encode() + b'"' not in relationships_xml:
            fail(
                f"{path.name}: font relationships must use the default namespace "
                "for LibreOffice compatibility"
            )

        relationships = etree.fromstring(relationships_xml)
        font_links = relationships.xpath(
            "./r:Relationship[@Type=$type and @Target=$target]",
            namespaces={"r": RELATIONSHIPS_NS},
            type=FONT_RELATIONSHIP_TYPE,
            target="fonts/Noto Sans SC.odttf",
        )
        if len(font_links) != 1:
            fail(f"{path.name}: expected exactly one embedded CJK font relationship")

        if package.getinfo("word/fonts/Noto Sans SC.odttf").file_size == 0:
            fail(f"{path.name}: embedded CJK font is empty")


def extract_pdf_text(path: Path) -> tuple[PdfReader, str]:
    reader = PdfReader(path)
    return reader, "".join(page.extract_text() or "" for page in reader.pages)


def assert_encrypted_pdf(path: Path) -> None:
    reader = PdfReader(path)
    if not reader.is_encrypted:
        fail(f"{path.name}: expected encryption")
    if reader.decrypt("incorrect-password") != 0:
        fail(f"{path.name}: an incorrect password unexpectedly succeeded")
    try:
        _ = reader.pages[0]
    except FileNotDecryptedError:
        pass
    else:
        fail(f"{path.name}: page access should fail without the correct password")

    reader = PdfReader(path)
    if reader.decrypt("zero-test") == 0 or len(reader.pages) != 1:
        fail(f"{path.name}: fixture password did not unlock the expected page")


def assert_malformed_inputs() -> None:
    malformed_docx = FIXTURES / "malformed.docx"
    if zipfile.is_zipfile(malformed_docx):
        fail("malformed.docx: unexpectedly parsed as a ZIP container")

    malformed_pdf = FIXTURES / "malformed.pdf"
    if malformed_pdf.read_bytes().startswith(b"%PDF-"):
        fail("malformed.pdf: unexpectedly contains a PDF header")
    try:
        PdfReader(malformed_pdf)
    except (PdfReadError, ValueError):
        pass
    else:
        fail("malformed.pdf: unexpectedly parsed as a PDF")

    if not (FIXTURES / "~$office-lock.docx").name.startswith("~$"):
        fail("Office lock fixture does not use the reserved prefix")
    if (FIXTURES / "unsupported.txt").suffix != ".txt":
        fail("unsupported fixture does not exercise an unsupported extension")


def main() -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if manifest.get("containsUserData") is not False:
        fail("manifest must explicitly declare containsUserData=false")

    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        fail("manifest cases must be a non-empty array")

    for case in cases:
        path = FIXTURES / case["file"]
        if not path.is_file():
            fail(f"missing fixture: {case['file']}")
        actual = sha256(path)
        if actual != case["sha256"]:
            fail(f"{path.name}: SHA-256 mismatch; expected {case['sha256']}, got {actual}")

    rich_docx = next(case for case in cases if case["file"] == "rich-layout.docx")
    required_coverage = {
        "latin",
        "cjk",
        "fonts",
        "headings",
        "table",
        "image",
        "columns",
        "header",
        "footer",
        "pageBreak",
    }
    missing_coverage = required_coverage - set(rich_docx["coverage"])
    if missing_coverage:
        fail(f"rich-layout.docx: missing coverage {sorted(missing_coverage)}")

    assert_valid_docx(FIXTURES / "rich-layout.docx")
    assert_valid_docx(FIXTURES / "large-structured.docx")

    rich_pdf, rich_text = extract_pdf_text(FIXTURES / "rich-layout.pdf")
    if len(rich_pdf.pages) != 2 or "中文" not in rich_text or "Table and image" not in rich_text:
        fail("rich-layout.pdf: expected two pages with extractable CJK and table labels")

    large_pdf, large_text = extract_pdf_text(FIXTURES / "large-structured.pdf")
    if len(large_pdf.pages) != 24 or "中文测试内容" not in large_text:
        fail("large-structured.pdf: expected 24 pages with extractable CJK text")

    scan_pdf, scan_text = extract_pdf_text(FIXTURES / "image-only-scan.pdf")
    if len(scan_pdf.pages) != 1 or scan_text.strip():
        fail("image-only-scan.pdf: expected one page with no text layer")

    assert_encrypted_pdf(FIXTURES / "encrypted.pdf")
    assert_malformed_inputs()

    print(f"Validated {len(cases)} file-conversion fixtures and all structural expectations.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"fixture validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
