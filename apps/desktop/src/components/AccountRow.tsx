import { AlertTriangle, Check, Star } from "lucide-react";
import type { AccountView, ProviderState } from "@/lib/api";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";
import { Badge, Spinner, Tag } from "./ui/primitives";
import { Button } from "./ui/button";

const toneText = { danger: "text-danger", warn: "text-warn" } as Record<string, string>;

export function StateBadge({ account }: { account: AccountView }) {
  return <Tag badge={account.stateBadge} />;
}

export function UsageInline({ account }: { account: AccountView }) {
  const u = account.inlineUsage;
  if (!u) return null;
  return (
    <span className={cn("tabular-nums", toneText[u.tone] ?? "text-muted")} title={u.title}>
      {u.text}
    </span>
  );
}

export function AccountRow({
  account,
  provider,
  compact = false,
  onOpen,
}: {
  account: AccountView;
  provider: ProviderState;
  compact?: boolean;
  onOpen?: () => void;
}) {
  const progress = useApp((s) => s.switching[provider.info.id]);
  const requestSwitch = useApp((s) => s.requestSwitch);
  const busy = progress?.accountId === account.id;
  const otherBusy = !!progress && !busy;
  const isActive = provider.activeAccountId === account.id;
  const drifted = isActive && provider.activationState === "drifted";

  return (
    <div
      className={cn("row-item group", compact ? "min-h-10 py-1.5" : "py-2", onOpen && "clickable cursor-pointer")}
      onClick={onOpen}
    >
      <span
        className={cn(
          "flex size-[18px] shrink-0 items-center justify-center rounded-full border",
          isActive && !drifted ? "border-transparent bg-brand text-white shadow-brand" : "border-border-strong bg-surface",
        )}
        aria-hidden
      >
        {isActive && !drifted && <Check className="size-3" strokeWidth={3.2} />}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="truncate text-[13.5px] font-medium text-text">{account.label}</span>
          {account.favorite && <Star className="size-3 shrink-0 fill-warn text-warn" aria-label="Favorite" />}
          {!compact && <StateBadge account={account} />}
        </div>
        <div className="flex items-center gap-1.5 truncate font-mono text-[10.5px] text-faint">
          {busy ? (
            <span className="text-accent">{progress.message}…</span>
          ) : (
            <>
              <span className="truncate">{account.subtitle}</span>
              {account.usage.length > 0 && <span className="text-faint">·</span>}
              <UsageInline account={account} />
            </>
          )}
        </div>
      </div>
      {compact && account.needsSignIn && (
        <span title={provider.signInHint}>
          <AlertTriangle className="size-3.5 text-danger" aria-label={provider.signInHint} />
        </span>
      )}
      {busy ? (
        <Spinner className="text-accent" />
      ) : isActive && !drifted ? (
        <Badge tone="ok">{provider.activationState === "new-sessions" ? "New sessions" : "Active"}</Badge>
      ) : (
        <Button
          size="sm"
          variant="secondary"
          className={cn(compact && "h-7 px-2.5 text-[12px]", "opacity-90 group-hover:opacity-100")}
          disabled={!provider.switchingEnabled || otherBusy}
          title={provider.switchingDisabledReason ?? undefined}
          onClick={(e) => {
            e.stopPropagation();
            void requestSwitch(account.id);
          }}
        >
          Switch
        </Button>
      )}
    </div>
  );
}
