import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AccountView, Overview, ProviderId, ProviderState, Settings, SwitchRecord } from "@/lib/api";

const now = () => new Date().toISOString();
const ago = (m: number) => new Date(Date.now() - m * 60000).toISOString();
const ahead = (m: number) => new Date(Date.now() + m * 60000).toISOString();

function acct(id: string, provider: ProviderId, label: string, email: string, plan: string, extra: Partial<AccountView> = {}): AccountView {
  return {
    id,
    provider,
    label,
    category: null,
    email,
    accountIdentifier: `${provider}:${email}`,
    displayName: null,
    organization: null,
    subscriptionType: plan || null,
    authMethod: provider === "codex" ? "chatgpt" : provider === "antigravity" ? "consumer" : "claude.ai",
    profileReference: `accounts/${id}`,
    status: "ready",
    health: "ok",
    favorite: false,
    sortOrder: 0,
    createdAt: ago(60 * 24 * 9),
    lastAuthenticatedAt: ago(60 * 24 * 9),
    lastUsedAt: ago(40),
    lastCapturedAt: ago(12),
    lastError: null,
    active: false,
    activationState: null,
    verifiedAt: null,
    usage: [],
    usageError: null,
    ...extra,
  };
}

const caps = (isolated: boolean, usage: "no" | "conditional", replaces = false) => ({
  multipleAccounts: true,
  accountSwitching: true,
  liveSwitching: false,
  isolatedLogin: isolated,
  loginReplacesCurrent: replaces,
  usageMonitoring: usage,
  restartRequired: "conditional" as const,
});

const providers: ProviderState[] = [
  {
    info: {
      id: "claude-code",
      name: "Claude Code",
      installed: true,
      version: "2.1.312 (Claude Code)",
      binaryPath: "/Users/dev/.local/bin/claude",
      configPath: "/Users/dev/.claude",
      compatibility: "supported",
      compatibilityNote: null,
      experimental: false,
      capabilities: caps(true, "conditional"),
      processes: [{ pid: 1, name: "claude", path: null, kind: "cli" }],
      switchMode: "new-sessions",
    },
    accounts: [
      acct("c1", "claude-code", "Personal", "sam@example.dev", "max", { favorite: true }),
      acct("c2", "claude-code", "Work", "sam@acme.co", "team", { organization: "Acme" }),
      acct("c3", "claude-code", "Secondary", "sam.alt@example.dev", "pro", { health: "expired" }),
    ],
    activeAccountId: "c1",
    activationState: "verified",
    live: null,
    liveError: null,
    switchingEnabled: true,
    switchingDisabledReason: null,
  },
  {
    info: {
      id: "codex",
      name: "Codex",
      installed: true,
      version: "codex-cli 0.164.0",
      binaryPath: "/Users/dev/.local/bin/codex",
      configPath: "/Users/dev/.codex",
      compatibility: "supported",
      compatibilityNote: null,
      experimental: false,
      capabilities: caps(true, "conditional"),
      processes: [],
      switchMode: "new-sessions",
    },
    accounts: [acct("x1", "codex", "ChatGPT Plus", "sam@example.dev", "plus"), acct("x2", "codex", "Work", "sam@acme.co", "team")],
    activeAccountId: "x2",
    activationState: "verified",
    live: null,
    liveError: null,
    switchingEnabled: true,
    switchingDisabledReason: null,
  },
  {
    info: {
      id: "antigravity",
      name: "Antigravity",
      installed: true,
      version: "App 1.4.2 · agy 0.9.1",
      binaryPath: "/Applications/Antigravity.app",
      configPath: "/Users/dev/.gemini",
      compatibility: "experimental",
      compatibilityNote: "Experimental: switches the shared Antigravity sign-in.",
      experimental: true,
      capabilities: caps(true, "conditional", true),
      processes: [{ pid: 2, name: "Antigravity", path: "/Applications/Antigravity.app/Contents/MacOS/Antigravity", kind: "app" }],
      switchMode: "restart-required",
    },
    accounts: [acct("a1", "antigravity", "Google Main", "sam@gmail.com", ""), acct("a2", "antigravity", "Google Secondary", "sam.alt@gmail.com", "")],
    activeAccountId: "a1",
    activationState: "verified",
    live: null,
    liveError: null,
    switchingEnabled: false,
    switchingDisabledReason: "Antigravity switching is experimental. Turn it on in Settings › Switching to use it.",
  },
];

