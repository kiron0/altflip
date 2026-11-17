import { useEffect, useMemo, useState } from "react";
import { ArrowDown, ArrowUp, Plus, Search, Star, Trash2 } from "lucide-react";
import { api, errorText, type AccountView, type ProviderId, type ProviderState } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { AccountRow, StateBadge } from "@/components/AccountRow";
import { ProviderIcon } from "@/components/ProviderIcon";
import { Button } from "@/components/ui/button";
import { Badge, Card, Dialog, Field, Input, Meter, Segmented, Select, Tag } from "@/components/ui/primitives";

const CATEGORIES = ["", "Personal", "Work", "University", "Secondary", "Testing"];

function Detail({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[130px_1fr] gap-3 py-1.5 text-[12.5px]">
      <span className="text-muted">{label}</span>
      <span className="selectable min-w-0 break-words">{children}</span>
    </div>
  );
}

function AccountDetails({ account, provider, onClose }: { account: AccountView; provider: ProviderState; onClose: () => void }) {
  const toast = useApp((s) => s.toast);
  const requestSwitch = useApp((s) => s.requestSwitch);
  const [label, setLabel] = useState(account.label);
  const [category, setCategory] = useState(account.category ?? "");
  const [confirmRemove, setConfirmRemove] = useState(false);
  const isActive = provider.activeAccountId === account.id;
  const idx = provider.accounts.findIndex((a) => a.id === account.id);

  const save = async (patch: Parameters<typeof api.updateAccount>[1]) => {
    try {
      await api.updateAccount(account.id, patch);
    } catch (e) {
      toast({ kind: "error", text: "Could not update account", detail: errorText(e) });
    }
  };

  const move = (dir: -1 | 1) =>
    api.moveAccount(account.id, dir).catch((e) => toast({ kind: "error", text: "Could not reorder", detail: errorText(e) }));

  return (
    <Dialog
      open
      onClose={onClose}
      width={540}
      title={
        <span className="flex items-center gap-2">
          <ProviderIcon id={provider.info.id} size={18} /> {account.label}
          <StateBadge account={account} />
        </span>
      }
      description={`${provider.info.name} account`}
      footer={
        <>
          <Button variant="ghost" className="mr-auto text-danger hover:text-danger" onClick={() => setConfirmRemove(true)}>
            <Trash2 /> Remove
          </Button>
          <Button onClick={onClose}>Close</Button>
          {!isActive && (
            <Button
              variant="primary"
              disabled={!provider.switchingEnabled}
              title={provider.switchingDisabledReason ?? undefined}
              onClick={() => {
                onClose();
                void requestSwitch(account.id);
              }}
            >
              Switch to this account
            </Button>
          )}
        </>
      }
    >
      <div className="grid grid-cols-1 items-end gap-2 sm:grid-cols-[1fr_170px_auto]">
        <Field label="Name">
          <Input value={label} onChange={(e) => setLabel(e.target.value)} onBlur={() => label.trim() && label !== account.label && void save({ label })} />
        </Field>
        <Field label="Category">
          <Select
            label="Category"
            className="w-full"
            value={category}
            options={CATEGORIES.map((c) => ({ value: c, label: c || "None" }))}
            onChange={(v) => {
              setCategory(v);
              void save({ category: v || null });
            }}
          />
        </Field>
        <div className="flex gap-1">
          <Button size="icon" aria-label={account.favorite ? "Remove from favorites" : "Add to favorites"} onClick={() => void save({ favorite: !account.favorite })}>
            <Star className={cn(account.favorite && "fill-warn text-warn")} />
          </Button>
          <Button size="icon" aria-label="Move up" disabled={idx <= 0} onClick={() => void move(-1)}>
            <ArrowUp />
          </Button>
          <Button size="icon" aria-label="Move down" disabled={idx >= provider.accounts.length - 1} onClick={() => void move(1)}>
            <ArrowDown />
          </Button>
        </div>
      </div>

      <h3 className="mt-5 mb-1 label-mono text-faint">Identity</h3>
      <div className="divide-y divide-border">
        <Detail label="Email">{account.email ?? "Not provided"}</Detail>
        {account.displayName && <Detail label="Name">{account.displayName}</Detail>}
        {account.organization && <Detail label="Organization">{account.organization}</Detail>}
        <Detail label="Plan">{account.planLabel ?? "Unknown"}</Detail>
        <Detail label="Sign-in method">{account.authMethod ?? "Unknown"}</Detail>
      </div>

      <h3 className="mt-5 mb-1 label-mono text-faint">Status</h3>
      <div className="divide-y divide-border">
        <Detail label="Authentication">
          <Tag badge={account.healthBadge} />
        </Detail>
        {account.needsSignIn && (
          <Detail label="To fix">
            <span className="text-danger">{provider.signInHint}</span>
          </Detail>
        )}
        <Detail label="Added">{account.addedText}</Detail>
        <Detail label="Last used">{account.lastUsedText}</Detail>
        <Detail label="Credentials saved">{account.capturedText}</Detail>
        {isActive && account.verifiedText && <Detail label="Last verified">{account.verifiedText}</Detail>}
        {account.lastError && (
          <Detail label="Last error">
            <span className="text-danger">{account.lastError}</span>
          </Detail>
        )}
      </div>

      {account.usage.length > 0 && (
        <>
          <h3 className="mt-5 mb-2 label-mono text-faint">Usage</h3>
          <div className="space-y-3">
            {account.usage.map((u) => (
              <div key={u.usageType}>
                <div className="mb-1 flex text-[12px]">
                  <span className="flex-1">{u.label}</span>
                  <span className="tabular-nums text-muted">
                    {u.percent != null ? `${u.percent}%` : "—"}
                    {u.countdown && ` · resets in ${u.countdown}`}
                  </span>
                </div>
                {u.percent != null && <Meter value={u.percent} />}
                <div className="mt-1 text-[11px] text-faint">
                  {u.source}, observed {u.observedText}
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      <Dialog
        open={confirmRemove}
        onClose={() => setConfirmRemove(false)}
        title={`Remove ${account.label}?`}
        description="AltFlip deletes its encrypted copy of this account's sign-in. The account itself and your current sign-in in the tool are not changed."
        footer={
          <>
            <Button onClick={() => setConfirmRemove(false)}>Cancel</Button>
            <Button
              variant="danger"
              onClick={async () => {
                try {
                  await api.removeAccount(account.id);
                  toast({ kind: "success", text: `Removed ${account.label}` });
                  onClose();
                } catch (e) {
                  toast({ kind: "error", text: "Could not remove", detail: errorText(e) });
                }
              }}
            >
              Remove account
            </Button>
          </>
        }
      />
    </Dialog>
  );
}

export function Accounts() {
  const overview = useApp((s) => s.overview);
  const focus = useApp((s) => s.focusAccount);
  const navigate = useApp((s) => s.navigate);
  const openAdd = useApp((s) => s.openAdd);
  const [filter, setFilter] = useState<ProviderId | "all">("all");
  const [query, setQuery] = useState("");
  const [openId, setOpenId] = useState<string | null>(null);

  useEffect(() => {
    if (focus) {
      setOpenId(focus);
      navigate("accounts", null);
    }
  }, [focus, navigate]);

  const accountsFilter = useApp((s) => s.accountsFilter);
  useEffect(() => {
    if (accountsFilter) {
      setFilter(accountsFilter);
      useApp.setState({ accountsFilter: null });
    }
  }, [accountsFilter]);

  const providers = useMemo(
    () => (overview?.providers ?? []).filter((p) => (p.info.installed || p.accounts.length) && (filter === "all" || p.info.id === filter)),
    [overview, filter],
  );
  const q = query.trim().toLowerCase();
  const match = (a: AccountView) =>
    !q || [a.label, a.email, a.category, a.organization, a.subscriptionType].some((v) => v?.toLowerCase().includes(q));

  const open = overview?.providers.flatMap((p) => p.accounts.map((a) => ({ a, p }))).find((x) => x.a.id === openId);

  return (
    <>
      <PageHeader
        title="Accounts"
        subtitle="Every saved account, grouped by tool."
        actions={
          <Button variant="primary" onClick={() => openAdd(filter === "all" ? "any" : filter)}>
            <Plus /> Add account
          </Button>
        }
      />
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative w-full min-w-0 sm:w-64">
          <Search className="pointer-events-none absolute top-2.5 left-3.5 size-4 text-faint" />
          <Input className="rounded-full pl-9" placeholder="Search accounts" value={query} onChange={(e) => setQuery(e.target.value)} aria-label="Search accounts" />
        </div>
        <Segmented
          label="Filter by tool"
          value={filter}
          onChange={setFilter}
          options={[
            { value: "all" as ProviderId | "all", label: "All" },
            ...(overview?.providers ?? []).map((p) => ({
              value: p.info.id as ProviderId | "all",
              label: (
                <span className="inline-flex items-center gap-1.5">
                  <ProviderIcon id={p.info.id} size={16} />
                  {p.info.name}
                </span>
              ),
            })),
          ]}
        />
      </div>
      <div className="space-y-4">
        {providers.map((p) => {
          const rows = p.accounts.filter(match);
          return (
            <Card key={p.info.id}>
              <div className="flex items-center gap-2.5 border-b border-border px-5 py-3">
                <ProviderIcon id={p.info.id} size={18} />
                <span className="font-semibold">{p.info.name}</span>
                <span className="text-[12px] text-muted">{p.accounts.length} saved</span>
                {p.info.experimental && <Badge tone="warn">Experimental</Badge>}
                <span className="flex-1" />
                <Button size="sm" variant="ghost" onClick={() => openAdd(p.info.id)}>
                  <Plus /> Add
                </Button>
              </div>
              <div className="p-2">
                {rows.length === 0 && (
                  <div className="px-2 py-4 text-center text-[12.5px] text-muted">{q ? "No matches." : "No saved accounts."}</div>
                )}
                {rows.map((a) => (
                  <AccountRow key={a.id} account={a} provider={p} onOpen={() => setOpenId(a.id)} />
                ))}
              </div>
            </Card>
          );
        })}
      </div>
      {open && <AccountDetails account={open.a} provider={open.p} onClose={() => setOpenId(null)} />}
    </>
  );
}
