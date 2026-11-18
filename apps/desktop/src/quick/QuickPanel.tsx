import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { emitTo, listen } from "@tauri-apps/api/event";
import {
  AlertTriangle,
  Check,
  CircleAlert,
  Info,
  Plus,
  Power,
  RefreshCw,
  LayoutDashboard,
  Settings2,
  Star,
  Timer,
} from "lucide-react";
import {
  api,
  type AccountView,
  type ProviderId,
  type ProviderState,
  type UsageView,
} from "@/lib/api";
import { useApp, wireEvents } from "@/lib/store";
import { ProviderIcon } from "@/components/ProviderIcon";
import { UsageOnlyButton } from "@/components/UsageOnly";
import { SwitchConfirmDialog } from "@/components/Dialogs";
import { Toasts } from "@/components/Toasts";
import { Spinner } from "@/components/ui/primitives";
import { cn } from "@/lib/utils";

const HUES: Record<ProviderId, string> = {
  "claude-code": "#c96442",
  codex: "#2f6fed",
  antigravity: "#7c5cf2",
};

const ARROW_TOP = 12;

function popoverPath(w: number, h: number, anchorX: number | null): string {
  const r = 18;
  const top = ARROW_TOP;
  const halfW = 16;
  const ax = Math.max(r + halfW, Math.min(anchorX ?? w / 2, w - r - halfW));
  return [
    `M ${r} ${top}`,
    `L ${ax - halfW} ${top}`,
    `C ${ax - 14.0} ${top}, ${ax - 12.0} ${top - 0.9}, ${ax - 10.4} ${top - 2.4}`,
    `L ${ax - 4.0} ${top - 9.2}`,
    `C ${ax - 1.8} ${top - 11.8}, ${ax + 1.8} ${top - 11.8}, ${ax + 4.0} ${top - 9.2}`,
    `L ${ax + 10.4} ${top - 2.4}`,
    `C ${ax + 12.0} ${top - 0.9}, ${ax + 14.0} ${top}, ${ax + halfW} ${top}`,
    `L ${w - r} ${top}`,
    `Q ${w} ${top} ${w} ${top + r}`,
    `L ${w} ${h - r}`,
    `Q ${w} ${h} ${w - r} ${h}`,
    `L ${r} ${h}`,
    `Q 0 ${h} 0 ${h - r}`,
    `L 0 ${top + r}`,
    `Q 0 ${top} ${r} ${top}`,
    `Z`,
  ].join(" ");
}

function usageTone(pct: number) {
  if (pct >= 85) return "var(--danger)";
  if (pct >= 60) return "var(--warn)";
  return "var(--ok)";
}

function windowLabel(u: UsageView) {
  const m = u.windowMinutes;
  if (!m) return u.label;
  if (m < 1440) return `${Math.max(1, Math.round(m / 60))}h`;
  return `${Math.round(m / 1440)}d`;
}

function Ring({ pct }: { pct: number }) {
  const r = 4.9;
  const c = 2 * Math.PI * r;
  const v = Math.min(100, Math.max(0, pct));
  return (
    <svg viewBox="0 0 12 12" className="size-3 shrink-0 -rotate-90" aria-hidden>
      <circle
        cx="6"
        cy="6"
        r={r}
        fill="none"
        stroke="currentColor"
        strokeOpacity={0.14}
        strokeWidth={2.2}
      />
      <circle
        cx="6"
        cy="6"
        r={r}
        fill="none"
        stroke={usageTone(v)}
        strokeWidth={2.2}
        strokeLinecap="round"
        strokeDasharray={`${(v / 100) * c} ${c}`}
        className="transition-[stroke-dasharray] duration-500 ease-out"
      />
    </svg>
  );
}

function UsagePill({
  u,
  showCountdown,
}: {
  u: UsageView;
  showCountdown: boolean;
}) {
  if (u.percent == null) return null;
  const hot = u.percent >= 60;
  const reset = showCountdown ? u.countdown : null;
  return (
    <span
      className="inline-flex shrink-0 items-center gap-1.5 rounded-full bg-text/[0.055] px-2 py-[3px] text-[10.5px] leading-none tabular-nums"
      title={`${u.label}: ${u.percent}% used · ${u.resetText ?? u.leftText}`}
    >
      <Ring pct={u.percent} />
      <span className="font-medium text-muted">{windowLabel(u)}</span>
      <span
        className="font-semibold"
        style={{ color: hot ? usageTone(u.percent) : undefined }}
      >
        {u.percent}%
      </span>
      {reset && (
        <span className="flex items-center gap-0.5 text-faint">
          <Timer className="size-[10px]" strokeWidth={2.4} />
          {reset}
        </span>
      )}
    </span>
  );
}

