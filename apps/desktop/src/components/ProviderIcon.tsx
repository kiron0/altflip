import type { ProviderId } from "@/lib/api";
import { cn } from "@/lib/utils";

export function ProviderIcon({ id, size = 20, className }: { id: ProviderId; size?: number; className?: string }) {
  const box = cn("inline-flex shrink-0 items-center justify-center rounded-[6px]", className);
  if (id === "claude-code") {
    return (
      <span className={box} style={{ width: size, height: size, background: "#d97757" }} aria-hidden>
        <svg viewBox="0 0 24 24" width={size * 0.66} height={size * 0.66} fill="none" stroke="#fff" strokeWidth={2.4} strokeLinecap="round">
          <path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" />
        </svg>
      </span>
    );
  }
  if (id === "codex") {
    return (
      <span className={cn(box, "dark:ring-1 dark:ring-white/15")} style={{ width: size, height: size, background: "#111114" }} aria-hidden>
        <svg viewBox="0 0 24 24" width={size * 0.62} height={size * 0.62} fill="none" stroke="#fff" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
          <path d="m7 8 4 4-4 4M13 16h5" />
        </svg>
      </span>
    );
  }
  return (
    <span className={box} style={{ width: size, height: size, background: "linear-gradient(135deg,#4f8bff,#7c4dff)" }} aria-hidden>
      <svg viewBox="0 0 24 24" width={size * 0.64} height={size * 0.64} fill="none" stroke="#fff" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 3 4.5 20h3l4.5-10.5L16.5 20h3z" />
      </svg>
    </span>
  );
}
