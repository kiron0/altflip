import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertTriangle, Check, Download, ExternalLink, KeyRound, LogIn, RotateCcw, Save, ShieldCheck } from "lucide-react";
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

const AGY_STEPS: { title: string; text: string; done: RegExp }[] = [
  { title: "Save the current account", text: "AltFlip keeps your current Antigravity account so you can switch back to it later.", done: /Saved the current|Closing Antigravity|Signed Antigravity out/ },
  { title: "Close and sign out Antigravity", text: "Antigravity closes and asks first if you have unsaved files. Its sign-in is cleared.", done: /Signed Antigravity out/ },
  { title: "Choose Sign in with Google", text: "Antigravity reopens on its welcome screen. Click Sign in with Google.", done: /choose Sign in with Google/ },
  { title: "Pick the other account in your browser", text: "Your browser opens Google's sign-in. Choose the account you want to add and allow access.", done: /Signed in as/ },
  { title: "AltFlip saves it", text: "As soon as Antigravity shows the new account, AltFlip saves it and makes it active.", done: /Signed in as/ },
];

function AntigravitySteps({ lines, running }: { lines: string[]; running: boolean }) {
  const text = lines.join("\n");
  const current = running ? AGY_STEPS.findIndex((s) => !s.done.test(text)) : -1;
  return (
    <ol className="space-y-0.5 rounded-2xl border border-border bg-card p-2 shadow-item">
      {AGY_STEPS.map((s, i) => {
        const done = s.done.test(text);
        const now = i === current;
        return (
          <li key={s.title} className={cn("flex gap-3 rounded-xl px-2.5 py-2 transition-colors", now && "bg-accent-soft")}>
            <span
              className={cn(
                "mt-px flex size-5 shrink-0 items-center justify-center rounded-full font-mono text-[10.5px] font-semibold",
                done ? "bg-ok text-white" : now ? "bg-accent text-white" : "bg-surface-3 text-muted",
              )}
            >
              {done ? <Check className="size-3" /> : i + 1}
            </span>
            <span className="min-w-0">
              <span className={cn("block text-[13px] font-medium", !done && !now && running && "text-muted")}>{s.title}</span>
              <span className="block text-[12px] text-faint">{s.text}</span>
            </span>
          </li>
        );
      })}
    </ol>
  );
}

const OPENCODE_STEPS = [
  { title: "Sign in to OpenCode", text: "Run opencode auth login in Terminal once for each account or key you want." },
  { title: "Keep OpenCode's background service running", text: "Open OpenCode or run any opencode command. AltFlip switches through it." },
  { title: "Import here", text: "Each service with more than one sign-in, such as two OpenCode Console logins, becomes one account per sign-in." },
];

const OPENCODE_LIMITS = [
  "Needs OpenCode 2. OpenCode 1.x is not supported.",
  "Switching is instant for sessions on the background service. Sessions started with --standalone keep the old sign-in until restarted.",
  "Sign-ins added later appear after you import again.",
  "Usage limits are not shown for OpenCode.",
];

function OpenCodeSteps() {
  return (
    <div className="space-y-2">
      <ol className="space-y-0.5 rounded-2xl border border-border bg-card p-2 shadow-item">
        {OPENCODE_STEPS.map((s, i) => (
          <li key={s.title} className="flex gap-3 rounded-xl px-2.5 py-2">
            <span className="mt-px flex size-5 shrink-0 items-center justify-center rounded-full bg-surface-3 font-mono text-[10.5px] font-semibold text-muted">
              {i + 1}
            </span>
            <span className="min-w-0">
              <span className="block text-[13px] font-medium">{s.title}</span>
              <span className="block text-[12px] text-faint">{s.text}</span>
            </span>
          </li>
        ))}
      </ol>
      <ul className="space-y-1 rounded-2xl bg-surface-2 px-4 py-3 text-[12px] text-muted">
        {OPENCODE_LIMITS.map((l) => (
          <li key={l} className="flex gap-2">
            <span className="mt-[7px] size-1 shrink-0 rounded-full bg-faint" aria-hidden />
            <span>{l}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

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
  const [method, setMethod] = useState<"login" | "current" | "import">("login");
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
    setMethod(state.info.id === "opencode" ? "import" : state.info.capabilities.isolatedLogin ? "login" : "current");
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
    if (method === "import") {
      try {
        const rows = await api.importAccounts(provider);
        toast({ kind: "success", text: rows.length === 1 ? `Imported ${rows[0].label}` : `Imported ${rows.length} sign-ins` });
        openAdd(null);
      } catch (e) {
        setError(errorText(e));
      } finally {
        setBusy(false);
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
  const canSubmit = !!provider && !busy && (method === "login" || method === "import" || !!live);

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
            {busy ? <Spinner /> : method === "login" ? <LogIn /> : method === "import" ? <Download /> : <Save />}
            {method === "login"
              ? busy
                ? "Waiting for sign-in"
                : "Sign in"
              : method === "import"
                ? "Import sign-ins"
                : alreadySaved
                  ? "Update saved account"
                  : "Save account"}
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
                      ? `Signs ${state.info.name} out and reopens it so you can sign in with Google in your browser. Your current account is saved first.`
                      : `Runs ${state.info.id === "codex" ? "codex login" : "claude auth login"} in a private, temporary profile. Your current ${state.info.name} sign-in stays as it is.`
                  }
                />
              )}
              {state.info.id === "opencode" && (
                <MethodOption
                  selected={method === "import"}
                  disabled={busy}
                  onSelect={() => setMethod("import")}
                  icon={<Download className="size-4" />}
                  title="Import every sign-in stored in OpenCode"
                  text="Adds one account for each sign-in OpenCode already holds, such as each OpenCode Console login. Add more with opencode auth login."
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
            {state.info.id === "antigravity" && method === "login" && <AntigravitySteps lines={lines} running={!!loginId} />}
            {state.info.id === "opencode" && method === "import" && <OpenCodeSteps />}
            {method !== "import" && (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_170px]">
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
            )}
            <p className="text-[11.5px] text-faint">
              {state.info.id === "opencode"
                ? "OpenCode keeps its own keys. AltFlip stores only which sign-in to activate and switches through OpenCode's background service."
                : `AltFlip reads the account email, plan and sign-in tokens for ${state.info.name}, and stores the tokens encrypted on this device only.`}
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
