import { useEffect, useState } from "react";
import { ArrowLeft, ArrowRight, Check, CircleSlash, Plus } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/components/ProviderIcon";
import { AccountRow } from "@/components/AccountRow";
import { Button } from "@/components/ui/button";
import { Badge, Card, Spinner, Switch } from "@/components/ui/primitives";

const STEPS = ["Welcome", "Tools", "Import", "Accounts", "Preferences"];

export function Onboarding({ isMac }: { isMac: boolean }) {
  const overview = useApp((s) => s.overview);
  const settings = useApp((s) => s.settings);
  const setSetting = useApp((s) => s.setSetting);
  const openAdd = useApp((s) => s.openAdd);
  const toast = useApp((s) => s.toast);
  const load = useApp((s) => s.load);
  const [step, setStep] = useState(0);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [importing, setImporting] = useState(false);

  const providers = overview?.providers ?? [];
  const importable = providers.filter((p) => p.info.installed && p.live && !p.live.savedAccountId);

  useEffect(() => {
    setSelected(Object.fromEntries(importable.map((p) => [p.info.id, true])));
  }, [overview?.generatedAt]);

  const doImport = async () => {
    setImporting(true);
    for (const p of importable) {
      if (!selected[p.info.id]) continue;
      try {
        await api.saveCurrent(p.info.id);
      } catch (e) {
        toast({ kind: "error", text: `Could not import ${p.info.name}`, detail: errorText(e) });
      }
    }
    await load(true);
    setImporting(false);
    setStep(3);
  };

  const finish = async () => {
    await setSetting("onboarded", true);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="h-10 shrink-0" data-tauri-drag-region />
      <div className="mx-auto flex w-full max-w-[560px] flex-1 flex-col px-6 pb-8">
        <div className="mb-6 flex items-center gap-1.5" aria-label={`Step ${step + 1} of ${STEPS.length}`}>
          {STEPS.map((s, i) => (
            <span key={s} className={cn("h-1 flex-1 rounded-full", i <= step ? "bg-accent" : "bg-surface-3")} />
          ))}
        </div>

        {step === 0 && (
          <div className="animate-in my-auto text-center">
            <img src="/icon.png" alt="" className="mx-auto mb-5 size-20" />
            <h1 className="text-[24px] font-semibold tracking-tight">Manage all your AI coding accounts from one place.</h1>
            <p className="mx-auto mt-3 max-w-[440px] text-[13.5px] text-muted">
              Save each account once, then switch Claude Code, Codex and Antigravity from the {isMac ? "menu bar" : "system tray"}. Sign-ins stay
              on this computer, encrypted.
            </p>
            <Button variant="primary" size="lg" className="mt-7" onClick={() => setStep(1)}>
              Get started <ArrowRight />
            </Button>
          </div>
        )}

        {step === 1 && (
          <div className="animate-in">
            <h1 className="text-[20px] font-semibold tracking-tight">Installed tools</h1>
            <p className="mt-1 mb-5 text-muted">AltFlip only looked; nothing was changed.</p>
            <Card className="divide-y divide-border">
              {providers.map((p) => (
                <div key={p.info.id} className="flex items-center gap-3 px-4 py-3">
                  <ProviderIcon id={p.info.id} size={24} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 font-medium">
                      {p.info.name}
                      {p.info.experimental && p.info.installed && <Badge tone="warn">Experimental</Badge>}
                    </div>
                    <div className="truncate text-[12px] text-muted">{p.info.installed ? (p.info.version ?? "Installed") : "Not found"}</div>
                  </div>
                  {p.info.installed ? <Check className="size-4 text-ok" /> : <CircleSlash className="size-4 text-faint" />}
                </div>
              ))}
            </Card>
          </div>
        )}

        {step === 2 && (
          <div className="animate-in">
            <h1 className="text-[20px] font-semibold tracking-tight">Import accounts you're signed in to</h1>
            <p className="mt-1 mb-5 text-muted">
              AltFlip reads the account email, plan and sign-in tokens, and stores an encrypted copy. Your existing configuration is not modified.
            </p>
            {importable.length === 0 ? (
              <Card className="px-4 py-5 text-center text-[12.5px] text-muted">
                {providers.some((p) => p.live?.savedAccountId) ? "Your signed-in accounts are already saved." : "No signed-in accounts were found."}
              </Card>
            ) : (
              <Card className="divide-y divide-border">
                {importable.map((p) => (
                  <label key={p.info.id} className="flex items-center gap-3 px-4 py-3">
                    <ProviderIcon id={p.info.id} size={22} />
                    <div className="min-w-0 flex-1">
                      <div className="font-medium">{p.info.name}</div>
                      <div className="truncate text-[12px] text-muted">{p.live?.identity.email ?? p.live?.identity.displayName}</div>
                    </div>
                    <Switch
                      label={`Import ${p.info.name}`}
                      checked={!!selected[p.info.id]}
                      onChange={(v) => setSelected((s) => ({ ...s, [p.info.id]: v }))}
                    />
                  </label>
                ))}
              </Card>
            )}
          </div>
        )}

        {step === 3 && (
          <div className="animate-in">
            <h1 className="text-[20px] font-semibold tracking-tight">Add more accounts</h1>
            <p className="mt-1 mb-5 text-muted">Sign in to other accounts through each tool's official sign-in. You can do this later too.</p>
            <div className="space-y-3">
              {providers
                .filter((p) => p.info.installed)
                .map((p) => (
                  <Card key={p.info.id} className="p-2">
                    <div className="flex items-center gap-2 px-2 py-1">
                      <ProviderIcon id={p.info.id} size={16} />
                      <span className="flex-1 font-medium">{p.info.name}</span>
                      <Button size="sm" variant="ghost" onClick={() => openAdd(p.info.id)}>
                        <Plus /> Add
                      </Button>
                    </div>
                    {p.accounts.map((a) => (
                      <AccountRow key={a.id} account={a} provider={{ ...p, switchingEnabled: false }} />
                    ))}
                  </Card>
                ))}
            </div>
          </div>
        )}

        {step === 4 && settings && (
          <div className="animate-in">
            <h1 className="text-[20px] font-semibold tracking-tight">Background behavior</h1>
            <p className="mt-1 mb-5 text-muted">Change these any time in Settings.</p>
            <Card className="divide-y divide-border">
              {(
                [
                  ["launchAtLogin", "Launch at login"],
                  ["startMinimized", `Start in the ${isMac ? "menu bar" : "tray"}`],
                  ["notifications", "Show notifications"],
                  ["usageMonitoring", "Monitor usage where providers report it"],
                ] as const
              ).map(([k, label]) => (
                <div key={k} className="flex items-center px-4 py-3">
                  <span className="flex-1">{label}</span>
                  <Switch label={label} checked={settings[k]} onChange={(v) => void setSetting(k, v)} />
                </div>
              ))}
            </Card>
          </div>
        )}

        {step > 0 && (
          <div className="mt-auto flex items-center pt-6">
            <Button variant="ghost" onClick={() => setStep(step - 1)}>
              <ArrowLeft /> Back
            </Button>
            <span className="flex-1" />
            {step === 2 && importable.length > 0 ? (
              <>
                <Button variant="ghost" onClick={() => setStep(3)}>
                  Skip
                </Button>
                <Button variant="primary" disabled={importing || !Object.values(selected).some(Boolean)} onClick={() => void doImport()}>
                  {importing && <Spinner />} Import selected
                </Button>
              </>
            ) : step < 4 ? (
              <Button variant="primary" onClick={() => setStep(step + 1)}>
                Continue <ArrowRight />
              </Button>
            ) : (
              <Button variant="primary" onClick={() => void finish()}>
                Finish
              </Button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