function Avatar({
  label,
  hue,
  size = 30,
}: {
  label: string;
  hue: string;
  size?: number;
}) {
  const initial = (label.trim()[0] ?? "?").toUpperCase();
  return (
    <span
      aria-hidden
      className="flex shrink-0 items-center justify-center rounded-full font-display font-semibold text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.25)]"
      style={{
        width: size,
        height: size,
        fontSize: Math.round(size * 0.42),
        background: `linear-gradient(180deg, color-mix(in oklab, ${hue} 60%, white), ${hue})`,
      }}
    >
      {initial}
    </span>
  );
}

function Divider() {
  return <div className="ml-[52px] h-px bg-border/70" />;
}

function AccountLine({
  account,
  provider,
  hue,
}: {
  account: AccountView;
  provider: ProviderState;
  hue: string;
}) {
  const progress = useApp((s) => s.switching[provider.info.id]);
  const requestSwitch = useApp((s) => s.requestSwitch);
  const showCountdown = useApp((s) => !!s.settings?.showResetCountdowns);
  const busy = progress?.accountId === account.id;
  const otherBusy = !!progress && !busy;
  const switchable = provider.info.capabilities.accountSwitching;
  const active =
    provider.activeAccountId === account.id &&
    provider.activationState !== "drifted";
  const canSwitch =
    switchable && provider.switchingEnabled && !active && !otherBusy && !busy;
  const pills = account.usage.filter((u) => u.percent != null).slice(0, 3);

  return (
    <button
      type="button"
      disabled={!canSwitch}
      onClick={() => void requestSwitch(account.id)}
      title={
        canSwitch
          ? `Switch ${provider.info.name} to ${account.label}`
          : undefined
      }
      className={cn(
        "group flex w-full items-center gap-2.5 px-2.5 py-2 text-left transition-[background-color,transform] duration-150",
        canSwitch
          ? "cursor-pointer hover:bg-text/[0.05] active:scale-[0.985]"
          : "cursor-default",
      )}
    >
      <Avatar label={account.label} hue={hue} />
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-1.5">
          <span className="truncate text-[13px] font-medium text-text">
            {account.label}
          </span>
          {account.favorite && (
            <Star
              className="size-3 shrink-0 fill-warn text-warn"
              aria-label="Favorite"
            />
          )}
        </span>
        <span className="flex items-center gap-1 truncate text-[11px] text-muted">
          {busy ? (
            <span className="truncate text-accent">{progress.message}…</span>
          ) : (
            <>
              {account.needsSignIn && (
                <CircleAlert
                  className="size-[11px] shrink-0 text-warn"
                  aria-label={provider.signInHint}
                />
              )}
              <span className="truncate">
                {account.needsSignIn
                  ? "Needs sign-in"
                  : (account.email ?? account.subtitle)}
              </span>
            </>
          )}
        </span>
        {(pills.length > 0 || account.usageError) && (
          <span className="mt-1.5 flex flex-wrap items-center gap-1">
            {pills.map((u) => (
              <UsagePill
                key={u.usageType}
                u={u}
                showCountdown={showCountdown}
              />
            ))}
            {pills.length === 0 && account.usageError && (
              <span
                className="inline-flex items-center gap-1 rounded-full bg-warn-soft px-2 py-[3px] text-[10.5px] leading-none text-warn"
                title={account.usageError}
              >
                <AlertTriangle className="size-[10px]" /> Usage unavailable
              </span>
            )}
          </span>
        )}
      </span>
      <span className="flex shrink-0 items-center self-center">
        {busy ? (
          <Spinner className="text-accent" />
        ) : active ? (
          <span
            className="flex size-[18px] items-center justify-center rounded-full text-white"
            style={{ background: hue }}
            title={switchable ? "Active" : `Signed in to ${provider.info.name}`}
          >
            <Check className="size-3" strokeWidth={3.2} />
          </span>
        ) : canSwitch ? (
          <span
            className="size-[18px] rounded-full border-[1.5px] border-border-strong transition-colors group-hover:border-muted"
            aria-hidden
          />
        ) : switchable ? (
          <span
            className="size-[18px] rounded-full border-[1.5px] border-border opacity-60"
            aria-hidden
          />
        ) : null}
      </span>
    </button>
  );
}

