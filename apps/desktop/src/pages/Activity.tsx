import { useEffect, useMemo, useState } from "react";
import { ArrowRight, Undo2 } from "lucide-react";
import { api, errorText, type ActivityRow, type SwitchRecord } from "@/lib/api";
import { useApp } from "@/lib/store";
import { clockTime, dayLabel } from "@/lib/utils";
import { PageHeader } from "@/components/PageHeader";
import { ProviderIcon } from "@/components/ProviderIcon";
import { RecoveryBanner } from "@/components/Dialogs";
import { StatusBadge } from "@/components/labels";
import { Button } from "@/components/ui/button";
import { Card, Dialog } from "@/components/ui/primitives";

export function ActivityPage() {
  const overview = useApp((s) => s.overview);
  const toast = useApp((s) => s.toast);
  const [history, setHistory] = useState<SwitchRecord[]>([]);
  const [log, setLog] = useState<ActivityRow[]>([]);
  const [tab, setTab] = useState<"switches" | "log">("switches");
  const [undo, setUndo] = useState<SwitchRecord | null>(null);

  useEffect(() => {
    void api.history(200).then(setHistory).catch(() => undefined);
    void api.activity(500).then(setLog).catch(() => undefined);
  }, [overview?.generatedAt]);

  const undoable = useMemo(() => {
    const seen = new Set<string>();
    const ids = new Set<string>();
    for (const r of history) {
      if (r.status !== "succeeded" || seen.has(r.provider)) continue;
      seen.add(r.provider);
      ids.add(r.id);
    }
    return ids;
  }, [history]);

  const grouped = useMemo(() => {
    const out: { day: string; rows: ActivityRow[] }[] = [];
    for (const r of log) {
      const d = dayLabel(r.at);
      if (out.at(-1)?.day !== d) out.push({ day: d, rows: [] });
      out.at(-1)!.rows.push(r);
    }
    return out;
  }, [log]);

  return (
    <>
      <PageHeader title="Activity" subtitle="Switch history and account events on this device. No tokens or passwords are recorded." />
      <RecoveryBanner />
      <div className="seg self-start">
        {(["switches", "log"] as const).map((t) => (
          <button key={t} onClick={() => setTab(t)} aria-pressed={tab === t}>
            {t === "switches" ? "Switches" : "Event log"}
          </button>
        ))}
      </div>
      {tab === "switches" ? (
        <Card className="divide-y divide-border">
          {history.length === 0 && <div className="px-4 py-6 text-center text-[12.5px] text-muted">No switches yet.</div>}
          {history.map((r) => (
            <div key={r.id} className="flex items-center gap-3 px-4 py-2.5">
              <span className="w-20 shrink-0 text-[11.5px] text-muted tabular-nums">
                {dayLabel(r.startedAt) === "Today" ? clockTime(r.startedAt) : dayLabel(r.startedAt)}
              </span>
              <ProviderIcon id={r.provider} size={16} />
              <div className="min-w-0 flex-1">
                <div className="truncate">
                  {r.fromLabel ?? "—"} <ArrowRight className="inline size-3 text-faint" /> <b className="font-medium">{r.toLabel}</b>
                </div>
                {r.message && r.status !== "succeeded" && <div className="selectable truncate text-[11.5px] text-muted">{r.message}</div>}
              </div>
              <StatusBadge status={r.status} />
              {undoable.has(r.id) && (
                <Button size="sm" variant="ghost" onClick={() => setUndo(r)}>
                  <Undo2 /> Undo
                </Button>
              )}
            </div>
          ))}
        </Card>
      ) : (
        <div className="space-y-4">
          {grouped.length === 0 && <Card className="px-4 py-6 text-center text-[12.5px] text-muted">Nothing recorded yet.</Card>}
          {grouped.map((g) => (
            <div key={g.day}>
              <h3 className="mb-1.5 label-mono text-faint">{g.day}</h3>
              <Card className="divide-y divide-border">
                {g.rows.map((r) => (
                  <div key={r.id} className="flex items-center gap-3 px-4 py-2">
                    <span className="w-12 shrink-0 text-[11.5px] text-muted tabular-nums">{clockTime(r.at)}</span>
                    {r.provider ? <ProviderIcon id={r.provider} size={14} /> : <span className="size-3.5" />}
                    <span className="selectable min-w-0 flex-1 text-[12.5px]">{r.message}</span>
                  </div>
                ))}
              </Card>
            </div>
          ))}
        </div>
      )}
      <Dialog
        open={!!undo}
        onClose={() => setUndo(null)}
        title="Undo this switch?"
        description={`Restores the ${undo?.provider} sign-in that was active before switching to ${undo?.toLabel}. Running sessions keep their current account.`}
        footer={
          <>
            <Button onClick={() => setUndo(null)}>Cancel</Button>
            <Button
              variant="primary"
              onClick={async () => {
                const r = undo!;
                setUndo(null);
                try {
                  await api.undoSwitch(r.id);
                  toast({ kind: "success", text: "Previous sign-in restored" });
                } catch (e) {
                  toast({ kind: "error", text: "Could not undo", detail: errorText(e) });
                }
              }}
            >
              Restore previous
            </Button>
          </>
        }
      />
    </>
  );
}
