export const PRODUCT_NAME = "Zero";

export const FIRST_PARTY_PLUGIN_IDS = {
  launch: "zero.launch",
  snap: "zero.snap",
  awake: "zero.awake",
  paper: "zero.paper",
  file: "zero.file",
} as const;

export type FirstPartyPluginId =
  (typeof FIRST_PARTY_PLUGIN_IDS)[keyof typeof FIRST_PARTY_PLUGIN_IDS];

const LEGACY_FIRST_PARTY_PLUGIN_IDS: Readonly<Record<string, FirstPartyPluginId>> = {
  "ztool.quick-launcher": FIRST_PARTY_PLUGIN_IDS.launch,
  "ztool.screenshot": FIRST_PARTY_PLUGIN_IDS.snap,
  "ztool.caffeine": FIRST_PARTY_PLUGIN_IDS.awake,
  "ztool.bing-wallpaper": FIRST_PARTY_PLUGIN_IDS.paper,
};

export function canonicalFirstPartyPluginId(pluginId: string): string {
  return LEGACY_FIRST_PARTY_PLUGIN_IDS[pluginId] ?? pluginId;
}

export function canonicalFirstPartyContributionId(contributionId: string): string {
  for (const [legacyId, canonicalId] of Object.entries(
    LEGACY_FIRST_PARTY_PLUGIN_IDS,
  )) {
    if (contributionId === legacyId) {
      return canonicalId;
    }

    if (contributionId.startsWith(`${legacyId}.`)) {
      return `${canonicalId}${contributionId.slice(legacyId.length)}`;
    }
  }

  return contributionId;
}

export function legacyFirstPartyPluginIds(
  canonicalId: string,
): readonly string[] {
  return Object.entries(LEGACY_FIRST_PARTY_PLUGIN_IDS)
    .filter(([, target]) => target === canonicalId)
    .map(([legacyId]) => legacyId);
}
