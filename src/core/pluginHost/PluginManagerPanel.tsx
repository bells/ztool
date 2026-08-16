import { useState, type ReactNode } from "react";
import type { TranslationKey } from "../preferences/i18n";
import { preferencesSettingFocusTargetId } from "../preferences/preferencesNavigation";
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
  | { kind: "market"; entry: PluginMarketEntry; permissions: PluginPermission[] }
  | { kind: "local"; packagePath: string; pluginName: string; permissions: PluginPermission[] };

interface ExtensionStatus {
  key: TranslationKey;
  detail?: string;
  tone: "info" | "error" | "success";
}

interface PluginManagerPanelProps {
  pluginHost: PluginHostController;
  t: (key: TranslationKey) => string;
}

export function PluginManagerPanel({ pluginHost, t }: PluginManagerPanelProps) {
  const market = usePluginMarket();
  const [localPackagePath, setLocalPackagePath] = useState("");
  const [validationReport, setValidationReport] = useState<PluginPackageValidationReport | null>(null);
  const [pendingInstall, setPendingInstall] = useState<PendingInstall | null>(null);
  const [status, setStatus] = useState<ExtensionStatus | null>(null);
  const [isWorking, setIsWorking] = useState(false);
  const diagnostics = [
    pluginHost.error,
    market.error,
    validationReport?.valid === false ? validationSummary(validationReport) : null,
  ].filter((message): message is string => Boolean(message));

  async function refreshMarket() {
    setStatus(null);
    try {
      await market.refresh();
      setStatus({ key: "extensions.status.marketRefreshed", tone: "success" });
    } catch (error) {
      setStatus({ key: "extensions.status.marketRefreshFailed", detail: String(error), tone: "error" });
    }
  }

  async function validateLocalPackage() {
    const packagePath = localPackagePath.trim();
    if (!packagePath) {
      setStatus({ key: "extensions.status.enterPath", tone: "error" });
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
        setStatus({ key: "extensions.status.packageValid", tone: "success" });
      } else {
        setPendingInstall(null);
        setStatus({ key: "extensions.status.packageInvalid", tone: "error" });
      }
    } catch (error) {
      setStatus({ key: "extensions.status.packageValidationFailed", detail: String(error), tone: "error" });
    } finally {
      setIsWorking(false);
    }
  }

  function requestMarketInstall(entry: PluginMarketEntry) {
    setPendingInstall({ kind: "market", entry, permissions: entry.permissions });
    setStatus({ key: "extensions.status.reviewPermissions", tone: "info" });
  }

  async function confirmInstall() {
    if (!pendingInstall) return;
    setIsWorking(true);
    setStatus({ key: "extensions.status.installing", tone: "info" });
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
      setStatus({ key: "extensions.status.installed", tone: "success" });
    } catch (error) {
      setStatus({ key: "extensions.status.installFailed", detail: String(error), tone: "error" });
    } finally {
      setIsWorking(false);
    }
  }

  function declineInstall() {
    setPendingInstall(null);
    setStatus({ key: "extensions.status.cancelled", tone: "info" });
  }

  async function runLifecycleAction(
    action: () => Promise<PluginRecord | PluginRecord[] | unknown>,
    successKey: TranslationKey,
  ) {
    setIsWorking(true);
    setStatus(null);
    try {
      await action();
      setStatus({ key: successKey, tone: "success" });
    } catch (error) {
      setStatus({ key: "extensions.status.lifecycleFailed", detail: String(error), tone: "error" });
    } finally {
      setIsWorking(false);
    }
  }

  return (
    <div className="preferences-extension-sections">
      <ExtensionSection
        id={preferencesSettingFocusTargetId("extensions.market")}
        title={t("extensions.market.title")}
        description={t("extensions.market.description")}
      >
        <div className="preferences-section-toolbar">
          <span className={`preferences-state ${market.stale ? "neutral" : "success"}`}>
            {t(market.stale ? "extensions.market.stale" : "extensions.market.ready")}
          </span>
          <button type="button" className="secondary-action" disabled={market.isLoading || isWorking} onClick={() => void refreshMarket()}>
            {t(market.isLoading ? "extensions.market.refreshing" : "extensions.market.refresh")}
          </button>
        </div>

        {market.installCards.length > 0 ? market.installCards.map((card) => (
          <div className="preferences-setting-row" key={`${card.name}@${card.version}`}>
            <div className="preferences-setting-copy">
              <strong>{card.title}</strong>
              <small>
                {card.version} · {t(card.checksumStatus === "verified" ? "extensions.market.verified" : "extensions.market.unsigned")} · {t(card.isInstalled ? "extensions.market.installed" : "extensions.market.notInstalled")}
              </small>
              {card.description ? <small>{card.description}</small> : null}
            </div>
            <button
              type="button"
              className="secondary-action"
              disabled={isWorking || card.isInstalled}
              onClick={() => {
                const entry = market.entries.find((candidate) => candidate.name === card.name && candidate.version === card.version);
                if (entry) requestMarketInstall(entry);
              }}
            >
              {t("extensions.action.install")}
            </button>
          </div>
        )) : <p className="preferences-empty-copy">{t("extensions.market.empty")}</p>}
      </ExtensionSection>

      <ExtensionSection
        id={preferencesSettingFocusTargetId("extensions.local")}
        title={t("extensions.local.title")}
        description={t("extensions.local.description")}
      >
        <label className="preferences-setting-row">
          <span className="preferences-setting-copy">
            <strong>{t("extensions.local.path")}</strong>
            <small>{t("extensions.local.pathHint")}</small>
          </span>
          <input
            className="preferences-path-input"
            type="text"
            value={localPackagePath}
            placeholder={t("extensions.local.placeholder")}
            onChange={(event) => setLocalPackagePath(event.currentTarget.value)}
          />
        </label>
        <div className="preferences-section-actions">
          <button type="button" className="secondary-action" disabled={isWorking} onClick={() => void validateLocalPackage()}>
            {t("extensions.local.validate")}
          </button>
        </div>
      </ExtensionSection>

      {pendingInstall ? (
        <ExtensionSection title={t("extensions.permission.title")} description={t("extensions.permission.description")}>
          <p className="preferences-permission-summary">
            <strong>{pendingInstall.kind === "market" ? pendingInstall.entry.name : pendingInstall.pluginName}</strong>
            <span>{t("extensions.permission.requests")}: {formatPermissions(pendingInstall.permissions, t)}</span>
          </p>
          <div className="preferences-section-actions">
            <button type="button" className="primary-action" disabled={isWorking} onClick={() => void confirmInstall()}>{t("extensions.permission.approve")}</button>
            <button type="button" className="secondary-action" disabled={isWorking} onClick={declineInstall}>{t("extensions.permission.decline")}</button>
          </div>
        </ExtensionSection>
      ) : null}

      <ExtensionSection
        id={preferencesSettingFocusTargetId("extensions.installed")}
        title={t("extensions.installed.title")}
        description={t("extensions.installed.description")}
      >
        {pluginHost.records.length > 0 ? pluginHost.records.map((record) => (
          <div className="preferences-setting-row" key={record.name}>
            <div className="preferences-setting-copy">
              <strong>{record.manifest.displayName ?? record.name}</strong>
              <small>{record.version} · {record.author}</small>
            </div>
            <div className="preferences-row-actions">
              <button
                type="button"
                className="secondary-action"
                disabled={isWorking}
                onClick={() => void runLifecycleAction(
                  () => pluginHost.setPluginEnabled({ name: record.name, enabled: record.health === "error" ? true : !record.enabled }),
                  record.health === "error" ? "extensions.status.retryRequested" : record.enabled ? "extensions.status.disabled" : "extensions.status.enabled",
                )}
              >
                {t(record.health === "error" ? "extensions.action.retry" : record.enabled ? "extensions.action.disable" : "extensions.action.enable")}
              </button>
              <button type="button" className="secondary-action danger" disabled={isWorking} onClick={() => void runLifecycleAction(() => pluginHost.uninstallPlugin({ name: record.name }), "extensions.status.uninstalled")}>
                {t("extensions.action.uninstall")}
              </button>
            </div>
          </div>
        )) : <p className="preferences-empty-copy">{t("extensions.installed.empty")}</p>}
      </ExtensionSection>

      <ExtensionSection
        id={preferencesSettingFocusTargetId("extensions.restore")}
        title={t("extensions.restore.title")}
        description={t("extensions.restore.description")}
      >
        <div className="preferences-section-actions">
          <button type="button" className="secondary-action" disabled={isWorking} onClick={() => void runLifecycleAction(pluginHost.restoreBundledPlugins, "extensions.status.restored")}>
            {t("extensions.restore.action")}
          </button>
        </div>
      </ExtensionSection>

      <ExtensionSection
        id={preferencesSettingFocusTargetId("extensions.diagnostics")}
        title={t("extensions.diagnostics.title")}
        description={t("extensions.diagnostics.description")}
      >
        {diagnostics.length > 0 ? diagnostics.map((diagnostic) => (
          <p className="preferences-inline-error" role="alert" key={diagnostic}>{diagnostic}</p>
        )) : <p className="preferences-empty-copy">{t("extensions.diagnostics.empty")}</p>}
      </ExtensionSection>

      {status ? (
        <p className={`preferences-operation-status ${status.tone}`} role={status.tone === "error" ? "alert" : "status"} aria-live={status.tone === "error" ? "assertive" : "polite"}>
          {t(status.key)}{status.detail ? `: ${status.detail}` : ""}
        </p>
      ) : null}
    </div>
  );
}

function ExtensionSection({ children, description, id, title }: { children: ReactNode; description: string; id?: string; title: string }) {
  return (
    <section className="preferences-section" id={id} tabIndex={id ? -1 : undefined}>
      <div className="preferences-section-heading"><h3>{title}</h3><p>{description}</p></div>
      <div className="preferences-settings-group">{children}</div>
    </section>
  );
}

function formatPermissions(permissions: PluginPermission[], t: (key: TranslationKey) => string) {
  return permissions.length > 0
    ? permissions.map((permission) => t(`extensions.permission.${permission}` as TranslationKey)).join(", ")
    : t("extensions.permission.none");
}

function validationSummary(report: PluginPackageValidationReport) {
  return report.issues.map((issue) => `${issue.code}: ${issue.message}`).join("; ");
}
