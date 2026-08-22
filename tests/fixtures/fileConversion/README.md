# Zero File conversion corpus

This directory contains generated, non-sensitive documents used to evaluate local PDF-to-DOCX and DOCX-to-PDF providers. No fixture comes from a private user document, and `expected.json` records the expected preflight result and SHA-256 of every executable test input.

## Coverage

- `rich-layout.docx` and `rich-layout.pdf`: Latin and CJK editable text, explicit fonts, headings, a table, an image, columns, headers/footers, and page breaks.
- `large-structured.docx` and `large-structured.pdf`: 24-page representative workloads with stable pagination and CJK text.
- `image-only-scan.pdf`: one raster-only page with no PDF text layer; expected to complete as a non-editable `layoutPreserving` DOCX without claiming OCR.
- `expected.json` records `editableReconstruction`, `layoutPreserving`, and `webRenderedPdf` expectations. Rotation/overlap confidence signals are covered by synthetic engine unit cases because rotating this timestamped corpus would make its visual baseline less representative.
- `encrypted.pdf`: one password-protected page; the fixture-only password is `zero-test`.
- `malformed.docx`, `malformed.pdf`, `~$office-lock.docx`, and `unsupported.txt`: invalid-container, malformed-header, Office lock-file, and unsupported-extension cases.
- `assets/OFL.txt`: the Noto Sans SC Open Font License notice. Each valid DOCX embeds a generated subset of the OFL font; no complete font binary is committed separately.

## Reproduction and validation

The generator never downloads a font or conversion engine. Reproduction requires an explicit embeddable CJK TTF path and local Python document dependencies:

```sh
python3 scripts/generate-file-conversion-fixtures.py --cjk-font /path/to/NotoSansSC.ttf
python3 scripts/validate-file-conversion-fixtures.py
```

Regeneration changes hashes because PDF metadata contains creation timestamps. Commit regenerated documents and their updated `expected.json` together only after repeating visual QA.

## Visual QA evidence

Checked on 2026-08-16 with LibreOffice-rendered DOCX previews and Poppler-rendered PDF pages:

- All three pages of `rich-layout.docx` and all 24 pages of `large-structured.docx` were inspected for CJK glyphs, headers/footers, tables, page breaks, clipping, and overlap.
- Both pages of `rich-layout.pdf` were inspected.
- Pages 1, 12, and 24 of `large-structured.pdf` were inspected as first/middle/last representatives.
- The inspected pages showed legible Latin/CJK text and no clipping, overlap, black replacement glyphs, or broken table/image layout.

The validation script additionally verifies every committed hash, PDF text-layer expectations, encryption behavior, PDF page counts, required OOXML entries, the embedded-font relationship, and malformed-input rejection.
