import type { TranslationKey } from "./i18n.js";

export interface SnapMenuItem {
  id: "screenshot";
  labelKey: TranslationKey;
}

export const SNAP_MENU_ITEMS = [
  { id: "screenshot", labelKey: "screenshot.menu.screenshot" },
] as const satisfies readonly SnapMenuItem[];
