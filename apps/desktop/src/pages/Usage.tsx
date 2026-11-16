import { useEffect, useRef, useState } from "react";
import { Info, RefreshCw } from "lucide-react";
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
      return "No reading yet. Codex records usage in its session logs while this account is active.";
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
        <span className="font-display text-[15px] font-semibold tabular-nums">
          {u.usedPercent != null ? `${Math.round(u.usedPercent)}%` : "—"}
        </span>
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
        {stale && <Badge tone="warn">Stale</Badge>}
      </div>
    </div>
  );
}

function ClaudeOptIn() {
  const setSetting = useApp((s) => s.setSetting);
  const load = useApp((s) => s.load);
  const toast = useApp((s) => s.toast);
  const [busy, setBusy] = useState(false);
  return (
    <div className="flex items-start gap-3 px-5 py-4">
      <Info className="mt-0.5 size-4 shrink-0 text-accent" />
      <div className="min-w-0 flex-1 text-[13px]">
        <div className="font-medium">Show Claude usage from Anthropic</div>
        <p className="mt-0.5 text-muted">
          Claude Code doesn't save its plan usage on this computer. With your permission, AltFlip sends each saved Claude login to
          api.anthropic.com to read the same 5-hour and weekly limits as /usage. Expired logins are renewed the way Claude Code
          renews them, and the active one is saved back for Claude Code. Logins are sent only to Anthropic.
        </p>
      </div>
      <Button
        size="sm"
        variant="primary"
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          await setSetting("claudeExactUsage", true);
          try {
            await api.refreshUsage();
            await load(false);
          } catch (e) {
            toast({ kind: "error", text: "Could not read Claude usage", detail: errorText(e) });
          }
          setBusy(false);
        }}
      >
        Turn on
      </Button>
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
      {!settings?.usageMonitoring && (
        <div className="rounded-[20px] bg-surface-2 px-5 py-4 text-[13px] text-muted">Usage monitoring is turned off in Settings.</div>
      )}
      {providers.map((p) => {
        const claudeOff = p.info.id === "claude-code" && !settings?.claudeExactUsage;
        return (
          <Card key={p.info.id} className="overflow-hidden">
            <div className="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
              <ProviderIcon id={p.info.id} size={20} />
              <span className="font-display text-[15px] font-semibold">{p.info.name}</span>
            </div>
            {claudeOff && <ClaudeOptIn />}
            <div className="divide-y divide-border">
              {!claudeOff && p.accounts.length === 0 && <p className="px-5 py-4 text-[13px] text-muted">No saved accounts.</p>}
              {!claudeOff &&
                p.accounts.map((a) => (
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
                        {a.usageError ?? (p.info.id === "claude-code" ? "No reading yet. Press Refresh." : emptyText(p))}
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
