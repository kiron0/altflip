import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type HTMLAttributes, type InputHTMLAttributes, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, X } from "lucide-react";
import { cn } from "@/lib/utils";

const badgeVariants = cva("inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11.5px] font-medium leading-4 whitespace-nowrap", {
  variants: {
    tone: {
      neutral: "bg-surface-3 text-muted",
      accent: "bg-accent-soft text-accent",
      ok: "bg-ok-soft text-ok",
      warn: "bg-warn-soft text-warn",
      danger: "bg-danger-soft text-danger",
      info: "bg-info-soft text-info",
      outline: "border border-border text-muted",
    },
  },
  defaultVariants: { tone: "neutral" },
});

export function Badge({ className, tone, ...props }: HTMLAttributes<HTMLSpanElement> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ tone }), className)} {...props} />;
}

export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("card", className)} {...props} />;
}

export function SectionLabel({ children, right, className }: { children: ReactNode; right?: ReactNode; className?: string }) {
  return (
    <div className={cn("mb-2.5 flex items-center justify-between gap-2", className)}>
      <h2 className="label-mono text-faint">{children}</h2>
      {right}
    </div>
  );
}

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={cn(
      "h-9.5 w-full rounded-xl border border-border bg-card px-3 text-[13.5px] text-text shadow-item placeholder:text-faint focus:border-accent focus:outline-none",
      className,
    )}
    {...props}
  />
));
Input.displayName = "Input";

export function Switch({
  checked,
  onChange,
  disabled,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn("toggle", disabled && "pointer-events-none opacity-40")}
    />
  );
}

