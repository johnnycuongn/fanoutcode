import { Avatar as AvatarPrimitive } from "radix-ui";
import { Children, cloneElement, isValidElement, type ComponentProps, type ReactElement } from "react";
import { Badge } from "./Badge";
import { cx } from "./cx";
import { StatusDot } from "./StatusDot";
import { swatchClasses, type AgentStatus, type Swatch } from "./tokens";

/** First letters of the first two words; one word gives its first two letters (a hyphenated one splits on hyphens). */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const parts = words.length === 1 ? words[0]!.split("-").filter(Boolean) : words;
  const letters = parts.length >= 2 ? parts[0]![0]! + parts[1]![0]! : (parts[0] ?? "").slice(0, 2);
  return letters.toUpperCase();
}

const SIZE = {
  xs: "size-4 text-[8px]",
  sm: "size-5 text-[9px]",
  md: "size-6 text-[10px]",
  lg: "size-8 text-xs",
} as const;

export type AvatarProps = ComponentProps<typeof AvatarPrimitive.Root> & {
  name: string;
  src?: string;
  size?: keyof typeof SIZE;
  swatch?: Swatch;
  shape?: "circle" | "square";
  /** A StatusDot overlay at the bottom-right. */
  status?: AgentStatus;
};

export function Avatar({
  name,
  src,
  size = "md",
  swatch,
  shape = "circle",
  status,
  className,
  ...props
}: AvatarProps): ReactElement {
  const root = (
    <AvatarPrimitive.Root
      role="img"
      aria-label={name}
      className={cx(
        "inline-flex shrink-0 select-none items-center justify-center overflow-hidden font-medium",
        SIZE[size],
        shape === "circle" ? "rounded-full" : "rounded-card",
        className,
      )}
      {...props}
    >
      {src && <AvatarPrimitive.Image src={src} alt="" className="size-full object-cover" />}
      <AvatarPrimitive.Fallback
        className={cx(
          "flex size-full items-center justify-center",
          swatch ? swatchClasses[swatch].soft : "bg-well text-mute",
        )}
      >
        {initials(name)}
      </AvatarPrimitive.Fallback>
    </AvatarPrimitive.Root>
  );
  if (!status) return root;
  return (
    <span className="relative inline-flex">
      {root}
      <StatusDot status={status} size="sm" className="absolute right-0 bottom-0 ring-2 ring-page" />
    </span>
  );
}

export type AvatarGroupProps = ComponentProps<"div"> & { max?: number; size?: AvatarProps["size"] };

/** An overlapping row; past `max` the rest collapse into a "+N" badge. */
export function AvatarGroup({ max = 3, size, className, children, ...props }: AvatarGroupProps): ReactElement {
  const all = Children.toArray(children);
  const shown = all.slice(0, max);
  const extra = all.length - shown.length;
  return (
    <div className={cx("flex items-center -space-x-1.5 [&>*]:ring-2 [&>*]:ring-page", className)} {...props}>
      {shown.map((child) =>
        isValidElement<AvatarProps>(child) && size && !child.props.size ? cloneElement(child, { size }) : child,
      )}
      {extra > 0 && <Badge size="sm" className="ml-1.5 rounded-full">{`+${extra}`}</Badge>}
    </div>
  );
}
