import { AlertTriangle, CheckCircle2, Info, X, XCircle } from "lucide-react";
import { useApp } from "@/lib/store";
import { cn } from "@/lib/utils";

const icons = {
  success: <CheckCircle2 className="size-4 text-ok" />,
  error: <XCircle className="size-4 text-danger" />,
  warning: <AlertTriangle className="size-4 text-warn" />,
  info: <Info className="size-4 text-info" />,
};

export function Toasts({ compact = false }: { compact?: boolean }) {
  const toasts = useApp((s) => s.toasts);
  const dismiss = useApp((s) => s.dismiss);
  return (
    <div
      className={cn("pointer-events-none fixed z-[60] flex flex-col gap-2", compact ? "inset-x-3 bottom-3" : "right-4 bottom-4 w-[360px]")}
      aria-live="polite"
    >
      {toasts.map((t) => (
        <div key={t.id} className="animate-in pointer-events-auto flex gap-2.5 rounded-2xl border border-border bg-surface px-4 py-3 shadow-panel">
          <span className="mt-0.5">{icons[t.kind]}</span>
          <div className="min-w-0 flex-1">
            <div className="font-medium">{t.text}</div>
            {t.detail && <div className="selectable mt-0.5 text-[12px] break-words text-muted">{t.detail}</div>}
          </div>
          <button aria-label="Dismiss" className="self-start rounded p-0.5 text-faint hover:text-text" onClick={() => dismiss(t.id)}>
            <X className="size-3.5" />
          </button>
        </div>
      ))}
    </div>
  );
}
