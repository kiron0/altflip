import type { Compatibility, SwitchMode } from "@/lib/api";
import { Badge } from "./ui/primitives";

export const compatTone: Record<Compatibility, "ok" | "warn" | "danger" | "neutral"> = {
  supported: "ok",
  experimental: "warn",
  untested: "warn",
  unsupported: "danger",
  "not-installed": "neutral",
};

export const compatText: Record<Compatibility, string> = {
  supported: "Supported",
  experimental: "Experimental",
  untested: "Untested version",
  unsupported: "Switching unavailable",
  "not-installed": "Not installed",
};

export const modeShort: Record<SwitchMode, string> = {
  immediate: "Instant",
  "new-sessions": "Applies to new sessions",
  "restart-required": "Restart required",
  manual: "Manual",
  unsupported: "Switching unavailable",
};

const statusMap: Record<string, { tone: "ok" | "warn" | "danger" | "neutral" | "info"; text: string }> = {
  succeeded: { tone: "ok", text: "Verified" },
  "rolled-back": { tone: "warn", text: "Rolled back" },
  "needs-recovery": { tone: "danger", text: "Needs recovery" },
  cancelled: { tone: "neutral", text: "Cancelled" },
  abandoned: { tone: "neutral", text: "Interrupted" },
  recovered: { tone: "info", text: "Restored" },
  resolved: { tone: "neutral", text: "Resolved" },
  started: { tone: "info", text: "In progress" },
  applying: { tone: "warn", text: "Interrupted" },
};

export function StatusBadge({ status }: { status: string }) {
  const b = statusMap[status] ?? { tone: "neutral" as const, text: status };
  return <Badge tone={b.tone}>{b.text}</Badge>;
}
