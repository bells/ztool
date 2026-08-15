import type { TranslationKey } from "./i18n";
import { bingWallpaperDisplayTitle } from "./bingWallpaperModel";
import { useBingWallpaper } from "./useBingWallpaper";

interface BingWallpaperPanelProps {
  t: (key: TranslationKey) => string;
}

export function BingWallpaperPanel({ t }: BingWallpaperPanelProps) {
  const wallpaper = useBingWallpaper();
  const selected = wallpaper.navigation.selected;
  const previewReady = wallpaper.preview?.wallpaperId === selected?.id;
  const actionDisabled = !selected || wallpaper.isApplying || wallpaper.isSaving;
  const title = bingWallpaperDisplayTitle(selected, t("wallpaper.fallbackTitle"));

  return (
    <section className="plugin-panel bing-wallpaper-panel" aria-busy={wallpaper.isLoading}>
      <div className="wallpaper-header">
        <div>
          <span className="eyebrow">Bing</span>
          <h2>{t("wallpaper.title")}</h2>
        </div>
        <div className="wallpaper-actions" aria-label={t("wallpaper.actions")}>
          <button
            type="button"
            className="wallpaper-icon-action"
            aria-label={t("wallpaper.download")}
            title={t("wallpaper.download")}
            disabled={actionDisabled}
            onClick={() => void wallpaper.save()}
          >
            {wallpaper.isSaving ? "…" : "↓"}
          </button>
          <button
            type="button"
            className="wallpaper-icon-action wallpaper-apply-action"
            aria-label={t("wallpaper.apply")}
            title={t("wallpaper.apply")}
            disabled={actionDisabled || wallpaper.snapshot?.platform.supported === false}
            onClick={() => void wallpaper.apply()}
          >
            {wallpaper.isApplying ? "…" : "✓"}
          </button>
          <button
            type="button"
            className="wallpaper-icon-action"
            aria-label={t("wallpaper.older")}
            title={t("wallpaper.older")}
            disabled={!wallpaper.navigation.canSelectOlder}
            onClick={wallpaper.selectOlder}
          >
            ‹
          </button>
          <button
            type="button"
            className="wallpaper-icon-action"
            aria-label={t("wallpaper.newer")}
            title={t("wallpaper.newer")}
            disabled={!wallpaper.navigation.canSelectNewer}
            onClick={wallpaper.selectNewer}
          >
            ›
          </button>
        </div>
      </div>

      {wallpaper.isLoading && !selected ? (
        <div className="wallpaper-empty-state">
          <span className="wallpaper-loading-orb" aria-hidden="true" />
          <strong>{t("wallpaper.loading")}</strong>
        </div>
      ) : selected ? (
        <div className="wallpaper-content">
          <button
            type="button"
            className="wallpaper-preview-button"
            aria-label={t("wallpaper.applyPreview")}
            title={t("wallpaper.applyPreview")}
            disabled={!previewReady || wallpaper.isApplying || wallpaper.snapshot?.platform.supported === false}
            onClick={() => void wallpaper.apply()}
          >
            {previewReady ? (
              <img src={wallpaper.preview?.dataUrl} alt={title} />
            ) : (
              <span className="wallpaper-preview-placeholder">
                {wallpaper.isPreviewLoading ? t("wallpaper.previewLoading") : t("wallpaper.previewUnavailable")}
              </span>
            )}
            {wallpaper.isApplying ? (
              <span className="wallpaper-preview-progress">{t("wallpaper.applying")}</span>
            ) : null}
          </button>

          <div className="wallpaper-meta">
            <div className="wallpaper-meta-topline">
              <span>{formatBingDate(selected.startDate)}</span>
              {wallpaper.isRefreshing ? <span>{t("wallpaper.refreshing")}</span> : null}
              {wallpaper.snapshot?.stale ? <span>{t("wallpaper.stale")}</span> : null}
            </div>
            <strong>{title}</strong>
            <p>{selected.attribution || t("wallpaper.attributionFallback")}</p>
          </div>
        </div>
      ) : (
        <div className="wallpaper-empty-state">
          <strong>{t("wallpaper.empty")}</strong>
          <button type="button" className="secondary-action" onClick={wallpaper.retry}>
            {t("wallpaper.retry")}
          </button>
        </div>
      )}

      {wallpaper.actionStatus ? (
        <p className="wallpaper-feedback" role="status">
          {wallpaper.actionStatus === "applied"
            ? t("wallpaper.applied")
            : t("wallpaper.saved")}
          {wallpaper.actionResult?.path ? <span>{wallpaper.actionResult.path}</span> : null}
        </p>
      ) : null}

      {wallpaper.error ? (
        <div className="wallpaper-error" role="alert">
          <span>{wallpaper.error}</span>
          <button type="button" onClick={wallpaper.retry}>{t("wallpaper.retry")}</button>
        </div>
      ) : null}

      {wallpaper.snapshot?.platform.supported === false ? (
        <p className="wallpaper-platform-note">
          {t("wallpaper.platformUnsupported")}
        </p>
      ) : null}
    </section>
  );
}

function formatBingDate(value: string) {
  if (!/^\d{8}$/.test(value)) {
    return value;
  }
  return `${value.slice(0, 4)}-${value.slice(4, 6)}-${value.slice(6, 8)}`;
}
