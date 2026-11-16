import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertTriangle, ExternalLink, KeyRound, LogIn, RotateCcw, Save, ShieldCheck } from "lucide-react";
import { api, errorText, type ApiError, type ProviderId, type ProviderState } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { Button } from "./ui/button";
import { Badge, Dialog, Field, Input, Select, Spinner } from "./ui/primitives";
import { ProviderIcon } from "./ProviderIcon";

const modeText = {
  immediate: "Takes effect immediately.",
  "new-sessions": "New sessions use the selected account. Sessions already running keep their current account.",
  "restart-required": "The app must restart to use the selected account.",
  manual: "Requires a manual step in the provider app.",
  unsupported: "Switching is not available.",
};

export function SwitchConfirmDialog() {
  const plan = useApp((s) => s.confirm);
  const close = useApp((s) => s.closeConfirm);
  const run = useApp((s) => s.runSwitch);
  const name = useApp((s) => s.overview?.providers.find((p) => p.info.id === plan?.provider)?.info.name);
  if (!plan) return null;
  const restart = plan.requiresConfirmation;
  return (
    <Dialog
      open
      onClose={close}
      title={restart ? `Restart ${plan.restartApps.join(" and ")}?` : `Switch ${name} to ${plan.label}?`}
      description={modeText[plan.mode]}
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button variant="primary" data-autofocus onClick={() => void run(plan.accountId, restart)}>
            {restart ? "Restart and switch" : "Switch"}
          </Button>
        </>
      }
    >
      <ul className="space-y-2">
        {plan.warnings.map((w) => (
          <li key={w} className="flex gap-2 rounded-xl bg-warn-soft px-3.5 py-2.5 text-[13px] text-text">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-warn" />
            <span>{w}</span>
          </li>
        ))}
      </ul>
      <p className="mt-3 flex items-center gap-1.5 text-[12px] text-muted">
        <ShieldCheck className="size-3.5" />
        A recovery snapshot is saved first. If verification fails, the previous account is restored.
      </p>
    </Dialog>
  );
}

const CATEGORIES = ["", "Personal", "Work", "University", "Secondary", "Testing"];

function LinkifiedLine({ line }: { line: string }) {
  const m = line.match(/https?:\/\/\S+/);
  if (!m) return <div className="break-words">{line}</div>;
  const url = m[0];
  return (
    <div className="break-words">
      {line.slice(0, m.index)}
      <button className="inline-flex items-center gap-1 text-accent underline-offset-2 hover:underline" onClick={() => void openUrl(url)}>
        {url.length > 60 ? url.slice(0, 60) + "…" : url}
        <ExternalLink className="size-3" />
      </button>
      {line.slice((m.index ?? 0) + url.length)}
    </div>
  );
}

