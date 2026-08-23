export type AppSurface = "tray" | "main" | "preferences" | "about" | "capture" | "pin" | "launcher" | "paper" | "snap-menu";

export function resolveAppSurface(label: string): AppSurface {
  if (label === "main" || label === "preferences" || label === "about" || label === "capture" || label === "launcher" || label === "paper" || label === "snap-menu") {
    return label;
  }

  if (label.startsWith("pin-")) {
    return "pin";
  }

  return "tray";
}
