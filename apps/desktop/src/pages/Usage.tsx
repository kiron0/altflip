import { useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { api, errorText, type ProviderState, type UsageRow } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn, countdown, relativeTime } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { Button } from "@/components/ui/button";
import { Badge, Card, Meter } from "@/components/ui/primitives";

const STALE_MS = 15 * 60 * 1000;

function useNow(every = 15000) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), every);
    return () => clearInterval(t);
  }, [every]);
  return now;
}

function resetText(iso: string | null | undefined, now: number) {
  if (!iso) return null;
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return null;
  if (t - now < 86400000) return `Resets in ${countdown(iso, now)}`;
  const d = new Date(t);
  return `Resets ${d.toLocaleDateString([], { weekday: "short" })} ${d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}`;
}

function emptyText(p: ProviderState) {
  switch (p.info.id) {
    case "codex":
      return "No reading yet. Press Refresh.";
    case "claude-code":
      return "No reading yet. Press Refresh.";
    case "antigravity":
      return "No reading yet. Live quota appears while Antigravity or agy is running with this account.";
    default:
      return "No reading yet.";
  }
}

function Reading({ u, now, showCountdown }: { u: UsageRow; now: number; showCountdown: boolean }) {
  const resetPassed = !!u.resetAt && Date.parse(u.resetAt) <= now;
  const pct = u.usedPercent == null ? null : resetPassed ? 0 : Math.round(u.usedPercent);
  const reset = resetText(u.resetAt, now);
  return (
    <div>
      <div className="mb-1.5 flex items-baseline gap-2 text-[12.5px]">
        <span className="min-w-0 flex-1 truncate">{u.label}</span>
        {pct != null ? (
          <span className="tabular-nums">
            <span className="font-display text-[15px] font-semibold">{pct}%</span>
            <span className="text-faint"> used</span>
          </span>
        ) : (
          <span className="text-faint">—</span>
        )}
      </div>
      {pct != null && <Meter value={pct} />}
      <div className="mt-1.5 text-[11px] text-faint tabular-nums">
        {resetPassed ? "New window started" : showCountdown && reset ? reset : `${Math.max(0, 100 - (pct ?? 0))}% left`}
      </div>
    </div>
  );
}

function Freshness({ usage, now, busy, onRefresh }: { usage: UsageRow[]; now: number; busy: boolean; onRefresh: () => void }) {
  const latest = usage.reduce<UsageRow | null>((m, u) => (!m || Date.parse(u.observedAt) > Date.parse(m.observedAt) ? u : m), null);
  if (!latest) return null;
  const stale = now - Date.parse(latest.observedAt) > STALE_MS;
  return (
    <div className="mt-3 flex items-center gap-1.5 text-[11px] text-faint">
      <span className={cn("size-1.5 rounded-full", stale ? "bg-warn" : "bg-ok", !stale && "animate-pulse")} aria-hidden />
      <span title={`Source: ${latest.source}`}>Updated {relativeTime(latest.observedAt, now)}</span>
      <button
        type="button"
        className="ml-0.5 rounded p-0.5 hover:text-text disabled:opacity-50"
        aria-label="Refresh usage"
        disabled={busy}
        onClick={onRefresh}
      >
        <RefreshCw className={cn("size-3", busy && "animate-spin")} />
      </button>
    </div>
  );
}

export function UsagePage() {
  const overview = useApp((s) => s.overview);
  const settings = useApp((s) => s.settings);
  const toast = useApp((s) => s.toast);
  const load = useApp((s) => s.load);
  const [busy, setBusy] = useState(false);
  const first = useRef(true);
  const now = useNow();
  const lastAuto = useRef(0);

  const refresh = async (quiet = false) => {
    setBusy(true);
    try {
      await api.refreshUsage();
      await load(false);
    } catch (e) {
      if (!quiet) toast({ kind: "error", text: "Could not refresh usage", detail: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (first.current && settings?.usageMonitoring) {
      first.current = false;
      void refresh(true);
    }
  }, [settings?.usageMonitoring]);

  useEffect(() => {
    if (!overview || !settings?.usageMonitoring || busy) return;
    const crossed = overview.providers.some((p) =>
      p.accounts.some((a) => a.usage.some((u) => u.resetAt && Date.parse(u.resetAt) <= now && Date.parse(u.observedAt) < Date.parse(u.resetAt))),
    );
    if (crossed && now - lastAuto.current > 60000) {
      lastAuto.current = now;
      void refresh(true);
    }
  }, [now, overview, settings?.usageMonitoring]);

  if (!overview) return null;
  const providers = overview.providers.filter((p) => p.info.installed || p.accounts.length);

  return (
    <>
      <PageHeader
        title="Usage"
        subtitle="Only usage reported by each provider is shown. Nothing here is estimated."
        actions={
          <Button disabled={busy || !settings?.usageMonitoring} onClick={() => void refresh()}>
            <RefreshCw className={cn(busy && "animate-spin")} /> Refresh
          </Button>
        }
      />
      {settings?.usageMonitoring && !settings.liveUsage && (
        <div className="rounded-[20px] bg-surface-2 px-5 py-4 text-[13px] text-muted">
          Live usage is off, so Claude and Codex show local estimates that can be hours old. Turn on Live usage in Settings for exact numbers.
        </div>
      )}
      {!settings?.usageMonitoring && (
        <div className="rounded-[20px] bg-surface-2 px-5 py-4 text-[13px] text-muted">Usage monitoring is turned off in Settings.</div>
      )}
      {providers.map((p) => {
        return (
          <Card key={p.info.id} className="overflow-hidden">
            <div className="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
              <ProviderIcon id={p.info.id} size={20} />
              <span className="font-display text-[15px] font-semibold">{p.info.name}</span>
            </div>
            <div className="divide-y divide-border">
              {p.accounts.length === 0 && <p className="px-5 py-4 text-[13px] text-muted">No saved accounts.</p>}
              {p.accounts.map((a) => (
                  <div key={a.id} className="grid grid-cols-1 gap-3 px-5 py-4 md:grid-cols-[190px_1fr] md:gap-5">
                    <div className="min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="truncate font-medium">{a.label}</span>
                        {a.active && <Badge tone="ok">Active</Badge>}
                      </div>
                      <div className="truncate font-mono text-[10.5px] text-faint">{a.email}</div>
                    </div>
                    <div className="min-w-0">
                      {a.usageError && (
                        <p className="mb-3 rounded-[12px] bg-surface-2 px-3 py-2 text-[12.5px] text-muted">{a.usageError}</p>
                      )}
                      {a.usage.length === 0 ? (
                        !a.usageError && <p className="text-[13px] text-muted">{emptyText(p)}</p>
                      ) : (
                        <div className="grid grid-cols-1 gap-x-6 gap-y-4 sm:grid-cols-2">
                          {a.usage.map((u) => (
                            <Reading key={u.usageType} u={u} now={now} showCountdown={!!settings?.showResetCountdowns} />
                          ))}
                        </div>
                      )}
                      <Freshness usage={a.usage} now={now} busy={busy} onRefresh={() => void refresh()} />
                    </div>
                  </div>
                ))}
            </div>
          </Card>
        );
      })}
    </>
  );
}