function sync() {
  for (const p of providers) {
    for (const a of p.accounts) {
      a.active = a.id === p.activeAccountId;
      a.activationState = a.active ? "verified" : null;
    }
    const active = p.accounts.find((a) => a.active)!;
    p.live = {
      identity: { key: active.accountIdentifier, email: active.email, displayName: null, organization: null, plan: null, authMethod: null },
      health: "ok",
      savedAccountId: active.id,
    };
  }
}
sync();

providers[1].accounts[1].usage = [
  { accountId: "x2", provider: "codex", usageType: "primary", label: "5-hour usage", usedPercent: 42, usedValue: null, limitValue: null, windowMinutes: 300, resetAt: ahead(195), source: "Codex session log", confidence: "provider-reported", observedAt: ago(8), retrievedAt: ago(2) },
  { accountId: "x2", provider: "codex", usageType: "secondary", label: "Weekly usage", usedPercent: 63, usedValue: null, limitValue: null, windowMinutes: 10080, resetAt: ahead(3000), source: "Codex session log", confidence: "provider-reported", observedAt: ago(8), retrievedAt: ago(2) },
];

providers[2].accounts[0].usage = [
  { accountId: "a2", provider: "antigravity", usageType: "quota:gemini-5h", label: "Gemini · 5-hour", usedPercent: 35, usedValue: null, limitValue: null, windowMinutes: 300, resetAt: ahead(180), source: "Antigravity (live)", confidence: "provider-reported", observedAt: ago(1), retrievedAt: ago(1) },
  { accountId: "a2", provider: "antigravity", usageType: "quota:gemini-weekly", label: "Gemini · Weekly", usedPercent: 52, usedValue: null, limitValue: null, windowMinutes: 10080, resetAt: ahead(4000), source: "Antigravity (live)", confidence: "provider-reported", observedAt: ago(1), retrievedAt: ago(1) },
  { accountId: "a2", provider: "antigravity", usageType: "quota:3p-weekly", label: "Claude and GPT · Weekly", usedPercent: 91, usedValue: null, limitValue: null, windowMinutes: 10080, resetAt: ahead(4000), source: "Antigravity (live)", confidence: "provider-reported", observedAt: ago(1), retrievedAt: ago(1) },
  { accountId: "a1", provider: "antigravity", usageType: "model:flash", label: "Gemini 3.8 Flash (High)", usedPercent: 12, usedValue: null, limitValue: null, windowMinutes: null, resetAt: ahead(140), source: "Antigravity (local)", confidence: "provider-reported", observedAt: ago(20), retrievedAt: ago(2) },
  { accountId: "a1", provider: "antigravity", usageType: "model:sonnet", label: "Claude Sonnet 4.6 (Thinking)", usedPercent: 84, usedValue: null, limitValue: null, windowMinutes: null, resetAt: ahead(60), source: "Antigravity (local)", confidence: "provider-reported", observedAt: ago(20), retrievedAt: ago(2) },
];

let settings: Settings = {
  onboarded: new URLSearchParams(location.search).get("onboarding") !== "1",
  theme: "system",
  launchAtLogin: false,
  startMinimized: true,
  closeToTray: true,
  showDockIcon: false,
  trayClick: "popover",
  globalShortcut: null,
  notifications: true,
  notifySwitch: true,
  notifyUsage: true,
  notifyAuth: true,
  usageMonitoring: true,
  usageRefreshMinutes: 15,
  usageThreshold: 80,
  showResetCountdowns: true,
  warnRunningSessions: true,
  experimentalAntigravity: false,
  liveUsage: true,
  diagnosticLevel: "info",
};

