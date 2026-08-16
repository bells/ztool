import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import type { PluginMeta } from "../pluginHost/pluginTypes";
import type { PluginHostController } from "../pluginHost/usePluginHost";
import type { StatusBarController } from "../../services/useStatusBar";
import { useGlobalShortcuts } from "../../services/useGlobalShortcuts";
import type { TranslationKey } from "./i18n";
import type { usePreferences } from "./usePreferences";
import {
  createPreferencesDestinations,
  createPreferencesSettingIndex,
  filterPreferencesSettings,
  pluginIdFromPreferencesDestination,
  resolvePreferencesDestination,
  shouldClearPreferencesSearch,
  type PreferencesDestination,
  type PreferencesDestinationId,
  type PreferencesNavigationGroup,
} from "./preferencesNavigation";
import {
  createPreferenceOperationGate,
  updatePreferenceFeedback,
  type PreferenceFeedbackMap,
} from "./preferencesFeedback";
import {
  ExtensionsPreferencesPage,
  GeneralPreferencesPage,
  ShortcutsPreferencesPage,
  StatusBarPreferencesPage,
  ToolPreferencesPage,
  ToolsOverviewPage,
} from "./PreferencesPages";

type PreferencesController = ReturnType<typeof usePreferences>;

interface PreferencesSettingsCenterProps {
  localizedPlugins: PluginMeta[];
  pluginHost: PluginHostController;
  preferences: PreferencesController;
  statusBar: StatusBarController;
  t: (key: TranslationKey) => string;
}

const NAVIGATION_GROUP_ORDER: PreferencesNavigationGroup[] = [
  "zero",
  "tools",
  "extensions",
];

