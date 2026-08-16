import type { ReactNode } from "react";
import { StatusBarGlyph } from "../../components/StatusBarGlyph";
import type { PluginRecord } from "../pluginHost/contracts";
import type { PluginHostController } from "../pluginHost/usePluginHost";
import type { PluginMeta } from "../pluginHost/pluginTypes";
import { PluginManagerPanel } from "../pluginHost/PluginManagerPanel";
import type { GlobalShortcutSnapshot } from "../../services/globalShortcuts";
import { shortcutStatusMessageKey } from "../../services/globalShortcutServiceCore";
import type { StatusBarController } from "../../services/useStatusBar";
import type { TranslationKey } from "./i18n";
import type { AppPreferences, LanguagePreference } from "./preferencesModel";
import {
  preferencesSettingFocusTargetId,
  type PreferencesDestinationId,
} from "./preferencesNavigation";
import type { PreferenceFeedback, PreferenceFeedbackMap } from "./preferencesFeedback";

export type PreferencesOperationRunner = (
  operationKey: string,
  operation: () => void | Promise<unknown>,
) => Promise<boolean>;

interface SharedPageProps {
  feedback: PreferenceFeedbackMap;
  runOperation: PreferencesOperationRunner;
  t: (key: TranslationKey) => string;
}

interface GeneralPreferencesPageProps extends SharedPageProps {
  isAutostartBusy: boolean;
  preferences: AppPreferences;
  onLanguageChange: (language: LanguagePreference) => void;
  onLaunchAtLoginChange: (enabled: boolean) => Promise<void>;
}

export function GeneralPreferencesPage({
  feedback,
  isAutostartBusy,
  preferences,
  runOperation,
  t,
  onLanguageChange,
  onLaunchAtLoginChange,
}: GeneralPreferencesPageProps) {
  return (
    <SettingsSection title={t("prefs.general.title")}>
      <SettingRow
        title={t("prefs.launchAtLogin.title")}
        description={t("prefs.launchAtLogin.description")}
        feedback={feedback["general.autostart"]}
        t={t}
      >
        <input
          id={preferencesSettingFocusTargetId("general.open-at-login")}
          type="checkbox"
          role="switch"
          checked={preferences.launchAtLogin}
          disabled={isAutostartBusy || feedback["general.autostart"]?.status === "pending"}
          onChange={(event) => {
            const enabled = event.currentTarget.checked;
            void runOperation("general.autostart", () => onLaunchAtLoginChange(enabled));
          }}
        />
      </SettingRow>

      <SettingRow
        title={t("prefs.language.title")}
        description={t("prefs.language.description")}
        feedback={feedback["general.language"]}
        t={t}
      >
        <select
          id={preferencesSettingFocusTargetId("general.language")}
          className="language-select"
          value={preferences.language}
          onChange={(event) => {
            const language = event.currentTarget.value as LanguagePreference;
            void runOperation("general.language", () => onLanguageChange(language));
          }}
        >
          <option value="system">{t("prefs.language.system")}</option>
          <option value="zh-CN">{t("prefs.language.zh")}</option>
          <option value="en-US">{t("prefs.language.en")}</option>
        </select>
      </SettingRow>
    </SettingsSection>
  );
}

interface StatusBarPreferencesPageProps extends SharedPageProps {
  plugins: PluginMeta[];
  records: PluginRecord[];
  statusBar: StatusBarController;
}

