import { ScreenshotAction, useScreenshotPlugin } from "./useScreenshotPlugin";
import { SCREENSHOT_SHORTCUT, SCREENSHOT_USAGE_KEYS } from "./screenshotMeta";
import type { TranslationKey } from "./i18n";

const pendingTools = [
  { key: "rectangle", icon: "□", labelKey: "tool.rectangle" },
  { key: "ellipse", icon: "○", labelKey: "tool.ellipse" },
  { key: "arrow", icon: "↗", labelKey: "tool.arrow" },
  { key: "pen", icon: "✎", labelKey: "tool.pen" },
  { key: "mosaic", icon: "▦", labelKey: "tool.mosaic" },
  { key: "text", icon: "T", labelKey: "tool.text" },
  { key: "pin", icon: "⌖", labelKey: "tool.pin" },
];

interface ScreenshotPanelProps {
  t: (key: TranslationKey) => string;
}

export function ScreenshotPanel({ t }: ScreenshotPanelProps) {
  const screenshot = useScreenshotPlugin();
  const message = screenshot.messageDetail
    ? `${t(screenshot.messageKey)}: ${screenshot.messageDetail}`
    : t(screenshot.messageKey);

  const launch = (action: ScreenshotAction) => {
    screenshot.start(action);
  };

  return (
    <section className="plugin-panel screenshot-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">{t("screenshot.eyebrow")}</span>
          <h2>{t("screenshot.title")}</h2>
        </div>
        <span className="status-pill active">{SCREENSHOT_SHORTCUT.display}</span>
      </div>

      <div className="shortcut-card" aria-label="截图快捷键">
        <span>{t("screenshot.shortcut")}</span>
        <strong>{SCREENSHOT_SHORTCUT.display}</strong>
        <small>
          {screenshot.capabilities.platform} · {t("screenshot.shortcutHint")}
        </small>
      </div>

      <div className="usage-list">
        {SCREENSHOT_USAGE_KEYS.map((key) => (
          <p key={key}>{t(key)}</p>
        ))}
      </div>

      <div className="compact-tools" aria-label="截图工具状态">
        {pendingTools.map((tool) => (
          <button
            type="button"
            className="tool-button pending"
            key={tool.key}
            title={`${t(tool.labelKey as TranslationKey)} (${t("screenshot.pendingSuffix")})`}
            disabled
          >
            {tool.icon}
          </button>
        ))}
        <span className="tool-divider" />
        <button
          type="button"
          className="tool-button"
          title={t("screenshot.save")}
          onClick={() => launch("save")}
          disabled={screenshot.isBusy}
        >
          ↓
        </button>
        <button
          type="button"
          className="tool-button confirm"
          title={t("screenshot.copy")}
          onClick={() => launch("copy")}
          disabled={screenshot.isBusy}
        >
          ✓
        </button>
      </div>

      <div className="panel-copy">
        <strong>
          {screenshot.capabilities.selection_visual
            ? t("screenshot.readyTitle")
            : t("screenshot.fallbackTitle")}
        </strong>
        <span>{message}</span>
      </div>

      <div className="button-row">
        <button
          type="button"
          className="primary-action"
          onClick={() => launch("copy")}
          disabled={screenshot.isBusy}
        >
          <span className="button-icon">✓</span>
          {t("screenshot.copyAction")}
        </button>
        <button
          type="button"
          className="secondary-action"
          onClick={() => launch("save")}
          disabled={screenshot.isBusy}
        >
          <span className="button-icon">↓</span>
          {t("screenshot.saveAction")}
        </button>
      </div>
    </section>
  );
}
