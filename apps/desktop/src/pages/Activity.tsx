import { useEffect, useRef, useState } from "react";
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
import { api, errorText, type Category, type Feed, type FeedItem, type ProviderId, type SwitchView } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { RecoveryBanner } from "@/components/Dialogs";
import { Button } from "@/components/ui/button";
import { Badge, Card, Dialog, Input, Segmented, Select, Tag } from "@/components/ui/primitives";

type Range = "1" | "7" | "30" | "all";

const ICON: Record<string, React.ReactNode> = {
  added: <UserPlus />,
  updated: <UserPen />,
  renamed: <UserPen />,
  removed: <UserMinus />,
  renewed: <KeyRound />,
  external: <Zap />,
  drift: <ShieldAlert />,
  interrupted: <AlertTriangle />,
  "needs-recovery": <AlertTriangle />,
  usage: <AlertTriangle />,
  recovered: <Undo2 />,
  settings: <Settings2 />,
};

const CATEGORY_TONE: Record<Category, string> = {
  switch: "bg-accent-soft text-accent",
  account: "bg-ok-soft text-ok",
  problem: "bg-warn-soft text-warn",
  system: "bg-surface-3 text-muted",
};

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

function ItemRow({ item, open, onToggle, onUndo }: { item: FeedItem; open: boolean; onToggle: () => void; onUndo: () => void }) {
  const toast = useApp((s) => s.toast);
  const r = item.switch;
  const icon = ICON[item.kind];
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
        <span className="hidden w-[60px] shrink-0 whitespace-nowrap text-[11.5px] text-muted tabular-nums sm:block">{item.clock}</span>
        <span className="relative mt-0.5 shrink-0 sm:mt-0">
          {item.provider ? (
            <ProviderIcon id={item.provider} size={22} />
          ) : (
            <span className="flex size-[22px] items-center justify-center rounded-[6px] bg-surface-3 text-muted">
              <Settings2 className="size-3" />
            </span>
          )}
          {!r && icon && (
            <span
              className={cn(
                "absolute -bottom-1 -right-1 flex size-3.5 items-center justify-center rounded-full ring-2 ring-[var(--card)] [&_svg]:size-2",
                CATEGORY_TONE[item.category],
              )}
            >
              {icon}
            </span>
          )}
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
            {r ? (
              <span className="min-w-0 truncate text-[13px]">
                <span className="text-muted">{r.fromText}</span>
                <ArrowRight className="mx-1 inline size-3 text-faint" />
                <b className="font-medium">{r.toText}</b>
              </span>
            ) : (
              <span className="text-[13px] font-medium">{item.title}</span>
            )}
            {r && <Tag badge={r.statusBadge} />}
          </span>
          <span className="mt-0.5 block truncate text-[11.5px] text-muted">
            <span className="tabular-nums sm:hidden">{item.clock} · </span>
            {item.text}
          </span>
        </span>
        <span className="hidden shrink-0 text-[11.5px] text-faint sm:block">{r?.durationText}</span>
        <ChevronDown className={cn("mt-0.5 size-4 shrink-0 text-faint transition-transform sm:mt-0", open && "rotate-180")} />
      </button>
      {open && (
        <div className="px-3 pb-4 sm:px-4 sm:pl-[109px]">
          <dl className="grid grid-cols-1 gap-x-6 gap-y-3 rounded-2xl border border-border bg-card p-4 sm:grid-cols-2 lg:grid-cols-3">
            {r ? (
              <>
                <Detail label="From">{r.fromLabel ?? "No saved account"}</Detail>
                <Detail label="To">{r.toText}</Detail>
                <Detail label="Result">
                  <Tag badge={r.statusBadge} />
                </Detail>
                <Detail label="How it applied">{r.modeLabel}</Detail>
                <Detail label="Started">{r.startedFull}</Detail>
                <Detail label="Took">{r.tookText}</Detail>
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
                  {r.undoable && (
                    <Button size="sm" onClick={onUndo}>
                      <Undo2 /> Undo this switch
                    </Button>
                  )}
                </div>
              </>
            ) : (
              <>
                <Detail label="Event">{item.title}</Detail>
                <Detail label="Tool">{item.providerName}</Detail>
                <Detail label="When">{item.fullTime}</Detail>
                {item.accountName && <Detail label="Account">{item.accountName}</Detail>}
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
  const generatedAt = useApp((s) => s.overview?.generatedAt);
  const toast = useApp((s) => s.toast);
  const [feed, setFeed] = useState<Feed | null>(null);
  const [undo, setUndo] = useState<SwitchView | null>(null);
  const [openKey, setOpenKey] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState<ProviderId | "all">("all");
  const [category, setCategory] = useState<Category | "all">("all");
  const [range, setRange] = useState<Range>("30");
  const seq = useRef(0);

  const reload = () => {
    const n = ++seq.current;
    return api
      .feed({
        query,
        provider: provider === "all" ? undefined : provider,
        category: category === "all" ? undefined : category,
        rangeDays: range === "all" ? 0 : Number(range),
      })
      .then((f) => {
        if (n === seq.current) setFeed(f);
      })
      .catch(() => undefined);
  };

  useEffect(() => {
    void reload();
  }, [generatedAt, query, provider, category, range]);

  const clear = () => {
    setQuery("");
    setProvider("all");
    setCategory("all");
    setRange("30");
  };

  const c = feed?.counts;
  const filtering = !!feed?.filtering;

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
        {feed?.stats.map((s) => <Stat key={s.label} label={s.label} value={s.value} hint={s.hint} />)}
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
              options={[{ value: "all" as const, label: "All tools" }, ...(feed?.providers ?? []).map((p) => ({ value: p.id, label: p.name }))]}
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
              { value: "all" as const, label: `All · ${c?.all ?? 0}` },
              { value: "switch" as const, label: `Switches · ${c?.switch ?? 0}` },
              { value: "account" as const, label: `Accounts · ${c?.account ?? 0}` },
              { value: "problem" as const, label: `Problems · ${c?.problem ?? 0}` },
              { value: "system" as const, label: `App · ${c?.system ?? 0}` },
            ]}
          />
          {filtering && (
            <Button size="sm" variant="ghost" className="ml-auto shrink-0" onClick={clear}>
              <X /> Clear
            </Button>
          )}
        </div>
      </Card>

      {!feed ? null : feed.groups.length === 0 ? (
        <Card className="flex flex-col items-center gap-2 px-6 py-12 text-center">
          <span className="flex size-10 items-center justify-center rounded-2xl bg-surface-2 text-faint">
            <Search className="size-4" />
          </span>
          <p className="font-medium">{feed.total === 0 ? "Nothing recorded yet" : "No activity matches these filters"}</p>
          <p className="max-w-sm text-[12.5px] text-muted">
            {feed.total === 0
              ? "Switches, sign-in renewals and account changes appear here as they happen."
              : "Try another tool, event type or time range."}
          </p>
          {filtering && feed.total > 0 && (
            <Button size="sm" className="mt-1" onClick={clear}>
              Clear filters
            </Button>
          )}
        </Card>
      ) : (
        <div className="space-y-4">
          {feed.groups.map((g) => (
            <section key={g.day}>
              <h3 className="mb-1.5 flex items-center gap-2 label-mono text-faint">
                {g.day}
                <Badge tone="neutral">{g.items.length}</Badge>
              </h3>
              <Card className="divide-y divide-border overflow-hidden">
                {g.items.map((i) => (
                  <ItemRow
                    key={i.key}
                    item={i}
                    open={openKey === i.key}
                    onToggle={() => setOpenKey(openKey === i.key ? null : i.key)}
                    onUndo={() => setUndo(i.switch)}
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