function Section({
  p,
  index,
  onAdd,
}: {
  p: ProviderState;
  index: number;
  onAdd: () => void;
}) {
  const hue = HUES[p.info.id];
  const switchable = p.info.capabilities.accountSwitching;
  const style: CSSProperties = {
    animationDelay: `${index * 25}ms`,
    animationFillMode: "both",
  };
  return (
    <section className="animate-in" style={style}>
      <div className="mb-1.5 flex items-center gap-2 px-1">
        <ProviderIcon id={p.info.id} size={18} />
        <span className="flex-1 truncate text-[12px] font-semibold text-text">
          {p.info.name}
        </span>
        {switchable ? (
          <span className="text-[11px] text-faint">
            {p.accounts.length === 1
              ? "1 account"
              : `${p.accounts.length} accounts`}
          </span>
        ) : (
          <UsageOnlyButton compact />
        )}
      </div>
      <div className="overflow-hidden rounded-[12px] border border-border/80 bg-card/70 shadow-[0_1px_2px_rgb(0_0_0/0.04)]">
        {p.accounts.map((a, i) => (
          <div key={a.id}>
            {i > 0 && <Divider />}
            <AccountLine account={a} provider={p} hue={hue} />
          </div>
        ))}
        {p.accounts.length > 0 && <Divider />}
        <button
          type="button"
          onClick={onAdd}
          className="flex w-full items-center gap-2.5 px-2.5 py-2 text-left text-[12.5px] font-medium transition-colors hover:bg-text/[0.05]"
          style={{ color: hue }}
        >
          <span
            className="flex size-[30px] shrink-0 items-center justify-center rounded-full border-[1.5px] border-dashed"
            style={{
              borderColor: `color-mix(in oklab, ${hue} 45%, transparent)`,
            }}
          >
            <Plus className="size-3.5" strokeWidth={2.4} />
          </span>
          {p.accounts.length === 0
            ? `Add a ${p.info.name} account`
            : "Add account…"}
        </button>
      </div>
    </section>
  );
}

function HeaderButton({
  label,
  onClick,
  children,
  tone,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
  tone?: "warn";
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "inline-flex size-[26px] items-center justify-center rounded-full transition-[background-color,color,transform] duration-150 active:scale-[0.94]",
        tone === "warn"
          ? "bg-warn-soft text-warn"
          : "bg-text/[0.07] text-muted hover:bg-text/[0.12] hover:text-text",
      )}
    >
      {children}
    </button>
  );
}

