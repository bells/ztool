import type { TranslationKey } from "./i18n";
import { QuickLauncherView } from "./QuickLauncherView";
import { useQuickLauncher } from "./useQuickLauncher";

export function QuickLauncherPanel({
  t,
}: {
  t: (key: TranslationKey) => string;
}) {
  const controller = useQuickLauncher();
  return (
    <section className="plugin-panel quick-launcher-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Launcher</span>
          <h2>{t("launcher.title")}</h2>
        </div>
        <span className="status-pill">⌘⇧Space</span>
      </div>
      <QuickLauncherView controller={controller} surface="panel" t={t} />
    </section>
  );
}
