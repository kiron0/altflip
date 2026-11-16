import { useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { api, errorText, type ProviderState, type UsageRow } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn, countdown, relativeTime } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { Button } from "@/components/ui/button";
import { Badge, Card, Meter } from "@/components/ui/primitives";

const STALE_MS = 6 * 3600 * 1000;

function emptyText(p: ProviderState) {
  switch (p.info.id) {
    case "codex":
      return "No reading yet. Press Refresh.";
    case "claude-code":
      return "No reading yet. Press Refresh.";
    case "antigravity":
      return "No reading yet. Antigravity saves quota for the account signed in to the IDE when it starts.";
    default:
      return "No reading yet.";
  }
}

function Reading({ u, showCountdown }: { u: UsageRow; showCountdown: boolean }) {
  const now = Date.now();
  const resetPassed = !!u.resetAt && Date.parse(u.resetAt) < now;
  const stale = !resetPassed && now - Date.parse(u.observedAt) > STALE_MS;
  const reset = countdown(u.resetAt);
  return (
    <div className={cn(resetPassed && "opacity-60")}>
      <div className="mb-1.5 flex items-baseline gap-2 text-[12.5px]">
        <span className="min-w-0 flex-1 truncate">{u.label}</span>
        {u.usedPercent != null ? (
          <span className="tabular-nums">
            <span className="font-display text-[15px] font-semibold">{Math.round(u.usedPercent)}%</span>
            <span className="text-faint"> used · {Math.max(0, 100 - Math.round(u.usedPercent))}% left</span>
          </span>
        ) : (
          <span className="text-faint">—</span>
        )}
      </div>
      {u.usedPercent != null && <Meter value={u.usedPercent} />}
      <div className="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 font-mono text-[10.5px] text-faint">
        {resetPassed ? (
          <Badge tone="info">Reset since this reading</Badge>
        ) : (
          showCountdown && reset && <span>Resets in {reset}</span>
        )}
        <span>
          {u.source} · {relativeTime(u.observedAt)}
        </span>
        {u.confidence === "estimated" && <Badge tone="neutral">Estimated reset</Badge>}
        {stale && <Badge tone="warn">Stale</Badge>}
      </div>
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
                  <div key={a.id} className="grid grid-cols-[190px_1fr] gap-5 px-5 py-4">
                    <div className="min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="truncate font-medium">{a.label}</span>
                        {a.active && <Badge tone="ok">Active</Badge>}
                      </div>
                      <div className="truncate font-mono text-[10.5px] text-faint">{a.email}</div>
                    </div>
                    {a.usage.length === 0 ? (
                      <p className="text-[13px] text-muted">
                        {a.usageError ?? emptyText(p)}
                      </p>
                    ) : (
                      <div className="grid grid-cols-2 gap-x-6 gap-y-4">
                        {a.usage.map((u) => (
                          <Reading key={u.usageType} u={u} showCountdown={!!settings?.showResetCountdowns} />
                        ))}
                      </div>
                    )}
                  </div>
                ))}
            </div>
          </Card>
        );
      })}
    </>
  );
}
