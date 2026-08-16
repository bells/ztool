export type FileConversionDirection = "pdfToDocx" | "docxToPdf";

export type FileConversionProviderId =
  | "libreOffice"
  | "microsoftWordMacos"
  | "microsoftWordWindows";

export type FileConversionErrorCode =
  | "invalidInput"
  | "unsupportedFormat"
  | "duplicateSource"
  | "engineUnavailable"
  | "engineVersionUnsupported"
  | "automationPermissionDenied"
  | "providerActivationFailed"
  | "passwordRequired"
  | "ocrRequired"
  | "unsupportedInput"
  | "permissionDenied"
  | "timeout"
  | "cancelled"
  | "outputConflict"
  | "outputNotWritable"
  | "providerFailed"
  | "invalidProviderOutput"
  | "outputMissing"
  | "unknownJob"
  | "invalidJobState"
  | "internal";

export interface FileConversionError {
  code: FileConversionErrorCode;
  message: string;
  retryable: boolean;
  providerId?: FileConversionProviderId;
  diagnostic?: string;
}

export type FileConversionProviderAvailability =
  | { kind: "available" }
  | { kind: "unavailable"; error: FileConversionError };

export interface FileConversionProvider {
  id: FileConversionProviderId;
  displayName: string;
  version?: string;
  directions: FileConversionDirection[];
  availability: FileConversionProviderAvailability;
}

export interface FileConversionDirectionCapability {
  direction: FileConversionDirection;
  available: boolean;
  selectedProviderId?: FileConversionProviderId;
  providers: FileConversionProvider[];
  unavailability?: FileConversionError;
}

export interface FileConversionCapabilitySnapshot {
  directions: FileConversionDirectionCapability[];
  refreshedAtMs: number;
}

export type FileConversionCandidateValidation =
  | {
      status: "valid";
      direction: FileConversionDirection;
      proposedOutputName: string;
    }
  | { status: "rejected"; error: FileConversionError };

export interface FileConversionCandidate {
  sourcePath: string;
  sourceName: string;
  sizeBytes?: number;
  validation: FileConversionCandidateValidation;
}

export interface FileConversionEnqueueItem {
  sourcePath: string;
  outputDirectory?: string;
}

export interface FileConversionEnqueueRequest {
  items: FileConversionEnqueueItem[];
}

export interface FileConversionInspectRequest {
  sourcePaths: string[];
}

export interface FileConversionJobRequest {
  jobId: string;
}

export type FileConversionStage =
  | "validating"
  | "waitingForProvider"
  | "converting"
  | "finalizing";

export type FileConversionProgress =
  | { kind: "indeterminate"; stage: FileConversionStage }
  | { kind: "percentage"; stage: FileConversionStage; percent: number };

export interface FileConversionResult {
  outputPath: string;
  outputName: string;
  sizeBytes: number;
  completedAtMs: number;
}

export type FileConversionJobState =
  | { status: "queued" }
  | { status: "preparing"; stage: FileConversionStage }
  | { status: "running"; progress: FileConversionProgress }
  | { status: "completed"; result: FileConversionResult }
  | { status: "failed"; error: FileConversionError }
  | { status: "cancelled"; error: FileConversionError };

export interface FileConversionJobSnapshot {
  id: string;
  sourcePath: string;
  sourceName: string;
  sizeBytes: number;
  direction: FileConversionDirection;
  targetName: string;
  providerId?: FileConversionProviderId;
  createdAtMs: number;
  updatedAtMs: number;
  state: FileConversionJobState;
}

export interface FileConversionBatchResult {
  jobs: FileConversionJobSnapshot[];
  rejectedCandidates: FileConversionCandidate[];
}
