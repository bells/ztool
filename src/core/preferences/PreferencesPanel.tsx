import { StatusBarGlyph } from "../../components/StatusBarGlyph";
import type { StatusBarController } from "../../services/useStatusBar";
import type { PluginId, PluginMeta } from "../pluginHost/pluginTypes";
import { AppPreferences, LanguagePreference } from "./preferencesModel";
import type { TranslationKey } from "./i18n";

interface PreferencesPanelProps {
  plugins: PluginMeta[];
  preferences: AppPreferences;
  isAutostartBusy: boolean;
  message: string;
  statusBar: StatusBarController;
  t: (key: TranslationKey) => string;
  onLaunchAtLoginChange: (enabled: boolean) => void;
  onLanguageChange: (language: LanguagePreference) => void;
  onToolVisibleChange: (pluginId: PluginId, visible: boolean) => void;
}

export function PreferencesPanel({
  plugins,
  preferences,
  isAutostartBusy,
  message,
  statusBar,
  t,
  onLaunchAtLoginChange,
  onLanguageChange,
  onToolVisibleChange,
}: PreferencesPanelProps) {
  return (
    <section className="plugin-panel system-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">{t("prefs.eyebrow")}</span>
          <h2>{t("prefs.title")}</h2>
        </div>
        <span className="status-pill active">{t("prefs.saved")}</span>
      </div>

      <div className="settings-group">
        <label className="setting-row">
          <span>
            <strong>{t("prefs.launchAtLogin.title")}</strong>
            <small>{t("prefs.launchAtLogin.description")}</small>
          </span>
          <input
            type="checkbox"
            role="switch"
            checked={preferences.launchAtLogin}
            disabled={isAutostartBusy}
            onChange={(event) => onLaunchAtLoginChange(event.currentTarget.checked)}
          />
        </label>
      </div>

      <div className="settings-group">
        <label className="setting-row">
          <span>
            <strong>{t("prefs.language.title")}</strong>
            <small>{t("prefs.language.description")}</small>
          </span>
          <select
            className="language-select"
            value={preferences.language}
            onChange={(event) => onLanguageChange(event.currentTarget.value as LanguagePreference)}
          >
            <option value="system">{t("prefs.language.system")}</option>
            <option value="zh-CN">{t("prefs.language.zh")}</option>
            <option value="en-US">{t("prefs.language.en")}</option>
          </select>
        </label>
      </div>

      <div className="settings-group">
        <div className="settings-title">{t("prefs.tools.title")}</div>
        {plugins.map((plugin) => (
          <label className="setting-row compact" key={plugin.id}>
            <span>
              <strong>{plugin.title}</strong>
              <small>{plugin.subtitle}</small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={preferences.visibleTools[plugin.id]}
              onChange={(event) => onToolVisibleChange(plugin.id, event.currentTarget.checked)}
            />
          </label>
        ))}
      </div>

      <div className="settings-group status-bar-settings">
        <div className="settings-title">{t("statusBar.title")}</div>
        <label className="setting-row">
          <span>
            <strong>{t("statusBar.enabled.title")}</strong>
            <small>{t("statusBar.enabled.description")}</small>
          </span>
          <input
            type="checkbox"
            role="switch"
            checked={statusBar.settings.enabled}
            disabled={statusBar.isLoading || statusBar.isBusy}
            onChange={(event) => {
              void statusBar.setEnabled(event.currentTarget.checked);
            }}
          />
        </label>

        <label className="setting-row">
          <span>
            <strong>{t("statusBar.launch.title")}</strong>
            <small>{t("statusBar.launch.description")}</small>
          </span>
          <input
            type="checkbox"
            role="switch"
            checked={statusBar.settings.showPluginItemsOnLaunch}
            disabled={!statusBar.settings.enabled || statusBar.isLoading || statusBar.isBusy}
            onChange={(event) => {
              void statusBar.setShowPluginItemsOnLaunch(event.currentTarget.checked);
            }}
          />
        </label>

        <div className="status-bar-preview-block">
          <div className="status-bar-preview-heading">
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

        <div className="status-bar-item-list">
          <div className="settings-title">{t("statusBar.items.title")}</div>
          {statusBar.preferenceItems.length > 0 ? (
            statusBar.preferenceItems.map((item) => (
              <label className="setting-row compact status-bar-item-row" key={item.id}>
                <span className="status-bar-item-copy">
                  <span className="status-bar-item-title">
                    <StatusBarGlyph icon={item.icon} />
                    <strong>{item.title}</strong>
                  </span>
                  <small>{item.pluginName}</small>
                </span>
                <input
                  type="checkbox"
                  role="switch"
                  checked={item.visible}
                  disabled={item.disabled || statusBar.isLoading || statusBar.isBusy}
                  onChange={(event) => {
                    void statusBar.setPluginItemVisible(
                      item.pluginName,
                      event.currentTarget.checked,
                    );
                  }}
                />
              </label>
            ))
          ) : (
            <p className="settings-inline-message">{t("statusBar.items.empty")}</p>
          )}
        </div>

        <p className={`settings-message ${statusBar.error ? "error" : ""}`}>
          {statusBar.messageDetail
            ? `${t(statusBar.messageKey)}: ${statusBar.messageDetail}`
            : t(statusBar.messageKey)}
        </p>
      </div>

      <p className="settings-message">{message}</p>
    </section>
  );
}
