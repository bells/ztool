import type { StatusBarIconId } from "../core/pluginHost/contracts";

export const STATUS_BAR_ICON_FILES = {
  zero: "zero.svg",
  launch: "zero-launch.svg",
  "caffeine-empty": "zero-awake.svg",
  "caffeine-full": "zero-awake-active.svg",
  screenshot: "zero-snap.svg",
  paper: "zero-paper.svg",
  extension: "extension.svg",
} as const satisfies Record<StatusBarIconId, string>;

const STATUS_BAR_ICON_SOURCES = {
  zero: new URL("../assets/icons/zero.svg", import.meta.url).href,
  launch: new URL("../assets/icons/zero-launch.svg", import.meta.url).href,
  "caffeine-empty": new URL("../assets/icons/zero-awake.svg", import.meta.url).href,
  "caffeine-full": new URL(
    "../assets/icons/zero-awake-active.svg",
    import.meta.url,
  ).href,
  screenshot: new URL("../assets/icons/zero-snap.svg", import.meta.url).href,
  paper: new URL("../assets/icons/zero-paper.svg", import.meta.url).href,
  extension: new URL("../assets/icons/extension.svg", import.meta.url).href,
} as const satisfies Record<StatusBarIconId, string>;

export function statusBarIconSource(icon: StatusBarIconId): string {
  return STATUS_BAR_ICON_SOURCES[icon];
}