export function PreferencesSettingsCenter({
  localizedPlugins,
  pluginHost,
  preferences,
  statusBar,
  t,
}: PreferencesSettingsCenterProps) {
  const shortcutRefreshKey = pluginHost.records
    .map((record) => `${record.name}:${record.enabled}:${record.health}`)
    .join("\u0000");
  const shortcuts = useGlobalShortcuts(shortcutRefreshKey);
  const [query, setQuery] = useState("");
  const [selectedDestination, setSelectedDestination] =
    useState<PreferencesDestinationId>("general");
  const [narrowPageOpen, setNarrowPageOpen] = useState(false);
  const [pendingFocusTarget, setPendingFocusTarget] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<PreferenceFeedbackMap>({});
  const contentHeadingRef = useRef<HTMLHeadingElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const feedbackTimers = useRef<Record<string, number>>({});
  const operationGate = useRef(createPreferenceOperationGate());
  const translateNavigation = useCallback(
    (key: string) => t(key as TranslationKey),
    [t],
  );

  const tools = useMemo(
    () => localizedPlugins.map((plugin) => ({ id: plugin.id, title: plugin.title, subtitle: plugin.subtitle })),
    [localizedPlugins],
  );
  const destinations = useMemo(
    () => createPreferencesDestinations(tools, translateNavigation),
    [tools, translateNavigation],
  );
  const settingIndex = useMemo(
    () => createPreferencesSettingIndex({ destinations, tools, t: translateNavigation }),
    [destinations, tools, translateNavigation],
  );
  const searchResults = useMemo(
    () => filterPreferencesSettings(settingIndex, query),
    [query, settingIndex],
  );
  const resolvedDestinationId = resolvePreferencesDestination(selectedDestination, destinations);
  const destination = destinations.find((candidate) => candidate.id === resolvedDestinationId) ?? destinations[0];

  useEffect(() => {
    if (resolvedDestinationId !== selectedDestination) {
      setSelectedDestination(resolvedDestinationId);
    }
  }, [resolvedDestinationId, selectedDestination]);

  useEffect(() => () => {
    Object.values(feedbackTimers.current).forEach((timer) => window.clearTimeout(timer));
  }, []);

  useEffect(() => {
    const targetId = pendingFocusTarget;
    const frame = window.requestAnimationFrame(() => {
      if (targetId) {
        const target = document.getElementById(targetId);
        target?.scrollIntoView({ block: "center", behavior: "smooth" });
        target?.focus({ preventScroll: true });
        setPendingFocusTarget(null);
      } else if (narrowPageOpen) {
        contentHeadingRef.current?.focus();
      }
    });
    return () => window.cancelAnimationFrame(frame);
  }, [resolvedDestinationId, pendingFocusTarget, narrowPageOpen]);

  const runOperation = useCallback(async (
    operationKey: string,
    operation: () => void | Promise<unknown>,
  ) => {
    if (!operationGate.current.tryStart(operationKey)) {
      return false;
    }
    window.clearTimeout(feedbackTimers.current[operationKey]);
    setFeedback((current) => updatePreferenceFeedback(current, operationKey, "pending"));
    try {
      await operation();
      setFeedback((current) => updatePreferenceFeedback(current, operationKey, "saved"));
      feedbackTimers.current[operationKey] = window.setTimeout(() => {
        setFeedback((current) => updatePreferenceFeedback(current, operationKey, "idle"));
      }, 1800);
      return true;
    } catch (error) {
      setFeedback((current) =>
        updatePreferenceFeedback(
          current,
          operationKey,
          "error",
          error instanceof Error ? error.message : String(error),
        ),
      );
      return false;
    } finally {
      operationGate.current.finish(operationKey);
    }
  }, []);

  const selectDestination = useCallback((
    destinationId: PreferencesDestinationId,
    focusTargetId?: string,
  ) => {
    setSelectedDestination(destinationId);
    setPendingFocusTarget(focusTargetId ?? null);
    setNarrowPageOpen(true);
  }, []);

  if (!destination) {
    return null;
  }

  return (
    <div className={`preferences-center ${narrowPageOpen ? "page-open" : "navigation-open"}`}>
      <PreferencesSidebar
        destinations={destinations}
        query={query}
        searchResults={searchResults}
        selectedDestination={resolvedDestinationId}
        searchInputRef={searchInputRef}
        t={t}
        onQueryChange={setQuery}
        onSelectDestination={selectDestination}
      />

      <section className="preferences-content" aria-labelledby="preferences-content-title">
        <header className="preferences-content-header" data-tauri-drag-region>
          <button
            type="button"
            className="preferences-back-button"
            onClick={() => {
              setNarrowPageOpen(false);
              window.requestAnimationFrame(() => searchInputRef.current?.focus());
            }}
          >
            <span aria-hidden="true">‹</span>
            {t("prefs.back")}
          </button>
          <div>
            <h2 id="preferences-content-title" ref={contentHeadingRef} tabIndex={-1}>
              {destination.title}
            </h2>
            <p>{destination.description}</p>
          </div>
        </header>

        <div className="preferences-content-scroll">
          <PreferencesDestinationContent
            destinationId={resolvedDestinationId}
            feedback={feedback}
            localizedPlugins={localizedPlugins}
            pluginHost={pluginHost}
            preferences={preferences}
            runOperation={runOperation}
            shortcuts={shortcuts}
            statusBar={statusBar}
            t={t}
            onSelectDestination={selectDestination}
          />
        </div>
      </section>
    </div>
  );
}

