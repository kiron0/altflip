import { useEffect, useState } from "react";
import { emitTo, listen } from "@tauri-apps/api/event";
import { AlertTriangle, LayoutDashboard, Plus, Power, Settings as Gear } from "lucide-react";
import { api } from "@/lib/api";
import { useApp, wireEvents } from "@/lib/store";
import { AccountRow } from "@/components/AccountRow";
import { ProviderIcon } from "@/components/ProviderIcon";
import { SwitchConfirmDialog } from "@/components/Dialogs";
import { Toasts } from "@/components/Toasts";
import { Badge, Spinner } from "@/components/ui/primitives";
import { cn } from "@/lib/utils";

export function QuickPanel() {
  const overview = useApp((s) => s.overview);
  const loading = useApp((s) => s.loading);
  const load = useApp((s) => s.load);
  const [isMac, setIsMac] = useState(true);

  useEffect(() => {
    wireEvents();
    void load(false);
    void api.appInfo().then((i) => setIsMac(i.os === "macos")).catch(() => undefined);
    const un = listen("altflip://quick-shown", () => void load(false));
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !useApp.getState().confirm) void api.hideQuick();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      void un.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  }, [load]);

  const providers = (overview?.providers ?? []).filter((p) => p.info.installed || p.accounts.length > 0);
  const attention = !!overview?.stats.needsReview;
  const open = (page?: string) => void api.showMain(page);
  const addFor = (target: string) => {
    void api.showMain("accounts").then(() => setTimeout(() => void emitTo("main", "altflip://add-account", target), 120));
  };

  return (
    <div className="quick-shell relative flex h-full flex-col text-[13px]">
      <header className={cn("flex shrink-0 items-center gap-2.5 px-4 pb-3", isMac ? "pt-4" : "pt-3.5")} data-tauri-drag-region>
        <img src="/icon.png" alt="" className="size-7 rounded-[8px] drop-shadow-[0_3px_6px_rgb(0_0_0/0.25)]" draggable={false} />
        <span className="font-display flex-1 text-[16px] font-semibold">AltFlip</span>
        <button
          aria-label="Add account"
          title="Add account"
          onClick={() => addFor("any")}
          className="inline-flex size-8 items-center justify-center rounded-xl text-muted transition-colors hover:bg-surface-3 hover:text-text"
        >
          <Plus className="size-[17px]" />
        </button>
        <button
          aria-label="Settings"
          title="Settings"
          onClick={() => open("settings")}
          className="inline-flex size-8 items-center justify-center rounded-xl text-muted transition-colors hover:bg-surface-3 hover:text-text"
        >
          <Gear className="size-[17px]" />
        </button>
      </header>

      {attention && (
        <button
          onClick={() => open("activity")}
          className="mx-3 mb-2.5 flex items-center gap-2 rounded-xl bg-warn-soft px-3 py-2 text-left text-[12.5px]"
        >
          <AlertTriangle className="size-3.5 shrink-0 text-warn" />
          A sign-in changed outside AltFlip. Review it.
        </button>
      )}

      <div className="mx-2 mb-2 flex min-h-0 flex-1 flex-col overflow-hidden rounded-[16px] border border-border bg-canvas shadow-card">
        <div className="min-h-0 flex-1 overflow-auto px-2 py-2">
          {loading && !overview && (
            <div className="flex h-32 items-center justify-center text-muted">
              <Spinner />
            </div>
          )}
          {overview && providers.length === 0 && (
            <div className="flex min-h-[140px] items-center justify-center px-4 text-center text-faint">
              No supported AI tools were found on this computer.
            </div>
          )}
          {providers.map((p, i) => (
            <section key={p.info.id} className={cn(i > 0 && "mt-2")}>
              <div className="label-mono flex items-center gap-2 px-3 pt-1 pb-1.5 text-faint">
                <ProviderIcon id={p.info.id} size={14} />
                <span className="flex-1 truncate">{p.info.name}</span>
                {p.info.experimental && <Badge tone="warn" className="normal-case tracking-normal">Experimental</Badge>}
              </div>
              {p.accounts.length === 0 ? (
                <button
                  onClick={() => addFor(p.info.id)}
                  className="row-item clickable min-h-10 text-[13px] text-muted"
                >
                  <Plus className="size-4 text-faint" /> Add a {p.info.name} account
                </button>
              ) : (
                p.accounts.map((a) => <AccountRow key={a.id} account={a} provider={p} compact />)
              )}
              {!p.switchingEnabled && p.accounts.length > 1 && p.switchingDisabledReason && (
                <p className="px-3 pt-0.5 text-[11.5px] text-faint">{p.switchingDisabledReason}</p>
              )}
            </section>
          ))}
        </div>

        <footer className="flex shrink-0 items-center gap-2 border-t border-border bg-surface-2 px-3 py-2.5">
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
            title="Quit AltFlip"
            className="flex h-9 items-center justify-center gap-1.5 rounded-xl border border-border bg-surface px-4 text-[12.5px] font-medium text-muted shadow-xs transition-colors hover:border-danger/40 hover:bg-danger-soft hover:text-danger"
          >
            <Power className="size-3.5" />
            Quit
          </button>
        </footer>
      </div>
      <SwitchConfirmDialog />
      <Toasts compact />
    </div>
  );
}