export function QuickPanel() {
  const overview = useApp((s) => s.overview);
  const loading = useApp((s) => s.loading);
  const load = useApp((s) => s.load);
  const refreshAll = useApp((s) => s.refreshAll);
  const refreshing = useApp((s) => s.refreshingAll);
  const [isMac, setIsMac] = useState(true);
  const arrow = document.documentElement.dataset.os === "macos";
  const frame = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 420, h: 592 });
  const [anchorX, setAnchorX] = useState<number | null>(null);
  const path = useMemo(
    () => popoverPath(size.w, size.h, anchorX),
    [size.w, size.h, anchorX],
  );

  useEffect(() => {
    const el = frame.current;
    if (!arrow || !el) return;
    const measure = () => {
      if (el.clientWidth > 0 && el.clientHeight > 0)
        setSize({ w: el.clientWidth, h: el.clientHeight });
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    const un = listen<number>("altflip://quick-anchor", (e) =>
      setAnchorX(e.payload),
    );
    return () => {
      ro.disconnect();
      void un.then((f) => f());
    };
  }, [arrow]);

  useEffect(() => {
    wireEvents();
    void load(false);
    void api
      .appInfo()
      .then((i) => setIsMac(i.os === "macos"))
      .catch(() => undefined);
    const un = listen("altflip://quick-shown", () => void load(false));
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !useApp.getState().confirm)
        void api.hideQuick();
      if (!(e.metaKey || e.ctrlKey)) return;
      const k = e.key.toLowerCase();
      if (k === "q") {
        e.preventDefault();
        void api.quit();
      } else if (k === ",") {
        e.preventDefault();
        void api.showMain("settings");
      } else if (k === "o") {
        e.preventDefault();
        void api.showMain();
      } else if (k === "r") {
        e.preventDefault();
        void useApp.getState().refreshAll();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      void un.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  }, [load]);

  const providers = (overview?.providers ?? []).filter(
    (p) => p.info.installed || p.accounts.length > 0,
  );
  const attention = !!overview?.stats.needsReview;
  const total = providers.reduce((n, p) => n + p.accounts.length, 0);
  const activeCount = providers.filter(
    (p) => p.activeAccountId && p.info.capabilities.accountSwitching,
  ).length;
  const subtitle = overview
    ? `${total === 1 ? "1 account" : `${total} accounts`}${activeCount ? ` · ${activeCount} active` : ""}`
    : "Loading…";
  const open = (page?: string) => void api.showMain(page);
  const addFor = (target: string) => {
    void api
      .showMain("accounts")
      .then(() =>
        setTimeout(
          () => void emitTo("main", "altflip://add-account", target),
          120,
        ),
      );
  };

  return (
    <div ref={frame} className="relative h-full w-full">
      {arrow && (
        <>
          <svg
            className="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
            aria-hidden
          >
            <defs>
              <clipPath id="quick-popover-clip" clipPathUnits="userSpaceOnUse">
                <path d={path} />
              </clipPath>
            </defs>
            <path d={path} fill="var(--window)" />
          </svg>
          <svg
            className="pointer-events-none absolute inset-0 z-30 h-full w-full overflow-visible"
            aria-hidden
          >
            <path
              d={path}
              fill="none"
              stroke="var(--glass-edge, var(--border))"
              strokeWidth="1"
            />
          </svg>
        </>
      )}
      <div
        className={cn(
          "quick-shell relative flex h-full flex-col text-[13px]",
          arrow && "quick-arrow",
        )}
        style={
          arrow
            ? {
                clipPath: "url(#quick-popover-clip)",
                WebkitClipPath: "url(#quick-popover-clip)",
                paddingTop: ARROW_TOP,
              }
            : undefined
        }
      >
        <header
          className={cn(
            "flex shrink-0 items-center gap-2.5 px-3.5 pb-3",
            isMac ? "pt-3.5" : "pt-3",
          )}
          data-tauri-drag-region
        >
          <img
            src="/icon.png"
            alt=""
            className="size-8 rounded-[8px] drop-shadow-[0_3px_6px_rgb(0_0_0/0.25)]"
            draggable={false}
          />
          <div className="min-w-0 flex-1" data-tauri-drag-region>
            <div className="font-display text-[14px] font-semibold leading-tight">
              AltFlip
            </div>
            <div className="truncate text-[11px] text-muted tabular-nums">
              {subtitle}
            </div>
          </div>
          <div className="flex items-center gap-1.5">
            {attention && (
              <HeaderButton
                label="A sign-in changed outside AltFlip. Review it"
                onClick={() => open("activity")}
                tone="warn"
              >
                <AlertTriangle className="size-3" strokeWidth={2.4} />
              </HeaderButton>
            )}
            <HeaderButton
              label="Refresh (⌘R)"
              onClick={() => void refreshAll()}
            >
              <RefreshCw
                className={cn("size-3", refreshing && "animate-spin")}
                strokeWidth={2.4}
              />
            </HeaderButton>
            <HeaderButton label="Add account" onClick={() => addFor("any")}>
              <Plus className="size-3.5" strokeWidth={2.4} />
            </HeaderButton>
            <HeaderButton
              label="Settings (⌘,)"
              onClick={() => open("settings")}
            >
              <Settings2 className="size-3" strokeWidth={2.4} />
            </HeaderButton>
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-auto px-3 pb-2">
          {loading && !overview && (
            <div className="flex h-32 items-center justify-center text-muted">
              <Spinner />
            </div>
          )}
          {overview && providers.length === 0 && (
            <div className="flex min-h-[140px] flex-col items-center justify-center gap-2 rounded-[12px] border border-border/80 bg-card/70 px-4 text-center text-faint">
              <Info className="size-5" />
              No supported AI tools were found on this computer.
            </div>
          )}
          <div className="flex flex-col gap-3.5">
            {providers.map((p, i) => (
              <Section
                key={p.info.id}
                p={p}
                index={i}
                onAdd={() => addFor(p.info.id)}
              />
            ))}
          </div>
        </div>

        <footer className="flex shrink-0 items-center gap-2 border-t border-border bg-surface-2/70 px-3 py-2.5">
          <button
            type="button"
            onClick={() => open()}
            className="flex h-9 flex-1 items-center justify-center gap-2 rounded-xl border border-border bg-surface px-3 text-[12.5px] font-medium text-text shadow-xs transition-colors hover:border-border-strong hover:bg-surface-2"
          >
            <LayoutDashboard className="size-3.5 text-accent" />
            Manage accounts
          </button>
          <button
            type="button"
            onClick={() => void api.quit()}
            title="Quit AltFlip (⌘Q)"
            className="flex h-9 items-center justify-center gap-1.5 rounded-xl border border-border bg-surface px-4 text-[12.5px] font-medium text-muted shadow-xs transition-colors hover:border-danger/40 hover:bg-danger-soft hover:text-danger"
          >
            <Power className="size-3.5" />
            Quit
          </button>
        </footer>
        <SwitchConfirmDialog />
        <Toasts compact />
      </div>
    </div>
  );
}
