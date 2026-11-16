import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap font-medium transition-[filter,background-color,border-color,transform] active:scale-[0.98] disabled:pointer-events-none disabled:opacity-45 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        primary: "bg-brand text-white shadow-brand hover:brightness-110 active:brightness-95",
        secondary: "border border-border bg-surface text-text shadow-item hover:border-border-strong active:bg-surface-2",
        ghost: "text-muted hover:bg-surface-3 hover:text-text",
        danger: "border border-border bg-surface text-danger shadow-item hover:bg-danger-soft",
        subtle: "bg-surface-3 text-text hover:brightness-95",
      },
      size: {
        sm: "h-7.5 rounded-full px-3 text-[12.5px] [&_svg]:size-3.5",
        md: "h-9 rounded-full px-4 text-[13px] [&_svg]:size-4",
        lg: "h-10 rounded-full px-5 text-[14px] [&_svg]:size-4",
        icon: "size-8 rounded-xl [&_svg]:size-[17px]",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, type = "button", ...props }, ref) => (
    <button ref={ref} type={type} className={cn(buttonVariants({ variant, size }), className)} {...props} />
  ),
);
Button.displayName = "Button";
