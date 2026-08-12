export type AppSurface = "tray" | "main" | "preferences" | "about" | "capture" | "pin" | "launcher" | "paper";

export function resolveAppSurface(label: string): AppSurface {
  if (label === "main" || label === "preferences" || label === "about" || label === "capture" || label === "launcher" || label === "paper") {
    return label;
  }

  if (label.startsWith("pin-")) {
    return "pin";
  }

  return "tray";
}
