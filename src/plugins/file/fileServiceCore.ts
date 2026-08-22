import type {
  FileConversionBatchResult,
  FileConversionCandidate,
  FileConversionCapabilitySnapshot,
  FileConversionEnqueueItem,
  FileConversionEnqueueRequest,
  FileConversionError,
  FileConversionErrorCode,
  FileConversionInspectRequest,
  FileConversionJobRequest,
  FileConversionJobSnapshot,
} from "./contracts";

export const FILE_CONVERSION_COMMANDS = {
  capabilities: "get_file_conversion_capabilities",
  refreshCapabilities: "refresh_file_conversion_capabilities",
  choose: "choose_file_conversion_inputs",
  inspect: "inspect_file_conversion_inputs",
  enqueue: "enqueue_file_conversions",
  list: "list_file_conversion_jobs",
  start: "start_file_conversion_queue",
  cancel: "cancel_file_conversion_job",
  remove: "remove_file_conversion_job",
  retry: "retry_file_conversion_job",
  clearCompleted: "clear_completed_file_conversion_jobs",
  open: "open_file_conversion_output",
  reveal: "reveal_file_conversion_output",
} as const;

export const FILE_CONVERSION_JOB_UPDATED_EVENT =
  "zero://file-conversion/job-updated";

export type FileConversionCommand =
  (typeof FILE_CONVERSION_COMMANDS)[keyof typeof FILE_CONVERSION_COMMANDS];
export type FileConversionInvokeArgs =
  | { input: FileConversionInspectRequest }
  | { input: FileConversionEnqueueRequest }
  | { input: FileConversionJobRequest };
export type FileConversionUnlisten = () => void;

export interface FileConversionInvokeBridge {
  <T>(command: FileConversionCommand, args?: FileConversionInvokeArgs): Promise<T>;
}

export interface FileConversionEventBridge {
  (
    eventName: typeof FILE_CONVERSION_JOB_UPDATED_EVENT,
    handler: (snapshot: FileConversionJobSnapshot) => void,
  ): Promise<FileConversionUnlisten>;
}

export function createFileConversionService(
  invokeBridge: FileConversionInvokeBridge,
  eventBridge: FileConversionEventBridge,
) {
  const jobInput = (jobId: string): { input: FileConversionJobRequest } => ({
    input: { jobId },
  });
  return {
    getCapabilities: () =>
      invokeBridge<FileConversionCapabilitySnapshot>(
        FILE_CONVERSION_COMMANDS.capabilities,
      ),
    refreshCapabilities: () =>
      invokeBridge<FileConversionCapabilitySnapshot>(
        FILE_CONVERSION_COMMANDS.refreshCapabilities,
      ),
    chooseInputs: () =>
      invokeBridge<FileConversionCandidate[]>(FILE_CONVERSION_COMMANDS.choose),
    inspectInputs: (sourcePaths: string[]) =>
      invokeBridge<FileConversionCandidate[]>(FILE_CONVERSION_COMMANDS.inspect, {
        input: { sourcePaths },
      }),
    enqueue: (items: FileConversionEnqueueItem[]) =>
      invokeBridge<FileConversionBatchResult>(FILE_CONVERSION_COMMANDS.enqueue, {
        input: { items },
      }),
    listJobs: () =>
      invokeBridge<FileConversionJobSnapshot[]>(FILE_CONVERSION_COMMANDS.list),
    start: () =>
      invokeBridge<FileConversionJobSnapshot[]>(FILE_CONVERSION_COMMANDS.start),
    cancel: (jobId: string) =>
      invokeBridge<FileConversionJobSnapshot[]>(
        FILE_CONVERSION_COMMANDS.cancel,
        jobInput(jobId),
      ),
    remove: (jobId: string) =>
      invokeBridge<FileConversionJobSnapshot>(
        FILE_CONVERSION_COMMANDS.remove,
        jobInput(jobId),
      ),
    retry: (jobId: string) =>
      invokeBridge<FileConversionJobSnapshot>(
        FILE_CONVERSION_COMMANDS.retry,
        jobInput(jobId),
      ),
    clearCompleted: () =>
      invokeBridge<FileConversionJobSnapshot[]>(
        FILE_CONVERSION_COMMANDS.clearCompleted,
      ),
    open: (jobId: string) =>
      invokeBridge<void>(FILE_CONVERSION_COMMANDS.open, jobInput(jobId)),
    reveal: (jobId: string) =>
      invokeBridge<void>(FILE_CONVERSION_COMMANDS.reveal, jobInput(jobId)),
    subscribe: (handler: (snapshot: FileConversionJobSnapshot) => void) =>
      eventBridge(FILE_CONVERSION_JOB_UPDATED_EVENT, handler),
  };
}

const ERROR_CODES = new Set<FileConversionErrorCode>([
  "invalidInput",
  "unsupportedFormat",
  "duplicateSource",
  "engineUnavailable",
  "engineVersionUnsupported",
  "automationPermissionDenied",
  "providerActivationFailed",
  "passwordRequired",
  "ocrRequired",
  "unsupportedInput",
  "permissionDenied",
  "timeout",
  "cancelled",
  "outputConflict",
  "outputNotWritable",
  "providerFailed",
  "invalidProviderOutput",
  "outputMissing",
  "unknownJob",
  "invalidJobState",
  "internal",
]);

export function normalizeFileConversionError(value: unknown): FileConversionError {
  if (
    isRecord(value) &&
    typeof value.code === "string" &&
    ERROR_CODES.has(value.code as FileConversionErrorCode) &&
    typeof value.message === "string" &&
    typeof value.retryable === "boolean"
  ) {
    return {
      code: value.code as FileConversionErrorCode,
      message: value.message,
      retryable: value.retryable,
      ...(isProviderId(value.providerId) ? { providerId: value.providerId } : {}),
      ...(typeof value.diagnostic === "string"
        ? { diagnostic: value.diagnostic }
        : {}),
    };
  }
  return {
    code: "internal",
    message: value instanceof Error ? value.message : String(value),
    retryable: true,
  };
}

function isProviderId(value: unknown): value is NonNullable<FileConversionError["providerId"]> {
  return (
    value === "zeroFilePdfToDocx" ||
    value === "zeroFileDocxToPdfMacos" ||
    value === "libreOffice" ||
    value === "microsoftWordMacos" ||
    value === "microsoftWordWindows"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
