import { useEffect, useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Copy, Download, FolderOpen, RotateCcw, Upload } from "lucide-react";
import { api, errorText, type AppInfo, type Settings, type StatusLineState } from "@/lib/api";
import { relativeTime } from "@/lib/utils";
import { useApp } from "@/lib/store";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { compatText, compatTone, modeShort } from "@/components/labels";
import { Button } from "@/components/ui/button";
import { Badge, Card, Dialog, Field, Input, SectionLabel, Segmented, Select, Switch } from "@/components/ui/primitives";
import { SETTINGS_GROUPS } from "@/components/settingsNav";

function Section({ title, right, children }: { title?: string; right?: React.ReactNode; children: React.ReactNode }) {
  return (
    <section className="pb-4 last:pb-0">
      {title && <SectionLabel right={right}>{title}</SectionLabel>}
      <Card className="divide-y divide-border px-5 py-1">{children}</Card>
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex min-h-14 flex-wrap items-center justify-between gap-x-6 gap-y-2 py-3">
      <div className="min-w-0 flex-1 basis-56">
        <div className="text-[13px] font-medium">{label}</div>
        {hint && <div className="text-[12px] text-faint">{hint}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Toggle<K extends keyof Settings>({ k, label, hint, disabled }: { k: K; label: string; hint?: string; disabled?: boolean }) {
  const value = useApp((s) => s.settings?.[k]) as boolean;
  const set = useApp((s) => s.setSetting);
  return (
    <Row label={label} hint={hint}>
      <Switch label={label} checked={!!value} disabled={disabled} onChange={(v) => void set(k, v as Settings[K])} />
    </Row>
  );
}

function ShortcutRecorder() {
  const value = useApp((s) => s.settings?.globalShortcut ?? null);
  const set = useApp((s) => s.setSetting);
  const [recording, setRecording] = useState(false);
  return (
    <div className="flex items-center gap-2">
      <button
        className="h-9 min-w-44 rounded-xl border border-border bg-card px-3 text-left text-[13px] shadow-item transition-colors hover:border-border-strong focus:border-accent focus:outline-none"
        onClick={() => setRecording(true)}
        onBlur={() => setRecording(false)}
        onKeyDown={(e) => {
          if (!recording) return;
          e.preventDefault();
          if (e.key === "Escape") return setRecording(false);
          if (["Meta", "Control", "Shift", "Alt"].includes(e.key)) return;
          const mods = [e.metaKey || e.ctrlKey ? "CommandOrControl" : "", e.altKey ? "Alt" : "", e.shiftKey ? "Shift" : ""].filter(Boolean);
          if (mods.length === 0) return;
          const key = e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.code;
          setRecording(false);
          void set("globalShortcut", [...mods, key].join("+"));
        }}
      >
        {recording ? <span className="text-accent">Press keys…</span> : value ? value.replace("CommandOrControl", "Cmd/Ctrl") : <span className="text-faint">Not set</span>}
      </button>
      {value && (
        <Button size="sm" variant="ghost" onClick={() => void set("globalShortcut", null)}>
          Clear
        </Button>
      )}
    </div>
  );
}

function BackupDialog({ mode, onClose }: { mode: "export" | "import" | null; onClose: () => void }) {
  const toast = useApp((s) => s.toast);
  const [pass, setPass] = useState("");
  const [pass2, setPass2] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    setPass("");
    setPass2("");
  }, [mode]);
  if (!mode) return null;
  const valid = pass.length >= 8 && (mode === "import" || pass === pass2);
  const run = async () => {
    const path =
      mode === "export"
        ? await saveDialog({ defaultPath: `altflip-backup-${new Date().toISOString().slice(0, 10)}.altflip`, filters: [{ name: "AltFlip backup", extensions: ["altflip"] }] })
        : await openDialog({ multiple: false, filters: [{ name: "AltFlip backup", extensions: ["altflip"] }] });
    if (!path || Array.isArray(path)) return;
    setBusy(true);
    try {
      const n = mode === "export" ? await api.exportBackup(path, pass) : await api.importBackup(path, pass);
      toast({ kind: "success", text: mode === "export" ? `Exported ${n} accounts` : `Restored ${n} accounts` });
      onClose();
    } catch (e) {
      toast({ kind: "error", text: mode === "export" ? "Export failed" : "Restore failed", detail: errorText(e) });
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onClose={onClose}
      title={mode === "export" ? "Export encrypted backup" : "Restore from backup"}
      description={
        mode === "export"
          ? "The file contains your saved sign-ins, encrypted with this passphrase. Anyone with the file and passphrase can use those accounts, so keep both private."
          : "Accounts from the backup are added or updated. Your active accounts are not changed."
      }
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!valid || busy} onClick={() => void run()}>
            {mode === "export" ? "Choose location…" : "Choose file…"}
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        <Field label="Passphrase" hint="At least 8 characters.">
          <Input type="password" value={pass} onChange={(e) => setPass(e.target.value)} autoComplete="new-password" />
        </Field>
        {mode === "export" && (
          <Field label="Confirm passphrase">
            <Input type="password" value={pass2} onChange={(e) => setPass2(e.target.value)} autoComplete="new-password" />
          </Field>
        )}
      </div>
    </Dialog>
  );
}

function ClaudeStatusLineRow({ disabled }: { disabled: boolean }) {
  const toast = useApp((s) => s.toast);
  const [st, setSt] = useState<StatusLineState | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void api.claudeStatusLine().then(setSt).catch(() => setSt(null));
  }, []);
  const toggle = async (on: boolean) => {
    setBusy(true);
    try {
      setSt(await api.setClaudeStatusLine(on));
      toast({ kind: "success", text: on ? "Claude Code status line installed" : "Claude Code status line removed" });
    } catch (e) {
      toast({ kind: "error", text: "Could not change Claude Code's status line", detail: errorText(e) });
    } finally {
      setBusy(false);
    }
  };
  const hint = !st?.installed
    ? "Claude Code reports usage to its status line after every reply. AltFlip adds a small script there so the Usage page updates instantly and never waits on Anthropic."
    : [
        st.lastReadingAt ? `Last reading ${relativeTime(st.lastReadingAt)}.` : "Waiting for the next Claude Code reply.",
        st.chainedCommand ? `Your previous status line still runs: ${st.chainedCommand}` : null,
      ]
        .filter(Boolean)
        .join(" ");
  return (
    <Row label="Instant usage from Claude Code" hint={hint}>
      <Switch
        label="Instant usage from Claude Code"
        checked={!!st?.installed}
        disabled={disabled || busy || !st}
        onChange={(v) => void toggle(v)}
      />
    </Row>
  );
}

