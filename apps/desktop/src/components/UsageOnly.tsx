import { useState } from "react";
import { Check, CircleHelp, X } from "lucide-react";
import { Button } from "./ui/button";
import { Dialog } from "./ui/primitives";
import { ProviderIcon } from "./ProviderIcon";
import { cn } from "@/lib/utils";

const WHY = [
  "Antigravity only checks which account is signed in when it starts up.",
  "So changing the account while it's open does nothing until Antigravity is closed and opened again.",
  "That means every switch would close Antigravity for you, and you could lose work that isn't saved.",
  "Chats you already have open would also keep using the old account.",
];

const CAN = [
  "See live usage for every saved Antigravity account, right here in AltFlip.",
  "Get a heads-up when an account is close to its limit.",
  "To use another account, sign out in Antigravity and sign back in with that account.",
];

export function UsageOnlyButton({ className, compact = false }: { className?: string; compact?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setOpen(true);
        }}
        title="Why Antigravity shows usage only"
        className={cn(
          "inline-flex items-center gap-1 rounded-full bg-text/[0.06] font-medium text-muted transition-colors hover:bg-text/[0.1] hover:text-text",
          compact ? "px-2 py-[2px] text-[10.5px]" : "px-2.5 py-[3px] text-[11.5px]",
          className,
        )}
      >
        Usage only
        <CircleHelp className={compact ? "size-3" : "size-3.5"} />
      </button>
      <UsageOnlyDialog open={open} onClose={() => setOpen(false)} />
    </>
  );
}

export function UsageOnlyDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      width={440}
      icon={<ProviderIcon id="antigravity" size={22} />}
      title="Why Antigravity can't switch accounts"
      description="AltFlip switches Claude Code and Codex in one click. Antigravity works differently, so AltFlip shows its usage instead."
      footer={
        <Button variant="primary" onClick={onClose}>
          Got it
        </Button>
      }
    >
      <div className="space-y-4 text-[13px]">
        <section>
          <h3 className="mb-2 text-[12px] font-semibold text-text">Why switching doesn't work well</h3>
          <ul className="space-y-2">
            {WHY.map((t) => (
              <li key={t} className="flex gap-2.5 text-muted">
                <span className="mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full bg-warn-soft text-warn">
                  <X className="size-2.5" strokeWidth={3} />
                </span>
                {t}
              </li>
            ))}
          </ul>
        </section>
        <section>
          <h3 className="mb-2 text-[12px] font-semibold text-text">What you can do</h3>
          <ul className="space-y-2">
            {CAN.map((t) => (
              <li key={t} className="flex gap-2.5 text-muted">
                <span className="mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full bg-ok-soft text-ok">
                  <Check className="size-2.5" strokeWidth={3} />
                </span>
                {t}
              </li>
            ))}
          </ul>
        </section>
      </div>
    </Dialog>
  );
}