export function StatusBarPreferencesPage({
  feedback,
  plugins,
  records,
  runOperation,
  statusBar,
  t,
}: StatusBarPreferencesPageProps) {
  const disabled = statusBar.isLoading || statusBar.isBusy;

  return (
    <>
      <SettingsSection title={t("statusBar.title")}>
        <SettingRow
          title={t("statusBar.enabled.title")}
          description={t("statusBar.enabled.description")}
          feedback={feedback["status-bar.enabled"]}
          t={t}
        >
          <input
            id={preferencesSettingFocusTargetId("status-bar.enabled")}
            type="checkbox"
            role="switch"
            checked={statusBar.settings.enabled}
            disabled={disabled}
            onChange={(event) => {
              const enabled = event.currentTarget.checked;
              void runOperation("status-bar.enabled", () => statusBar.setEnabled(enabled));
            }}
          />
        </SettingRow>

        <SettingRow
          title={t("statusBar.launch.title")}
          description={t("statusBar.launch.description")}
          hint={!statusBar.settings.enabled ? t("statusBar.disabledHint") : undefined}
          feedback={feedback["status-bar.launch"]}
          t={t}
        >
          <input
            id={preferencesSettingFocusTargetId("status-bar.launch")}
            type="checkbox"
            role="switch"
            checked={statusBar.settings.showPluginItemsOnLaunch}
            disabled={!statusBar.settings.enabled || disabled}
            onChange={(event) => {
              const enabled = event.currentTarget.checked;
              void runOperation("status-bar.launch", () =>
                statusBar.setShowPluginItemsOnLaunch(enabled),
              );
            }}
          />
        </SettingRow>

        <SettingRow
          title={t("statusBar.collapsed.title")}
          description={t("statusBar.collapsed.description")}
          hint={!statusBar.settings.enabled ? t("statusBar.disabledHint") : undefined}
          feedback={feedback["status-bar.collapsed"]}
          t={t}
        >
          <input
            id={preferencesSettingFocusTargetId("status-bar.collapsed")}
            type="checkbox"
            role="switch"
            checked={statusBar.settings.pluginItemsCollapsed}
            disabled={!statusBar.settings.enabled || disabled}
            onChange={(event) => {
              const collapsed = event.currentTarget.checked;
              void runOperation("status-bar.collapsed", () =>
                statusBar.setPluginItemsCollapsed(collapsed),
              );
            }}
          />
        </SettingRow>

        <div className="preferences-setting-row preferences-preview-row">
          <div className="preferences-setting-copy">
            <strong>{t("statusBar.preview.title")}</strong>
            <small>{t("statusBar.preview.description")}</small>
          </div>
          <div className="status-bar-preview-strip" aria-label={t("statusBar.preview.title")}>
            {statusBar.previewItems.map((item) => (
              <span className="status-bar-preview-item" title={item.title} key={item.id}>
                <StatusBarGlyph icon={item.icon} />
              </span>
            ))}
          </div>
        </div>
      </SettingsSection>

      <SettingsSection
        title={t("statusBar.items.title")}
        description={t("statusBar.items.description")}
        focusTargetId={preferencesSettingFocusTargetId("status-bar.items")}
      >
        {records.some((record) => record.manifest.contributes?.statusBarItems?.length) ? (
          records.flatMap((record) => {
            const contribution = record.manifest.contributes?.statusBarItems?.[0];
            if (!contribution) return [];
            const plugin = plugins.find((candidate) => candidate.id === record.name);
            const operationKey = `tool.${record.name}.status-bar`;
            const disabledHint = !record.enabled
              ? t("prefs.tool.disabledHint")
              : !statusBar.settings.enabled
                ? t("statusBar.disabledHint")
                : undefined;
            return (
              <SettingRow
                key={record.name}
                title={plugin?.title ?? contribution.title}
                description={plugin?.subtitle ?? record.name}
                hint={disabledHint}
                feedback={feedback[operationKey]}
                t={t}
                icon={<StatusBarGlyph icon={contribution.icon} />}
              >
                <input
                  type="checkbox"
                  role="switch"
                  checked={statusBar.settings.visiblePluginItems[record.name] ?? true}
                  disabled={!record.enabled || !statusBar.settings.enabled || disabled}
                  onChange={(event) => {
                    const visible = event.currentTarget.checked;
                    void runOperation(operationKey, () =>
                      statusBar.setPluginItemVisible(record.name, visible),
                    );
                  }}
                />
              </SettingRow>
            );
          })
        ) : (
          <p className="preferences-empty-copy">{t("statusBar.items.empty")}</p>
        )}
      </SettingsSection>

      {statusBar.error ? (
        <p className="preferences-inline-error" role="alert">
          {t("statusBar.message.error")}: {statusBar.error}
        </p>
      ) : null}
    </>
  );
}

