// The task body and its dock as one resizable split: below it (vertical) or beside it (the right inspector). The
// dock's share is the store's per-stage height; a drag sets it directly, a grow or a stage entry animates to it with
// the motion helpers (a spring, or a 150 ms ease under reduced motion).
import { animate } from "motion";
import { useEffect, useRef, useState, type ReactElement, type ReactNode } from "react";
import { Resizable } from "@/components/ui";
import type { DockPosition, Stage } from "@/lib/bindings";
import { transitionFor, useReducedMotion } from "@/lib/motion";
import { dockStore, type DockStore } from "../store";
import { DEFAULT_HEIGHT } from "../tabRules";

export interface DockSplitProps {
  stage: Stage;
  position: DockPosition;
  open: boolean;
  body: ReactNode;
  dock: ReactNode;
  store?: DockStore;
}

function useAnimatedShare(target: number): [number, (dragged: number) => void] {
  const reduced = useReducedMotion();
  const [shown, setShown] = useState(target);
  const current = useRef(target);
  const dragged = useRef<number | null>(null);
  useEffect(() => {
    if (dragged.current !== null && Math.abs(dragged.current - target) < 1e-6) {
      dragged.current = null;
      return;
    }
    const controls = animate(current.current, target, {
      ...transitionFor(reduced),
      onUpdate: (v: number) => {
        current.current = v;
        setShown(v);
      },
    });
    return () => controls.stop();
  }, [target, reduced]);
  const onDrag = (v: number) => {
    dragged.current = v;
    current.current = v;
    setShown(v);
  };
  return [shown, onDrag];
}

export function DockSplit({ stage, position, open, body, dock, store = dockStore }: DockSplitProps): ReactElement {
  const height = store.store.useSelector((s) => s.heightByStage[stage] ?? DEFAULT_HEIGHT);
  const [share, onDrag] = useAnimatedShare(height);
  if (!open) return <div className="h-full min-h-0">{body}</div>;
  return (
    <Resizable
      direction={position === "Right" ? "horizontal" : "vertical"}
      ratio={1 - share}
      min={0.15}
      handleLabel="Resize dock"
      onRatioChange={(r) => {
        onDrag(1 - r);
        store.setHeight(stage, 1 - r);
      }}
      first={body}
      second={dock}
    />
  );
}
