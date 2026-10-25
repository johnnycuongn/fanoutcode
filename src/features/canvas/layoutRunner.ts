// Debounced layout and persistence for the canvas store: a graph change re-lays out 150 ms after the last change
// (tidy, Build start and the first open run at once), and positions are saved 500 ms after they settle.
import { live } from "@/lib/ipcResult";
import { layoutGraph } from "./layout";
import { prunePositions, resolvePositions, type LayoutTrigger } from "./positions";
import { getCanvas, isCanvasOpen, markSaved, setPositions } from "./canvasState";

export const LAYOUT_DEBOUNCE_MS = 150;
export const PERSIST_DEBOUNCE_MS = 500;

const ipc = live("canvasPositionsSet");
const layoutTimers = new Map<string, ReturnType<typeof setTimeout>>();
const persistTimers = new Map<string, ReturnType<typeof setTimeout>>();
const generation = new Map<string, number>();

export function cancelLayout(taskId: string): void {
  clearTimeout(layoutTimers.get(taskId));
  layoutTimers.delete(taskId);
}

export function scheduleLayout(taskId: string, trigger: LayoutTrigger): void {
  cancelLayout(taskId);
  if (trigger !== "change") {
    void runLayout(taskId, trigger);
    return;
  }
  layoutTimers.set(
    taskId,
    setTimeout(() => {
      layoutTimers.delete(taskId);
      void runLayout(taskId, "change");
    }, LAYOUT_DEBOUNCE_MS),
  );
}

/** Lays the current graph out and applies the Auto re-flow / Pinned policy; a newer run supersedes an older one. */
export async function runLayout(taskId: string, trigger: LayoutTrigger): Promise<void> {
  const s = getCanvas(taskId);
  if (!s.task || !isCanvasOpen(taskId)) return;
  const gen = (generation.get(taskId) ?? 0) + 1;
  generation.set(taskId, gen);
  let laidOut;
  try {
    laidOut = await layoutGraph(s.graph, s.settings.arrangement, { stackDone: s.settings.layout === "AutoReflow" });
  } catch {
    return;
  }
  if (generation.get(taskId) !== gen || !isCanvasOpen(taskId)) return;
  const now = getCanvas(taskId);
  const nodeIds = now.graph.nodes.map((n) => n.id);
  setPositions(
    taskId,
    resolvePositions({ mode: now.settings.layout, trigger, previous: now.positions, laidOut, nodeIds }),
  );
  schedulePersist(taskId);
}

function persistNow(taskId: string): void {
  const s = getCanvas(taskId);
  if (!s.task) return;
  const positions = prunePositions(
    s.positions,
    s.graph.nodes.map((n) => n.id),
  );
  void ipc.canvasPositionsSet(taskId, positions).then(
    () =>
      markSaved(
        taskId,
        positions.map((p) => p.node_id),
      ),
    () => undefined,
  );
}

export function schedulePersist(taskId: string): void {
  clearTimeout(persistTimers.get(taskId));
  persistTimers.set(
    taskId,
    setTimeout(() => {
      persistTimers.delete(taskId);
      persistNow(taskId);
    }, PERSIST_DEBOUNCE_MS),
  );
}

/** Saves a pending change now (the canvas is closing). */
export function flushPersist(taskId: string): void {
  if (!persistTimers.has(taskId)) return;
  clearTimeout(persistTimers.get(taskId));
  persistTimers.delete(taskId);
  persistNow(taskId);
}
