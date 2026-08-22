import {
  AlignmentType,
  Document,
  ImageRun,
  Packer,
  Paragraph,
  TextRun,
} from "docx";
import {
  GlobalWorkerOptions,
  OPS,
  PasswordResponses,
  getDocument,
  type PDFDocumentProxy,
  type PDFPageProxy,
} from "pdfjs-dist";
import {
  classifyPdfComplexity,
  type PdfPageComplexityInput,
  type PdfTextSample,
} from "./pdfComplexity";

const engineAssetUrl = (path: string) =>
  new URL(`./file-engine/${path}`, document.baseURI).toString();

GlobalWorkerOptions.workerSrc = engineAssetUrl("pdf.worker.min.mjs");

export class FileEngineCancelledError extends Error {
  override name = "FileEngineCancelledError";
}

export interface PdfToDocxProgress {
  stage: "parsing" | "analyzing" | "rendering" | "packaging";
  percent?: number;
}

export interface PdfToDocxResult {
  bytes: Uint8Array;
  qualityProfile: "editableReconstruction" | "layoutPreserving";
  warningKeys: string[];
  pageCount: number;
}

export async function convertPdfToDocx(
  bytes: Uint8Array,
  signal: AbortSignal,
  onProgress: (progress: PdfToDocxProgress) => Promise<void>,
): Promise<PdfToDocxResult> {
  await onProgress({ stage: "parsing", percent: 2 });
  assertNotCancelled(signal);
  let passwordRequired = false;
  const loadingTask = getDocument({
    data: bytes,
    cMapUrl: engineAssetUrl("cmaps/"),
    cMapPacked: true,
    standardFontDataUrl: engineAssetUrl("standard_fonts/"),
    wasmUrl: engineAssetUrl("wasm/"),
    useWasm: true,
  });
  loadingTask.onPassword = (_updatePassword: (password: string) => void, reason: number) => {
    passwordRequired =
      reason === PasswordResponses.NEED_PASSWORD || reason === PasswordResponses.INCORRECT_PASSWORD;
    loadingTask.destroy();
  };

  let pdf: PDFDocumentProxy;
  try {
    pdf = await loadingTask.promise;
  } catch (error) {
    if (passwordRequired || isPasswordError(error)) throw new Error("passwordRequired");
    throw error;
  }
  try {
    const pages: PdfPageComplexityInput[] = [];
    for (let pageNumber = 1; pageNumber <= pdf.numPages; pageNumber += 1) {
      assertNotCancelled(signal);
      const page = await pdf.getPage(pageNumber);
      try {
        const viewport = page.getViewport({ scale: 1 });
        const content = await page.getTextContent();
        assertNotCancelled(signal);
        const operatorList = await page.getOperatorList();
        const text = content.items.flatMap<PdfTextSample>((item) => {
          if (!("str" in item) || item.str.trim().length === 0) return [];
          const [a, b, , d, x, y] = item.transform;
          return [{
            text: item.str,
            x,
            y: viewport.height - y,
            width: Math.abs(item.width),
            height: Math.max(Math.abs(d), Math.abs(item.height), 1),
            rotationDegrees: Math.atan2(b, a) * (180 / Math.PI),
          }];
        });
        const vectorOperationCount = operatorList.fnArray.filter((operation) =>
          operation === OPS.constructPath || operation === OPS.paintFormXObjectBegin,
        ).length;
        const imageCount = operatorList.fnArray.filter((operation) =>
          operation === OPS.paintImageXObject || operation === OPS.paintInlineImageXObject,
        ).length;
        pages.push({
          width: viewport.width,
          height: viewport.height,
          text,
          vectorOperationCount,
          imageCount,
        });
      } finally {
        page.cleanup();
      }
      await onProgress({ stage: "analyzing", percent: Math.round((pageNumber / pdf.numPages) * 30) });
    }

    const decision = classifyPdfComplexity(pages);
    if (decision.profile === "editableReconstruction") {
      try {
        const output = await createEditableDocx(pages, signal);
        return {
          bytes: output,
          qualityProfile: "editableReconstruction",
          warningKeys: ["file.quality.editableReconstructionWarning"],
          pageCount: pdf.numPages,
        };
      } catch (error) {
        if (error instanceof FileEngineCancelledError) throw error;
      }
    }

    const output = await createLayoutDocx(pdf, pages, signal, onProgress);
    return {
      bytes: output,
      qualityProfile: "layoutPreserving",
      warningKeys: ["file.quality.layoutPreservingNonEditable"],
      pageCount: pdf.numPages,
    };
  } finally {
    await loadingTask.destroy();
  }
}

async function createEditableDocx(pages: PdfPageComplexityInput[], signal: AbortSignal) {
  const sections = pages.map((page) => {
    assertNotCancelled(signal);
    const lines = groupTextLines(page.text);
    return {
      properties: pageProperties(page.width, page.height),
      children: lines.map((line) => new Paragraph({
        alignment: line.x > page.width * 0.6 ? AlignmentType.RIGHT : AlignmentType.LEFT,
        children: line.items.map((item) => new TextRun({
          text: item.text,
          size: Math.max(12, Math.min(144, Math.round(item.height * 2))),
        })),
      })),
    };
  });
  assertNotCancelled(signal);
  const blob = await Packer.toBlob(new Document({ sections }));
  assertNotCancelled(signal);
  const output = new Uint8Array(await blob.arrayBuffer());
  assertNotCancelled(signal);
  return output;
}

