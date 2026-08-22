import {
  Component,
  lazy,
  Suspense,
  useEffect,
  useRef,
  type ComponentType,
  type ErrorInfo,
  type LazyExoticComponent,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  BundledPluginPanelProps,
} from "../core/pluginHost/pluginModule";
import type { ResolvedLanguage } from "../core/preferences/i18n";
import { bundledPluginPanelLoader } from "./bundledPluginModules";

const panelComponents = new Map<
  string,
  LazyExoticComponent<ComponentType<BundledPluginPanelProps>>
>();
const activatedPluginIds = new Set<string>();

interface LazyPluginPanelProps {
  pluginId: string;
  language: ResolvedLanguage;
  fallback: ReactNode;
  unavailable: ReactNode;
}

export function LazyPluginPanel({
  pluginId,
  language,
  fallback,
  unavailable,
}: LazyPluginPanelProps) {
  const loader = bundledPluginPanelLoader(pluginId);
  if (!loader) return unavailable;

  let Panel = panelComponents.get(pluginId);
  if (!Panel) {
    Panel = lazy(loader);
    panelComponents.set(pluginId, Panel);
  }

  return (
    <PluginImportBoundary key={pluginId} fallback={unavailable}>
      <Suspense fallback={fallback}>
        <Panel language={language} />
        <PluginActivationMarker pluginId={pluginId} />
      </Suspense>
    </PluginImportBoundary>
  );
}

function PluginActivationMarker({ pluginId }: { pluginId: string }) {
  const startedAtMs = useRef(performance.now());

  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      const first = !activatedPluginIds.has(pluginId);
      activatedPluginIds.add(pluginId);
      void invoke("record_plugin_activation", {
        input: {
          pluginId,
          first,
          durationUs: Math.max(0, Math.round((performance.now() - startedAtMs.current) * 1_000)),
        },
      }).catch(() => undefined);
    });
    return () => cancelAnimationFrame(frame);
  }, [pluginId]);

  return null;
}

interface PluginImportBoundaryProps {
  children: ReactNode;
  fallback: ReactNode;
}

interface PluginImportBoundaryState {
  failed: boolean;
}

class PluginImportBoundary extends Component<
  PluginImportBoundaryProps,
  PluginImportBoundaryState
> {
  state: PluginImportBoundaryState = { failed: false };

  static getDerivedStateFromError(): PluginImportBoundaryState {
    return { failed: true };
  }

  componentDidCatch(_error: Error, _info: ErrorInfo) {
    // The compact fallback keeps sibling navigation available after a chunk failure.
  }

  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}
