import { useEffect, useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Copy, Download, FolderOpen, RotateCcw, Upload } from "lucide-react";
import { api, errorText, type AppInfo, type Settings } from "@/lib/api";
import { useApp } from "@/lib/store";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { compatText, compatTone, modeShort } from "@/components/labels";
import { Button } from "@/components/ui/button";
import { Badge, Card, Dialog, Field, Input, Select, Switch } from "@/components/ui/primitives";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section>
      <h2 className="mb-2.5 label-mono text-faint">{title}</h2>
      <Card className="divide-y divide-border overflow-hidden">{children}</Card>
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-4 px-5 py-3.5">
      <div className="min-w-0 flex-1">
        <div className="font-medium">{label}</div>
        {hint && <div className="text-[12px] text-muted">{hint}</div>}
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
        className="h-8 min-w-40 rounded-md border border-border bg-surface px-2.5 text-left text-[12.5px] focus:border-accent"
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

export function SettingsPage({ isMac, info }: { isMac: boolean; info: AppInfo | null }) {
  const settings = useApp((s) => s.settings);
  const overview = useApp((s) => s.overview);
  const set = useApp((s) => s.setSetting);
  const toast = useApp((s) => s.toast);
  const [backup, setBackup] = useState<"export" | "import" | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  if (!settings) return null;

  return (
    <>
      <PageHeader title="Settings" />
      <Section title="General">
        <Row label="Appearance">
          <Select
            label="Appearance"
            value={settings.theme}
            onChange={(v) => void set("theme", v)}
            options={[
              { value: "system", label: "Match system" },
              { value: "light", label: "Light" },
              { value: "dark", label: "Dark" },
            ]}
          />
        </Row>
        <Toggle k="launchAtLogin" label="Launch at login" />
        <Toggle k="startMinimized" label="Start in the background" hint={`Open only the ${isMac ? "menu bar" : "tray"} icon when AltFlip starts.`} />
        <Toggle k="closeToTray" label={`Keep running when the window is closed`} hint={`AltFlip stays in the ${isMac ? "menu bar" : "system tray"} until you choose Quit.`} />
        {isMac && <Toggle k="showDockIcon" label="Always show Dock icon" />}
        <Row label={`${isMac ? "Menu bar" : "Tray"} icon click`}>
          <Select
            label="Icon click"
            value={settings.trayClick}
            onChange={(v) => void set("trayClick", v)}
            options={[
              { value: "popover", label: "Open quick switcher" },
              { value: "dashboard", label: "Open this window" },
            ]}
          />
        </Row>
        <Row label="Quick switcher shortcut" hint="Opens the account switcher from anywhere.">
          <ShortcutRecorder />
        </Row>
      </Section>

      <Section title="Switching">
        <Toggle k="warnRunningSessions" label="Warn about running sessions" hint="Ask before switching while a session of that tool is running." />
        <Row label="Verification and rollback" hint="Every switch is verified. If it fails, the previous account is restored automatically.">
          <Badge tone="ok">Always on</Badge>
        </Row>
        <Row label="Restarting apps" hint="AltFlip always asks before closing an app such as Antigravity.">
          <Badge tone="ok">Always asks</Badge>
        </Row>
      </Section>

      <Section title="Notifications">
        <Toggle k="notifications" label="Show notifications" />
        <Toggle k="notifySwitch" label="Switch results" hint="For switches started from the menu or shortcut." disabled={!settings.notifications} />
        <Toggle k="notifyUsage" label="Usage warnings" disabled={!settings.notifications} />
        <Toggle k="notifyAuth" label="Expired sign-ins" disabled={!settings.notifications} />
      </Section>

      <Section title="Usage monitoring">
        <Toggle k="usageMonitoring" label="Monitor usage" hint="Reads usage that Codex and Antigravity record on this computer." />
        <Row label="Check every">
          <Select
            label="Refresh interval"
            value={settings.usageRefreshMinutes}
            onChange={(v) => void set("usageRefreshMinutes", v)}
            options={[5, 15, 30, 60, 120].map((m) => ({ value: m, label: m < 60 ? `${m} minutes` : `${m / 60} hour${m > 60 ? "s" : ""}` }))}
          />
        </Row>
        <Row label="Warn at">
          <Select
            label="Warning threshold"
            value={settings.usageThreshold}
            onChange={(v) => void set("usageThreshold", v)}
            options={[50, 70, 80, 90, 95, 100].map((v) => ({ value: v, label: `${v}% used` }))}
          />
        </Row>
        <Toggle
          k="claudeExactUsage"
          label="Claude usage from Anthropic"
          hint="Sends each saved Claude login to api.anthropic.com to read 5-hour and weekly limits, renewing expired logins like Claude Code does. Off by default."
        />
        <Toggle k="showResetCountdowns" label="Show reset countdowns" />
      </Section>

      <Section title="Privacy and security">
        <Row label="Credential storage" hint={overview?.credentialStore.message ?? "Saved sign-ins are encrypted. The key is kept in the system credential store."}>
          <Badge tone={overview?.credentialStore.available ? "ok" : "danger"}>
            {overview?.credentialStore.backend} {overview?.credentialStore.available ? "ready" : "unavailable"}
          </Badge>
        </Row>
        <Row label="Backup" hint="Export or restore saved accounts as a passphrase-protected file.">
          <div className="flex gap-2">
            <Button size="sm" onClick={() => setBackup("export")}>
              <Download /> Export
            </Button>
            <Button size="sm" onClick={() => setBackup("import")}>
              <Upload /> Restore
            </Button>
          </div>
        </Row>
        <Row label="Data folder" hint={<span className="selectable">{info?.dataDir}</span>}>
          <Button size="sm" disabled={!info} onClick={() => info && void revealItemInDir(info.dataDir)}>
            <FolderOpen /> Show
          </Button>
        </Row>
        <Row label="Reset settings" hint="Restores default preferences. Saved accounts are kept.">
          <Button size="sm" onClick={() => setConfirmReset(true)}>
            <RotateCcw /> Reset
          </Button>
        </Row>
      </Section>

      <Section title="Advanced">
        <Toggle
          k="experimentalAntigravity"
          label="Antigravity switching (experimental)"
          hint="Switches the Google sign-in shared by the Antigravity app, agy CLI and IDE 2.6+. Running Antigravity apps are restarted after you confirm."
        />
        {overview?.providers.map((p) => (
          <div key={p.info.id} className="flex items-start gap-3 px-5 py-3.5">
            <ProviderIcon id={p.info.id} size={18} className="mt-0.5" />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="font-medium">{p.info.name}</span>
                <Badge tone={compatTone[p.info.compatibility]}>{compatText[p.info.compatibility]}</Badge>
              </div>
              <div className="selectable mt-0.5 space-y-0.5 text-[11.5px] text-muted">
                <div>Version: {p.info.version ?? "unknown"}</div>
                {p.info.binaryPath && <div className="truncate">Program: {p.info.binaryPath}</div>}
                {p.info.configPath && <div className="truncate">Config: {p.info.configPath}</div>}
                <div>Switch mode: {modeShort[p.info.switchMode]}</div>
                {p.info.compatibilityNote && <div className="text-faint">{p.info.compatibilityNote}</div>}
              </div>
            </div>
          </div>
        ))}
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

      <BackupDialog mode={backup} onClose={() => setBackup(null)} />
      <Dialog
        open={confirmReset}
        onClose={() => setConfirmReset(false)}
        title="Reset all settings?"
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