export function Segmented<T extends string | number>({
  value,
  onChange,
  options,
  label,
  className,
}: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: ReactNode }[];
  label: string;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [thumb, setThumb] = useState<{ x: number; w: number } | null>(null);
  const [ready, setReady] = useState(false);
  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    const measure = () => {
      const el = root.querySelector<HTMLElement>("button[aria-checked='true']");
      setThumb(el ? { x: el.offsetLeft, w: el.offsetWidth } : null);
    };
    measure();
    const ro = typeof ResizeObserver !== "undefined" ? new ResizeObserver(measure) : null;
    ro?.observe(root);
    const t = requestAnimationFrame(() => setReady(true));
    return () => {
      ro?.disconnect();
      cancelAnimationFrame(t);
    };
  }, [value, options.length]);
  return (
    <div ref={ref} role="radiogroup" aria-label={label} className={cn("seg", thumb && "has-thumb", className)}>
      {thumb && <span aria-hidden className={cn("seg-thumb", ready && "is-ready")} style={{ transform: `translateX(${thumb.x}px)`, width: thumb.w }} />}
      {options.map((o) => (
        <button key={String(o.value)} type="button" role="radio" aria-checked={value === o.value} onClick={() => value !== o.value && onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export interface SelectOption<T extends string | number> {
  value: T;
  label: string;
  group?: string;
  icon?: ReactNode;
}

export function Select<T extends string | number>({
  value,
  options,
  onChange,
  label,
  className,
  emptyLabel = "None",
  size = "md",
}: {
  value: T;
  options: SelectOption<T>[];
  onChange: (v: T) => void;
  label: string;
  className?: string;
  emptyLabel?: string;
  size?: "sm" | "md";
}) {
  const id = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [rect, setRect] = useState<{ left: number; top: number; width: number; up: boolean; maxHeight: number } | null>(null);
  const typed = useRef({ text: "", at: 0 });
  const selectedIndex = options.findIndex((o) => o.value === value);
  const selected = options[selectedIndex];

  const place = useCallback(() => {
    const r = triggerRef.current?.getBoundingClientRect();
    if (!r) return;
    const margin = 8;
    const below = window.innerHeight - r.bottom - margin;
    const above = r.top - margin;
    const wanted = Math.min(320, options.length * 36 + 16);
    const up = below < wanted && above > below;
    const width = Math.min(Math.max(r.width, 200), window.innerWidth - margin * 2);
    const left = Math.min(Math.max(r.left, margin), window.innerWidth - width - margin);
    setRect({ left, top: up ? r.top : r.bottom, width, up, maxHeight: Math.max(120, Math.min(320, (up ? above : below) - 6)) });
  }, [options.length]);

  const openList = () => {
    place();
    setActive(Math.max(0, selectedIndex));
    setOpen(true);
  };
  const close = (refocus = true) => {
    setOpen(false);
    if (refocus) triggerRef.current?.focus();
  };
  const choose = (i: number) => {
    const o = options[i];
    if (o) onChange(o.value);
    close();
  };

  useLayoutEffect(() => {
    const list = listRef.current;
    const el = list?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    if (!open || !list || !el) return;
    if (el.offsetTop < list.scrollTop) list.scrollTop = el.offsetTop - 6;
    else if (el.offsetTop + el.offsetHeight > list.scrollTop + list.clientHeight) list.scrollTop = el.offsetTop + el.offsetHeight - list.clientHeight + 6;
  }, [open, active]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (!listRef.current?.contains(t) && !triggerRef.current?.contains(t)) close(false);
    };
    const onScroll = (e: Event) => {
      if (listRef.current?.contains(e.target as Node)) return;
      const r = triggerRef.current?.getBoundingClientRect();
      if (!r || r.bottom < 0 || r.top > window.innerHeight) close(false);
      else place();
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", place);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", place);
    };
  }, [open, place]);

  const onKey = (e: React.KeyboardEvent) => {
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        openList();
      }
      return;
    }
    const last = options.length - 1;
    if (e.key === "ArrowDown") setActive((a) => Math.min(last, a + 1));
    else if (e.key === "ArrowUp") setActive((a) => Math.max(0, a - 1));
    else if (e.key === "Home") setActive(0);
    else if (e.key === "End") setActive(last);
    else if (e.key === "Enter" || e.key === " ") choose(active);
    else if (e.key === "Escape" || e.key === "Tab") {
      if (e.key === "Escape") e.stopPropagation();
      return close(e.key === "Escape");
    } else if (e.key.length === 1) {
      const now = Date.now();
      typed.current = { text: (now - typed.current.at > 700 ? "" : typed.current.text) + e.key.toLowerCase(), at: now };
      const hit = options.findIndex((o) => o.label.toLowerCase().startsWith(typed.current.text));
      if (hit >= 0) setActive(hit);
      return;
    } else return;
    e.preventDefault();
  };

  const rows = useMemo(() => {
    const out: ({ kind: "group"; label: string } | { kind: "option"; o: SelectOption<T>; i: number })[] = [];
    let g: string | undefined;
    options.forEach((o, i) => {
      if (o.group && o.group !== g) out.push({ kind: "group", label: o.group });
      g = o.group;
      out.push({ kind: "option", o, i });
    });
    return out;
  }, [options]);

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={`${id}-list`}
        aria-activedescendant={open ? `${id}-${active}` : undefined}
        onClick={() => (open ? close() : openList())}
        onKeyDown={onKey}
        className={cn(
          "flex min-w-44 items-center gap-2 rounded-xl border border-border bg-card pr-2.5 pl-3 text-left shadow-item transition-colors hover:border-border-strong",
          size === "sm" ? "h-8 text-[12.5px]" : "h-9.5 text-[13.5px]",
          open && "border-accent",
          className,
        )}
      >
        {selected?.icon && <span className="flex size-3.5 shrink-0 items-center justify-center text-muted">{selected.icon}</span>}
        <span className={cn("min-w-0 flex-1 truncate", selected ? "text-text" : "text-faint")}>{selected?.label ?? emptyLabel}</span>
        <ChevronDown className={cn("size-[15px] shrink-0 text-faint transition-transform", open && "rotate-180")} />
      </button>
      {open &&
        rect &&
        createPortal(
          <div
            ref={listRef}
            id={`${id}-list`}
            role="listbox"
            aria-label={label}
            className="animate-pop fixed z-[80] overflow-auto overscroll-contain rounded-2xl border border-border bg-surface p-1.5 shadow-panel"
            style={{ left: rect.left, width: rect.width, maxHeight: rect.maxHeight, ...(rect.up ? { bottom: window.innerHeight - rect.top + 6 } : { top: rect.top + 6 }) }}
            onKeyDown={onKey}
          >
            {rows.map((r, k) =>
              r.kind === "group" ? (
                <div key={`g${k}`} className="label-mono px-2.5 pt-2 pb-1 text-faint">
                  {r.label}
                </div>
              ) : (
                <div
                  key={String(r.o.value)}
                  id={`${id}-${r.i}`}
                  data-index={r.i}
                  role="option"
                  aria-selected={r.o.value === value}
                  onMouseEnter={() => setActive(r.i)}
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => choose(r.i)}
                  className={cn("flex h-9 cursor-pointer items-center gap-2.5 rounded-xl px-2.5 text-[13px]", r.i === active ? "bg-accent-soft text-text" : "text-muted")}
                >
                  {r.o.icon && <span className="flex size-3.5 shrink-0 items-center justify-center text-muted">{r.o.icon}</span>}
                  <span className="min-w-0 flex-1 truncate">{r.o.label}</span>
                  {r.o.value === value && <Check className="size-3.5 text-accent" />}
                </div>
              ),
            )}
          </div>,
          document.body,
        )}
    </>
  );
}

