import { canonicalFirstPartyPluginId } from "../../brand/identity.js";
import type { BundledPluginModule } from "./pluginModule";
import { validatePluginManifest } from "./validation.js";

export interface BundledPluginRegistry {
  modules: readonly BundledPluginModule[];
  manifests: readonly BundledPluginModule["manifest"][];
  find(pluginId: string): BundledPluginModule | undefined;
}

export function createBundledPluginRegistry(
  modules: readonly BundledPluginModule[],
): BundledPluginRegistry {
  const pluginIds = new Set<string>();
  const contributionIds = new Map<string, string>();

  for (const module of modules) {
    const report = validatePluginManifest(module.manifest);
    if (!report.valid) {
      throw new Error(
        `Invalid bundled plugin ${module.manifest.name}: ${report.issues
          .map((issue) => `${issue.path || "manifest"}: ${issue.message}`)
          .join("; ")}`,
      );
    }

    if (!module.kind.trim() || !module.accentClass.trim()) {
      throw new Error(`Bundled plugin ${module.manifest.name} is missing presentation metadata.`);
    }

    const canonicalId = canonicalFirstPartyPluginId(
      module.manifest.id ?? module.manifest.name,
    );
    if (pluginIds.has(canonicalId)) {
      throw new Error(`Duplicate bundled plugin id: ${canonicalId}`);
    }
    pluginIds.add(canonicalId);

    for (const contribution of collectContributionIds(module)) {
      const owner = contributionIds.get(contribution);
      if (owner) {
        throw new Error(
          `Duplicate bundled contribution id ${contribution}: ${owner} and ${canonicalId}`,
        );
      }
      contributionIds.set(contribution, canonicalId);
    }
  }

  const registeredModules = [...modules];
  return {
    modules: registeredModules,
    manifests: registeredModules.map((module) => module.manifest),
    find(pluginId) {
      const canonicalId = canonicalFirstPartyPluginId(pluginId);
      return registeredModules.find(
        (module) =>
          canonicalFirstPartyPluginId(module.manifest.id ?? module.manifest.name) ===
          canonicalId,
      );
    },
  };
}

function collectContributionIds(module: BundledPluginModule) {
  const contributions = module.manifest.contributes;
  return [
    ...(contributions?.views ?? []).map((view) => view.id),
    ...(contributions?.commands ?? []).map((command) => command.id),
    ...(contributions?.statusBarItems ?? []).map((item) => item.id),
  ];
}
