import type { CSSProperties } from "react";
import type { StatusBarIconId } from "../plugins/pluginHost/contracts";
import { statusBarIconSource } from "./statusBarIconSources";

interface StatusBarGlyphProps {
  icon: StatusBarIconId;
}

export function StatusBarGlyph({ icon }: StatusBarGlyphProps) {
  const style = {
    "--status-bar-icon": `url(${JSON.stringify(statusBarIconSource(icon))})`,
  } as CSSProperties;

  return (
    <span className="status-bar-glyph" aria-hidden="true">
      <span className="status-bar-glyph-mask" style={style} />
    </span>
  );
}
