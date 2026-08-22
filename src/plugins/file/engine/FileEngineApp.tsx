import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { FileConversionQualityProfile } from "../contracts";
import { renderDocxForNativePrint } from "./docxToPdf";
import {
  FILE_ENGINE_CANCEL_EVENT,
  FILE_ENGINE_PROTOCOL_VERSION,
  FILE_ENGINE_RUN_EVENT,
  type FileEngineCancelRequest,
  type FileEngineCompletionRequest,
  type FileEngineProgressRequest,
  type FileEngineReadyRequest,
  type FileEngineRunRequest,
} from "./engineContracts";
import { installFileEngineRuntimePolyfills } from "./runtimePolyfills";
import "./fileEngine.css";

const ENGINE_VERSION = "1.0.0";
const controllers = new Map<string, AbortController>();

export function FileEngineApp() {
  useEffect(() => {
    let disposed = false;
    let unlisteners: Array<() => void> = [];
    void Promise.all([
      listen<FileEngineRunRequest>(FILE_ENGINE_RUN_EVENT, ({ payload }) => {
        if (!disposed) void runJob(payload);
      }),
      listen<FileEngineCancelRequest>(FILE_ENGINE_CANCEL_EVENT, ({ payload }) => {
        if (payload.protocolVersion === FILE_ENGINE_PROTOCOL_VERSION) {
          controllers.get(payload.token)?.abort();
        }
      }),
    ]).then(async (registeredUnlisteners) => {
      if (disposed) {
        registeredUnlisteners.forEach((unlisten) => unlisten());
        return;
      }
      unlisteners = registeredUnlisteners;
      await invoke<void>("file_engine_ready", {
        request: {
          protocolVersion: FILE_ENGINE_PROTOCOL_VERSION,
          engineVersion: ENGINE_VERSION,
          pluginId: "zero.file",
        } satisfies FileEngineReadyRequest,
      });
    });
    return () => {
      disposed = true;
      controllers.forEach((controller) => controller.abort());
      controllers.clear();
      unlisteners.forEach((unlisten) => unlisten());
      unlisteners = [];
    };
  }, []);

  return <main id="zero-file-engine-document" aria-hidden="true" />;
}

async function runJob(request: FileEngineRunRequest) {
  if (
    request.protocolVersion !== FILE_ENGINE_PROTOCOL_VERSION ||
    request.pluginId !== "zero.file" ||
    request.engineVersion !== ENGINE_VERSION ||
    request.deadlineMs <= Date.now()
  ) {
    return;
  }
  const controller = new AbortController();
  controllers.set(request.token, controller);
  try {
    const input = await readInput(request);
    if (request.direction === "pdfToDocx") {
      installFileEngineRuntimePolyfills();
      const { convertPdfToDocx } = await import("./pdfToDocx");
      const result = await convertPdfToDocx(input, controller.signal, async (progress) => {
        await reportProgress(request, progress.stage, progress.percent);
      });
      await invoke<void>("file_engine_write_output", result.bytes, tokenHeaders(request));
      await reportCompletion(request, "completed", result.qualityProfile, result.warningKeys, result.pageCount);
    } else {
      await reportProgress(request, "rendering", 20);
      const measurement = await renderDocxForNativePrint(input, request, controller.signal);
      await reportProgress(request, "printing", 80);
      await invoke<void>("file_engine_print_rendered", { request: measurement });
      await reportCompletion(
        request,
        "completed",
        "webRenderedPdf",
        ["file.quality.webRenderedPdfWarning"],
        measurement.measuredPageCount,
      );
    }
  } catch (error) {
    const cancelled = controller.signal.aborted ||
      (error instanceof Error && error.name === "FileEngineCancelledError") ||
      (error instanceof DOMException && error.name === "AbortError");
    const passwordRequired = error instanceof Error && error.message === "passwordRequired";
    await invoke<void>("file_engine_complete", {
      request: {
        protocolVersion: FILE_ENGINE_PROTOCOL_VERSION,
        token: request.token,
        engineVersion: request.engineVersion,
        jobId: request.jobId,
        status: cancelled ? "cancelled" : "failed",
        warningKeys: [],
        errorCode: cancelled ? "cancelled" : passwordRequired ? "passwordRequired" : "providerFailed",
        diagnostic: boundedDiagnostic(error),
      } satisfies FileEngineCompletionRequest,
    }).catch(() => undefined);
  } finally {
    controllers.delete(request.token);
  }
}

async function readInput(request: FileEngineRunRequest) {
  const value = await invoke<ArrayBuffer>("file_engine_read_input", new Uint8Array(), tokenHeaders(request));
  const bytes = new Uint8Array(value);
  if (bytes.byteLength === 0 || bytes.byteLength > request.maxInputBytes) {
    throw new Error("The staged input size is outside the approved limit.");
  }
  return bytes;
}

function tokenHeaders(request: FileEngineRunRequest) {
  return {
    headers: {
      "x-zero-file-token": request.token,
      "x-zero-file-job": request.jobId,
      "x-zero-file-engine": request.engineVersion,
    },
  };
}

async function reportProgress(
  request: FileEngineRunRequest,
  stage: FileEngineProgressRequest["stage"],
  percent?: number,
) {
  await invoke<void>("file_engine_progress", {
    request: {
      protocolVersion: FILE_ENGINE_PROTOCOL_VERSION,
      token: request.token,
      engineVersion: request.engineVersion,
      jobId: request.jobId,
      stage,
      percent,
    } satisfies FileEngineProgressRequest,
  });
}

async function reportCompletion(
  request: FileEngineRunRequest,
  status: "completed",
  qualityProfile: FileConversionQualityProfile,
  warningKeys: string[],
  pageCount: number,
) {
  await invoke<void>("file_engine_complete", {
    request: {
      protocolVersion: FILE_ENGINE_PROTOCOL_VERSION,
      token: request.token,
      engineVersion: request.engineVersion,
      jobId: request.jobId,
      status,
      qualityProfile,
      warningKeys,
      pageCount,
    } satisfies FileEngineCompletionRequest,
  });
}

function boundedDiagnostic(error: unknown) {
  const message = error instanceof Error
    ? `${error.name}: ${error.message}`
    : typeof error === "string"
      ? error
      : "Unknown engine error";
  return message.replace(/[\r\n\t]+/g, " ").slice(0, 512);
}