export function AddAccountDialog() {
  const addFor = useApp((s) => s.addFor);
  const openAdd = useApp((s) => s.openAdd);
  const overview = useApp((s) => s.overview);
  const toast = useApp((s) => s.toast);
  const [provider, setProvider] = useState<ProviderId | null>(null);
  const [method, setMethod] = useState<"login" | "current">("login");
  const [label, setLabel] = useState("");
  const [category, setCategory] = useState("");
  const [busy, setBusy] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const [loginId, setLoginId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const loginRef = useRef<string | null>(null);

  const installed = useMemo(() => (overview?.providers ?? []).filter((p) => p.info.installed), [overview]);
  const state: ProviderState | undefined = installed.find((p) => p.info.id === provider);

  useEffect(() => {
    if (!addFor) return;
    const first = addFor === "any" ? (installed.length === 1 ? installed[0].info.id : null) : addFor;
    setProvider(first);
    setLabel("");
    setCategory("");
    setLines([]);
    setError(null);
    setBusy(false);
    setLoginId(null);
  }, [addFor]);

  useEffect(() => {
    if (!state) return;
    setMethod(state.info.capabilities.isolatedLogin ? "login" : "current");
  }, [state?.info.id]);

  useEffect(() => {
    const a = listen<{ loginId: string; line: string }>("altflip://login-output", (e) => {
      if (e.payload.loginId === loginRef.current) setLines((l) => [...l.slice(-40), e.payload.line]);
    });
    const b = listen<{ loginId: string; account: { label: string } | null; error: ApiError | null }>("altflip://login-done", (e) => {
      if (e.payload.loginId !== loginRef.current) return;
      loginRef.current = null;
      setLoginId(null);
      setBusy(false);
      if (e.payload.account) {
        toast({ kind: "success", text: `Added ${e.payload.account.label}`, detail: "Your active account was not changed." });
        openAdd(null);
      } else if (e.payload.error) {
        setError(e.payload.error.message);
      }
    });
    return () => {
      void a.then((f) => f());
      void b.then((f) => f());
    };
  }, [openAdd, toast]);

  const close = () => {
    if (loginRef.current) void api.cancelLogin(loginRef.current);
    loginRef.current = null;
    openAdd(null);
  };

  const submit = async () => {
    if (!provider) return;
    setError(null);
    setBusy(true);
    if (method === "login") {
      setLines([]);
      try {
        const id = await api.startLogin(provider, label, category || null);
        loginRef.current = id;
        setLoginId(id);
      } catch (e) {
        setBusy(false);
        setError(errorText(e));
      }
      return;
    }
    try {
      const row = await api.saveCurrent(provider, label, category || null);
      toast({ kind: "success", text: `Saved ${row.label}` });
      openAdd(null);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  if (!addFor) return null;
  const live = state?.live;
  const alreadySaved = live?.savedAccountId ? state?.accounts.find((a) => a.id === live.savedAccountId) : undefined;
  const canSubmit = !!provider && !busy && (method === "login" || !!live);

  return (
    <Dialog
      open
      onClose={close}
      width={500}
      title="Add account"
      description="Accounts are added through each tool's own sign-in. AltFlip never asks for your password."
      footer={
        <>
          <Button onClick={close}>{loginId ? "Cancel sign-in" : "Cancel"}</Button>
          <Button variant="primary" disabled={!canSubmit} onClick={() => void submit()}>
            {busy ? <Spinner /> : method === "login" ? <LogIn /> : <Save />}
            {method === "login" ? (busy ? "Waiting for sign-in" : "Sign in") : alreadySaved ? "Update saved account" : "Save account"}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {addFor === "any" && (
          <div className="grid grid-cols-3 gap-2">
            {installed.map((p) => (
              <button
                key={p.info.id}
                disabled={busy}
                onClick={() => setProvider(p.info.id)}
                className={cn(
                  "flex flex-col items-center gap-1.5 rounded-2xl border px-2 py-3.5 text-[12.5px] font-medium transition-colors",
                  provider === p.info.id ? "border-accent bg-accent-soft shadow-item" : "border-border bg-surface hover:border-border-strong",
                )}
              >
                <ProviderIcon id={p.info.id} size={24} />
                {p.info.name}
                {p.info.experimental && <Badge tone="warn">Experimental</Badge>}
              </button>
            ))}
            {installed.length === 0 && <p className="col-span-3 text-muted">No supported tools are installed.</p>}
          </div>
        )}

        {state && (
          <>
            <div className="space-y-2">
              {state.info.capabilities.isolatedLogin && (
                <MethodOption
                  selected={method === "login"}
                  disabled={busy}
                  onSelect={() => setMethod("login")}
                  icon={<KeyRound className="size-4" />}
                  title="Sign in with another account"
                  text={
                    state.info.capabilities.loginReplacesCurrent
                      ? `AltFlip saves your current ${state.info.name} account, opens ${state.info.name}, and saves the new account as soon as you sign in to it there. The new account becomes active.`
                      : `Runs ${state.info.id === "codex" ? "codex login" : "claude auth login"} in a private, temporary profile. Your current ${state.info.name} sign-in stays as it is.`
                  }
                />
              )}
              <MethodOption
                selected={method === "current"}
                disabled={busy}
                onSelect={() => setMethod("current")}
                icon={<Save className="size-4" />}
                title="Save the account that's signed in now"
                text={
                  live
                    ? `${state.info.name} is signed in as ${live.identity.email ?? live.identity.displayName ?? "an account"}.${alreadySaved ? ` Already saved as “${alreadySaved.label}”.` : ""}`
                    : `${state.info.name} is not signed in. Sign in with ${state.info.name} first, then come back.`
                }
              />
            </div>
            <div className="grid grid-cols-[1fr_150px] gap-3">
              <Field label="Name">
                <Input value={label} placeholder={live?.identity.email?.split("@")[0] ?? "Work"} onChange={(e) => setLabel(e.target.value)} disabled={busy} />
              </Field>
              <Field label="Category">
                <Select
                  label="Category"
                  className="w-full"
                  value={category}
                  onChange={setCategory}
                  options={CATEGORIES.map((c) => ({ value: c, label: c || "None" }))}
                />
              </Field>
            </div>
            <p className="text-[11.5px] text-faint">
              AltFlip reads the account email, plan and sign-in tokens for {state.info.name}, and stores the tokens encrypted on this device only.
            </p>
          </>
        )}

        {(lines.length > 0 || loginId) && (
          <div className="selectable max-h-40 overflow-auto rounded-2xl border border-border bg-canvas px-3.5 py-2.5 font-mono text-[11px] leading-5 text-muted">
            {lines.map((l, i) => (
              <LinkifiedLine key={i} line={l} />
            ))}
            {loginId && <div className="text-faint">Waiting for the browser sign-in to finish…</div>}
          </div>
        )}
        {error && (
          <div className="flex gap-2 rounded-xl bg-danger-soft px-3.5 py-2.5 text-[13px]">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-danger" />
            <span className="selectable">{error}</span>
          </div>
        )}
      </div>
    </Dialog>
  );
}

function MethodOption({
  selected,
  onSelect,
  icon,
  title,
  text,
  disabled,
}: {
  selected: boolean;
  onSelect: () => void;
  icon: React.ReactNode;
  title: string;
  text: string;
  disabled?: boolean;
}) {
  return (
    <button
      disabled={disabled}
      onClick={onSelect}
      className={cn(
        "flex w-full gap-3 rounded-2xl border px-4 py-3 text-left transition-colors",
        selected ? "border-accent bg-accent-soft/60 shadow-item" : "border-border bg-surface hover:border-border-strong",
      )}
    >
      <span className={cn("mt-0.5", selected ? "text-accent" : "text-muted")}>{icon}</span>
      <span>
        <span className="block font-medium">{title}</span>
        <span className="block text-[12px] text-muted">{text}</span>
      </span>
    </button>
  );
}

export function RecoveryBanner() {
  const incomplete = useApp((s) => s.overview?.incomplete ?? []);
  const toast = useApp((s) => s.toast);
  const [busy, setBusy] = useState(false);
  if (incomplete.length === 0) return null;
  const r = incomplete[0];
  const act = async (action: "restore-previous" | "keep-current") => {
    setBusy(true);
    try {
      await api.resolveIncomplete(r.id, action);
      toast({ kind: "success", text: action === "restore-previous" ? "Previous account restored" : "Kept the current sign-in" });
    } catch (e) {
      toast({ kind: "error", text: "Recovery failed", detail: errorText(e) });
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex items-start gap-3 rounded-[20px] border border-warn/40 bg-warn-soft px-5 py-4">
      <RotateCcw className="mt-0.5 size-4 shrink-0 text-warn" />
      <div className="min-w-0 flex-1">
        <div className="font-medium">A switch to {r.toLabel ?? "another account"} was interrupted</div>
        <p className="text-[12.5px] text-muted">
          AltFlip stopped while changing the {r.provider} sign-in. Restore the account that was active before, or keep whatever is signed in now.
        </p>
      </div>
      <div className="flex shrink-0 gap-2">
        <Button size="sm" disabled={busy} onClick={() => void act("keep-current")}>
          Keep current
        </Button>
        <Button size="sm" variant="primary" disabled={busy} onClick={() => void act("restore-previous")}>
          Restore previous
        </Button>
      </div>
    </div>
  );
}