export function SettingsPage({ isMac, info }: { isMac: boolean; info: AppInfo | null }) {
  const settings = useApp((s) => s.settings);
  const overview = useApp((s) => s.overview);
  const set = useApp((s) => s.setSetting);
  const toast = useApp((s) => s.toast);
  const [backup, setBackup] = useState<"export" | "import" | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  const settingsTab = useApp((s) => s.settingsTab);
  if (!settings) return null;

  const tab = settingsTab;
  const group = SETTINGS_GROUPS.find((g) => g.id === tab) ?? SETTINGS_GROUPS[0];
  const shell = isMac ? "menu bar" : "system tray";

  return (
    <>
      <PageHeader title={group.label} subtitle={group.subtitle} />
      <div key={tab} className="animate-in">
        {tab === "general" && (
          <>
            <Section title="Appearance">
              <Row label="Theme" hint="Match your system, or keep AltFlip light or dark.">
                <Segmented
                  label="Theme"
                  value={settings.theme}
                  onChange={(v) => void set("theme", v)}
                  options={[
                    { value: "system", label: "System" },
                    { value: "light", label: "Light" },
                    { value: "dark", label: "Dark" },
                  ]}
                />
              </Row>
            </Section>
            <Section title="Startup">
              <Toggle k="launchAtLogin" label="Launch at login" hint={`Start AltFlip in the ${shell} when you log in.`} />
              <Toggle k="startMinimized" label="Start in the background" hint={`Open only the ${shell} icon when AltFlip starts.`} />
            </Section>
            <Section title="Window">
              <Toggle k="closeToTray" label="Keep running when the window is closed" hint={`AltFlip stays in the ${shell} until you choose Quit.`} />
              {isMac && <Toggle k="showDockIcon" label="Always show Dock icon" />}
              <Row label={`${isMac ? "Menu bar" : "Tray"} icon click`} hint="What opens when you click the icon.">
                <Segmented
                  label="Icon click"
                  value={settings.trayClick}
                  onChange={(v) => void set("trayClick", v)}
                  options={[
                    { value: "popover", label: "Quick switcher" },
                    { value: "dashboard", label: "Overview" },
                  ]}
                />
              </Row>
            </Section>
            <Section title="Global shortcut">
              <Row label="Quick switcher" hint="Opens the account switcher from anywhere.">
                <ShortcutRecorder />
              </Row>
            </Section>
          </>
        )}

        {tab === "switching" && (
          <>
            <Section title="Before switching">
              <Toggle k="warnRunningSessions" label="Warn about running sessions" hint="Ask before switching while a session of that tool is running." />
              <Row label="Restarting apps" hint="AltFlip always asks before closing an app such as Antigravity.">
                <Badge tone="ok">Always asks</Badge>
              </Row>
            </Section>
            <Section title="Safety">
              <Row label="Verification and rollback" hint="Every switch is verified. If it fails, the previous account is restored automatically.">
                <Badge tone="ok">Always on</Badge>
              </Row>
              <Row label="Recovery snapshot" hint="The current sign-in is saved, encrypted, before every switch.">
                <Badge tone="ok">Always on</Badge>
              </Row>
            </Section>
            <Section title="Experimental">
              <Toggle
                k="experimentalAntigravity"
                label="Antigravity switching"
                hint="Switches the Google sign-in shared by the Antigravity app, agy CLI and IDE 2.6+. Running Antigravity apps are restarted after you confirm."
              />
            </Section>
          </>
        )}

        {tab === "notifications" && (
          <>
            <Section>
              <Toggle k="notifications" label="Show notifications" hint="Turn this off to silence everything below." />
            </Section>
            <Section title="Notify me about">
              <Toggle k="notifySwitch" label="Switch results" hint="For switches started from the menu or shortcut." disabled={!settings.notifications} />
              <Toggle k="notifyUsage" label="Usage warnings" hint={`When an account passes ${settings.usageThreshold}% of a limit.`} disabled={!settings.notifications} />
              <Toggle k="notifyAuth" label="Sign-ins that need login" hint="Includes the command to run to sign in again." disabled={!settings.notifications} />
            </Section>
          </>
        )}

        {tab === "usage" && (
          <>
            <Section>
              <Toggle k="usageMonitoring" label="Monitor usage" hint="Shows 5-hour, weekly and per-group limits for every saved account." />
            </Section>
            <Section title="Source">
              <Toggle
                k="liveUsage"
                label="Live usage from Anthropic and OpenAI"
                hint="The same numbers Claude Code's /usage and Codex's usage panel show, checked at most once a minute per account. Antigravity is read from the running app on this computer."
                disabled={!settings.usageMonitoring}
              />
              <ClaudeStatusLineRow disabled={!settings.usageMonitoring} />
              <Row label="Check in the background every" hint="While the window is open, usage updates every minute.">
                <Select
                  label="Refresh interval"
                  value={settings.usageRefreshMinutes}
                  onChange={(v) => void set("usageRefreshMinutes", v)}
                  options={[5, 15, 30, 60, 120].map((m) => ({ value: m, label: m < 60 ? `${m} minutes` : `${m / 60} hour${m > 60 ? "s" : ""}` }))}
                />
              </Row>
            </Section>
            <Section title="Display">
              <Row label="Warn at" hint="Shown in the menu and, if enabled, as a notification.">
                <Select
                  label="Warning threshold"
                  value={settings.usageThreshold}
                  onChange={(v) => void set("usageThreshold", v)}
                  options={[50, 70, 80, 90, 95, 100].map((v) => ({ value: v, label: `${v}% used` }))}
                />
              </Row>
              <Toggle k="showResetCountdowns" label="Show reset countdowns" />
            </Section>
          </>
        )}

        {tab === "privacy" && (
          <>
            <Section title="Credentials">
              <Row label="Credential storage" hint={overview?.credentialStore.message ?? "Saved sign-ins are encrypted. The key is kept in the system credential store."}>
                <Badge tone={overview?.credentialStore.available ? "ok" : "danger"}>
                  {overview?.credentialStore.backend} {overview?.credentialStore.available ? "ready" : "unavailable"}
                </Badge>
              </Row>
            </Section>
            <Section title="Backups">
              <Row label="Encrypted backup" hint="Export or restore saved accounts as a passphrase-protected file.">
                <div className="flex gap-2">
                  <Button size="sm" onClick={() => setBackup("export")}>
                    <Download /> Export
                  </Button>
                  <Button size="sm" onClick={() => setBackup("import")}>
                    <Upload /> Restore
                  </Button>
                </div>
              </Row>
            </Section>
            <Section title="Storage">
              <Row label="Data folder" hint={<span className="selectable break-all">{info?.dataDir}</span>}>
                <Button size="sm" disabled={!info} onClick={() => info && void revealItemInDir(info.dataDir)}>
                  <FolderOpen /> Show
                </Button>
              </Row>
              <Row label="Reset settings" hint="Restores default preferences. Saved accounts are kept.">
                <Button size="sm" variant="danger" onClick={() => setConfirmReset(true)}>
                  <RotateCcw /> Reset
                </Button>
              </Row>
            </Section>
          </>
        )}

        {tab === "tools" && (
          <>
            {overview?.providers.map((p) => (
              <Section key={p.info.id} title={p.info.name} right={<Badge tone={compatTone[p.info.compatibility]}>{compatText[p.info.compatibility]}</Badge>}>
                <div className="flex items-center gap-3 py-3.5">
                  <ProviderIcon id={p.info.id} size={30} />
                  <div className="min-w-0 flex-1">
                    <div className="text-[13px] font-medium">{p.info.installed ? `Version ${p.info.version ?? "unknown"}` : "Not installed"}</div>
                    <div className="text-[12px] text-faint">{modeShort[p.info.switchMode]}</div>
                  </div>
                  <span className="font-mono text-[11px] text-faint">
                    {p.accounts.length} saved
                  </span>
                </div>
                {p.info.binaryPath && (
                  <Row label="Program">
                    <span className="selectable block max-w-[22rem] truncate font-mono text-[11.5px] text-muted">{p.info.binaryPath}</span>
                  </Row>
                )}
                {p.info.configPath && (
                  <Row label="Config">
                    <span className="selectable block max-w-[22rem] truncate font-mono text-[11.5px] text-muted">{p.info.configPath}</span>
                  </Row>
                )}
                {p.info.compatibilityNote && <div className="py-3 text-[12px] text-faint">{p.info.compatibilityNote}</div>}
              </Section>
            ))}
            <Section title="Support">
              <Row label="Diagnostics" hint="Copies a report without emails, tokens or file contents.">
                <Button
                  size="sm"
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(await api.diagnostics());
                      toast({ kind: "success", text: "Diagnostics copied" });
                    } catch (e) {
                      toast({ kind: "error", text: "Could not copy diagnostics", detail: errorText(e) });
                    }
                  }}
                >
                  <Copy /> Copy report
                </Button>
              </Row>
            </Section>
          </>
        )}
      </div>

      <BackupDialog mode={backup} onClose={() => setBackup(null)} />
      <Dialog
        open={confirmReset}
        onClose={() => setConfirmReset(false)}
        title="Reset all settings?"
        icon={<RotateCcw />}
        tone="danger"
        description="Preferences return to their defaults. Saved accounts, history and backups are not affected."
        footer={
          <>
            <Button onClick={() => setConfirmReset(false)}>Cancel</Button>
            <Button
              variant="danger"
              onClick={async () => {
                setConfirmReset(false);
                try {
                  useApp.setState({ settings: await api.resetSettings() });
                  toast({ kind: "success", text: "Settings reset" });
                } catch (e) {
                  toast({ kind: "error", text: "Could not reset", detail: errorText(e) });
                }
              }}
            >
              Reset settings
            </Button>
          </>
        }
      />
    </>
  );
}
