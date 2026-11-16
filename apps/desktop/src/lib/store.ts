import type { SettingsTab } from "@/components/settingsNav";
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api, errorCode, errorText, type Overview, type Progress, type ProviderId, type Settings, type SwitchPlan } from "./api";

export type Page = "dashboard" | "accounts" | "usage" | "activity" | "settings";
export const PAGES: Page[] = ["dashboard", "accounts", "usage", "activity", "settings"];

export interface Toast {
  id: number;
  kind: "success" | "error" | "info" | "warning";
  text: string;
  detail?: string;
}

interface State {
  overview: Overview | null;
  settings: Settings | null;
  loading: boolean;
  page: Page;
  focusAccount: string | null;
  toasts: Toast[];
  switching: Partial<Record<ProviderId, Progress>>;
  confirm: SwitchPlan | null;
  addFor: ProviderId | "any" | null;
  load: (refresh?: boolean) => Promise<void>;
  navigate: (p: Page, focusAccount?: string | null) => void;
  settingsTab: SettingsTab;
  setSettingsTab: (t: SettingsTab) => void;
  toast: (t: Omit<Toast, "id">) => void;
  dismiss: (id: number) => void;
  setSetting: <K extends keyof Settings>(k: K, v: Settings[K]) => Promise<void>;
  requestSwitch: (accountId: string) => Promise<void>;
  runSwitch: (accountId: string, confirmRestart: boolean) => Promise<boolean>;
  closeConfirm: () => void;
  openAdd: (p: ProviderId | "any" | null) => void;
}

let toastSeq = 1;

export function applyTheme(theme: Settings["theme"]) {
  try {
    localStorage.setItem("altflip.theme", theme);
  } catch {
    return;
  }
  const dark = theme === "dark" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
}

export const useApp = create<State>((set, get) => ({
  overview: null,
  settings: null,
  loading: true,
  page: "dashboard",
  settingsTab: "general",
  setSettingsTab: (settingsTab) => set({ settingsTab }),
  focusAccount: null,
  toasts: [],
  switching: {},
  confirm: null,
  addFor: null,

  load: async (refresh = false) => {
    try {
      const [overview, settings] = await Promise.all([api.overview(refresh), api.settings()]);
      applyTheme(settings.theme);
      set({ overview, settings, loading: false });
    } catch (e) {
      set({ loading: false });
      get().toast({ kind: "error", text: "Could not load accounts", detail: errorText(e) });
    }
  },

  navigate: (page, focusAccount = null) => set({ page, focusAccount }),

  toast: (t) => {
    const id = toastSeq++;
    set((s) => ({ toasts: [...s.toasts.slice(-3), { ...t, id }] }));
    setTimeout(() => get().dismiss(id), t.kind === "error" ? 9000 : 4500);
  },

  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  setSetting: async (k, v) => {
    try {
      const settings = await api.setSetting(k, v);
      if (k === "theme") applyTheme(settings.theme);
      set({ settings });
    } catch (e) {
      get().toast({ kind: "error", text: "Setting not changed", detail: errorText(e) });
    }
  },

  requestSwitch: async (accountId) => {
    try {
      const plan = await api.planSwitch(accountId);
      if (plan.alreadyActive) {
        await get().runSwitch(accountId, false);
        return;
      }
      if (plan.blocked) {
        get().toast({ kind: "warning", text: `Can't switch to ${plan.label}`, detail: plan.blocked });
        return;
      }
      const warn = get().settings?.warnRunningSessions ?? true;
      if (plan.requiresConfirmation || (warn && plan.warnings.length > 0)) {
        set({ confirm: plan });
        return;
      }
      await get().runSwitch(accountId, false);
    } catch (e) {
      get().toast({ kind: "error", text: "Switch failed", detail: errorText(e) });
    }
  },

  runSwitch: async (accountId, confirmRestart) => {
    set({ confirm: null });
    try {
      const out = await api.switchAccount(accountId, confirmRestart);
      const extra = out.restarted.length ? ` Reopened ${out.restarted.join(", ")}.` : "";
      get().toast({
        kind: out.status === "succeeded" || out.status === "already-active" ? "success" : "info",
        text: out.message,
        detail: (out.verification ?? "") + extra || undefined,
      });
      return true;
    } catch (e) {
      if (errorCode(e) === "confirmation_required") {
        const plan = await api.planSwitch(accountId).catch(() => null);
        if (plan) set({ confirm: plan });
        return false;
      }
      get().toast({ kind: "error", text: "Switch failed", detail: errorText(e) });
      return false;
    } finally {
      set((s) => {
        const next = { ...s.switching };
        for (const k of Object.keys(next) as ProviderId[]) if (next[k]?.accountId === accountId) delete next[k];
        return { switching: next };
      });
      void get().load(false);
    }
  },

  closeConfirm: () => set({ confirm: null }),
  openAdd: (p) => set({ addFor: p }),
}));

let wired = false;

export function wireEvents() {
  if (wired) return;
  wired = true;
  const s = useApp.setState;
  void listen<Overview>("altflip://overview", (e) => s({ overview: e.payload }));
  let usageTimer: ReturnType<typeof setTimeout> | undefined;
  void listen("altflip://usage", () => {
    clearTimeout(usageTimer);
    usageTimer = setTimeout(() => {
      void api
        .overview(false)
        .then((overview) => s({ overview }))
        .catch(() => undefined);
    }, 150);
  });
  void listen<Settings>("altflip://settings", (e) => {
    applyTheme(e.payload.theme);
    s({ settings: e.payload });
  });
  void listen<Progress>("altflip://switch-progress", (e) =>
    s((st) => ({ switching: { ...st.switching, [e.payload.provider]: e.payload } })),
  );
  void listen<string>("altflip://navigate", (e) => {
    const p = e.payload as Page;
    if (PAGES.includes(p)) s({ page: p });
  });
  void listen<string>("altflip://confirm-switch", (e) => void useApp.getState().requestSwitch(e.payload));
  void listen<string>("altflip://add-account", (e) => s({ addFor: e.payload as ProviderId, page: "accounts" }));
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const t = useApp.getState().settings?.theme;
    if (t === "system") applyTheme(t);
  });
}
