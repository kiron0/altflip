import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { AccountView, AuthHealth, ProviderState } from "./api";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function relativeTime(iso: string | null | undefined, now = Date.now()): string {
  if (!iso) return "never";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "unknown";
  const s = Math.round((now - t) / 1000);
  const abs = Math.abs(s);
  const fmt = (n: number, unit: string) => `${n} ${unit}${n === 1 ? "" : "s"}`;
  let out: string;
  if (abs < 45) return s >= 0 ? "just now" : "in a moment";
  if (abs < 3600) out = fmt(Math.round(abs / 60), "min");
  else if (abs < 86400) out = fmt(Math.round(abs / 3600), "hour");
  else out = fmt(Math.round(abs / 86400), "day");
  return s >= 0 ? `${out} ago` : `in ${out}`;
}

export function countdown(iso: string | null | undefined, now = Date.now()): string | null {
  if (!iso) return null;
  const ms = Date.parse(iso) - now;
  if (Number.isNaN(ms)) return null;
  if (ms <= 0) return "now";
  const m = Math.floor(ms / 60000);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mm = m % 60;
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${mm}m`;
  return `${mm}m`;
}

export function clockTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function dayLabel(iso: string): string {
  const d = new Date(iso);
  const today = new Date();
  const y = new Date();
  y.setDate(today.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return "Today";
  if (d.toDateString() === y.toDateString()) return "Yesterday";
  return d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
}

export function planLabel(plan: string | null | undefined): string | null {
  if (!plan) return null;
  const map: Record<string, string> = {
    pro: "Pro",
    max: "Max",
    team: "Team",
    enterprise: "Enterprise",
    plus: "Plus",
    free: "Free",
    api: "API",
    stripe_subscription: "Subscription",
  };
  return map[plan.toLowerCase()] ?? plan.replace(/[_-]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export type RowState =
  | "active"
  | "new-sessions"
  | "drifted"
  | "signed-out"
  | "external"
  | "expired"
  | "error"
  | "idle";

export function rowState(a: AccountView): RowState {
  if (a.active) {
    switch (a.activationState) {
      case "new-sessions":
        return "new-sessions";
      case "drifted":
        return "drifted";
      case "signed-out":
        return "signed-out";
      case "external":
        return "external";
      default:
        return "active";
    }
  }
  if (a.health === "expired" || a.health === "needs-reauth") return "expired";
  if (a.lastError) return "error";
  return "idle";
}

export const healthText: Record<AuthHealth, string> = {
  ok: "Signed in",
  expiring: "Expiring soon",
  expired: "Sign-in expired",
  "needs-reauth": "Needs sign-in",
  unknown: "Unknown",
};

export function activeAccount(p: ProviderState): AccountView | undefined {
  return p.accounts.find((a) => a.id === p.activeAccountId);
}

export function plural(n: number, word: string) {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

export function initials(label: string) {
  const parts = label.trim().split(/[\s@._-]+/).filter(Boolean);
  return ((parts[0]?.[0] ?? "?") + (parts[1]?.[0] ?? "")).toUpperCase();
}
