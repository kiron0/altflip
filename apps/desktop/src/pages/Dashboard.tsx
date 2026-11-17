import { useEffect, useState } from "react";
import { AlertTriangle, ArrowRight, Plus, TerminalSquare } from "lucide-react";
import { api, errorText, type ProviderState, type SwitchView } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { AccountRow } from "@/components/AccountRow";
import { ProviderIcon } from "@/components/ProviderIcon";
import { RecoveryBanner } from "@/components/Dialogs";
import { Button } from "@/components/ui/button";
import { Card, Tag } from "@/components/ui/primitives";

function Stat({ label, value, tone }: { label: string; value: string | number; tone?: "warn" }) {
  return (
    <Card className="px-5 py-4">
      <div className="label-mono text-faint">{label}</div>
      <div className={cn("font-display mt-1.5 text-[24px] font-semibold tabular-nums", tone === "warn" && "text-warn")}>{value}</div>
    </Card>
  );
}

function LiveLine({ p }: { p: ProviderState }) {
  const toast = useApp((s) => s.toast);
  const [saving, setSaving] = useState(false);
  const line = p.liveLine;
  if (!line) return null;
  if (line.kind === "error") return <p className="text-[12px] text-danger">{line.lead}</p>;
  if (line.kind === "signed-out") return <p className="text-[12px] text-muted">{line.lead}</p>;
  return (
    <div className="flex items-center gap-2 rounded-xl bg-warn-soft px-3.5 py-2.5 text-[13px]">
      <AlertTriangle className="size-3.5 shrink-0 text-warn" />
      <span className="flex-1">
        {line.lead} <b className="font-medium">{line.who}</b>
        {line.tail}
      </span>
      <Button
        size="sm"
        disabled={saving}
        onClick={async () => {
          setSaving(true);
          try {
            const r = await api.saveCurrent(p.info.id);
            toast({ kind: "success", text: `Saved ${r.label}` });
          } catch (e) {
            toast({ kind: "error", text: "Could not save", detail: errorText(e) });
          } finally {
            setSaving(false);
          }
        }}
      >
        Save account
      </Button>
    </div>
  );
}

function ProviderCard({ p }: { p: ProviderState }) {
  const openAdd = useApp((s) => s.openAdd);
  const navigate = useApp((s) => s.navigate);
  const seeAll = useApp((s) => s.seeAll);
  const featured = p.accounts.find((a) => a.id === p.featuredAccountId);
  return (
    <Card className="flex flex-col">
      <div className="flex items-center gap-3 border-b border-border px-5 py-3.5">
        <ProviderIcon id={p.info.id} size={26} />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-display text-[15px] font-semibold">{p.info.name}</span>
            <Tag badge={p.compatBadge} />
          </div>
          <div className="truncate text-[11.5px] text-muted">{p.subtitle}</div>
        </div>
        <Button size="icon" variant="ghost" aria-label={`Add ${p.info.name} account`} onClick={() => openAdd(p.info.id)}>
          <Plus />
        </Button>
      </div>
      <div className="flex flex-1 flex-col gap-2 p-2">
        <div className="px-2 empty:hidden">
          <LiveLine p={p} />
        </div>
        {p.driftNote && <p className="px-2 text-[12px] text-warn">{p.driftNote}</p>}
        <div>
          {featured && <AccountRow account={featured} provider={p} onOpen={() => navigate("accounts", featured.id)} />}
        </div>
        {!p.switchingEnabled && p.switchingDisabledReason && p.accounts.length > 0 && (
          <p className="px-2 pb-1 text-[11.5px] text-faint">{p.switchingDisabledReason}</p>
        )}
        <button
          type="button"
          onClick={() => (p.seeAll ? seeAll(p.info.id) : openAdd(p.info.id))}
          className="mx-2 mb-1 mt-auto flex items-center justify-center gap-1.5 rounded-xl px-3 py-2 text-[12.5px] font-medium text-muted transition-colors hover:bg-surface-2 hover:text-text"
        >
          {p.seeAll ? (
            <>
              {p.moreLabel} <ArrowRight className="size-3.5" />
            </>
          ) : (
            <>
              <Plus className="size-3.5" /> {p.moreLabel}
            </>
          )}
        </button>
      </div>
    </Card>
  );
}

