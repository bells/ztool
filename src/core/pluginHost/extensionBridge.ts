import type {
  NetworkFetchRequest,
  PluginHealth,
  PluginPermission,
  PluginRecord,
  StorageWriteFileRequest,
  SystemSetWallpaperRequest,
} from "./contracts";
import type {
  QuickLauncherActivateInput,
  QuickLauncherSearchInput,
} from "./launcherContracts";

export interface ExtensionSurfacePolicy {
  sandbox: "allow-scripts";
  csp: string;
}

export interface ExtensionApiRequest {
  requestId: string;
  pluginName: string;
  method: string;
  payload?: unknown;
}

export interface ExtensionApiError {
  code: string;
  message: string;
}

export interface ExtensionApiResponse {
  requestId: string;
  ok: boolean;
  result?: unknown;
  error?: ExtensionApiError;
}

export interface ExtensionHostApis {
  showMessage?: (message: string) => Promise<void> | void;
  storageGet?: (pluginName: string, key: string) => Promise<unknown> | unknown;
  storageSet?: (pluginName: string, key: string, value: unknown) => Promise<void> | void;
  commandRegister?: (pluginName: string, commandId: string) => Promise<void> | void;
  commandExecute?: (pluginName: string, commandId: string) => Promise<unknown> | unknown;
  settingsGet?: (pluginName: string, key: string) => Promise<unknown> | unknown;
  settingsSet?: (pluginName: string, key: string, value: unknown) => Promise<void> | void;
  diagnosticsReport?: (pluginName: string, message: string) => Promise<void> | void;
  networkFetch?: (pluginName: string, request: NetworkFetchRequest) => Promise<unknown> | unknown;
  storageWriteFile?: (pluginName: string, request: StorageWriteFileRequest) => Promise<unknown> | unknown;
  systemSetWallpaper?: (pluginName: string, request: SystemSetWallpaperRequest) => Promise<unknown> | unknown;
  launcherScanApps?: (pluginName: string) => Promise<unknown> | unknown;
  launcherSearch?: (pluginName: string, request: QuickLauncherSearchInput) => Promise<unknown> | unknown;
  launcherLaunchOrFocus?: (pluginName: string, request: QuickLauncherActivateInput) => Promise<unknown> | unknown;
  launcherOpenSystemSetting?: (pluginName: string, request: QuickLauncherActivateInput) => Promise<unknown> | unknown;
}

export interface ExtensionBridge {
  handle(request: unknown): Promise<ExtensionApiResponse>;
}

export function buildExtensionSurfacePolicy(): ExtensionSurfacePolicy {
  return {
    sandbox: "allow-scripts",
    csp: [
      "default-src 'none'",
      "script-src 'self'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data:",
      "connect-src 'none'",
      "frame-ancestors 'none'",
    ].join("; "),
  };
}

export function createExtensionBridge(
  record: PluginRecord,
  hostApis: ExtensionHostApis,
): ExtensionBridge {
  return {
    async handle(request: unknown) {
      const parsed = parseRequest(request);
      if ("response" in parsed) {
        return parsed.response;
      }

      if (parsed.request.pluginName !== record.name) {
        return denied(parsed.request.requestId, "plugin.identity", "Plugin identity mismatch.");
      }

      if (!record.enabled || record.health === "disabled") {
        return denied(parsed.request.requestId, "plugin.disabled", "Plugin is disabled.");
      }

      const permissions = requiredPermissions(parsed.request.method);
      if (!permissions) {
        return denied(parsed.request.requestId, "method.unsupported", "Extension API method is not supported.");
      }

      const missingPermission = permissions.find((permission) =>
        !record.manifest.permissions.includes(permission) ||
        !record.approvedPermissions.includes(permission));
      if (missingPermission) {
        return denied(parsed.request.requestId, "permission.denied", `Missing permission ${missingPermission}.`);
      }

      try {
        const result = await dispatchHostApi(record.name, parsed.request, hostApis);
        const response: ExtensionApiResponse = {
          requestId: parsed.request.requestId,
          ok: true,
        };
        if (result !== undefined) {
          response.result = result;
        }
        return response;
      } catch (error) {
        return denied(
          parsed.request.requestId,
          "host.error",
          error instanceof Error ? error.message : String(error),
        );
      }
    },
  };
}

export function markPluginFailure(
  record: PluginRecord,
  message: string,
): PluginRecord & { lastError: string } {
  return {
    ...record,
    enabled: false,
    health: "error" satisfies PluginHealth,
    lastError: message,
  };
}

function parseRequest(request: unknown):
  | { ok: true; request: ExtensionApiRequest }
  | { ok: false; response: ExtensionApiResponse } {
  if (!isRecord(request)) {
    return {
      ok: false,
      response: denied("", "request.invalid", "Extension request must be an object."),
    };
  }

  const requestId = request.requestId;
  const pluginName = request.pluginName;
  const method = request.method;

  if (typeof requestId !== "string" ||
    typeof pluginName !== "string" ||
    typeof method !== "string"
  ) {
    return {
      ok: false,
      response: denied(
        typeof requestId === "string" ? requestId : "",
        "request.invalid",
        "Extension request requires string requestId, pluginName, and method.",
      ),
    };
  }

  return {
    ok: true,
    request: {
      requestId,
      pluginName,
      method,
      payload: request.payload,
    },
  };
}

