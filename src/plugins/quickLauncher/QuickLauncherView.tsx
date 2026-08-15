import { useEffect, useMemo, useRef } from "react";
import type { KeyboardEvent } from "react";
import type { TranslationKey } from "./i18n";
import type { QuickLauncherController } from "./useQuickLauncher";

interface QuickLauncherViewProps {
  controller: QuickLauncherController;
  surface: "panel" | "floating";
  focusEpoch?: number;
  t: (key: TranslationKey) => string;
  onDismiss?: () => void;
  onActivationSuccess?: () => void;
}

export function QuickLauncherView({
  controller,
  surface,
  focusEpoch = 0,
  t,
  onDismiss,
  onActivationSuccess,
}: QuickLauncherViewProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const selectedRef = useRef<HTMLButtonElement>(null);
  const support = controller.snapshot?.platformSupport;
  const statusText = useMemo(() => {
    if (support === "unsupported") {
      return t("launcher.unsupported");
    }
    if (controller.error) {
      return localizeLauncherError(controller.error.code, controller.error.message, t);
    }
    if (controller.activatingId) {
      return t("launcher.activating");
    }
    if (controller.lastAction) {
      return activationMessage(controller.lastAction.action, t);
    }
    if (controller.isRefreshing || controller.snapshot?.refreshing) {
      return t("launcher.refreshing");
    }
    if (controller.isLoading) {
      return t("launcher.loading");
    }
    if (support === "degraded") {
      return t("launcher.degraded");
    }
    return t("launcher.ready");
  }, [controller, support, t]);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [focusEpoch]);

  useEffect(() => {
    selectedRef.current?.scrollIntoView({ block: "nearest" });
  }, [controller.selectedId]);

  async function activate(itemId?: string) {
    const result = await controller.activate(itemId);
    if (result) {
      onActivationSuccess?.();
    }
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      controller.moveSelection(event.key === "ArrowDown" ? 1 : -1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      void activate();
      return;
    }
    if (event.key === "Escape" && surface === "floating") {
      event.preventDefault();
      onDismiss?.();
    }
  }

  const showEmpty = !controller.isLoading &&
    support !== "unsupported" &&
    !controller.error &&
    controller.items.length === 0;

  return (
    <section className={`quick-launcher-view ${surface}`} aria-label={t("launcher.title")}>
      <div className="quick-launcher-search-shell">
        <span aria-hidden="true">⌕</span>
        <input
          ref={inputRef}
          type="search"
          value={controller.query}
          placeholder={t("launcher.placeholder")}
          aria-label={t("launcher.searchLabel")}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => controller.setQuery(event.currentTarget.value)}
          onKeyDown={onKeyDown}
        />
        <kbd>{surface === "floating" ? "esc" : "⌘⇧Space"}</kbd>
      </div>

      <div className="quick-launcher-toolbar">
        <span className={support === "degraded" ? "launcher-status warning" : "launcher-status"} aria-live="polite">
          {statusText}
        </span>
        <button
          type="button"
          className="launcher-refresh"
          disabled={controller.isRefreshing || support === "unsupported"}
          onClick={() => void controller.refresh()}
        >
          {t("launcher.refresh")}
        </button>
      </div>

      <div className="quick-launcher-results" role="listbox" aria-label={t("launcher.results")}>
        {controller.items.map((item) => {
          const selected = item.id === controller.selectedId;
          const icon = controller.icons[item.id];
          const action = item.kind === "systemSetting"
            ? t("launcher.open")
            : item.running === "running"
              ? t("launcher.focus")
              : t("launcher.launch");
          return (
            <button
              ref={selected ? selectedRef : undefined}
              type="button"
              role="option"
              aria-selected={selected}
              className={`quick-launcher-row${selected ? " selected" : ""}`}
              key={item.id}
              disabled={controller.activatingId !== null}
              onMouseMove={() => controller.setSelectedId(item.id)}
              onClick={() => void activate(item.id)}
            >
              <span className={`launcher-icon ${item.kind}`} aria-hidden="true">
                {icon ? <img src={icon} alt="" /> : item.kind === "systemSetting" ? "⚙" : "◈"}
              </span>
              <span className="launcher-copy">
                <strong>{item.title}</strong>
                <small>{item.subtitle}</small>
              </span>
              <span className={item.running === "running" ? "launcher-action running" : "launcher-action"}>
                {controller.activatingId === item.id ? t("launcher.activating") : action}
              </span>
            </button>
          );
        })}
        {showEmpty ? (
          <div className="quick-launcher-empty" role="status">
            <strong>{controller.query ? t("launcher.empty") : t("launcher.emptyRecent")}</strong>
            <span>{t("launcher.emptyHint")}</span>
          </div>
        ) : null}
        {support === "unsupported" ? (
          <div className="quick-launcher-empty" role="status">
            <strong>{t("launcher.unsupported")}</strong>
            <span>{t("launcher.unsupportedHint")}</span>
          </div>
        ) : null}
      </div>

      <footer className="quick-launcher-footer">
        <span>↑↓ {t("launcher.navigate")}</span>
        <span>↵ {t("launcher.activate")}</span>
        {surface === "floating" ? <span>esc {t("launcher.close")}</span> : null}
      </footer>
    </section>
  );
}

function activationMessage(
  action: "focused" | "launched" | "launchedFallback" | "openedSetting",
  t: (key: TranslationKey) => string,
) {
  if (action === "focused") return t("launcher.successFocused");
  if (action === "openedSetting") return t("launcher.successSetting");
  return t("launcher.successLaunched");
}

function localizeLauncherError(
  code: string,
  fallback: string,
  t: (key: TranslationKey) => string,
) {
  if (code === "launcher.item_stale") return t("launcher.errorStale");
  if (code === "launcher.platform_unsupported") return t("launcher.unsupported");
  if (code.includes("focus")) return t("launcher.errorFocus");
  if (code.includes("setting") || code.includes("shell_execute")) return t("launcher.errorSetting");
  return fallback || t("launcher.errorGeneric");
}