interface ToolsOverviewPageProps {
  plugins: PluginMeta[];
  records: PluginRecord[];
  preferences: AppPreferences;
  statusBar: StatusBarController;
  t: (key: TranslationKey) => string;
  onSelectDestination: (destinationId: PreferencesDestinationId) => void;
}

export function ToolsOverviewPage({
  plugins,
  records,
  preferences,
  statusBar,
  t,
  onSelectDestination,
}: ToolsOverviewPageProps) {
  return (
    <SettingsSection
      title={t("prefs.tools.overview")}
      description={t("prefs.tools.description")}
      focusTargetId={preferencesSettingFocusTargetId("tools.overview")}
    >
      {records.map((record) => {
        const plugin = plugins.find((candidate) => candidate.id === record.name);
        const hasStatusBar = Boolean(record.manifest.contributes?.statusBarItems?.length);
        return (
          <button
            type="button"
            className="preferences-tool-summary"
            key={record.name}
            onClick={() => onSelectDestination(`tool:${record.name}`)}
          >
            <span className="preferences-tool-summary-copy">
              <strong>{plugin?.title ?? record.manifest.displayName ?? record.name}</strong>
              <small>{plugin?.subtitle ?? record.manifest.description ?? record.name}</small>
            </span>
            <span className="preferences-tool-states" aria-hidden="true">
              <StateChip active={record.enabled} label={t("prefs.tool.enabled.title")} />
              <StateChip
                active={Boolean(preferences.visibleTools[record.name])}
                label={t("prefs.tool.navigation.title")}
              />
              {hasStatusBar ? (
                <StateChip
                  active={Boolean(statusBar.settings.visiblePluginItems[record.name])}
                  label={t("prefs.tool.statusBar.title")}
                />
              ) : null}
            </span>
            <span className="preferences-disclosure" aria-hidden="true">›</span>
          </button>
        );
      })}
    </SettingsSection>
  );
}

interface ToolPreferencesPageProps extends SharedPageProps {
  plugin: PluginMeta;
  preferences: AppPreferences;
  record: PluginRecord;
  shortcut: GlobalShortcutSnapshot | undefined;
  statusBar: StatusBarController;
  pluginHost: PluginHostController;
  onToolVisibleChange: (pluginId: string, visible: boolean) => boolean;
}

