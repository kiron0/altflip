import { ArrowLeftRight, Bell, Gauge, ShieldCheck, SlidersHorizontal, Wrench, type LucideIcon } from "lucide-react";

export type SettingsTab = "general" | "switching" | "notifications" | "usage" | "privacy" | "tools";

export const SETTINGS_GROUPS: { id: SettingsTab; label: string; icon: LucideIcon; subtitle: string }[] = [
  { id: "general", label: "General", icon: SlidersHorizontal, subtitle: "Appearance, startup and window behavior. Changes save immediately." },
  { id: "switching", label: "Switching", icon: ArrowLeftRight, subtitle: "How AltFlip changes the signed-in account of each tool." },
  { id: "notifications", label: "Notifications", icon: Bell, subtitle: "Which events AltFlip tells you about." },
  { id: "usage", label: "Usage", icon: Gauge, subtitle: "Live limits from Anthropic, OpenAI and Antigravity for every saved account." },
  { id: "privacy", label: "Privacy & data", icon: ShieldCheck, subtitle: "Where sign-ins are kept, backups and the data folder." },
  { id: "tools", label: "Tools", icon: Wrench, subtitle: "Each supported tool on this computer, its version and how switching works." },
];
