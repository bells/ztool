import { renderAsync } from "docx-preview";
import type { FileEngineRenderMeasurement } from "./engineContracts";

const RESOURCE_TIMEOUT_MS = 15_000;

export async function renderDocxForNativePrint(
  bytes: Uint8Array,
  request: Pick<
    FileEngineRenderMeasurement,
    "protocolVersion" | "token" | "engineVersion" | "jobId"
  >,
  signal: AbortSignal,
): Promise<FileEngineRenderMeasurement> {
  const root = engineDocumentRoot();
  document.body.classList.remove("zero-file-engine-export");
  root.replaceChildren();
  root.className = "zero-file-engine-document";

  await renderAsync(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength), root, root, {
    className: "zero-file-docx",
    inWrapper: true,
    ignoreWidth: false,
    ignoreHeight: false,
    ignoreFonts: false,
    breakPages: true,
    ignoreLastRenderedPageBreak: false,
    useBase64URL: true,
    renderHeaders: true,
    renderFooters: true,
    renderFootnotes: true,
    renderEndnotes: true,
  });
  assertNotCancelled(signal);
  await withTimeout(waitForImages(root, signal), RESOURCE_TIMEOUT_MS, "Embedded images timed out.");
  if (document.fonts) {
    await withTimeout(document.fonts.ready, RESOURCE_TIMEOUT_MS, "Document fonts timed out.");
  }
  assertNotCancelled(signal);

  document.body.classList.add("zero-file-engine-export");
  void root.offsetHeight;
  const sections = [...root.querySelectorAll<HTMLElement>("section.zero-file-docx")];
  const sectionCount = Math.max(1, sections.length);
  const pageRects = measurePageRects(root, sections);
  const measuredPageCount = pageRects.length;
  return {
    protocolVersion: request.protocolVersion,
    token: request.token,
    engineVersion: request.engineVersion,
    jobId: request.jobId,
    sectionCount,
    measuredPageCount,
    pageRects,
  };
}

function measurePageRects(root: HTMLElement, sections: HTMLElement[]) {
  const candidates = sections.length > 0 ? sections : [root];
  return candidates.flatMap((section) => {
    const bounds = section.getBoundingClientRect();
    const style = window.getComputedStyle(section);
    const pageWidth = Math.max(1, bounds.width || Number.parseFloat(style.width));
    const declaredPageHeight = Number.parseFloat(style.minHeight) * 0.75;
    const pageHeight = Math.max(
      1,
      declaredPageHeight || bounds.height || window.innerHeight,
    );
    const contentHeight = Math.max(
      pageHeight,
      bounds.height,
      section.scrollHeight * 0.75,
      sections.length === 1 ? root.scrollHeight : 0,
    );
    const pageCount = Math.max(1, Math.ceil(contentHeight / pageHeight));
    return Array.from({ length: pageCount }, (_, index) => ({
      x: Math.max(0, bounds.left + window.scrollX),
      y: Math.max(0, bounds.top + window.scrollY + index * pageHeight),
      width: pageWidth,
      height: pageHeight,
    }));
  });
}

function engineDocumentRoot() {
  const existing = document.getElementById("zero-file-engine-document");
  if (existing instanceof HTMLElement) return existing;
  const root = document.createElement("main");
  root.id = "zero-file-engine-document";
  document.body.append(root);
  return root;
}

async function waitForImages(root: HTMLElement, signal: AbortSignal) {
  await Promise.all([...root.querySelectorAll("img")].map(async (image) => {
    assertNotCancelled(signal);
    if (image.complete) {
      if (image.naturalWidth === 0) throw new Error("An embedded image could not be decoded.");
      return;
    }
    await image.decode();
    assertNotCancelled(signal);
  }));
}

async function withTimeout<T>(promise: Promise<T>, timeoutMs: number, message: string) {
  let timer: number | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<never>((_resolve, reject) => {
        timer = window.setTimeout(() => reject(new Error(message)), timeoutMs);
      }),
    ]);
  } finally {
    if (timer !== undefined) window.clearTimeout(timer);
  }
}

function assertNotCancelled(signal: AbortSignal) {
  if (signal.aborted) throw new DOMException("Conversion cancelled.", "AbortError");
}
