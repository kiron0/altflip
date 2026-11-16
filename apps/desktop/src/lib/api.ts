import { invoke } from "@tauri-apps/api/core";

export type StatusLineState = {
  installed: boolean;
  scriptPath: string;
  chainedCommand: string | null;
  lastReadingAt: string | null;
};

export type ProviderId = "claude-code" | "codex" | "antigravity" | "opencode";
export type Compatibility = "supported" | "experimental" | "untested" | "unsupported" | "not-installed";
export type SwitchMode = "immediate" | "new-sessions" | "restart-required" | "manual" | "unsupported";
export type AuthHealth = "ok" | "expiring" | "expired" | "needs-reauth" | "unknown";
export type ProcessKind = "cli" | "app" | "ide" | "helper";
export type Support = "yes" | "no" | "conditional";

export interface Identity {
  key: string;
  email: string | null;
  displayName: string | null;
  organization: string | null;
  plan: string | null;
  authMethod: string | null;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  path: string | null;
  kind: ProcessKind;
}

export interface Capabilities {
  multipleAccounts: boolean;
  accountSwitching: boolean;
  liveSwitching: boolean;
  isolatedLogin: boolean;
  loginReplacesCurrent: boolean;
  usageMonitoring: Support;
  restartRequired: Support;
}

export interface ProviderInfo {
  id: ProviderId;
  name: string;
  installed: boolean;
  version: string | null;
  binaryPath: string | null;
  configPath: string | null;
  compatibility: Compatibility;
  compatibilityNote: string | null;
  experimental: boolean;
  capabilities: Capabilities;
  processes: ProcessInfo[];
  switchMode: SwitchMode;
}

export interface UsageRow {
  accountId: string;
  provider: ProviderId;
  usageType: string;
  label: string;
  usedPercent: number | null;
  usedValue: number | null;
  limitValue: number | null;
  windowMinutes: number | null;
  resetAt: string | null;
  source: string;
  confidence: string;
  observedAt: string;
  retrievedAt: string;
}

export interface Account {
  id: string;
  provider: ProviderId;
  label: string;
  category: string | null;
  email: string | null;
  accountIdentifier: string;
  displayName: string | null;
  organization: string | null;
  subscriptionType: string | null;
  authMethod: string | null;
  profileReference: string;
  status: string;
  health: AuthHealth;
  favorite: boolean;
  sortOrder: number;
  createdAt: string;
  lastAuthenticatedAt: string | null;
  lastUsedAt: string | null;
  lastCapturedAt: string | null;
  lastError: string | null;
}

export interface AccountView extends Account {
  active: boolean;
  activationState: string | null;
  verifiedAt: string | null;
  usage: UsageRow[];
  usageError: string | null;
}

export interface LiveAccount {
  identity: Identity;
  health: AuthHealth;
  savedAccountId: string | null;
}

export interface ProviderState {
  info: ProviderInfo;
  accounts: AccountView[];
  activeAccountId: string | null;
  activationState: string | null;
  live: LiveAccount | null;
  liveError: string | null;
  switchingEnabled: boolean;
  switchingDisabledReason: string | null;
}

export interface SwitchRecord {
  id: string;
  provider: ProviderId;
  fromAccountId: string | null;
  toAccountId: string | null;
  fromLabel: string | null;
  toLabel: string | null;
  status: string;
  switchMode: string;
  errorCode: string | null;
  message: string | null;
  startedAt: string;
  completedAt: string | null;
}

export interface Overview {
  providers: ProviderState[];
  incomplete: SwitchRecord[];
  credentialStore: { backend: string; available: boolean; message: string | null };
  generatedAt: string;
}

export interface SwitchPlan {
  provider: ProviderId;
  accountId: string;
  label: string;
  mode: SwitchMode;
  alreadyActive: boolean;
  blocked: string | null;
  requiresConfirmation: boolean;
  restartApps: string[];
  runningSessions: number;
  warnings: string[];
}

export interface SwitchOutcome {
  operationId: string | null;
  provider: ProviderId;
  accountId: string;
  status: string;
  mode: SwitchMode;
  message: string;
  verification: string | null;
  restarted: string[];
  runningSessions: number;
}

export interface Progress {
  provider: ProviderId;
  accountId: string;
  stage: string;
  message: string;
}

export interface ActivityRow {
  id: number;
  at: string;
  provider: ProviderId | null;
  accountId: string | null;
  kind: string;
  message: string;
}

export interface Settings {
  onboarded: boolean;
  theme: "system" | "light" | "dark";
  launchAtLogin: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  showDockIcon: boolean;
  trayClick: "popover" | "dashboard";
  globalShortcut: string | null;
  notifications: boolean;
  notifySwitch: boolean;
  notifyUsage: boolean;
  notifyAuth: boolean;
  usageMonitoring: boolean;
  usageRefreshMinutes: number;
  usageThreshold: number;
  showResetCountdowns: boolean;
  warnRunningSessions: boolean;
  experimentalAntigravity: boolean;
  liveUsage: boolean;
  diagnosticLevel: string;
}

export interface AccountPatch {
  label?: string;
  category?: string | null;
  favorite?: boolean;
  sortOrder?: number;
}

export interface AppInfo {
  version: string;
  os: string;
  dataDir: string;
}

export interface ApiError {
  code: string;
  message: string;
}

export function errorText(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as ApiError).message);
  return String(e);
}

export function errorCode(e: unknown): string | undefined {
  if (e && typeof e === "object" && "code" in e) return String((e as ApiError).code);
  return undefined;
}

export const api = {
  overview: (refresh = false) => invoke<Overview>("get_overview", { refresh }),
  planSwitch: (accountId: string) => invoke<SwitchPlan>("plan_switch", { accountId }),
  switchAccount: (accountId: string, confirmRestart = false) =>
    invoke<SwitchOutcome>("switch_account", { accountId, confirmRestart }),
  importAccounts: (providerId: ProviderId) => invoke<Account[]>("import_accounts", { providerId }),
  saveCurrent: (providerId: ProviderId, label?: string, category?: string | null) =>
    invoke<Account>("save_current", { providerId, label: label || null, category: category || null }),
  startLogin: (providerId: ProviderId, label?: string, category?: string | null) =>
    invoke<string>("start_login", { providerId, label: label || null, category: category || null }),
  cancelLogin: (loginId: string) => invoke<void>("cancel_login", { loginId }),
  updateAccount: (id: string, patch: AccountPatch) => invoke<Account>("update_account", { id, patch }),
  reorder: (providerId: ProviderId, ids: string[]) => invoke<void>("reorder_accounts", { providerId, ids }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  history: (limit?: number) => invoke<SwitchRecord[]>("get_history", { limit }),
  activity: (limit?: number) => invoke<ActivityRow[]>("get_activity", { limit }),
  resolveIncomplete: (operationId: string, action: "restore-previous" | "keep-current") =>
    invoke<void>("resolve_incomplete", { operationId, action }),
  undoSwitch: (operationId: string) => invoke<void>("undo_switch", { operationId }),
  refreshUsage: () => invoke<UsageRow[]>("refresh_usage"),
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
