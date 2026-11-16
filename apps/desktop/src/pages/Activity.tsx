import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  ChevronDown,
  Copy,
  KeyRound,
  RefreshCw,
  Search,
  Settings2,
  ShieldAlert,
  Undo2,
  UserMinus,
  UserPen,
  UserPlus,
  X,
  Zap,
} from "lucide-react";
import { api, errorText, type ActivityRow, type ProviderId, type SwitchMode, type SwitchRecord } from "@/lib/api";
import { useApp } from "@/lib/store";
import { clockTime, cn, dayLabel, relativeTime } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { RecoveryBanner } from "@/components/Dialogs";
import { StatusBadge, modeShort } from "@/components/labels";
import { Button } from "@/components/ui/button";
import { Badge, Card, Dialog, Input, Segmented, Select } from "@/components/ui/primitives";

type Category = "switch" | "account" | "problem" | "system";
type Range = "1" | "7" | "30" | "all";

type Item = {
  key: string;
  at: string;
  provider: ProviderId | null;
  category: Category;
  kind: string;
  title: string;
  text: string;
  switch?: SwitchRecord;
  event?: ActivityRow;
};

const DUPLICATE_KINDS = new Set(["switched", "rolled-back"]);

const KIND: Record<string, { label: string; category: Category; icon: React.ReactNode }> = {
  added: { label: "Account added", category: "account", icon: <UserPlus /> },
  updated: { label: "Account updated", category: "account", icon: <UserPen /> },
  renamed: { label: "Account renamed", category: "account", icon: <UserPen /> },
  removed: { label: "Account removed", category: "account", icon: <UserMinus /> },
  renewed: { label: "Sign-in renewed", category: "account", icon: <KeyRound /> },
  external: { label: "Switched outside AltFlip", category: "switch", icon: <Zap /> },
  drift: { label: "Unsaved sign-in detected", category: "problem", icon: <ShieldAlert /> },
  interrupted: { label: "Switch interrupted", category: "problem", icon: <AlertTriangle /> },
  "needs-recovery": { label: "Needs recovery", category: "problem", icon: <AlertTriangle /> },
  usage: { label: "Usage unavailable", category: "problem", icon: <AlertTriangle /> },
  recovered: { label: "Previous sign-in restored", category: "system", icon: <Undo2 /> },
  settings: { label: "Settings changed", category: "system", icon: <Settings2 /> },
};

const CATEGORY_TONE: Record<Category, string> = {
  switch: "bg-accent-soft text-accent",
  account: "bg-ok-soft text-ok",
  problem: "bg-warn-soft text-warn",
  system: "bg-surface-3 text-muted",
};