export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  width = 480,
  icon,
  tone = "accent",
  dismissible = true,
}: {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  width?: number;
  icon?: ReactNode;
  tone?: "accent" | "danger" | "warn";
  dismissible?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(open);
  const [closing, setClosing] = useState(false);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const dismissRef = useRef(dismissible);
  dismissRef.current = dismissible;

  useEffect(() => {
    if (open) {
      setActive(true);
      setClosing(false);
      return;
    }
    if (!active) return;
    setClosing(true);
    const t = setTimeout(() => {
      setActive(false);
      setClosing(false);
    }, 130);
    return () => clearTimeout(t);
  }, [open]);

  useEffect(() => {
    if (!active) return;
    const prev = document.activeElement as HTMLElement | null;
    const t = setTimeout(() => {
      (ref.current?.querySelector<HTMLElement>("[data-autofocus], input, [role='combobox']") ?? ref.current)?.focus();
    }, 10);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && dismissRef.current) {
        e.preventDefault();
        closeRef.current();
        return;
      }
      if (e.key !== "Tab" || !ref.current) return;
      const items = Array.from(ref.current.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), textarea, [href], [tabindex]:not([tabindex='-1'])"));
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && (document.activeElement === first || document.activeElement === ref.current)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      clearTimeout(t);
      window.removeEventListener("keydown", onKey);
      prev?.focus?.();
    };
  }, [active]);

  if (!active || typeof document === "undefined") return null;
  const tones = { accent: "bg-accent-soft text-accent", danger: "bg-danger-soft text-danger", warn: "bg-warn-soft text-warn" };
  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6">
      <div
        className={cn("fixed inset-0 bg-black/40 backdrop-blur-sm", closing ? "modal-backdrop-out" : "modal-backdrop-in")}
        onMouseDown={() => dismissible && onClose()}
      />
      <div
        ref={ref}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label={typeof title === "string" ? title : undefined}
        onMouseDown={(e) => e.stopPropagation()}
        className={cn(
          "relative z-10 flex max-h-full w-full flex-col overflow-hidden rounded-[22px] border border-border bg-surface shadow-panel outline-none",
          closing ? "modal-dialog-out" : "modal-dialog-in",
        )}
        style={{ maxWidth: width }}
      >
        <div className="flex items-start justify-between gap-3 px-6 pt-5 pb-1">
          <div className="flex min-w-0 items-start gap-2.5">
            {icon && <span className={cn("mt-px flex size-7 shrink-0 items-center justify-center rounded-lg [&_svg]:size-[15px]", tones[tone])}>{icon}</span>}
            <div className="min-w-0">
              <h2 className="text-[15px] font-semibold leading-7">{title}</h2>
              {description && <p className="mt-0.5 text-[13px] leading-relaxed text-muted">{description}</p>}
            </div>
          </div>
          {dismissible && (
            <button
              type="button"
              data-close
              aria-label="Close"
              onClick={onClose}
              className="-mr-2 inline-flex size-8 shrink-0 items-center justify-center rounded-xl text-muted transition-colors hover:bg-surface-3 hover:text-text"
            >
              <X className="size-4" />
            </button>
          )}
        </div>
        {children && <div className="min-h-0 overflow-auto px-6 py-3">{children}</div>}
        {footer && <div className="flex flex-wrap justify-end gap-2.5 px-6 pt-2 pb-5">{footer}</div>}
      </div>
    </div>,
    document.body,
  );
}

export function Field({ label, hint, children, className }: { label: string; hint?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1 flex items-baseline justify-between gap-2 text-[12px] font-medium text-muted">
        {label}
        {hint && <span className="font-normal text-faint">{hint}</span>}
      </span>
      {children}
    </label>
  );
}

export function Spinner({ className }: { className?: string }) {
  return <span className={cn("inline-block size-3.5 animate-spin rounded-full border-2 border-current border-t-transparent", className)} aria-hidden />;
}

export function Meter({ value }: { value: number }) {
  const v = Math.max(0, Math.min(100, value));
  const tone = v >= 90 ? "danger" : v >= 70 ? "warn" : "";
  return (
    <div className="bar" role="meter" aria-valuenow={v} aria-valuemin={0} aria-valuemax={100}>
      <span className={cn("used", tone)} style={{ width: `${v}%` }} />
    </div>
  );
}
