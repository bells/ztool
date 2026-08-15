import { CAFFEINE_DURATION_OPTIONS } from "./caffeineDuration";
import { useCaffeinePlugin } from "./useCaffeinePlugin";
import type { TranslationKey } from "./i18n";

interface CaffeinePanelProps {
  t: (key: TranslationKey) => string;
}

export function CaffeinePanel({ t }: CaffeinePanelProps) {
  const caffeine = useCaffeinePlugin();
  const message = caffeine.error
    ? t("caffeine.message.error") + ": " + caffeine.error
    : caffeine.enabled
      ? caffeine.remaining
        ? t("caffeine.message.remaining") + " " + caffeine.remaining
        : t("caffeine.message.activeNoLimit")
      : t("caffeine.message.inactive");
  const dialLabel = caffeine.enabled ? (caffeine.remaining ?? caffeine.elapsed) : "OFF";
  const currentDurationMinutes = caffeine.enabled
    ? caffeine.durationMinutes
    : caffeine.selectedDurationMinutes;
  const currentDurationOption =
    CAFFEINE_DURATION_OPTIONS.find((option) => option.minutes === currentDurationMinutes) ??
    CAFFEINE_DURATION_OPTIONS[0];
  const currentDurationLabel = t(currentDurationOption.labelKey as TranslationKey);

  return (
    <section className="plugin-panel caffeine-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">{t("caffeine.eyebrow")}</span>
          <h2>{t("caffeine.title")}</h2>
        </div>
        <span className={caffeine.enabled ? "status-pill active" : "status-pill"}>
          {caffeine.enabled ? t("caffeine.active") : t("caffeine.inactive")}
        </span>
      </div>

      <div className="caffeine-dial" aria-hidden="true">
        <div className={caffeine.enabled ? "dial-core active" : "dial-core"}>
          <span>{dialLabel}</span>
        </div>
      </div>

      <div className="duration-picker" aria-label={t("caffeine.duration.title")}>
        {CAFFEINE_DURATION_OPTIONS.map((option) => (
          <button
            type="button"
            className={[
              "duration-chip",
              option.minutes === caffeine.selectedDurationMinutes ? "selected" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            key={option.compactLabel}
            title={t(option.labelKey as TranslationKey)}
            onClick={() => caffeine.selectDuration(option.minutes)}
            disabled={caffeine.isBusy}
            aria-pressed={option.minutes === caffeine.selectedDurationMinutes}
          >
            {option.compactLabel}
          </button>
        ))}
      </div>

      <div className="panel-copy">
        <strong>
          {caffeine.enabled ? t("caffeine.activeTitle") : t("caffeine.inactiveTitle")}
        </strong>
        <span>{message}</span>
      </div>

      <div className="caffeine-metrics" aria-label={t("caffeine.metrics")}>
        <span>
          <strong>{t("caffeine.elapsed")}</strong>
          {caffeine.elapsed}
        </span>
        <span>
          <strong>{t("caffeine.duration.current")}</strong>
          {currentDurationLabel}
        </span>
      </div>

      <button
        type="button"
        className={caffeine.enabled ? "primary-action danger" : "primary-action"}
        onClick={caffeine.enabled ? caffeine.disable : caffeine.enable}
        disabled={caffeine.isBusy}
        aria-pressed={caffeine.enabled}
      >
        <span className="button-icon">{caffeine.enabled ? "×" : "✓"}</span>
        {caffeine.enabled ? t("caffeine.disable") : t("caffeine.enable")}
      </button>
    </section>
  );
}