function duration(r: SwitchRecord) {
  if (!r.completedAt) return null;
  const ms = Date.parse(r.completedAt) - Date.parse(r.startedAt);
  if (!Number.isFinite(ms) || ms < 0) return null;
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(ms < 10000 ? 1 : 0)} s`;
}

function fullTime(iso: string) {
  return new Date(iso).toLocaleString([], { dateStyle: "medium", timeStyle: "medium" });
}

function toItems(history: SwitchRecord[], log: ActivityRow[]): Item[] {
  const out: Item[] = history.map((r) => ({
    key: `s-${r.id}`,
    at: r.startedAt,
    provider: r.provider,
    category: r.status === "succeeded" || r.status === "recovered" ? "switch" : "problem",
    kind: r.status,
    title: `${r.fromLabel ?? "No account"} → ${r.toLabel ?? "—"}`,
    text: r.status === "succeeded" ? (r.message ?? "Switched and verified") : (r.message ?? r.status),
    switch: r,
  }));
  for (const e of log) {
    if (DUPLICATE_KINDS.has(e.kind)) continue;
    const k = KIND[e.kind];
    out.push({
      key: `e-${e.id}`,
      at: e.at,
      provider: e.provider,
      category: k?.category ?? "system",
      kind: e.kind,
      title: k?.label ?? e.kind,
      text: e.message,
      event: e,
    });
  }
  return out.sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <Card className="px-4 py-3.5">
      <div className="label-mono text-faint">{label}</div>
      <div className="font-display mt-1 text-[20px] font-semibold tabular-nums">{value}</div>
      {hint && <div className="mt-0.5 truncate text-[11.5px] text-muted">{hint}</div>}
    </Card>
  );
}

function Detail({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="min-w-0">
      <dt className="label-mono text-faint">{label}</dt>
      <dd className="selectable mt-0.5 break-words text-[12.5px]">{children}</dd>
    </div>
  );
}

function ProviderName({ id }: { id: ProviderId }) {
  const name = useApp((s) => s.overview?.providers.find((p) => p.info.id === id)?.info.name);
  return <>{name ?? id}</>;
}

function ItemRow({
  item,
  open,
  onToggle,
  canUndo,
  onUndo,
  accountName,
}: {
  item: Item;
  open: boolean;
  onToggle: () => void;
  canUndo: boolean;
  onUndo: () => void;
  accountName: (id: string | null) => string | null;
}) {
  const toast = useApp((s) => s.toast);
  const r = item.switch;
  const kind = KIND[item.kind];
  const copy = (text: string) => {
    void navigator.clipboard.writeText(text).then(() => toast({ kind: "success", text: "Copied" }));
  };
  return (
    <div className={cn("transition-colors", open && "bg-surface-2/60")}>
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={open}
        className="flex w-full items-start gap-3 px-4 py-3 text-left hover:bg-surface-2/60 sm:items-center"
      >
        <span className="hidden w-[60px] shrink-0 whitespace-nowrap text-[11.5px] text-muted tabular-nums sm:block">{clockTime(item.at)}</span>
        <span className="relative mt-0.5 shrink-0 sm:mt-0">
          {item.provider ? (
            <ProviderIcon id={item.provider} size={22} />
          ) : (
            <span className="flex size-[22px] items-center justify-center rounded-[6px] bg-surface-3 text-muted">
              <Settings2 className="size-3" />
            </span>
          )}
          {!r && kind && (
            <span
              className={cn(
                "absolute -bottom-1 -right-1 flex size-3.5 items-center justify-center rounded-full ring-2 ring-[var(--card)] [&_svg]:size-2",
                CATEGORY_TONE[item.category],
              )}
            >
              {kind.icon}
            </span>
          )}
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
            {r ? (
              <span className="min-w-0 truncate text-[13px]">
                <span className="text-muted">{r.fromLabel ?? "No account"}</span>
                <ArrowRight className="mx-1 inline size-3 text-faint" />
                <b className="font-medium">{r.toLabel ?? "—"}</b>
              </span>
            ) : (
              <span className="text-[13px] font-medium">{item.title}</span>
            )}
            {r && <StatusBadge status={r.status} />}
          </span>
          <span className="mt-0.5 block truncate text-[11.5px] text-muted">
            <span className="tabular-nums sm:hidden">{clockTime(item.at)} · </span>
            {item.text}
          </span>
        </span>
        <span className="hidden shrink-0 text-[11.5px] text-faint sm:block">{r ? duration(r) : null}</span>
        <ChevronDown className={cn("mt-0.5 size-4 shrink-0 text-faint transition-transform sm:mt-0", open && "rotate-180")} />
      </button>
      {open && (
        <div className="px-3 pb-4 sm:px-4 sm:pl-[109px]">
          <dl className="grid grid-cols-1 gap-x-6 gap-y-3 rounded-2xl border border-border bg-card p-4 sm:grid-cols-2 lg:grid-cols-3">
            {r ? (
              <>
                <Detail label="From">{r.fromLabel ?? "No saved account"}</Detail>
                <Detail label="To">{r.toLabel ?? "—"}</Detail>
                <Detail label="Result">
                  <StatusBadge status={r.status} />
                </Detail>
                <Detail label="How it applied">{modeShort[r.switchMode as SwitchMode] ?? r.switchMode}</Detail>
                <Detail label="Started">{fullTime(r.startedAt)}</Detail>
                <Detail label="Took">{duration(r) ?? (r.completedAt ? "—" : "Did not finish")}</Detail>
                <div className="sm:col-span-2 lg:col-span-3">
                  <Detail label={r.status === "succeeded" ? "Verification" : "What happened"}>
                    {r.message ?? "No details were recorded."}
                    {r.errorCode && <span className="ml-1.5 font-mono text-[11px] text-faint">({r.errorCode})</span>}
                  </Detail>
                </div>
                <div className="flex flex-wrap items-center gap-2 sm:col-span-2 lg:col-span-3">
                  <button
                    type="button"
                    onClick={() => copy(r.id)}
                    className="flex min-w-0 items-center gap-1.5 rounded-lg px-2 py-1 font-mono text-[11px] text-faint hover:bg-surface-2 hover:text-text"
                  >
                    <Copy className="size-3 shrink-0" /> <span className="truncate">{r.id}</span>
                  </button>
                  <span className="flex-1" />
                  {canUndo && (
                    <Button size="sm" onClick={onUndo}>
                      <Undo2 /> Undo this switch
                    </Button>
                  )}
                </div>
              </>
            ) : (
              <>
                <Detail label="Event">{item.title}</Detail>
                <Detail label="Tool">{item.provider ? <ProviderName id={item.provider} /> : "AltFlip"}</Detail>
                <Detail label="When">{fullTime(item.at)}</Detail>
                {item.event?.accountId && <Detail label="Account">{accountName(item.event.accountId) ?? "Removed account"}</Detail>}
                <div className="sm:col-span-2 lg:col-span-3">
                  <Detail label="Details">{item.text}</Detail>
                </div>
              </>
            )}
          </dl>
        </div>
      )}
    </div>
  );
}

export function ActivityPage() {
  const overview = useApp((s) => s.overview);
  const toast = useApp((s) => s.toast);
  const [history, setHistory] = useState<SwitchRecord[]>([]);
  const [log, setLog] = useState<ActivityRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [undo, setUndo] = useState<SwitchRecord | null>(null);
  const [openKey, setOpenKey] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState<ProviderId | "all">("all");
  const [category, setCategory] = useState<Category | "all">("all");
  const [range, setRange] = useState<Range>("30");

  const reload = () =>
    Promise.all([api.history(500), api.activity(1000)])
      .then(([h, l]) => {
        setHistory(h);
        setLog(l);
      })
      .catch(() => undefined)
      .finally(() => setLoading(false));

  useEffect(() => {
    void reload();
  }, [overview?.generatedAt]);

  const undoable = useMemo(() => {
    const seen = new Set<string>();
    const ids = new Set<string>();
    for (const r of history) {
      if (seen.has(r.provider)) continue;
      if (r.status === "succeeded") ids.add(r.id);
      if (r.status === "succeeded" || r.status === "recovered") seen.add(r.provider);
    }
    return ids;
  }, [history]);

  const items = useMemo(() => toItems(history, log), [history, log]);
  const providers = useMemo(() => {
    const ids = new Set(items.map((i) => i.provider).filter(Boolean) as ProviderId[]);
    return (overview?.providers ?? []).filter((p) => ids.has(p.info.id));
  }, [items, overview]);

  const accountName = (id: string | null) => {
    if (!id) return null;
    for (const p of overview?.providers ?? []) {
      const a = p.accounts.find((x) => x.id === id);
      if (a) return a.label;
    }
    return null;
  };

  const since = range === "all" ? 0 : Date.now() - Number(range) * 86400000;
  const q = query.trim().toLowerCase();
  const inScope = (i: Item) => Date.parse(i.at) >= since && (provider === "all" || i.provider === provider);
  const filtered = items.filter(
    (i) =>
      inScope(i) &&
      (category === "all" || i.category === category) &&
      (!q || [i.title, i.text, i.switch?.fromLabel, i.switch?.toLabel].some((v) => v?.toLowerCase().includes(q))),
  );
  const count = (c: Category | "all") => items.filter((i) => inScope(i) && (c === "all" || i.category === c)).length;

  const grouped: { day: string; rows: Item[] }[] = [];
  for (const r of filtered) {
    const d = dayLabel(r.at);
    if (grouped.at(-1)?.day !== d) grouped.push({ day: d, rows: [] });
    grouped.at(-1)!.rows.push(r);
  }

  const switches = filtered.filter((i) => i.switch).map((i) => i.switch!);
  const ok = switches.filter((s) => s.status === "succeeded").length;
  const durations = switches
    .map((s) => (s.completedAt ? Date.parse(s.completedAt) - Date.parse(s.startedAt) : NaN))
    .filter((n) => Number.isFinite(n) && n >= 0);
  const avg = durations.length ? durations.reduce((a, b) => a + b, 0) / durations.length : null;
  const problems = filtered.filter((i) => i.category === "problem").length;
  const last = switches[0];
  const filtering = q !== "" || provider !== "all" || category !== "all" || range !== "30";
  const rangeText = range === "all" ? "All time" : range === "1" ? "Last 24 hours" : `Last ${range} days`;

  const clear = () => {
    setQuery("");
    setProvider("all");
    setCategory("all");
    setRange("30");
  };

  return (
    <>
      <PageHeader
        title="Activity"
        subtitle="Every switch and account event on this device, with what happened and how long it took. No tokens or passwords are recorded."
        actions={
          <Button onClick={() => void reload()}>
            <RefreshCw /> Reload
          </Button>
        }
      />
      <RecoveryBanner />

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat label="Switches" value={String(switches.length)} hint={rangeText} />
        <Stat
          label="Succeeded"
          value={switches.length ? `${Math.round((ok / switches.length) * 100)}%` : "—"}
          hint={`${ok} of ${switches.length} verified`}
        />
        <Stat
          label="Average time"
          value={avg == null ? "—" : avg < 1000 ? `${Math.round(avg)} ms` : `${(avg / 1000).toFixed(1)} s`}
          hint="Start to verified"
        />
        <Stat label="Needs a look" value={String(problems)} hint={last ? `Last switch ${relativeTime(last.startedAt)}` : "No switches yet"} />
      </div>

      <Card className="flex flex-col gap-3 p-3">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <div className="relative min-w-0 flex-1">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-faint" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search accounts or messages"
              className="w-full pl-8 pr-8"
              aria-label="Search activity"
            />
            {query && (
              <button
                type="button"
                aria-label="Clear search"
                onClick={() => setQuery("")}
                className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-faint hover:text-text"
              >
                <X className="size-3.5" />
              </button>
            )}
          </div>
          <div className="grid min-w-0 grid-cols-2 gap-2 sm:flex [&>*]:min-w-0">
            <Select
              label="Tool"
              className="w-full sm:w-40"
              value={provider}
              onChange={setProvider}
              options={[{ value: "all" as const, label: "All tools" }, ...providers.map((p) => ({ value: p.info.id, label: p.info.name }))]}
            />
            <Select
              label="Time range"
              className="w-full sm:w-40"
              value={range}
              onChange={setRange}
              options={[
                { value: "1" as const, label: "Last 24 hours" },
                { value: "7" as const, label: "Last 7 days" },
                { value: "30" as const, label: "Last 30 days" },
                { value: "all" as const, label: "All time" },
              ]}
            />
          </div>
        </div>
        <div className="-mx-1 flex items-center gap-2 overflow-x-auto px-1 pb-0.5">
          <Segmented
            label="Event type"
            className="shrink-0"
            value={category}
            onChange={setCategory}
            options={[
              { value: "all" as const, label: `All · ${count("all")}` },
              { value: "switch" as const, label: `Switches · ${count("switch")}` },
              { value: "account" as const, label: `Accounts · ${count("account")}` },
              { value: "problem" as const, label: `Problems · ${count("problem")}` },
              { value: "system" as const, label: `App · ${count("system")}` },
            ]}
          />
          {filtering && (
            <Button size="sm" variant="ghost" className="ml-auto shrink-0" onClick={clear}>
              <X /> Clear
            </Button>
          )}
        </div>
      </Card>

      {loading ? null : grouped.length === 0 ? (
        <Card className="flex flex-col items-center gap-2 px-6 py-12 text-center">
          <span className="flex size-10 items-center justify-center rounded-2xl bg-surface-2 text-faint">
            <Search className="size-4" />
          </span>
          <p className="font-medium">{items.length === 0 ? "Nothing recorded yet" : "No activity matches these filters"}</p>
          <p className="max-w-sm text-[12.5px] text-muted">
            {items.length === 0
              ? "Switches, sign-in renewals and account changes appear here as they happen."
              : "Try another tool, event type or time range."}
          </p>
          {filtering && items.length > 0 && (
            <Button size="sm" className="mt-1" onClick={clear}>
              Clear filters
            </Button>
          )}
        </Card>
      ) : (
        <div className="space-y-4">
          {grouped.map((g) => (
            <section key={g.day}>
              <h3 className="mb-1.5 flex items-center gap-2 label-mono text-faint">
                {g.day}
                <Badge tone="neutral">{g.rows.length}</Badge>
              </h3>
              <Card className="divide-y divide-border overflow-hidden">
                {g.rows.map((i) => (
                  <ItemRow
                    key={i.key}
                    item={i}
                    open={openKey === i.key}
                    onToggle={() => setOpenKey(openKey === i.key ? null : i.key)}
                    canUndo={!!i.switch && undoable.has(i.switch.id)}
                    onUndo={() => setUndo(i.switch!)}
                    accountName={accountName}
                  />
                ))}
              </Card>
            </section>
          ))}
        </div>
      )}

      <Dialog
        open={!!undo}
        onClose={() => setUndo(null)}
        title="Undo this switch?"
        description={`Restores the sign-in that was active before switching to ${undo?.toLabel ?? "this account"}. Running sessions keep their current account.`}
        footer={
          <>
            <Button onClick={() => setUndo(null)}>Cancel</Button>
            <Button
              variant="primary"
              onClick={async () => {
                const r = undo!;
                setUndo(null);
                try {
                  await api.undoSwitch(r.id);
                  toast({ kind: "success", text: "Previous sign-in restored" });
                  void reload();
                } catch (e) {
                  toast({ kind: "error", text: "Could not undo", detail: errorText(e) });
                }
              }}
            >
              Restore previous
            </Button>
          </>
        }
      />
    </>
  );
}
