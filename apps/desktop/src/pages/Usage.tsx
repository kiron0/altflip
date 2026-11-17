import { useEffect, useRef, useState } from "react";
import { Info, RefreshCw } from "lucide-react";
import { api, errorText, type AccountView, type UsageView } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { Button } from "@/components/ui/button";
import { Badge, Card, Meter } from "@/components/ui/primitives";

function Reading({ u, showCountdown }: { u: UsageView; showCountdown: boolean }) {
  return (
    <div>
      <div className="mb-1.5 flex items-baseline gap-2 text-[12.5px]">
        <span className="min-w-0 flex-1 truncate">{u.label}</span>
        {u.percent != null ? (
          <span className="tabular-nums">
            <span className="font-display text-[15px] font-semibold">{u.percent}%</span>
            <span className="text-faint"> used</span>
          </span>
        ) : (
          <span className="text-faint">—</span>
        )}
      </div>
      {u.percent != null && <Meter value={u.percent} />}
      <div className="mt-1.5 text-[11px] text-faint tabular-nums">{showCountdown && u.resetText ? u.resetText : u.leftText}</div>
    </div>
  );
}

function Freshness({ account, busy, onRefresh }: { account: AccountView; busy: boolean; onRefresh: () => void }) {
  const f = account.freshness;
  if (!f) return null;
  return (
    <div className="mt-3 flex items-center gap-1.5 text-[11px] text-faint">
      <span className={cn("size-1.5 rounded-full", f.stale ? "bg-warn" : "bg-ok", !f.stale && "animate-pulse")} aria-hidden />
      <span title={`Source: ${f.source}`}>{f.text}</span>
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
  const lastAuto = useRef(0);

  const refresh = async (quiet = false) => {
    setBusy(true);
    try {
      const notice = await api.refreshUsage();
      await load(false);
      if (!quiet && notice) toast({ kind: "info", text: notice.text, detail: notice.detail });
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

  const rolled = !!overview?.stats.usageWindowRolled;
  useEffect(() => {
    if (!rolled || !settings?.usageMonitoring || busy || Date.now() - lastAuto.current < 60000) return;
    lastAuto.current = Date.now();
    void refresh(true);
  }, [rolled, overview?.generatedAt, settings?.usageMonitoring]);

  if (!overview) return null;
  const providers = overview.providers.filter((p) => p.info.installed || p.accounts.length);
  const showCountdown = !!settings?.showResetCountdowns;

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
      {providers.map((p) => (
        <Card key={p.info.id} className="overflow-hidden">
          <div className="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
            <ProviderIcon id={p.info.id} size={20} />
            <span className="font-display text-[15px] font-semibold">{p.info.name}</span>
          </div>
          {!p.usageSupported ? (
            <div className="flex items-start gap-2.5 px-5 py-4 text-[13px] text-muted">
              <Info className="mt-0.5 size-3.5 shrink-0 text-faint" />
              <span>{p.usageNote}</span>
            </div>
          ) : (
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
                    {a.usageError && <p className="mb-3 rounded-[12px] bg-surface-2 px-3 py-2 text-[12.5px] text-muted">{a.usageError}</p>}
                    {a.usage.length === 0 ? (
                      !a.usageError && <p className="text-[13px] text-muted">{p.usageNote}</p>
                    ) : (
                      <div className="grid grid-cols-1 gap-x-6 gap-y-4 sm:grid-cols-2">
                        {a.usage.map((u) => (
                          <Reading key={u.usageType} u={u} showCountdown={showCountdown} />
                        ))}
                      </div>
                    )}
                    <Freshness account={a} busy={busy} onRefresh={() => void refresh()} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      ))}
    </>
  );
}
