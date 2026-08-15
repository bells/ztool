import type {
  PluginManifest,
  PluginManifestValidationReport,
  PluginMarketIndex,
  PluginMarketValidationReport,
  PluginPermission,
  PluginValidationIssue,
} from "./contracts";

const SUPPORTED_EXTENSION_API_VERSION = "1";
const SUPPORTED_ZERO_HOST_VERSION = "0.1.0";

export const SUPPORTED_PLUGIN_PERMISSIONS = [
  "clipboard.read",
  "clipboard.write",
  "network",
  "storage.plugin",
  "ui.message",
  "process.execute",
  "system.wallpaper",
  "system.apps.read",
  "system.apps.execute",
  "system.window.focus",
  "system.settings.open",
] as const;

const SEMVER_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
const NAME_PATTERN = /^[a-z0-9][a-z0-9._-]{1,63}$/;
const SHA256_PATTERN = /^[a-fA-F0-9]{64}$/;

export function validatePluginManifest(value: unknown): PluginManifestValidationReport {
  const issues: PluginValidationIssue[] = [];

  if (!isRecord(value)) {
    return {
      valid: false,
      issues: [
        issue("manifest.shape", "", "Manifest must be an object."),
      ],
    };
  }

  const name = readRequiredString(value, "name", issues, "manifest.required");
  const version = readRequiredString(value, "version", issues, "manifest.required");
  readRequiredString(value, "author", issues, "manifest.required");
  const main = readRequiredString(value, "main", issues, "manifest.required");
  readPermissions(value.permissions, "permissions", issues, "manifest");

  if (name !== null && !NAME_PATTERN.test(name)) {
    issues.push(issue("manifest.name.invalid", "name", "Plugin name must use lowercase letters, numbers, dots, underscores, or dashes."));
  }

  if (version !== null && !SEMVER_PATTERN.test(version)) {
    issues.push(issue("manifest.version.invalid", "version", "Plugin version must be a semantic version."));
  }

  if (main !== null && !isSafePackageRelativePath(main)) {
    issues.push(issue("manifest.main.unsafe", "main", "Plugin main must be a safe package-relative path."));
  }

  if (isRecord(value.engines)) {
    const api = value.engines.api;
    if (typeof api === "string" && api !== SUPPORTED_EXTENSION_API_VERSION) {
      issues.push(issue("manifest.api.incompatible", "engines.api", "Plugin targets an unsupported Extension API version."));
    }

    const zero = value.engines.zero;
    const legacyZtool = value.engines.ztool;
    const hostRange = typeof zero === "string" ? zero : legacyZtool;
    const hostPath = typeof zero === "string" ? "engines.zero" : "engines.ztool";
    const issueCode =
      typeof zero === "string"
        ? "manifest.zero.incompatible"
        : "manifest.ztool.incompatible";
    if (typeof hostRange === "string" && !isCompatibleZeroHostRange(hostRange)) {
      issues.push(issue(issueCode, hostPath, "Plugin targets an unsupported Zero host version."));
    }
  }

  if (issues.length > 0) {
    return { valid: false, issues };
  }

  return {
    valid: true,
    issues: [],
    manifest: normalizeManifestEngines(value) as unknown as PluginManifest,
  };
}

export function validatePluginMarketIndex(value: unknown): PluginMarketValidationReport {
  const issues: PluginValidationIssue[] = [];

  if (!isRecord(value)) {
    return {
      valid: false,
      issues: [
        issue("market.shape", "", "Market index must be an object."),
      ],
    };
  }

  if (value.schemaVersion !== 1) {
    issues.push(issue("market.schemaVersion", "schemaVersion", "Market schemaVersion must be 1."));
  }

  if (!Array.isArray(value.plugins)) {
    issues.push(issue("market.plugins.required", "plugins", "Market plugins must be an array."));
  } else {
    value.plugins.forEach((entry, index) => validateMarketEntry(entry, index, issues));
  }

  if (issues.length > 0) {
    return { valid: false, issues };
  }

  return {
    valid: true,
    issues: [],
    market: value as unknown as PluginMarketIndex,
  };
}

