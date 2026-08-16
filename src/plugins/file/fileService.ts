import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { FileConversionJobSnapshot } from "./contracts";
import {
  createFileConversionService,
  type FileConversionCommand,
  type FileConversionInvokeArgs,
} from "./fileServiceCore";

export * from "./fileServiceCore";

export const fileService = createFileConversionService(
  <T>(command: FileConversionCommand, args?: FileConversionInvokeArgs) =>
    invoke<T>(command, args),
  async (eventName, handler) =>
    listen<FileConversionJobSnapshot>(eventName, (event) => handler(event.payload)),
);
