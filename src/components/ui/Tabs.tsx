import { Tabs as TabsPrimitive } from "radix-ui";
import { createContext, useContext, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { Badge } from "./Badge";
import { cx, focusRing } from "./cx";
import type { Tone } from "./tokens";

type ListVariant = "underline" | "pill" | "bar";
type ListSize = "sm" | "md";

const ListContext = createContext<{ variant: ListVariant; size: ListSize }>({ variant: "underline", size: "md" });

export const Tabs = TabsPrimitive.Root;

export type TabsListProps = ComponentProps<typeof TabsPrimitive.List> & {
  /** "bar" is the window tab-strip look on bg-bar. */
  variant?: ListVariant;
  size?: ListSize;
};

const LIST: Record<ListVariant, string> = {
  underline: "gap-1 border-b border-line",
  pill: "inline-flex gap-0.5 rounded-well bg-well p-0.5",
  bar: "gap-0.5 bg-bar px-1 pt-1",
};

export function TabsList({ variant = "underline", size = "md", className, ...props }: TabsListProps): ReactElement {
  return (
    <ListContext.Provider value={{ variant, size }}>
      <TabsPrimitive.List
        data-variant={variant}
        className={cx("flex items-center", LIST[variant], className)}
        {...props}
      />
    </ListContext.Provider>
  );
}

const TRIGGER: Record<ListVariant, string> = {
  underline:
    "-mb-px border-b-2 border-transparent text-mute hover:text-ink data-[state=active]:border-accent data-[state=active]:text-ink",
  pill: "rounded-card text-mute hover:text-ink data-[state=active]:bg-card data-[state=active]:text-ink data-[state=active]:shadow-card",
  bar: "rounded-t-card text-mute hover:text-ink data-[state=active]:bg-page data-[state=active]:text-ink",
};

export type TabsTriggerProps = ComponentProps<typeof TabsPrimitive.Trigger> & {
  icon?: ReactNode;
  /** An amber badge; hidden when 0 or undefined. */
  count?: number;
  countTone?: Tone;
};

export function TabsTrigger({ icon, count, countTone, className, children, ...props }: TabsTriggerProps): ReactElement {
  const { variant, size } = useContext(ListContext);
  return (
    <TabsPrimitive.Trigger
      className={cx(
        "inline-flex items-center gap-1.5 whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50",
        size === "sm" ? "h-6 px-2 text-xs" : "h-7 px-2.5 text-sm",
        TRIGGER[variant],
        focusRing,
        className,
      )}
      {...props}
    >
      {icon}
      {children}
      {count ? (
        <Badge tone={countTone ?? "warn"} size="sm">
          {count}
        </Badge>
      ) : null}
    </TabsPrimitive.Trigger>
  );
}

export function TabsContent({ className, ...props }: ComponentProps<typeof TabsPrimitive.Content>): ReactElement {
  return <TabsPrimitive.Content className={cx("outline-none", focusRing, className)} {...props} />;
}
