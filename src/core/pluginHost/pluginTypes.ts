export type PluginId = string;

export type PluginHealth =
  | "ready"
  | "active"
  | "disabled"
  | "incompatible"
  | "pending"
  | "error";

export interface PluginMeta {
  id: PluginId;
  title: string;
  subtitle: string;
  health: PluginHealth;
  enabled?: boolean;
  assetUrl?: string;
}