function validateMarketEntry(
  entry: unknown,
  index: number,
  issues: PluginValidationIssue[],
) {
  const prefix = `plugins[${index}]`;

  if (!isRecord(entry)) {
    issues.push(issue("market.entry.shape", prefix, "Market plugin entry must be an object."));
    return;
  }

  const name = readRequiredString(entry, "name", issues, "market.required", prefix);
  const version = readRequiredString(entry, "version", issues, "market.required", prefix);
  readRequiredString(entry, "author", issues, "market.required", prefix);
  const repository = readRequiredString(entry, "repository", issues, "market.required", prefix);
  const releaseUrl = readRequiredString(entry, "releaseUrl", issues, "market.required", prefix);
  const downloadUrl = readRequiredString(
    entry,
    "downloadUrl",
    issues,
    "market.downloadUrl.required",
    prefix,
  );
  readPermissions(entry.permissions, `${prefix}.permissions`, issues, "market");

  if (name !== null && !NAME_PATTERN.test(name)) {
    issues.push(issue("market.name.invalid", `${prefix}.name`, "Market plugin name must use lowercase letters, numbers, dots, underscores, or dashes."));
  }

  if (version !== null && !SEMVER_PATTERN.test(version)) {
    issues.push(issue("market.version.invalid", `${prefix}.version`, "Market plugin version must be a semantic version."));
  }

  if (repository !== null && !isHttpsGitHubUrl(repository)) {
    issues.push(issue("market.repository.url", `${prefix}.repository`, "Repository must be an HTTPS GitHub URL."));
  }

  if (releaseUrl !== null && !isHttpsGitHubUrl(releaseUrl)) {
    issues.push(issue("market.releaseUrl.url", `${prefix}.releaseUrl`, "Release URL must be an HTTPS GitHub URL."));
  }

  if (downloadUrl !== null) {
    if (!isHttpsGitHubUrl(downloadUrl)) {
      issues.push(issue("market.downloadUrl.url", `${prefix}.downloadUrl`, "Download URL must be an HTTPS GitHub URL."));
    } else if (!urlPathname(downloadUrl).endsWith(".zplugin")) {
      issues.push(issue("market.downloadUrl.extension", `${prefix}.downloadUrl`, "Download URL must point to a .zplugin asset."));
    }
  }

  if (entry.sha256 !== undefined && (typeof entry.sha256 !== "string" || !SHA256_PATTERN.test(entry.sha256))) {
    issues.push(issue("market.sha256.invalid", `${prefix}.sha256`, "sha256 must be a 64-character hex string."));
  }
}

function readRequiredString(
  record: Record<string, unknown>,
  key: string,
  issues: PluginValidationIssue[],
  code: string,
  prefix?: string,
) {
  const value = record[key];
  const path = prefix ? `${prefix}.${key}` : key;

  if (typeof value !== "string" || value.trim().length === 0) {
    issues.push(issue(code, path, `${key} is required.`));
    return null;
  }

  return value;
}

function readPermissions(
  value: unknown,
  path: string,
  issues: PluginValidationIssue[],
  codePrefix: "manifest" | "market",
) {
  if (!Array.isArray(value)) {
    issues.push(issue(`${codePrefix}.permissions.required`, path, "permissions must be an array."));
    return [];
  }

  const permissions: PluginPermission[] = [];

  value.forEach((permission, index) => {
    if (isPluginPermission(permission)) {
      permissions.push(permission);
    } else {
      issues.push(issue(`${codePrefix}.permission.unsupported`, `${path}[${index}]`, "Permission is not supported."));
    }
  });

  return permissions;
}

function isPluginPermission(value: unknown): value is PluginPermission {
  return typeof value === "string" && SUPPORTED_PLUGIN_PERMISSIONS.includes(value as PluginPermission);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isSafePackageRelativePath(value: string) {
  if (value.includes("\0") || value.trim() !== value || value.length === 0) {
    return false;
  }

  if (
    value.startsWith("/") ||
    value.startsWith("\\") ||
    /^[A-Za-z]:[\\/]/.test(value) ||
    /^[A-Za-z][A-Za-z0-9+.-]*:/.test(value)
  ) {
    return false;
  }

  return value
    .split(/[\\/]+/)
    .every((segment) => segment.length > 0 && segment !== "." && segment !== "..");
}

function isHttpsGitHubUrl(value: string) {
  try {
    const url = new URL(value);
    return url.protocol === "https:" && url.hostname === "github.com";
  } catch {
    return false;
  }
}

function urlPathname(value: string) {
  try {
    return new URL(value).pathname;
  } catch {
    return "";
  }
}

function isCompatibleZeroHostRange(value: string) {
  return (
    value === "*" ||
    value === SUPPORTED_ZERO_HOST_VERSION ||
    value === `^${SUPPORTED_ZERO_HOST_VERSION}` ||
    value === `>=${SUPPORTED_ZERO_HOST_VERSION}`
  );
}

function normalizeManifestEngines(
  value: Record<string, unknown>,
): Record<string, unknown> {
  if (!isRecord(value.engines)) {
    return value;
  }

  const zero =
    typeof value.engines.zero === "string"
      ? value.engines.zero
      : value.engines.ztool;
  const engines: Record<string, unknown> = { ...value.engines };
  delete engines.ztool;
  if (typeof zero === "string") {
    engines.zero = zero;
  } else {
    delete engines.zero;
  }

  return {
    ...value,
    engines,
  };
}

function issue(code: string, path: string, message: string): PluginValidationIssue {
  return { code, path, message };
}