function requiredPermissions(method: string): readonly PluginPermission[] | null {
  if (method === "ui.showMessage") {
    return ["ui.message"];
  }

  if (method.startsWith("storage.")) {
    return ["storage.plugin"];
  }

  if (method.startsWith("command.") || method.startsWith("settings.")) {
    return ["storage.plugin"];
  }

  if (method === "diagnostics.report") {
    return ["ui.message"];
  }

  if (method === "process.execute") {
    return ["process.execute"];
  }

  if (method === "network.fetch") {
    return ["network"];
  }

  if (method === "system.setWallpaper") {
    return ["system.wallpaper"];
  }

  if (method === "launcher.scanApps" || method === "launcher.search") {
    return ["system.apps.read"];
  }

  if (method === "launcher.launchOrFocus") {
    return ["system.apps.execute", "system.window.focus"];
  }

  if (method === "launcher.openSystemSetting") {
    return ["system.settings.open"];
  }

  return null;
}

async function dispatchHostApi(
  pluginName: string,
  request: ExtensionApiRequest,
  hostApis: ExtensionHostApis,
) {
  const payload = isRecord(request.payload) ? request.payload : {};

  switch (request.method) {
    case "ui.showMessage": {
      const message = readString(payload.message, "message");
      await hostApis.showMessage?.(message);
      return undefined;
    }
    case "storage.get": {
      const key = readString(payload.key, "key");
      return hostApis.storageGet?.(pluginName, key);
    }
    case "storage.set": {
      const key = readString(payload.key, "key");
      await hostApis.storageSet?.(pluginName, key, payload.value);
      return undefined;
    }
    case "command.register": {
      const commandId = readString(payload.commandId, "commandId");
      await hostApis.commandRegister?.(pluginName, commandId);
      return undefined;
    }
    case "command.execute": {
      const commandId = readString(payload.commandId, "commandId");
      return hostApis.commandExecute?.(pluginName, commandId);
    }
    case "settings.get": {
      const key = readString(payload.key, "key");
      return hostApis.settingsGet?.(pluginName, key);
    }
    case "settings.set": {
      const key = readString(payload.key, "key");
      await hostApis.settingsSet?.(pluginName, key, payload.value);
      return undefined;
    }
    case "diagnostics.report": {
      const message = readString(payload.message, "message");
      await hostApis.diagnosticsReport?.(pluginName, message);
      return undefined;
    }
    case "network.fetch": {
      const url = readString(payload.url, "url");
      const method = payload.method === undefined
        ? undefined
        : readLiteral(payload.method, "method", ["GET"] as const);
      return hostApis.networkFetch?.(pluginName, { url, method });
    }
    case "storage.writeFile": {
      const relativePath = readString(payload.relativePath, "relativePath");
      const dataBase64 = readString(payload.dataBase64, "dataBase64");
      return hostApis.storageWriteFile?.(pluginName, { relativePath, dataBase64 });
    }
    case "system.setWallpaper": {
      const relativePath = readString(payload.relativePath, "relativePath");
      return hostApis.systemSetWallpaper?.(pluginName, { relativePath });
    }
    case "launcher.scanApps": {
      assertExactKeys(payload, []);
      return hostApis.launcherScanApps?.(pluginName);
    }
    case "launcher.search": {
      assertExactKeys(payload, ["query", "limit"]);
      const query = readQuery(payload.query);
      const limit = payload.limit === undefined ? undefined : readLimit(payload.limit);
      return hostApis.launcherSearch?.(pluginName, { query, limit });
    }
    case "launcher.launchOrFocus": {
      const input = readLauncherActivation(payload);
      return hostApis.launcherLaunchOrFocus?.(pluginName, input);
    }
    case "launcher.openSystemSetting": {
      const input = readLauncherActivation(payload);
      return hostApis.launcherOpenSystemSetting?.(pluginName, input);
    }
    default:
      throw new Error(`Unsupported method ${request.method}`);
  }
}

function readLauncherActivation(payload: Record<string, unknown>): QuickLauncherActivateInput {
  assertExactKeys(payload, ["itemId", "revision"]);
  return {
    itemId: readString(payload.itemId, "itemId"),
    revision: readNonNegativeInteger(payload.revision, "revision"),
  };
}

function readQuery(value: unknown) {
  if (typeof value !== "string" || value.length > 128) {
    throw new Error("query must be a string of at most 128 characters");
  }
  return value;
}

function readLimit(value: unknown) {
  const limit = readNonNegativeInteger(value, "limit");
  if (limit < 1 || limit > 50) {
    throw new Error("limit must be between 1 and 50");
  }
  return limit;
}

function readNonNegativeInteger(value: unknown, key: string) {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${key} must be a non-negative safe integer`);
  }
  return value;
}

function assertExactKeys(
  payload: Record<string, unknown>,
  allowed: readonly string[],
) {
  const unexpected = Object.keys(payload).find((key) => !allowed.includes(key));
  if (unexpected) {
    throw new Error(`Unexpected launcher field ${unexpected}`);
  }
}

function readString(value: unknown, key: string) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${key} must be a non-empty string`);
  }

  return value;
}

function readLiteral<T extends string>(
  value: unknown,
  key: string,
  allowed: readonly T[],
): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    throw new Error(`${key} must be one of: ${allowed.join(", ")}`);
  }
  return value as T;
}

function denied(
  requestId: string,
  code: string,
  message: string,
): ExtensionApiResponse {
  return {
    requestId,
    ok: false,
    error: { code, message },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
