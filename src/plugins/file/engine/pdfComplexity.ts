export interface PdfTextSample {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotationDegrees: number;
}

export interface PdfPageComplexityInput {
  width: number;
  height: number;
  text: PdfTextSample[];
  vectorOperationCount: number;
  imageCount: number;
}

export interface PdfComplexityPolicy {
  minimumCharactersPerPage: number;
  minimumTextCoverage: number;
  maximumOverlapRatio: number;
  maximumColumnGapRatio: number;
  maximumVectorOperationsPerPage: number;
  supportedRotationToleranceDegrees: number;
}

export interface PdfComplexityDecision {
  profile: "editableReconstruction" | "layoutPreserving";
  signals: string[];
}

export const DEFAULT_PDF_COMPLEXITY_POLICY: PdfComplexityPolicy = {
  minimumCharactersPerPage: 24,
  minimumTextCoverage: 0.0008,
  maximumOverlapRatio: 0.12,
  maximumColumnGapRatio: 0.28,
  maximumVectorOperationsPerPage: 16,
  supportedRotationToleranceDegrees: 2,
};

export function classifyPdfComplexity(
  pages: PdfPageComplexityInput[],
  policy = DEFAULT_PDF_COMPLEXITY_POLICY,
): PdfComplexityDecision {
  const signals = new Set<string>();
  for (const page of pages) {
    const pageArea = Math.max(1, page.width * page.height);
    const characters = page.text.reduce((sum, item) => sum + item.text.trim().length, 0);
    const textArea = page.text.reduce(
      (sum, item) => sum + Math.max(0, item.width) * Math.max(0, item.height),
      0,
    );
    if (characters < policy.minimumCharactersPerPage) signals.add("imageOnlyOrSparseText");
    if (textArea / pageArea < policy.minimumTextCoverage) signals.add("lowTextCoverage");
    if (
      page.text.some(
        (item) => normalizedRotation(item.rotationDegrees) > policy.supportedRotationToleranceDegrees,
      )
    ) {
      signals.add("unsupportedRotation");
    }
    if (overlapRatio(page.text) > policy.maximumOverlapRatio) signals.add("overlappingText");
    if (hasAmbiguousColumns(page, policy.maximumColumnGapRatio)) signals.add("ambiguousColumns");
    if (page.vectorOperationCount > policy.maximumVectorOperationsPerPage) {
      signals.add("denseVectorLayout");
    }
  }
  return {
    profile: signals.size === 0 ? "editableReconstruction" : "layoutPreserving",
    signals: [...signals].sort(),
  };
}

function normalizedRotation(value: number) {
  const rotation = Math.abs(value % 360);
  return Math.min(rotation, Math.abs(360 - rotation), Math.abs(180 - rotation));
}

function overlapRatio(items: PdfTextSample[]) {
  if (items.length < 2) return 0;
  let overlaps = 0;
  for (let index = 1; index < items.length; index += 1) {
    const previous = items[index - 1];
    const current = items[index];
    if (
      current.x < previous.x + previous.width &&
      current.x + current.width > previous.x &&
      current.y < previous.y + previous.height &&
      current.y + current.height > previous.y
    ) {
      overlaps += 1;
    }
  }
  return overlaps / (items.length - 1);
}

function hasAmbiguousColumns(page: PdfPageComplexityInput, maximumGapRatio: number) {
  const centers = page.text
    .filter((item) => item.text.trim().length > 0)
    .map((item) => item.x + item.width / 2)
    .sort((left, right) => left - right);
  if (centers.length < 8) return false;
  let largestGap = 0;
  for (let index = 1; index < centers.length; index += 1) {
    largestGap = Math.max(largestGap, centers[index] - centers[index - 1]);
  }
  return largestGap / Math.max(1, page.width) > maximumGapRatio;
}