export function ToolPreferencesPage({
  feedback,
  plugin,
  pluginHost,
  preferences,
  record,
  runOperation,
  shortcut,
  statusBar,
  t,
  onToolVisibleChange,
}: ToolPreferencesPageProps) {
  const enabledKey = `tool.${record.name}.enabled`;
  const navigationKey = `tool.${record.name}.navigation`;
  const statusBarKey = `tool.${record.name}.status-bar`;
  const hasStatusBar = Boolean(record.manifest.contributes?.statusBarItems?.length);
  const storedStatusBarVisible = statusBar.settings.visiblePluginItems[record.name] ?? true;
  const dependentDisabled = !record.enabled;

  return (
    <>
      <dl className="preferences-tool-metadata" aria-label={t("prefs.tool.description")}>
        <Metadata label={t("prefs.tool.version")} value={record.version} />
        <Metadata label={t("prefs.tool.source")} value={localizedSource(record.source, t)} />
        <Metadata label={t("prefs.tool.health")} value={localizedHealth(record.health, t)} />
      </dl>

      <SettingsSection title={plugin.title} description={plugin.subtitle}>
        <SettingRow
          title={t("prefs.tool.enabled.title")}
          description={t("prefs.tool.enabled.description")}
          feedback={feedback[enabledKey]}
          t={t}
        >
          <input
            id={preferencesSettingFocusTargetId(`tool.${record.name}.enabled`)}
            type="checkbox"
            role="switch"
            checked={record.enabled}
            disabled={pluginHost.isBusy}
            onChange={(event) => {
              const enabled = event.currentTarget.checked;
              void runOperation(enabledKey, () =>
                pluginHost.setPluginEnabled({ name: record.name, enabled }),
              );
            }}
          />
        </SettingRow>

        <SettingRow
          title={t("prefs.tool.navigation.title")}
          description={t("prefs.tool.navigation.description")}
          hint={dependentDisabled ? t("prefs.tool.disabledHint") : undefined}
          feedback={feedback[navigationKey]}
          t={t}
        >
          <input
            id={preferencesSettingFocusTargetId(`tool.${record.name}.navigation`)}
            type="checkbox"
            role="switch"
            checked={Boolean(preferences.visibleTools[record.name])}
            disabled={dependentDisabled}
            onChange={(event) => {
              const visible = event.currentTarget.checked;
              void runOperation(navigationKey, () => {
                if (!onToolVisibleChange(record.name, visible)) {
                  throw new Error(t("prefs.tool.lastVisible"));
                }
              });
            }}
          />
        </SettingRow>

        {hasStatusBar ? (
          <SettingRow
            title={t("prefs.tool.statusBar.title")}
            description={t("prefs.tool.statusBar.description")}
            hint={
              dependentDisabled
                ? t("prefs.tool.disabledHint")
                : !statusBar.settings.enabled
                  ? t("statusBar.disabledHint")
                  : undefined
            }
            feedback={feedback[statusBarKey]}
            t={t}
          >
            <input
              id={preferencesSettingFocusTargetId(`tool.${record.name}.status-bar`)}
              type="checkbox"
              role="switch"
              checked={storedStatusBarVisible}
              disabled={dependentDisabled || !statusBar.settings.enabled || statusBar.isBusy}
              onChange={(event) => {
                const visible = event.currentTarget.checked;
                void runOperation(statusBarKey, () =>
                  statusBar.setPluginItemVisible(record.name, visible),
                );
              }}
            />
          </SettingRow>
        ) : (
          <ReadOnlySetting
            focusTargetId={preferencesSettingFocusTargetId(`tool.${record.name}.status-bar`)}
            title={t("prefs.tool.statusBar.title")}
            description={t("prefs.tool.noStatusBar")}
          />
        )}

        {shortcut ? (
          <ShortcutSetting shortcut={shortcut} t={t} toolScoped />
        ) : (
          <ReadOnlySetting
            focusTargetId={preferencesSettingFocusTargetId(`tool.${record.name}.shortcut`)}
            title={t("prefs.tool.shortcut.title")}
            description={t("prefs.tool.noShortcut")}
          />
        )}
      </SettingsSection>

      <p className="preferences-empty-copy">{t("prefs.tool.noAdditionalSettings")}</p>
    </>
  );
}

interface ShortcutsPreferencesPageProps {
  error: string | null;
  isLoading: boolean;
  shortcuts: GlobalShortcutSnapshot[];
  t: (key: TranslationKey) => string;
}

export function ShortcutsPreferencesPage({
  error,
  isLoading,
  shortcuts,
  t,
}: ShortcutsPreferencesPageProps) {
  return (
    <SettingsSection
      title={t("prefs.shortcuts.title")}
      description={t("prefs.shortcuts.readOnly")}
    >
      {isLoading ? <p className="preferences-empty-copy">{t("prefs.shortcuts.loading")}</p> : null}
      {!isLoading
        ? shortcuts.map((shortcut) => (
            <ShortcutSetting shortcut={shortcut} t={t} key={shortcut.id} />
          ))
        : null}
      {error ? (
        <p className="preferences-inline-error" role="alert">
          {t("prefs.shortcuts.loadError")}: {error}
        </p>
      ) : null}
    </SettingsSection>
  );
}

export function ExtensionsPreferencesPage({
  pluginHost,
  t,
}: {
  pluginHost: PluginHostController;
  t: (key: TranslationKey) => string;
}) {
  return <PluginManagerPanel pluginHost={pluginHost} t={t} />;
}

