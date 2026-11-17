import { invoke } from "@tauri-apps/api/core";
import type {
  AccountPatch,
  AccountRow,
  ApiError,
  AppInfo,
  Feed,
  FeedQuery,
  Notice,
  Overview,
  ProviderId,
  RecoveryAction,
  Settings,
  StatusLineState,
  SwitchOutcome,
  SwitchPlan,
  SwitchView,
} from "./bindings";

export type * from "./bindings";

export function errorText(e: unknown): string {
  return e && typeof e === "object" && "message" in e ? String((e as ApiError).message) : String(e);
}

export function errorCode(e: unknown): string | undefined {
  return e && typeof e === "object" && "code" in e ? String((e as ApiError).code) : undefined;
}

export const api = {
  overview: (refresh = false) => invoke<Overview>("get_overview", { refresh }),
  planSwitch: (accountId: string) => invoke<SwitchPlan>("plan_switch", { accountId }),
  switchAccount: (accountId: string, confirmRestart = false) => invoke<SwitchOutcome>("switch_account", { accountId, confirmRestart }),
  saveCurrent: (providerId: ProviderId, label?: string, category?: string | null) =>
    invoke<AccountRow>("save_current", { providerId, label: label || null, category: category || null }),
  startLogin: (providerId: ProviderId, label?: string, category?: string | null) =>
    invoke<string>("start_login", { providerId, label: label || null, category: category || null }),
  cancelLogin: (loginId: string) => invoke<void>("cancel_login", { loginId }),
  updateAccount: (id: string, patch: AccountPatch) => invoke<AccountRow>("update_account", { id, patch }),
  moveAccount: (id: string, delta: number) => invoke<void>("move_account", { id, delta }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  history: (limit?: number) => invoke<SwitchView[]>("get_history", { limit }),
  feed: (query: FeedQuery) => invoke<Feed>("activity_feed", { query }),
  resolveIncomplete: (operationId: string, action: RecoveryAction) => invoke<void>("resolve_incomplete", { operationId, action }),
  undoSwitch: (operationId: string) => invoke<void>("undo_switch", { operationId }),
  refreshUsage: () => invoke<Notice | null>("refresh_usage"),
  claudeStatusLine: () => invoke<StatusLineState>("claude_status_line"),
  setClaudeStatusLine: (on: boolean) => invoke<StatusLineState>("set_claude_status_line", { on }),
  settings: () => invoke<Settings>("get_settings"),
  setSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => invoke<Settings>("set_setting", { key, value }),
  resetSettings: () => invoke<Settings>("reset_settings"),
  exportBackup: (path: string, passphrase: string) => invoke<number>("export_backup", { path, passphrase }),
  importBackup: (path: string, passphrase: string) => invoke<number>("import_backup", { path, passphrase }),
  diagnostics: () => invoke<string>("diagnostics"),
  appInfo: () => invoke<AppInfo>("app_info"),
  showMain: (page?: string) => invoke<void>("show_main_window", { page: page ?? null }),
  hideQuick: () => invoke<void>("hide_quick"),
  quit: () => invoke<void>("quit_app"),
};
