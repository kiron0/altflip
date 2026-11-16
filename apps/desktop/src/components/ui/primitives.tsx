import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, useEffect, useRef, type HTMLAttributes, type InputHTMLAttributes, type ReactNode } from "react";
import { X } from "lucide-react";
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
      "h-9 w-full rounded-xl border border-border bg-surface px-3 text-[13px] text-text shadow-item placeholder:text-faint focus:border-accent focus:outline-none",
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

export function Select<T extends string | number>({
  value,
  options,
  onChange,
  label,
  className,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  label: string;
  className?: string;
}) {
  return (
    <select
      aria-label={label}
      value={String(value)}
      onChange={(e) => {
        const o = options.find((o) => String(o.value) === e.target.value);
        if (o) onChange(o.value);
      }}
      className={cn(
        "h-9 rounded-xl border border-border bg-surface pr-8 pl-3 text-[13px] text-text shadow-item focus:border-accent focus:outline-none",
        className,
      )}
    >
      {options.map((o) => (
        <option key={String(o.value)} value={String(o.value)}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  width = 460,
}: {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  width?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const prev = document.activeElement as HTMLElement | null;
    const t = setTimeout(() => {
      ref.current?.querySelector<HTMLElement>("[data-autofocus], input, button:not([data-close])")?.focus();
    }, 10);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      clearTimeout(t);
      window.removeEventListener("keydown", onKey);
      prev?.focus?.();
    };
  }, [open, onClose]);
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4 backdrop-blur-[3px]" onMouseDown={onClose}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        className="animate-in max-h-[90vh] w-full overflow-auto rounded-[24px] border border-border bg-surface shadow-panel"
        style={{ maxWidth: width }}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="flex items-start gap-3 px-6 pt-5">
          <div className="min-w-0 flex-1">
            <h2 className="font-display text-[17px] font-semibold">{title}</h2>
            {description && <p className="mt-1.5 text-[13px] text-muted">{description}</p>}
          </div>
          <button
            data-close
            aria-label="Close"
            onClick={onClose}
            className="-mr-2 inline-flex size-8 items-center justify-center rounded-xl text-muted hover:bg-surface-3 hover:text-text"
          >
            <X className="size-4" />
          </button>
        </div>
        {children && <div className="px-6 py-4">{children}</div>}
        {footer && <div className="flex justify-end gap-2 border-t border-border bg-surface-2 px-6 py-3.5">{footer}</div>}
      </div>
    </div>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-[12.5px] font-medium text-muted">{label}</span>
      {children}
      {hint && <span className="mt-1 block text-[11.5px] text-faint">{hint}</span>}
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