async function createLayoutDocx(
  pdf: PDFDocumentProxy,
  pages: PdfPageComplexityInput[],
  signal: AbortSignal,
  onProgress: (progress: PdfToDocxProgress) => Promise<void>,
) {
  const sections = [];
  for (let index = 0; index < pages.length; index += 1) {
    assertNotCancelled(signal);
    const complexity = pages[index];
    const page: PDFPageProxy = await pdf.getPage(index + 1);
    const scale = boundedScale(complexity.width, complexity.height);
    const viewport = page.getViewport({ scale });
    const canvas = document.createElement("canvas");
    try {
      canvas.width = Math.max(1, Math.floor(viewport.width));
      canvas.height = Math.max(1, Math.floor(viewport.height));
      const context = canvas.getContext("2d", { alpha: false });
      if (!context) throw new Error("Canvas rendering is unavailable.");
      await onProgress({
        stage: "rendering",
        percent: 30 + Math.round((index / pages.length) * 60),
      });
      await renderPage(page, canvas, context, viewport, signal);
      await onProgress({
        stage: "rendering",
        percent: 30 + Math.round(((index + 0.45) / pages.length) * 60),
      });
      assertNotCancelled(signal);
      const photographic = complexity.imageCount > 0 && complexity.text.length < 8;
      const mimeType = photographic ? "image/jpeg" : "image/png";
      const blob = await canvasToBlob(canvas, mimeType, photographic ? 0.9 : undefined);
      await onProgress({
        stage: "rendering",
        percent: 30 + Math.round(((index + 0.75) / pages.length) * 60),
      });
      const image = new Uint8Array(await blob.arrayBuffer());
      assertNotCancelled(signal);
      sections.push({
        properties: pageProperties(complexity.width, complexity.height),
        children: [new Paragraph({
          spacing: { before: 0, after: 0 },
          children: [new ImageRun({
            data: image,
            type: photographic ? "jpg" : "png",
            transformation: {
              width: Math.round(complexity.width * (96 / 72)),
              height: Math.round(complexity.height * (96 / 72)),
            },
          })],
        })],
      });
    } finally {
      canvas.width = 1;
      canvas.height = 1;
      page.cleanup();
    }
    await onProgress({ stage: "rendering", percent: 30 + Math.round(((index + 1) / pages.length) * 60) });
  }
  assertNotCancelled(signal);
  await onProgress({ stage: "packaging", percent: 95 });
  const blob = await Packer.toBlob(new Document({ sections }));
  assertNotCancelled(signal);
  return new Uint8Array(await blob.arrayBuffer());
}

async function renderPage(
  page: PDFPageProxy,
  canvas: HTMLCanvasElement,
  context: CanvasRenderingContext2D,
  viewport: ReturnType<PDFPageProxy["getViewport"]>,
  signal: AbortSignal,
) {
  const renderTask = page.render({ canvas, canvasContext: context, viewport, intent: "print" });
  const cancel = () => renderTask.cancel();
  signal.addEventListener("abort", cancel, { once: true });
  try {
    await renderTask.promise;
    assertNotCancelled(signal);
  } catch (error) {
    if (signal.aborted) throw new FileEngineCancelledError("Conversion cancelled.");
    throw error;
  } finally {
    signal.removeEventListener("abort", cancel);
  }
}

function pageProperties(widthPoints: number, heightPoints: number) {
  return {
    page: {
      size: { width: Math.round(widthPoints * 20), height: Math.round(heightPoints * 20) },
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
    },
  };
}

function groupTextLines(items: PdfTextSample[]) {
  const sorted = [...items].sort((left, right) => left.y - right.y || left.x - right.x);
  const lines: Array<{ y: number; x: number; items: PdfTextSample[] }> = [];
  for (const item of sorted) {
    const line = lines.find((candidate) => Math.abs(candidate.y - item.y) <= Math.max(2, item.height * 0.45));
    if (line) {
      line.items.push(item);
      line.items.sort((left, right) => left.x - right.x);
      line.x = Math.min(line.x, item.x);
    } else {
      lines.push({ y: item.y, x: item.x, items: [item] });
    }
  }
  return lines;
}

function boundedScale(width: number, height: number) {
  const maximumPixels = 12_000_000;
  return Math.min(2, Math.sqrt(maximumPixels / Math.max(1, width * height)));
}

function canvasToBlob(canvas: HTMLCanvasElement, type: string, quality?: number) {
  return new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Page image encoding failed.")), type, quality);
  });
}

function assertNotCancelled(signal: AbortSignal) {
  if (signal.aborted) throw new FileEngineCancelledError("Conversion cancelled.");
}

function isPasswordError(error: unknown) {
  return error instanceof Error && /password/i.test(`${error.name} ${error.message}`);
}
