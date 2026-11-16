import type { ReactNode, SVGProps } from "react";

export type NavIconProps = SVGProps<SVGSVGElement> & { size?: number };

function Base({ size = 20, className, children, ...rest }: NavIconProps & { children: ReactNode }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className ? `ic ${className}` : "ic"} aria-hidden="true" {...rest}>
      {children}
    </svg>
  );
}

const SHIELD = "M12 3.5 19 6.5v5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9v-5z";
const GEAR =
  "M18.39 10.36 20.29 10.63 20.29 13.37 18.39 13.64 17.68 15.36 18.83 16.89 16.89 18.83 15.36 17.68 13.64 18.39 13.37 20.29 10.63 20.29 10.36 18.39 8.64 17.68 7.11 18.83 5.17 16.89 6.32 15.36 5.61 13.64 3.71 13.37 3.71 10.63 5.61 10.36 6.32 8.64 5.17 7.11 7.11 5.17 8.64 6.32 10.36 5.61 10.63 3.71 13.37 3.71 13.64 5.61 15.36 6.32 16.89 5.17 18.83 7.11 17.68 8.64z";
const BELL = "M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 1.5h-15z";

export const DashboardIcon = (p: NavIconProps) => (
  <Base {...p}>
    <rect className="s" x="3.5" y="3.5" width="7.5" height="7.5" rx="2.2" />
    <rect className="s" x="13" y="3.5" width="7.5" height="7.5" rx="2.2" />
    <circle className="a" cx="16.75" cy="7.25" r="1.8" />
    <rect className="f" x="3.5" y="13" width="7.5" height="7.5" rx="2.2" />
    <rect className="s" x="3.5" y="13" width="7.5" height="7.5" rx="2.2" />
    <rect className="s" x="13" y="13" width="7.5" height="7.5" rx="2.2" />
  </Base>
);

export const AccountsIcon = (p: NavIconProps) => (
  <Base {...p}>
    <circle className="s" cx="16.5" cy="8" r="2.6" />
    <path className="s" d="M15.5 13.6c3-.4 5.5 1.3 5.5 4.4" />
    <circle className="f" cx="9.5" cy="8.5" r="3.4" />
    <circle className="s" cx="9.5" cy="8.5" r="3.4" />
    <path className="f" d="M3.5 19.5c0-3.4 2.7-5.6 6-5.6s6 2.2 6 5.6z" />
    <path className="s" d="M3.5 19.5c0-3.4 2.7-5.6 6-5.6s6 2.2 6 5.6z" />
    <circle className="a" cx="19.2" cy="5.2" r="1.5" />
  </Base>
);

export const UsageIcon = (p: NavIconProps) => (
  <Base {...p}>
    <rect className="s" x="4" y="12" width="4" height="8" rx="1.4" />
    <rect className="f" x="10" y="7" width="4" height="13" rx="1.4" />
    <rect className="s" x="10" y="7" width="4" height="13" rx="1.4" />
    <rect className="s" x="16" y="10" width="4" height="10" rx="1.4" />
    <circle className="a" cx="18" cy="5.5" r="1.6" />
  </Base>
);

export const ActivityIcon = (p: NavIconProps) => (
  <Base {...p}>
    <rect className="f" x="3.5" y="4" width="17" height="16" rx="3.5" />
    <rect className="s" x="3.5" y="4" width="17" height="16" rx="3.5" />
    <path className="s" d="M6.5 12.5h2.6l1.6-3.6 2.6 7 1.7-3.4h2.5" />
    <circle className="a" cx="17.5" cy="12.5" r="1.4" />
  </Base>
);

export const SettingsIcon = (p: NavIconProps) => (
  <Base {...p}>
    <path className="s" d="M4 7h9 M19 7h1 M4 17h3 M13 17h7" />
    <circle className="f" cx="16" cy="7" r="2.6" />
    <circle className="s" cx="16" cy="7" r="2.6" />
    <circle className="f" cx="10" cy="17" r="2.6" />
    <circle className="s" cx="10" cy="17" r="2.6" />
    <circle className="a" cx="16" cy="7" r="1.1" />
  </Base>
);

export const BackIcon = (p: NavIconProps) => (
  <Base {...p}>
    <circle className="f" cx="12" cy="12" r="8.5" />
    <path className="s" d="M16 12H8m3.5-3.5L8 12l3.5 3.5" />
  </Base>
);

export const SwitchingIcon = (p: NavIconProps) => (
  <Base {...p}>
    <rect className="f" x="3.5" y="5" width="17" height="14" rx="4" />
    <path className="s" d="M7 9.5h10m-2.8-2.8L17 9.5l-2.8 2.8" />
    <path className="s" d="M17 14.5H7m2.8-2.8L7 14.5l2.8 2.8" />
    <circle className="a" cx="19.5" cy="5.5" r="1.5" />
  </Base>
);

export const BellIcon = (p: NavIconProps) => (
  <Base {...p}>
    <path className="f" d={BELL} />
    <path className="s" d={BELL} />
    <path className="s" d="M10 20.5a2.2 2.2 0 0 0 4 0" />
    <circle className="a" cx="17.5" cy="6" r="1.8" />
  </Base>
);

export const GaugeIcon = (p: NavIconProps) => (
  <Base {...p}>
    <path className="f" d="M3.5 16a8.5 8.5 0 0 1 17 0z" />
    <path className="s" d="M3.5 16a8.5 8.5 0 0 1 17 0" />
    <path className="s" d="M12 16l3.6-4.4" />
    <circle className="a" cx="12" cy="16" r="1.6" />
  </Base>
);

export const ShieldIcon = (p: NavIconProps) => (
  <Base {...p}>
    <path className="f" d={SHIELD} />
    <path className="s" d={SHIELD} />
    <path className="s" d="m9 12 2.2 2.2 4.3-4.3" />
  </Base>
);

export const ToolsIcon = (p: NavIconProps) => (
  <Base {...p}>
    <path className="f" d={GEAR} />
    <path className="s" d={GEAR} />
    <circle className="s" cx="12" cy="12" r="2.6" />
    <circle className="a" cx="12" cy="12" r="1.1" />
  </Base>
);
