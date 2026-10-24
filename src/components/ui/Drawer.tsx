import { createContext, useContext, type ComponentProps, type ReactElement } from "react";
import { Drawer as VaulDrawer } from "vaul";
import { useReducedMotion } from "@/lib/motion";
import { cx } from "./cx";

type Side = "bottom" | "right";
const SideContext = createContext<Side>("bottom");

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

// Vaul's root props are a union (with and without `fadeFromIndex`), so the omit has to distribute over it.
export type DrawerProps = DistributiveOmit<
  ComponentProps<typeof VaulDrawer.Root>,
  "direction" | "shouldScaleBackground"
> & {
  side?: Side;
};

export function Drawer({ side = "bottom", ...props }: DrawerProps): ReactElement {
  return (
    <SideContext.Provider value={side}>
      <VaulDrawer.Root
        direction={side}
        shouldScaleBackground={false}
        {...(props as ComponentProps<typeof VaulDrawer.Root>)}
      />
    </SideContext.Provider>
  );
}

export const DrawerTrigger = VaulDrawer.Trigger;
export const DrawerClose = VaulDrawer.Close;
export const DrawerPortal = VaulDrawer.Portal;

export type DrawerContentProps = ComponentProps<typeof VaulDrawer.Content> & { size?: "sm" | "md" | "lg" };

const BOTTOM = { sm: "max-h-[40vh]", md: "max-h-[60vh]", lg: "max-h-[85vh]" } as const;
const RIGHT = { sm: "w-[320px]", md: "w-[420px]", lg: "w-[560px]" } as const;

/** Vaul content already carries `data-vaul-drawer`, so `data-motion="reduced"` lets primitives.css cap it at 150 ms. */
export function DrawerContent({ size = "md", className, children, ...props }: DrawerContentProps): ReactElement {
  const side = useContext(SideContext);
  const reduced = useReducedMotion();
  const motion = reduced ? "reduced" : "full";
  return (
    <VaulDrawer.Portal>
      <VaulDrawer.Overlay data-motion={motion} className="fixed inset-0 z-50 bg-ink/40" />
      <VaulDrawer.Content
        data-motion={motion}
        className={cx(
          "fixed z-50 flex flex-col border-line bg-card text-ink shadow-card outline-none",
          side === "bottom"
            ? cx("inset-x-0 bottom-0 rounded-t-well border-t", BOTTOM[size])
            : cx("inset-y-0 right-0 rounded-l-well border-l", RIGHT[size]),
          className,
        )}
        {...props}
      >
        {side === "bottom" && <VaulDrawer.Handle className="mt-2 mb-1" />}
        {children}
      </VaulDrawer.Content>
    </VaulDrawer.Portal>
  );
}

export function DrawerHeader({ className, ...props }: ComponentProps<"div">): ReactElement {
  return <div className={cx("flex flex-col gap-1 p-4 pb-2", className)} {...props} />;
}

export function DrawerTitle({ className, ...props }: ComponentProps<typeof VaulDrawer.Title>): ReactElement {
  return <VaulDrawer.Title className={cx("font-medium text-ink", className)} {...props} />;
}

export function DrawerDescription({
  className,
  ...props
}: ComponentProps<typeof VaulDrawer.Description>): ReactElement {
  return <VaulDrawer.Description className={cx("text-sm text-mute", className)} {...props} />;
}

export function DrawerFooter({ className, ...props }: ComponentProps<"div">): ReactElement {
  return <div className={cx("mt-auto flex justify-end gap-2 p-4 pt-2", className)} {...props} />;
}
