import type { AnnotationObject, ScreenshotUploadLease } from "./captureTypes";

export type ScreenshotCommitAction = "copy" | "save" | "pin";

export function serializeAnnotationObject(annotation: AnnotationObject): AnnotationObject {
  return structuredClone(annotation);
}

export function buildPrepareScreenshotCommitPayload(
  sessionId: string,
  action: ScreenshotCommitAction,
): {
  input: {
    sessionId: string;
    action: ScreenshotCommitAction;
  };
} {
  return { input: { sessionId, action } };
}

export function buildScreenshotUploadOptions(lease: ScreenshotUploadLease): {
  headers: Record<string, string>;
} {
  return {
    headers: {
      "x-zero-screenshot-lease": lease.token,
      "x-zero-screenshot-session": lease.sessionId,
      "x-zero-screenshot-action": lease.action,
    },
  };
}