function EmptyOverview() {
  const overview = useApp((s) => s.overview);
  const openAdd = useApp((s) => s.openAdd);
  const detected = (overview?.providers ?? []).filter((p) => p.info.installed);
  const missing = (overview?.providers ?? []).filter((p) => !p.info.installed);
  return (
    <div className="flex min-h-[calc(100dvh-96px)] flex-col items-center justify-center px-6 text-center">
      <div className="relative mb-7 flex items-center justify-center">
        <div className="absolute size-40 rounded-full bg-accent-soft blur-2xl" aria-hidden />
        <div className="relative flex -space-x-3">
          {(detected.length ? detected : overview?.providers ?? []).map((p, i) => (
            <span
              key={p.info.id}
              className="rounded-[14px] ring-4 ring-[var(--window)]"
              style={{ transform: `translateY(${i % 2 ? 6 : -6}px) rotate(${(i - 1.5) * 6}deg)` }}
            >
              <ProviderIcon id={p.info.id} size={52} />
            </span>
          ))}
        </div>
      </div>
      <h1 className="font-display text-[26px] font-semibold tracking-tight">Add your first account</h1>
      <p className="mt-2 max-w-[440px] text-[13.5px] leading-relaxed text-muted">
        Save each AI coding account once, then switch Claude Code, Codex and Antigravity between them in a click.
        AltFlip uses each tool's own sign-in and never asks for your password.
      </p>
      <Button variant="primary" className="mt-6" onClick={() => openAdd("any")}>
        <Plus /> Add account
      </Button>
      {detected.length > 0 && (
        <div className="mt-8 flex flex-wrap items-center justify-center gap-2">
          <span className="label-mono text-faint">Found on this Mac</span>
          {detected.map((p) => (
            <button
              key={p.info.id}
              type="button"
              onClick={() => openAdd(p.info.id)}
              className="flex items-center gap-1.5 rounded-full border border-border bg-surface px-2.5 py-1 text-[12px] text-muted transition-colors hover:border-border-strong hover:text-text"
            >
              <ProviderIcon id={p.info.id} size={14} />
              {p.info.name}
            </button>
          ))}
        </div>
      )}
      {missing.length > 0 && (
        <p className="mt-3 flex items-center gap-1.5 text-[11.5px] text-faint">
          <TerminalSquare className="size-3.5" />
          Not found: {missing.map((m) => m.info.name).join(", ")}
        </p>
      )}
    </div>
  );
}

export function Dashboard() {
  const overview = useApp((s) => s.overview);
  const navigate = useApp((s) => s.navigate);
  const openAdd = useApp((s) => s.openAdd);
  const [recent, setRecent] = useState<SwitchView[]>([]);

  useEffect(() => {
    void api.history(6).then(setRecent).catch(() => undefined);
  }, [overview?.generatedAt]);

  if (!overview) return null;
  const shown = overview.providers.filter((p) => p.accounts.length > 0);
  const missing = overview.providers.filter((p) => !p.info.installed && p.accounts.length === 0);
  const unsaved = overview.providers.filter((p) => p.info.installed && p.accounts.length === 0);
  const { accounts, attention } = overview.stats;

  if (accounts === 0 && overview.incomplete.length === 0) return <EmptyOverview />;

  return (
    <>
      <PageHeader title="Overview" subtitle="Your AI coding accounts and which one each tool uses." />
      <RecoveryBanner />
      {!overview.credentialStore.available && (
        <div className="rounded-[20px] border border-danger/40 bg-danger-soft px-5 py-4 text-[12.5px]">
          <b className="font-medium">{overview.credentialStore.backend} is unavailable.</b> Switching is off until the system
          credential store can be used. {overview.credentialStore.message}
        </div>
      )}
      <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
        <Stat label="Tools detected" value={overview.stats.toolsText} />
        <Stat label="Saved accounts" value={accounts} />
        <Stat label="Active selections" value={overview.stats.activeSelections} />
        <Stat label="Needs attention" value={attention} tone={attention ? "warn" : undefined} />
      </div>
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        {shown.map((p) => (
          <ProviderCard key={p.info.id} p={p} />
        ))}
      </div>
      {unsaved.length > 0 && (
        <div className="mt-1 flex flex-wrap items-center gap-2 text-[12px] text-faint">
          <span>No accounts yet for</span>
          {unsaved.map((p) => (
            <button
              key={p.info.id}
              type="button"
              onClick={() => openAdd(p.info.id)}
              className="flex items-center gap-1.5 rounded-full border border-border bg-surface px-2.5 py-1 text-muted transition-colors hover:border-border-strong hover:text-text"
            >
              <ProviderIcon id={p.info.id} size={14} />
              {p.info.name}
              <Plus className="size-3" />
            </button>
          ))}
        </div>
      )}
      {missing.length > 0 && (
        <p className="mt-3 flex items-center gap-1.5 text-[12px] text-faint">
          <TerminalSquare className="size-3.5" />
          Not found on this computer: {missing.map((m) => m.info.name).join(", ")}.
        </p>
      )}
      <div className="mt-3 flex items-center">
        <h2 className="label-mono flex-1 text-faint">Recent switches</h2>
        <Button size="sm" variant="ghost" onClick={() => navigate("activity")}>
          All activity <ArrowRight />
        </Button>
      </div>
      <Card className="divide-y divide-border overflow-hidden">
        {recent.length === 0 && <div className="px-4 py-5 text-center text-[12.5px] text-muted">No switches yet.</div>}
        {recent.map((r) => (
          <div key={r.id} className="flex items-center gap-3 px-5 py-3">
            <ProviderIcon id={r.provider} size={16} />
            <span className="min-w-0 flex-1 truncate">
              {r.fromLabel ?? "—"} <ArrowRight className="inline size-3 text-faint" /> <b className="font-medium">{r.toText}</b>
            </span>
            <Tag badge={r.statusBadge} />
            <span className="w-24 text-right text-[11.5px] text-muted">{r.startedText}</span>
          </div>
        ))}
      </Card>
    </>
  );
}
