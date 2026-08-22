import type {
  FileConversionDirection,
  FileConversionQualityProfile,
} from "../contracts";

export const FILE_ENGINE_PROTOCOL_VERSION = 1;
export const FILE_ENGINE_RUN_EVENT = "zero://file-engine/run";
export const FILE_ENGINE_CANCEL_EVENT = "zero://file-engine/cancel";

export interface FileEngineRunRequest {
  protocolVersion: number;
  token: string;
  pluginId: "zero.file";
  engineVersion: string;
  jobId: string;
  direction: FileConversionDirection;
  inputName: "input.pdf" | "input.docx";
  outputName: "provider-output.docx" | "provider-output.pdf";
  deadlineMs: number;
  maxInputBytes: number;
}

export interface FileEngineCancelRequest {
  protocolVersion: number;
  token: string;
  jobId: string;
}

export interface FileEngineReadyRequest {
  protocolVersion: number;
  engineVersion: string;
  pluginId: "zero.file";
}

export interface FileEngineProgressRequest {
  protocolVersion: number;
  token: string;
  engineVersion: string;
  jobId: string;
  stage: "parsing" | "analyzing" | "rendering" | "packaging" | "printing";
  percent?: number;
}

export interface FileEngineCompletionRequest {
  protocolVersion: number;
  token: string;
  engineVersion: string;
  jobId: string;
  status: "completed" | "failed" | "cancelled";
  qualityProfile?: FileConversionQualityProfile;
  warningKeys: string[];
  pageCount?: number;
  errorCode?:
    | "passwordRequired"
    | "invalidInput"
    | "unsupportedInput"
    | "timeout"
    | "cancelled"
    | "providerFailed"
    | "invalidProviderOutput";
  diagnostic?: string;
}

export interface FileEngineRenderMeasurement {
  protocolVersion: number;
  token: string;
  engineVersion: string;
  jobId: string;
  sectionCount: number;
  measuredPageCount: number;
  pageRects: FileEnginePageRect[];
}

export interface FileEnginePageRect {
  x: number;
  y: number;
  width: number;
  height: number;
}
