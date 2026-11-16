import type { ComponentType } from "react";
import { BellIcon, GaugeIcon, SettingsIcon, ShieldIcon, SwitchingIcon, ToolsIcon, type NavIconProps } from "./NavIcons";

export type SettingsTab = "general" | "switching" | "notifications" | "usage" | "privacy" | "tools";

export const SETTINGS_GROUPS: { id: SettingsTab; label: string; icon: ComponentType<NavIconProps>; subtitle: string }[] = [
  { id: "general", label: "General", icon: SettingsIcon, subtitle: "Appearance, startup and window behavior. Changes save immediately." },
  { id: "switching", label: "Switching", icon: SwitchingIcon, subtitle: "How AltFlip changes the signed-in account of each tool." },
  { id: "notifications", label: "Notifications", icon: BellIcon, subtitle: "Which events AltFlip tells you about." },
  { id: "usage", label: "Usage", icon: GaugeIcon, subtitle: "Live limits from Anthropic, OpenAI and Antigravity for every saved account." },
  { id: "privacy", label: "Privacy & data", icon: ShieldIcon, subtitle: "Where sign-ins are kept, backups and the data folder." },
  { id: "tools", label: "Tools", icon: ToolsIcon, subtitle: "Each supported tool on this computer, its version and how switching works." },
];
