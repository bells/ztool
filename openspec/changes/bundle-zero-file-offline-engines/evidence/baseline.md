# Zero File offline engine implementation baseline

Date: 2026-08-22
Workspace: `/Users/watson/work/zero`

## Verification before implementation

| Command | Result |
| --- | --- |
| `pnpm test` | Pass: 168/168 |
| `pnpm build` | Pass: TypeScript + Vite production build |
| `cargo fmt --check` | Pass |
| `cargo check` | Pass |
| `cargo test` | Pass: 116 Rust unit tests plus all non-ignored integration suites; one release-only Quick Launcher benchmark remains intentionally ignored |
| `git diff --check` | Pass |

No engine/provider or plugin-package implementation file was modified before this baseline was captured.

## Existing provider corpus baseline

The reusable corpus remains `tests/fixtures/fileConversion/expected.json`. The source and container validation paths were rerun by the complete frontend/Rust suites and passed, including valid, malformed, scanned, encrypted, CJK, rich-layout, and large fixtures.

The last external-provider execution evidence is intentionally preserved rather than rewritten:

- `openspec/changes/add-file-plugin/evidence/engine-benchmark.md`
- `openspec/changes/add-file-plugin/evidence/engine-decision.md`

That evidence records LibreOffice/Word as optional DOCX-to-PDF providers and rejects `pdf2docx 0.5.13` for redistribution and detection after its CJK/layout/license/runtime gates failed. The built-in engine implementation will append profile-specific results without altering those historical measurements.
