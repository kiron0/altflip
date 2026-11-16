import type { ProviderId } from "@/lib/api";
import { cn } from "@/lib/utils";

interface LogoProps {
  size?: number;
  className?: string;
  monochrome?: boolean;
}

function ClaudeLogo({ size = 16, className }: LogoProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="m4.7144 15.9555 4.7174-2.6471.079-.2307-.079-.1275h-.2307l-.7893-.0486-2.6956-.0729-2.3375-.0971-2.2646-.1214-.5707-.1215-.5343-.7042.0546-.3522.4797-.3218.686.0608 1.5179.1032 2.2767.1578 1.6514.0972 2.4468.255h.3886l.0546-.1579-.1336-.0971-.1032-.0972L6.973 9.8356l-2.55-1.6879-1.3356-.9714-.7225-.4918-.3643-.4614-.1578-1.0078.6557-.7225.8803.0607.2246.0607.8925.686 1.9064 1.4754 2.4893 1.8336.3643.3035.1457-.1032.0182-.0728-.164-.2733-1.3539-2.4467-1.445-2.4893-.6435-1.032-.17-.6194c-.0607-.255-.1032-.4674-.1032-.7285L6.287.1335 6.6997 0l.9957.1336.419.3642.6192 1.4147 1.0018 2.2282 1.5543 3.0296.4553.8985.2429.8318.091.255h.1579v-.1457l.1275-1.706.2368-2.0947.2307-2.6957.0789-.7589.3764-.9107.7468-.4918.5828.2793.4797.686-.0668.4433-.2853 1.8517-.5586 2.9021-.3643 1.9429h.2125l.2429-.2429.9835-1.3053 1.6514-2.0643.7286-.8196.85-.9046.5464-.4311h1.0321l.759 1.1293-.34 1.1657-1.0625 1.3478-.8804 1.1414-1.2628 1.7-.7893 1.36.0729.1093.1882-.0183 2.8535-.607 1.5421-.2794 1.8396-.3157.8318.3886.091.3946-.3278.8075-1.967.4857-2.3072.4614-3.4364.8136-.0425.0304.0486.0607 1.5482.1457.6618.0364h1.621l3.0175.2247.7892.522.4736.6376-.079.4857-1.2142.6193-1.6393-.3886-3.825-.9107-1.3113-.3279h-.1822v.1093l1.0929 1.0686 2.0035 1.8092 2.5075 2.3314.1275.5768-.3218.4554-.34-.0486-2.2039-1.6575-.85-.7468-1.9246-1.621h-.1275v.17l.4432.6496 2.3436 3.5214.1214 1.0807-.17.3521-.6071.2125-.6679-.1214-1.3721-1.9246L14.38 17.959l-1.1414-1.9428-.1397.079-.674 7.2552-.3156.3703-.7286.2793-.6071-.4614-.3218-.7468.3218-1.4753.3886-1.9246.3157-1.53.2853-1.9004.17-.6314-.0121-.0425-.1397.0182-1.4328 1.9672-2.1796 2.9446-1.7243 1.8456-.4128.164-.7164-.3704.0667-.6618.4008-.5889 2.386-3.0357 1.4389-1.882.929-1.0868-.0062-.1579h-.0546l-6.3385 4.1164-1.1293.1457-.4857-.4554.0608-.7467.2307-.2429 1.9064-1.3114Z" />
    </svg>
  );
}