function ShortcutSetting({
  shortcut,
  t,
  toolScoped = false,
}: {
  shortcut: GlobalShortcutSnapshot;
  t: (key: TranslationKey) => string;
  toolScoped?: boolean;
}) {
  const settingId = toolScoped
    ? `tool.${shortcut.pluginName}.shortcut`
    : shortcut.id === "snapCapture"
      ? "shortcuts.snap"
      : "shortcuts.launch";
  const title = shortcut.id === "snapCapture" ? t("prefs.shortcuts.snap") : t("prefs.shortcuts.launch");
  return (
    <ReadOnlySetting
      focusTargetId={preferencesSettingFocusTargetId(settingId)}
      title={toolScoped ? t("prefs.tool.shortcut.title") : title}
      description={`${t("prefs.shortcuts.owner")}: ${shortcut.pluginName}`}
      value={shortcut.accelerator}
      status={t(shortcutStatusMessageKey(shortcut.registrationState))}
      tone={shortcut.registrationState === "active" ? "success" : shortcut.registrationState === "conflict" ? "error" : "neutral"}
    />
  );
}

export function SettingsSection({
  children,
  description,
  focusTargetId,
  title,
}: {
  children: ReactNode;
  description?: string;
  focusTargetId?: string;
  title: string;
}) {
  return (
    <section className="preferences-section" id={focusTargetId} tabIndex={focusTargetId ? -1 : undefined}>
      <div className="preferences-section-heading">
        <h3>{title}</h3>
        {description ? <p>{description}</p> : null}
      </div>
      <div className="preferences-settings-group">{children}</div>
    </section>
  );
}

function SettingRow({
  children,
  description,
  feedback,
  hint,
  icon,
  t,
  title,
}: {
  children: ReactNode;
  description: string;
  feedback?: PreferenceFeedback;
  hint?: string;
  icon?: ReactNode;
  t: (key: TranslationKey) => string;
  title: string;
}) {
  return (
    <div className="preferences-setting-row">
      <div className="preferences-setting-copy">
        <span className="preferences-setting-title">{icon}{<strong>{title}</strong>}</span>
        <small>{description}</small>
        {hint ? <small className="preferences-setting-hint">{hint}</small> : null}
        <PreferenceFeedbackText feedback={feedback} t={t} />
      </div>
      <div className="preferences-setting-control">{children}</div>
    </div>
  );
}

function PreferenceFeedbackText({
  feedback,
  t,
}: {
  feedback?: PreferenceFeedback;
  t: (key: TranslationKey) => string;
}) {
  if (!feedback || feedback.status === "idle") {
    return null;
  }
  const label =
    feedback.status === "pending"
      ? t("prefs.feedback.saving")
      : feedback.status === "saved"
        ? t("prefs.feedback.saved")
        : t("prefs.feedback.error");
  return (
    <small
      className={`preferences-feedback ${feedback.status}`}
      role={feedback.status === "error" ? "alert" : undefined}
      aria-live={feedback.status === "error" ? "assertive" : "polite"}
    >
      {feedback.message ? `${label}: ${feedback.message}` : label}
    </small>
  );
}

function ReadOnlySetting({
  description,
  focusTargetId,
  status,
  title,
  tone = "neutral",
  value,
}: {
  description: string;
  focusTargetId: string;
  status?: string;
  title: string;
  tone?: "neutral" | "success" | "error";
  value?: string;
}) {
  return (
    <div className="preferences-setting-row" id={focusTargetId} tabIndex={-1}>
      <div className="preferences-setting-copy">
        <strong>{title}</strong>
        <small>{description}</small>
      </div>
      <div className="preferences-readonly-value">
        {value ? <kbd>{value}</kbd> : null}
        {status ? <span className={`preferences-state ${tone}`}>{status}</span> : null}
      </div>
    </div>
  );
}

function StateChip({ active, label }: { active: boolean; label: string }) {
  return <span className={active ? "preferences-state success" : "preferences-state neutral"}>{label}</span>;
}

function Metadata({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function localizedSource(source: PluginRecord["source"], t: (key: TranslationKey) => string) {
  return t(`prefs.tool.source.${source}` as TranslationKey);
}

function localizedHealth(health: PluginRecord["health"], t: (key: TranslationKey) => string) {
  return t(`prefs.tool.health.${health}` as TranslationKey);
}