function PreferencesSidebar({
  destinations,
  query,
  searchInputRef,
  searchResults,
  selectedDestination,
  t,
  onQueryChange,
  onSelectDestination,
}: {
  destinations: PreferencesDestination[];
  query: string;
  searchInputRef: RefObject<HTMLInputElement | null>;
  searchResults: ReturnType<typeof filterPreferencesSettings>;
  selectedDestination: PreferencesDestinationId;
  t: (key: TranslationKey) => string;
  onQueryChange: (query: string) => void;
  onSelectDestination: (destinationId: PreferencesDestinationId, focusTargetId?: string) => void;
}) {
  return (
    <aside className="preferences-sidebar" aria-label={t("prefs.title")}>
      <div className="preferences-sidebar-title" data-tauri-drag-region>
        <strong>{t("prefs.title")}</strong>
      </div>
      <label className="preferences-search">
        <span className="sr-only">{t("prefs.search.label")}</span>
        <span aria-hidden="true" className="preferences-search-icon">⌕</span>
        <input
          ref={searchInputRef}
          type="search"
          value={query}
          placeholder={t("prefs.search.placeholder")}
          onChange={(event) => onQueryChange(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (shouldClearPreferencesSearch(event.key) && query) {
              event.preventDefault();
              onQueryChange("");
            }
          }}
        />
      </label>

      <nav className="preferences-navigation" aria-label={t("prefs.title")}>
        {query ? (
          <div className="preferences-search-results">
            <span className="preferences-navigation-label">{t("prefs.search.results")}</span>
            {searchResults.length > 0 ? searchResults.map((result) => (
              <button
                type="button"
                className="preferences-search-result"
                key={result.id}
                onClick={() => onSelectDestination(result.destinationId, result.focusTargetId)}
              >
                <strong>{result.title}</strong>
                <small>{result.path}</small>
              </button>
            )) : (
              <div className="preferences-search-empty">
                <strong>{t("prefs.search.noResults")}</strong>
                <small>{t("prefs.search.noResultsHint")}</small>
              </div>
            )}
          </div>
        ) : (
          NAVIGATION_GROUP_ORDER.map((group) => {
            const groupDestinations = destinations.filter((candidate) => candidate.group === group);
            return (
              <div className="preferences-navigation-group" key={group}>
                <span className="preferences-navigation-label">
                  {group === "zero" ? t("prefs.nav.zero") : group === "tools" ? t("prefs.nav.tools") : t("prefs.nav.extensions")}
                </span>
                {groupDestinations.map((candidate) => (
                  <button
                    type="button"
                    className={candidate.id === selectedDestination ? "selected" : ""}
                    aria-current={candidate.id === selectedDestination ? "page" : undefined}
                    key={candidate.id}
                    onClick={() => onSelectDestination(candidate.id)}
                  >
                    <span>{candidate.title}</span>
                  </button>
                ))}
              </div>
            );
          })
        )}
      </nav>
    </aside>
  );
}

function PreferencesDestinationContent({
  destinationId,
  feedback,
  localizedPlugins,
  pluginHost,
  preferences,
  runOperation,
  shortcuts,
  statusBar,
  t,
  onSelectDestination,
}: {
  destinationId: PreferencesDestinationId;
  feedback: PreferenceFeedbackMap;
  localizedPlugins: PluginMeta[];
  pluginHost: PluginHostController;
  preferences: PreferencesController;
  runOperation: (operationKey: string, operation: () => void | Promise<unknown>) => Promise<boolean>;
  shortcuts: ReturnType<typeof useGlobalShortcuts>;
  statusBar: StatusBarController;
  t: (key: TranslationKey) => string;
  onSelectDestination: (destinationId: PreferencesDestinationId) => void;
}) {
  if (destinationId === "general") {
    return (
      <GeneralPreferencesPage
        feedback={feedback}
        isAutostartBusy={preferences.isAutostartBusy}
        preferences={preferences.preferences}
        runOperation={runOperation}
        t={t}
        onLanguageChange={preferences.setLanguage}
        onLaunchAtLoginChange={preferences.setLaunchAtLogin}
      />
    );
  }
  if (destinationId === "status-bar") {
    return <StatusBarPreferencesPage feedback={feedback} plugins={localizedPlugins} records={pluginHost.records} runOperation={runOperation} statusBar={statusBar} t={t} />;
  }
  if (destinationId === "shortcuts") {
    return <ShortcutsPreferencesPage error={shortcuts.error} isLoading={shortcuts.isLoading} shortcuts={shortcuts.snapshots} t={t} />;
  }
  if (destinationId === "tools") {
    return (
      <ToolsOverviewPage
        plugins={localizedPlugins}
        preferences={preferences.preferences}
        records={pluginHost.records}
        statusBar={statusBar}
        t={t}
        onSelectDestination={onSelectDestination}
      />
    );
  }
  if (destinationId === "extensions") {
    return <ExtensionsPreferencesPage pluginHost={pluginHost} t={t} />;
  }

  const pluginId = pluginIdFromPreferencesDestination(destinationId);
  const record = pluginHost.records.find((candidate) => candidate.name === pluginId);
  const plugin = localizedPlugins.find((candidate) => candidate.id === pluginId);
  if (!record || !plugin) return null;
  return (
    <ToolPreferencesPage
      feedback={feedback}
      plugin={plugin}
      pluginHost={pluginHost}
      preferences={preferences.preferences}
      record={record}
      runOperation={runOperation}
      shortcut={shortcuts.snapshots.find((candidate) => candidate.pluginName === record.name)}
      statusBar={statusBar}
      t={t}
      onToolVisibleChange={preferences.setToolVisible}
    />
  );
}
