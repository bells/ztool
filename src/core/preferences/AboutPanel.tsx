import type { PluginMeta } from "../pluginHost/pluginTypes";
import type { PluginHostSummary } from "../pluginHost/pluginHostModel";
import type { TranslationKey } from "./i18n";
import { StatusBarGlyph } from "../../components/StatusBarGlyph";

interface AboutPanelProps {
  plugins: PluginMeta[];
  summary?: PluginHostSummary;
  t: (key: TranslationKey) => string;
}

export function AboutPanel({ plugins, summary, t }: AboutPanelProps) {
  return (
    <section className="plugin-panel system-panel about-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">{t("about.eyebrow")}</span>
          <h2>{t("about.title")}</h2>
        </div>
        <span className="status-pill">v0.1.0</span>
      </div>

      <div className="about-mark">
        <StatusBarGlyph icon="zero" />
      </div>

      <div className="panel-copy">
        <strong>{t("about.descriptionTitle")}</strong>
        <span>{t("about.description")}</span>
      </div>

      <div className="about-grid">
        <span>{t("about.pluginCount")}</span>
        <strong>{summary?.total ?? plugins.length}</strong>
        <span>Bundled</span>
        <strong>{summary?.bundled ?? 0}</strong>
        <span>Market</span>
        <strong>{summary?.market ?? 0}</strong>
        <span>Local</span>
        <strong>{summary?.local ?? 0}</strong>
        <span>Disabled</span>
        <strong>{summary?.disabled ?? 0}</strong>
        <span>Failed</span>
        <strong>{summary?.failed ?? 0}</strong>
        <span>Incompatible</span>
        <strong>{summary?.incompatible ?? 0}</strong>
        <span>{t("about.runtime")}</span>
        <strong>Tray App</strong>
      </div>
    </section>
  );
}