function CodexLogo({ size = 16, className }: LogoProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M16.0005 13.5992C16.4972 13.5995 16.9007 14.0029 16.9009 14.4996C16.9009 14.9965 16.4974 15.3997 16.0005 15.4H13.0005C12.5035 15.4 12.1002 14.9967 12.1002 14.4996C12.1004 14.0027 12.5036 13.5992 13.0005 13.5992H16.0005Z" />
      <path d="M8.03765 8.72811C8.46377 8.47284 9.01638 8.61074 9.27203 9.03671L10.6177 11.2789C10.8836 11.7221 10.8834 12.277 10.6177 12.7203L9.27203 14.9625C9.01633 15.3887 8.46385 15.5267 8.03765 15.2711C7.61143 15.0153 7.47333 14.4629 7.72906 14.0367L8.95074 11.9996L7.72906 9.96249C7.47341 9.53628 7.61146 8.98382 8.03765 8.72811Z" />
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M9.3316 2.03768C11.1644 1.54688 13.0268 2.04668 14.356 3.20174C16.0859 2.86512 17.9511 3.36418 19.2935 4.70663C20.6356 6.04882 21.1326 7.91288 20.7964 9.64217C21.9527 10.9716 22.4534 12.8349 21.9625 14.6685C21.4711 16.5025 20.1051 17.8657 18.439 18.439C17.8658 20.1047 16.503 21.47 14.6695 21.9615C12.8356 22.4528 10.9727 21.9519 9.64312 20.7955C7.91371 21.1319 6.04993 20.635 4.70757 19.2926C3.36558 17.9505 2.86659 16.0866 3.20269 14.357C2.04693 13.0273 1.54757 11.1638 2.03863 9.33065C2.52997 7.49759 3.89436 6.13282 5.56011 5.55917C6.13382 3.8935 7.49837 2.52891 9.3316 2.03768ZM13.2623 4.63534C12.3606 3.80149 11.067 3.4361 9.79742 3.77596C8.52745 4.11626 7.58924 5.0796 7.22515 6.25253C7.08145 6.71575 6.71753 7.07928 6.25445 7.22323C5.08133 7.58702 4.11844 8.52622 3.77789 9.79647C3.43759 11.0667 3.80148 12.3606 4.63531 13.2623C4.96467 13.6184 5.09776 14.1154 4.99078 14.5885C4.71939 15.7864 5.05109 17.0891 5.98101 18.0191C6.91085 18.949 8.21345 19.2801 9.41168 19.0084L9.58941 18.9791C9.94695 18.9432 10.3065 19.0397 10.5982 19.2496L10.7378 19.3639L10.9117 19.5142C11.8015 20.2407 13.0127 20.5413 14.2037 20.2223C15.4733 19.8818 16.4111 18.9191 16.775 17.7457C16.9189 17.2824 17.2832 16.9186 17.7466 16.775L17.9634 16.6998C19.0376 16.2923 19.9051 15.3936 20.2242 14.2027C20.5641 12.9328 20.1989 11.6385 19.3648 10.7369C19.0353 10.3806 18.9021 9.88399 19.0093 9.41073L19.0533 9.18514C19.2372 8.0514 18.8917 6.85176 18.0201 5.98006C17.0901 5.05003 15.7865 4.71826 14.5884 4.98983C14.1153 5.09712 13.6186 4.96467 13.2623 4.63534Z"
      />
    </svg>
  );
}

function AntigravityLogo({ size = 16, className, monochrome }: LogoProps) {
  if (monochrome) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 96 96"
        fill="currentColor"
        className={className}
        aria-hidden="true"
      >
        <path d="M85.973 89.344c5.25 3.937 13.125 1.312 5.907-5.906-21.657-21-17.063-78.75-43.97-78.75-26.905 0-22.312 57.75-43.968 78.75-7.875 7.874.656 9.843 5.906 5.906C30.192 75.563 28.88 51.28 47.911 51.28c19.031 0 17.719 24.282 38.062 38.063Z" />
      </svg>
    );
  }

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 96 96"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <g clipPath="url(#agy-a)">
        <path
          d="M85.973 89.344c5.25 3.937 13.125 1.312 5.907-5.906-21.657-21-17.063-78.75-43.97-78.75-26.905 0-22.312 57.75-43.968 78.75-7.875 7.874.656 9.843 5.906 5.906C30.192 75.563 28.88 51.28 47.911 51.28c19.031 0 17.719 24.282 38.062 38.063Z"
          fill="#3186FF"
        />
        <mask id="agy-b" style={{ maskType: "alpha" }} maskUnits="userSpaceOnUse" x="0" y="4" width="95" height="88">
          <path
            d="M85.973 89.344c5.25 3.937 13.125 1.312 5.907-5.906-21.657-21-17.063-78.75-43.97-78.75-26.905 0-22.312 57.75-43.968 78.75-7.875 7.874.656 9.843 5.906 5.906C30.192 75.563 28.88 51.28 47.911 51.28c19.031 0 17.719 24.282 38.062 38.063Z"
            fill="#000"
          />
        </mask>
        <g mask="url(#agy-b)">
          <g filter="url(#agy-c)">
            <ellipse cx="25.636" cy="30.161" rx="25.636" ry="30.161" transform="rotate(96.476 44.769 9.835)" fill="#FFE432" />
          </g>
          <g filter="url(#agy-d)">
            <ellipse cx="93.614" cy="23.451" rx="33.188" ry="33.918" transform="rotate(76.924 93.614 23.45)" fill="#FC413D" />
          </g>
          <g filter="url(#agy-e)">
            <ellipse cx="-4.781" cy="30.81" rx="34.686" ry="44.934" transform="rotate(74.126 -4.78 30.81)" fill="#00B95C" />
          </g>
          <g filter="url(#agy-f)">
            <ellipse cx="-4.781" cy="30.81" rx="34.686" ry="44.934" transform="rotate(74.126 -4.78 30.81)" fill="#00B95C" />
          </g>
          <g filter="url(#agy-g)">
            <ellipse cx="-2.314" cy="32.19" rx="33.998" ry="37.428" transform="rotate(45.606 -2.314 32.19)" fill="#00B95C" />
          </g>
          <g filter="url(#agy-h)">
            <ellipse cx="70.286" cy="101.861" rx="32.645" ry="31.436" transform="rotate(76.924 70.286 101.861)" fill="#3186FF" />
          </g>
          <g filter="url(#agy-i)">
            <ellipse cx="37.762" cy="39.83" rx="37.762" ry="39.83" transform="rotate(114.176 60.204 15.464)" fill="#FBBC04" />
          </g>
          <g filter="url(#agy-j)">
            <path
              d="M-12.05 152.344c-20.676-8.117 14.66-74.901 23.447-97.284 8.787-22.383 32.672-33.948 53.35-25.83 20.677 8.117 45.185 52.825 36.398 75.208C92.358 126.8 8.6 160.461-12.049 152.344Z"
              fill="#3186FF"
            />
          </g>
          <g filter="url(#agy-k)">
            <path
              d="M113.238 69.22c-5.536 6.462-19.982 3.167-32.266-7.358-12.285-10.525-17.756-24.295-12.22-30.756 5.536-6.46 19.982-3.166 32.266 7.359 12.284 10.525 17.755 24.295 12.22 30.756Z"
              fill="#749BFF"
            />
          </g>
          <g filter="url(#agy-l)">
            <ellipse cx="89.249" cy="10.708" rx="49.771" ry="30.939" transform="rotate(34.076 89.25 10.708)" fill="#FC413D" />
          </g>
          <g filter="url(#agy-m)">
            <ellipse cx="11.494" cy="17.224" rx="26.671" ry="15.51" transform="rotate(112.516 11.494 17.224)" fill="#FFEE48" />
          </g>
        </g>
      </g>
      <defs>
        <filter id="agy-c" x="-12.133" y="-45.923" width="77.726" height="68.909" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="4.377" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-d" x="17.408" y="-52.101" width="152.412" height="151.102" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="21.159" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-e" x="-85.014" y="-40.729" width="160.467" height="143.078" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="17.987" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-f" x="-85.014" y="-40.729" width="160.467" height="143.078" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="17.987" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-g" x="-74.079" y="-39.502" width="143.529" height="143.384" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="17.987" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-h" x="4.594" y="35.084" width="131.384" height="133.554" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="17.093" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-i" x="-23.316" y="-84.096" width="140.966" height="138.215" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="15.491" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-j" x="-46.111" y="-1.033" width="176.759" height="182.283" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="13.834" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-k" x="41.834" y="3.041" width="98.322" height="94.246" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="12.379" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-l" x="23.605" y="-48.077" width="131.289" height="117.57" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="10.456" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <filter id="agy-m" x="-31.984" y="-34.006" width="86.956" height="102.461" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="12.941" result="effect1_foregroundBlur_6153_71" />
        </filter>
        <clipPath id="agy-a">
          <path fill="#fff" d="M0 0h96v96H0z" />
        </clipPath>
      </defs>
    </svg>
  );
}

