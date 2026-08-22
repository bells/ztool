# Zero File engine third-party notices

Zero File redistributes only browser assets and JavaScript code from the pinned packages below. The exact package versions are locked in `pnpm-lock.yaml`; release builds copy their license texts into the engine bundle.

## Direct components

| Component | Version | License | Upstream revision |
| --- | --- | --- | --- |
| pdfjs-dist | 6.2.108 | Apache-2.0 | `0365cbde028bd92e58f2dab1bb70cd30ac7acfd7` |
| docx | 9.7.1 | MIT | `4934d310c724520ad9d3e7e6d5d47430664ea9f7` |
| docx-preview | 0.4.0 | Apache-2.0 | `191d3e0db009da578fbe4da70d55305cd8d50226` |

## Runtime transitive components

The `docx` and `docx-preview` browser bundles resolve the following pinned runtime dependencies. Dual-licensed packages are used under the permissive license named first.

| Component | Version | License used |
| --- | --- | --- |
| hash.js | 1.1.7 | MIT |
| inherits | 2.0.4 | ISC |
| minimalistic-assert | 1.0.1 | ISC |
| jszip | 3.10.1 | MIT |
| lie | 3.3.0 | MIT |
| immediate | 3.0.6 | MIT |
| pako | 1.0.11 | MIT and Zlib |
| readable-stream | 2.3.8 | MIT |
| core-util-is | 1.0.3 | MIT |
| isarray | 1.0.0 | MIT |
| process-nextick-args | 2.0.1 | MIT |
| safe-buffer | 5.1.2 | MIT |
| string_decoder | 1.1.1 | MIT |
| util-deprecate | 1.0.2 | MIT |
| setimmediate | 1.0.5 | MIT |
| nanoid | 5.1.16 | MIT |
| xml | 1.0.1 | MIT |
| xml-js | 1.6.11 | MIT |
| sax | 1.6.1 | BlueOak-1.0.0 |

`@types/node` is a type-only dependency of `docx` and is not emitted into the runtime engine bundle. PDF.js's optional Node canvas dependency is not installed into or copied to the browser engine bundle.

No Python runtime, PyMuPDF, OpenCV, LibreOffice, ONLYOFFICE, Chromium, or Microsoft Office component is redistributed by Zero File.
