import { useState } from "react";
import type {
  PluginMarketEntry,
  PluginPackageValidationReport,
  PluginPermission,
  PluginRecord,
} from "./contracts";
import { pluginHostService } from "./pluginHostService";
import { usePluginMarket } from "./usePluginMarket";
import type { PluginHostController } from "./usePluginHost";

type PendingInstall =
  | {
      kind: "market";
      entry: PluginMarketEntry;
      permissions: PluginPermission[];
    }
  | {
      kind: "local";
      packagePath: string;
      pluginName: string;
      permissions: PluginPermission[];
    };

interface PluginManagerPanelProps {
  pluginHost: PluginHostController;
}

export function PluginManagerPanel({ pluginHost }: PluginManagerPanelProps) {
  const market = usePluginMarket();
  const [localPackagePath, setLocalPackagePath] = useState("");
  const [validationReport, setValidationReport] =
    useState<PluginPackageValidationReport | null>(null);
  const [pendingInstall, setPendingInstall] = useState<PendingInstall | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [isWorking, setIsWorking] = useState(false);

  const diagnostics = [
    pluginHost.error,
    market.error,
    validationReport?.valid === false ? validationSummary(validationReport) : null,
    status,
  ].filter((message): message is string => Boolean(message));

  async function refreshMarket() {
    setStatus(null);
    try {
      await market.refresh();
      setStatus("Market refreshed.");
    } catch (error) {
      setStatus(`Market refresh failed: ${String(error)}`);
    }
  }

  async function validateLocalPackage() {
    const packagePath = localPackagePath.trim();
    if (!packagePath) {
      setStatus("Enter a local .zplugin path first.");
      return;
    }

    setIsWorking(true);
    setStatus(null);
    try {
      const report = await pluginHostService.validatePackage({ packagePath });
      setValidationReport(report);
      if (report.valid && report.manifest) {
        setPendingInstall({
          kind: "local",
          packagePath,
          pluginName: report.manifest.name,
          permissions: report.manifest.permissions,
        });
        setStatus("Local package is valid. Review permissions before install.");
      } else {
        setPendingInstall(null);
        setStatus("Local package validation failed.");
      }
    } catch (error) {
      setStatus(`Local package validation failed: ${String(error)}`);
    } finally {
      setIsWorking(false);
    }
  }

  function requestMarketInstall(entry: PluginMarketEntry) {
    setPendingInstall({
      kind: "market",
      entry,
      permissions: entry.permissions,
    });
    setStatus("Review permissions before installing this market plugin.");
  }

  async function confirmInstall() {
    if (!pendingInstall) {
      return;
    }

    setIsWorking(true);
    setStatus("Installing plugin...");
    try {
      if (pendingInstall.kind === "market") {
        await pluginHostService.installMarketPlugin({
          entry: pendingInstall.entry,
          approvedPermissions: pendingInstall.permissions,
          enabled: true,
        });
      } else {
        await pluginHostService.installPluginPackage({
          packagePath: pendingInstall.packagePath,
          approvedPermissions: pendingInstall.permissions,
          enabled: true,
        });
      }

      setPendingInstall(null);
      setValidationReport(null);
      await pluginHost.reload();
      await market.loadCached().catch(() => undefined);
      setStatus("Plugin installed and enabled.");
    } catch (error) {
      setStatus(`Plugin install failed: ${String(error)}`);
    } finally {
      setIsWorking(false);
    }
  }

  function declineInstall() {
    setPendingInstall(null);
    setStatus("Install cancelled before permissions were approved.");
  }

  async function runLifecycleAction(
    action: () => Promise<PluginRecord | PluginRecord[] | unknown>,
    successMessage: string,
  ) {
    setIsWorking(true);
    setStatus(null);
    try {
      await action();
      setStatus(successMessage);
    } catch (error) {
      setStatus(`Plugin lifecycle action failed: ${String(error)}`);
    } finally {
      setIsWorking(false);
    }
  }

  return (
    <section className="plugin-panel system-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Plugin Host</span>
          <h2>Extensions</h2>
        </div>
        <span className={market.stale ? "status-pill" : "status-pill active"}>
          {market.stale ? "stale" : "ready"}
        </span>
      </div>

      <div className="settings-group">
        <div className="settings-title">Git-based market</div>
        <button
          type="button"
          className="secondary-action"
          disabled={market.isLoading || isWorking}
          onClick={refreshMarket}
        >
          {market.isLoading ? "Refreshing..." : "Refresh market.json"}
        </button>

        {market.installCards.map((card) => (
          <div className="setting-row compact" key={`${card.name}@${card.version}`}>
            <span>
              <strong>{card.title}</strong>
              <small>
                {card.version} · {card.checksumStatus} ·{" "}
                {card.isInstalled ? `installed ${card.installedVersion}` : "not installed"}
              </small>
              <small>{card.releaseUrl}</small>
            </span>
            <button
              type="button"
              className="secondary-action"
              disabled={isWorking || card.isInstalled}
              onClick={() => {
                const entry = market.entries.find(
                  (candidate) =>
                    candidate.name === card.name && candidate.version === card.version,
                );
                if (entry) {
                  requestMarketInstall(entry);
                }
              }}
            >
              Install
            </button>
          </div>
        ))}
      </div>

      <div className="settings-group">
        <div className="settings-title">Local .zplugin package</div>
        <label className="setting-row">
          <span>
            <strong>Package path</strong>
            <small>Validate before install; package assets stay under ~/.zero/plugins.</small>
          </span>
          <input
            type="text"
            value={localPackagePath}
            placeholder="/path/to/plugin.zplugin"
            onChange={(event) => setLocalPackagePath(event.currentTarget.value)}
          />
        </label>
        <button
          type="button"
          className="secondary-action"
          disabled={isWorking}
          onClick={validateLocalPackage}
        >
          Validate local package
        </button>
      </div>

      {pendingInstall ? (
        <div className="settings-group">
          <div className="settings-title">Permission review</div>
          <p className="settings-message">
            {pendingInstall.kind === "market"
              ? pendingInstall.entry.name
              : pendingInstall.pluginName}{" "}
            requests: {formatPermissions(pendingInstall.permissions)}
          </p>
          <button
            type="button"
            className="primary-action"
            disabled={isWorking}
            onClick={confirmInstall}
          >
            Approve and install
          </button>
          <button
            type="button"
            className="secondary-action"
            disabled={isWorking}
            onClick={declineInstall}
          >
            Decline
          </button>
        </div>
      ) : null}

      <div className="settings-group">
        <div className="settings-title">Installed plugins</div>
        {pluginHost.records.map((record) => (
          <div className="setting-row compact" key={record.name}>
            <span>
              <strong>{record.manifest.displayName ?? record.name}</strong>
              <small>
                {record.source} · {record.version} · {record.health}
              </small>
            </span>
            <button
              type="button"
              className="secondary-action"
              disabled={isWorking}
              onClick={() =>
                runLifecycleAction(
                  () =>
                    pluginHost.setPluginEnabled({
                      name: record.name,
                      enabled: record.health === "error" ? true : !record.enabled,
                    }),
                  record.health === "error"
                    ? "Plugin retry requested."
                    : record.enabled
                      ? "Plugin disabled."
                      : "Plugin enabled.",
                )
              }
            >
              {record.health === "error" ? "Retry" : record.enabled ? "Disable" : "Enable"}
            </button>
            <button
              type="button"
              className="secondary-action danger"
              disabled={isWorking}
              onClick={() =>
                runLifecycleAction(
                  () => pluginHost.uninstallPlugin({ name: record.name }),
                  "Plugin uninstalled.",
                )
              }
            >
              Uninstall
            </button>
          </div>
        ))}
        <button
          type="button"
          className="secondary-action"
          disabled={isWorking}
          onClick={() =>
            runLifecycleAction(
              pluginHost.restoreBundledPlugins,
              "Bundled defaults restored.",
            )
          }
        >
          Restore bundled defaults
        </button>
      </div>

      {diagnostics.length > 0 ? (
        <div className="settings-group">
          <div className="settings-title">Diagnostics</div>
          {diagnostics.map((diagnostic) => (
            <p className="settings-message" key={diagnostic}>
              {diagnostic}
            </p>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function formatPermissions(permissions: PluginPermission[]) {
  const labels: Partial<Record<PluginPermission, string>> = {
    "system.apps.read": "read installed applications",
    "system.apps.execute": "launch indexed applications",
    "system.window.focus": "focus running application windows",
    "system.settings.open": "open approved system settings",
  };
  return permissions.length > 0
    ? permissions.map((permission) => labels[permission] ?? permission).join(", ")
    : "no host permissions";
}

function validationSummary(report: PluginPackageValidationReport) {
  return report.issues
    .map((issue) => `${issue.code}: ${issue.message}`)
    .join("; ");
}