function OpenCodeLogo({ size = 16, className }: LogoProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path fillRule="evenodd" clipRule="evenodd" d="M5 3h14v18H5V3Zm3 3v12h8V6H8Z" />
      <path d="M10 9h4v6h-4z" opacity="0.55" />
    </svg>
  );
}

const COLORS: Record<ProviderId, string> = {
  "claude-code": "#c96442",
  codex: "#111111",
  antigravity: "#16171d",
  opencode: "#2a2626",
};

export function ProviderLogo({ id, size = 16, className, monochrome }: { id: ProviderId; size?: number; className?: string; monochrome?: boolean }) {
  if (id === "claude-code") return <ClaudeLogo size={size} className={className} />;
  if (id === "codex") return <CodexLogo size={size} className={className} />;
  if (id === "opencode") return <OpenCodeLogo size={size} className={className} />;
  return <AntigravityLogo size={size} className={className} monochrome={monochrome} />;
}

export function ProviderIcon({ id, size = 20, className }: { id: ProviderId; size?: number; className?: string }) {
  const color = COLORS[id];
  const isAgy = id === "antigravity";
  const logo = Math.max(11, Math.round(size * 0.6));
  return (
    <span
      aria-hidden
      className={cn(
        "relative flex shrink-0 items-center justify-center overflow-hidden text-white shadow-card",
        size >= 26 ? "rounded-[9px]" : "rounded-[6px]",
        isAgy && "border border-white/10",
        className,
      )}
      style={{
        width: size,
        height: size,
        background: isAgy
          ? "radial-gradient(circle at 50% 25%, #23252d 0%, #111216 100%)"
          : `linear-gradient(150deg, ${color}, color-mix(in oklab, ${color} 70%, black))`,
      }}
    >
      {isAgy && (
        <svg className="pointer-events-none absolute inset-0 h-full w-full" viewBox="0 0 30 30" fill="none">
          <path d="M0 6h30M0 12h30M0 18h30M0 24h30M6 0v30M12 0v30M18 0v30M24 0v30" stroke="rgba(255,255,255,0.08)" strokeWidth="0.5" />
        </svg>
      )}
      <ProviderLogo id={id} size={logo} className="relative" />
    </span>
  );
}
