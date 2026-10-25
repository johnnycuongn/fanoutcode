// The Fanout Code brand mark: the app icon's art (src-tauri/icons/source/icon.svg) drawn inline, coloured by the
// brand tokens and the stage tokens in src/styles.css. `spinning` turns the fan and lets the agents drift outward;
// under reduced motion nothing moves.
import { useId, type ComponentProps, type ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx } from "./cx";

export type AppMarkProps = Omit<ComponentProps<"svg">, "children" | "width" | "height"> & {
  /** Width and height in px. */
  size?: number;
  /** Turns the blades (1.6 s a turn) and drifts the agents outward; never under reduced motion. */
  spinning?: boolean;
  /** Accessible name; without one the mark is decorative. */
  label?: string;
};

// The five agents flying out of the fan: a trail and a dot each, in the pipeline's stage colours. Class names are
// literal so Tailwind sees them.
const AGENTS = [
  { fill: "fill-stage-spec", stroke: "stroke-stage-spec", from: [687, 406], at: [735, 330], r: 30, w: 26 },
  { fill: "fill-stage-plan", stroke: "stroke-stage-plan", from: [711, 463], at: [790, 420], r: 34, w: 30 },
  { fill: "fill-stage-build", stroke: "stroke-stage-build", from: [725, 512], at: [815, 512], r: 38, w: 34 },
  { fill: "fill-stage-review", stroke: "stroke-stage-review", from: [711, 561], at: [790, 604], r: 34, w: 30 },
  { fill: "fill-stage-commit", stroke: "stroke-stage-commit", from: [687, 618], at: [735, 694], r: 30, w: 26 },
] as const;

const BLADE_ANGLES = [0, 120, 240] as const;

export function AppMark({ size = 32, spinning = false, label, className, ...props }: AppMarkProps): ReactElement {
  const reduced = useReducedMotion();
  const spin = spinning && !reduced;
  // useId's characters are not all valid in a url(#…) reference.
  const id = `app-mark-${useId().replace(/[^\w-]/g, "")}`;
  const navy = `${id}-navy`;
  const blade = `${id}-blade`;
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 1024 1024"
      width={size}
      height={size}
      {...(label ? { role: "img", "aria-label": label } : { "aria-hidden": true })}
      data-spinning={spin ? "true" : "false"}
      className={cx("shrink-0", className)}
      {...props}
    >
      <defs>
        <linearGradient id={navy} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" className="[stop-color:var(--brand-navy-from)]" />
          <stop offset="1" className="[stop-color:var(--brand-navy-to)]" />
        </linearGradient>
        <linearGradient id={blade} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className="[stop-color:var(--brand-blade-from)]" />
          <stop offset="1" className="[stop-color:var(--brand-blade-to)]" />
        </linearGradient>
      </defs>
      <rect x="100" y="100" width="824" height="824" rx="185" fill={`url(#${navy})`} />
      <circle
        cx="400"
        cy="512"
        r="215"
        fill="none"
        strokeWidth="18"
        strokeOpacity=".9"
        className="[stroke:var(--brand-guard)]"
      />
      <circle
        cx="400"
        cy="512"
        r="150"
        fill="none"
        strokeWidth="8"
        strokeOpacity=".3"
        className="[stroke:var(--brand-guard)]"
      />
      <g data-part="blades" className={cx("origin-[400px_512px]", spin && "animate-app-mark-spin")}>
        {BLADE_ANGLES.map((a) => (
          <ellipse
            key={a}
            cx="400"
            cy="420"
            rx="56"
            ry="104"
            fill={`url(#${blade})`}
            transform={`rotate(${a} 400 512)`}
          />
        ))}
      </g>
      <circle cx="400" cy="512" r="36" className="[fill:var(--brand-hub)]" />
      <g data-part="agents" className={cx(spin && "animate-app-mark-drift")}>
        {AGENTS.map(({ fill, stroke, from, at, r, w }) => (
          <g key={fill}>
            <line
              x1={from[0]}
              y1={from[1]}
              x2={at[0]}
              y2={at[1]}
              strokeWidth={w}
              strokeLinecap="round"
              strokeOpacity=".38"
              className={stroke}
            />
            <circle cx={at[0]} cy={at[1]} r={r} className={fill} />
          </g>
        ))}
      </g>
    </svg>
  );
}