const history: SwitchRecord[] = [
  { id: "h1", provider: "codex", fromAccountId: "x1", toAccountId: "x2", fromLabel: "ChatGPT Plus", toLabel: "Work", status: "succeeded", switchMode: "new-sessions", errorCode: null, message: null, startedAt: ago(30), completedAt: ago(30) },
  { id: "h2", provider: "claude-code", fromAccountId: "c2", toAccountId: "c1", fromLabel: "Work", toLabel: "Personal", status: "succeeded", switchMode: "new-sessions", errorCode: null, message: null, startedAt: ago(95), completedAt: ago(95) },
  { id: "h3", provider: "claude-code", fromAccountId: "c1", toAccountId: "c3", fromLabel: "Personal", toLabel: "Secondary", status: "rolled-back", switchMode: "new-sessions", errorCode: "verification_failed", message: "claude auth status reports that no account is signed in", startedAt: ago(200), completedAt: ago(200) },
];

const overview = (): Overview => ({
  providers: structuredClone(providers),
  incomplete: [],
  credentialStore: { backend: "macOS Keychain", available: true, message: null },
  generatedAt: now(),
});

export function installMocks() {
  mockWindows(new URLSearchParams(location.search).get("surface") === "quick" ? "quick" : "main");
  let statusLine = { installed: false, scriptPath: "~/.claude/altflip-status.sh", chainedCommand: null as string | null, lastReadingAt: null as string | null };
  mockIPC(
    async (cmd, raw) => {
      const args = (raw ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "get_overview":
          return overview();
        case "get_settings":
          return settings;
        case "claude_status_line":
          return statusLine;
        case "set_claude_status_line":
          statusLine = { ...statusLine, installed: !!args.on, lastReadingAt: args.on ? new Date().toISOString() : null };
          return statusLine;
        case "set_setting":
          settings = { ...settings, [args.key as string]: args.value };
          if (args.key === "experimentalAntigravity") {
            providers[2].switchingEnabled = !!args.value;
            providers[2].switchingDisabledReason = args.value ? null : "Antigravity switching is experimental.";
          }
          return settings;
        case "app_info":
          return { version: "0.1.0", os: "macos", dataDir: "/Users/dev/Library/Application Support/io.coreify.altflip" };
        case "get_history":
          return history;
        case "get_activity":
          return history.map((h, i) => ({ id: i, at: h.startedAt, provider: h.provider, accountId: h.toAccountId, kind: "switched", message: `${h.provider} → ${h.toLabel}` }));
        case "plan_switch": {
          const p = providers.find((p) => p.accounts.some((a) => a.id === args.accountId))!;
          const a = p.accounts.find((a) => a.id === args.accountId)!;
          const restart = p.info.id === "antigravity";
          const cli = p.info.processes.filter((x) => x.kind === "cli").length;
          return {
            provider: p.info.id,
            accountId: a.id,
            label: a.label,
            mode: p.info.switchMode,
            alreadyActive: p.activeAccountId === a.id,
            blocked: p.switchingEnabled ? null : p.switchingDisabledReason,
            requiresConfirmation: restart,
            restartApps: restart ? ["Antigravity"] : [],
            runningSessions: cli,
            warnings: cli
              ? [`${cli} running ${p.info.name} session will keep the previous account until restarted.`]
              : restart
                ? ["Antigravity will be closed and reopened. Save your work first."]
                : [],
          };
        }
        case "switch_account": {
          await new Promise((r) => setTimeout(r, 700));
          const p = providers.find((p) => p.accounts.some((a) => a.id === args.accountId))!;
          p.activeAccountId = args.accountId as string;
          sync();
          const a = p.accounts.find((a) => a.id === args.accountId)!;
          return { operationId: "op", provider: p.info.id, accountId: a.id, status: "succeeded", mode: p.info.switchMode, message: `${p.info.name} is now using ${a.label}`, verification: "Verified", restarted: [], runningSessions: 0 };
        }
        case "refresh_usage":
          return [];
        case "diagnostics":
          return "{}";
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
