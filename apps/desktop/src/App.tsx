import { useEffect, useState } from "react";
import { Activity, BarChart3, LayoutDashboard, Plus, RefreshCw, Settings as Gear, Users } from "lucide-react";
import { api, type AppInfo } from "@/lib/api";
import { useApp, wireEvents, type Page } from "@/lib/store";
import { cn } from "@/lib/utils";
import { AddAccountDialog, SwitchConfirmDialog } from "@/components/Dialogs";
import { Toasts } from "@/components/Toasts";
import { Spinner } from "@/components/ui/primitives";
import { Dashboard } from "@/pages/Dashboard";
import { Accounts } from "@/pages/Accounts";
import { UsagePage } from "@/pages/Usage";
import { ActivityPage } from "@/pages/Activity";
import { SettingsPage } from "@/pages/Settings";
import { Onboarding } from "@/pages/Onboarding";

const NAV: { page: Page; label: string; icon: React.ReactNode; hint: string }[] = [
  { page: "dashboard", label: "Dashboard", icon: <LayoutDashboard />, hint: "Every tool and its active account" },
  { page: "accounts", label: "Accounts", icon: <Users />, hint: "All saved accounts" },
  { page: "usage", label: "Usage", icon: <BarChart3 />, hint: "Provider-reported usage" },
  { page: "activity", label: "Activity", icon: <Activity />, hint: "Switch history and events" },
];

function SummaryCard({ isMac }: { isMac: boolean }) {
  const overview = useApp((s) => s.overview);
  const navigate = useApp((s) => s.navigate);
  const providers = overview?.providers ?? [];
  const accounts = providers.reduce((n, p) => n + p.accounts.length, 0);
  const tools = providers.filter((p) => p.info.installed).length;
  const active = providers.filter((p) => p.activeAccountId).length;
  const pct = tools ? Math.round((active / tools) * 100) : 0;
  return (
    <button type="button" onClick={() => navigate("accounts")} className="disk w-full text-left">
      <div className="flex items-baseline justify-between text-[13px] font-semibold">
        <span>Your accounts</span>
        <span className="font-mono text-[11px] font-normal text-faint">{accounts}</span>
      </div>
      <div className="bar mt-1">
        <span className="used" style={{ width: `${Math.max(6, pct)}%` }} />
      </div>
      <div className="text-[11.5px] text-muted">
        {active} of {tools} tools have an active account
      </div>
      <div className="text-[11px] text-faint">Encrypted on this {isMac ? "Mac" : "computer"}</div>
    </button>
  );
}

export function App() {
  const { page, navigate, load, settings, loading, overview, openAdd } = useApp();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const isMac = info?.os === "macos";

  useEffect(() => {
    wireEvents();
    void load(true);
    void api.appInfo().then(setInfo).catch(() => undefined);
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (/^[1-4]$/.test(e.key)) {
        e.preventDefault();
        navigate(NAV[Number(e.key) - 1].page);
      } else if (e.key === ",") {
        e.preventDefault();
        navigate("settings");
      } else if (e.key.toLowerCase() === "r") {
        e.preventDefault();
        void load(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [load, navigate]);

  if (loading && !overview) {
    return (
      <div className="flex h-full items-center justify-center text-muted" data-tauri-drag-region>
        <Spinner />
      </div>
    );
  }

  if (settings && !settings.onboarded) {
    return (
      <>
        <Onboarding isMac={isMac} />
        <AddAccountDialog />
        <Toasts />
      </>
    );
  }

  const attention = (overview?.incomplete.length ?? 0) > 0 || (overview?.providers ?? []).some((p) => p.activationState === "drifted");

  const refresh = async () => {
    setRefreshing(true);
    await load(true);
    setRefreshing(false);
  };

  return (
    <div className="app-shell">
      <header data-tauri-drag-region className="app-titlebar">
        <div data-tauri-drag-region className="flex-1" />
        <button
          type="button"
          onClick={() => void refresh()}
          title="Refresh (⌘R)"
          aria-label="Refresh"
          className="inline-flex size-8.5 items-center justify-center rounded-full border border-border bg-card text-muted shadow-item transition-colors hover:text-text"
        >
          <RefreshCw className={cn("size-4", refreshing && "animate-spin")} />
        </button>
        <button
          type="button"
          onClick={() => openAdd("any")}
          className="flex h-8.5 items-center gap-2 rounded-full bg-brand px-4 text-[13px] font-medium text-white shadow-brand transition-[filter] hover:brightness-110"
        >
          <Plus className="size-4" /> Add account
        </button>
      </header>

      <div className="app-body">
        <aside className="app-nav">
          <div data-tauri-drag-region className="flex items-center gap-2.5 px-3 pb-4">
            <img src="/icon.png" alt="" className="size-7 rounded-[8px] drop-shadow-[0_3px_6px_rgb(0_0_0/0.25)]" draggable={false} />
            <span data-tauri-drag-region className="font-display text-[19px] font-medium">
              AltFlip
            </span>
          </div>
          <nav aria-label="Main" className="flex-1 overflow-auto">
            <ul className="space-y-1">
              {NAV.map((n) => (
                <li key={n.page}>
                  <button
                    type="button"
                    title={n.hint}
                    onClick={() => navigate(n.page)}
                    aria-current={page === n.page ? "page" : undefined}
                    className="nav-item"
                  >
                    {n.icon}
                    <span className="flex-1">{n.label}</span>
                    {n.page === "activity" && attention && <span className="size-2 rounded-full bg-warn" aria-label="Needs attention" />}
                  </button>
                </li>
              ))}
            </ul>
          </nav>
          <div className="space-y-2 pt-2">
            <button
              type="button"
              onClick={() => navigate("settings")}
              aria-current={page === "settings" ? "page" : undefined}
              className="nav-item"
            >
              <Gear />
              <span className="flex-1">Settings</span>
            </button>
            <SummaryCard isMac={isMac} />
          </div>
        </aside>

        <main className="app-main">
          <div className="mx-auto flex max-w-[1040px] flex-col gap-4 pb-6">
            {page === "dashboard" && <Dashboard />}
            {page === "accounts" && <Accounts />}
            {page === "usage" && <UsagePage />}
            {page === "activity" && <ActivityPage />}
            {page === "settings" && <SettingsPage isMac={isMac} info={info} />}
          </div>
        </main>
      </div>
      <SwitchConfirmDialog />
      <AddAccountDialog />
      <Toasts />
    </div>
  );
}
