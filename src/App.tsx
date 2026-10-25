// The window's root: the shell for a project or workspace window, otherwise the start screen (or onboarding on
// first run). Dev builds also serve the primitives gallery at `#/dev/primitives` and the canvas playground at
// `#/dev/canvas`.
import { lazy, Suspense, useEffect, useState, type ReactElement } from "react";
import { UiProvider } from "@/components/ui";
import { commands, type WindowInfo } from "@/lib/bindings";

// Each kind of window loads only what it shows.
const ShellLayout = lazy(() => import("@/features/shell").then((m) => ({ default: m.ShellLayout })));
const StartScreen = lazy(() => import("@/features/start-screen").then((m) => ({ default: m.StartScreen })));
const Onboarding = lazy(() => import("@/features/start-screen").then((m) => ({ default: m.Onboarding })));
// The review of a workspace file from elsewhere, shown over any window (Plan 10).
const WorkspaceImportDialog = lazy(() =>
  import("@/features/workspace").then((m) => ({ default: m.WorkspaceImportDialog })),
);

// Dev builds only: `#/dev/primitives` shows the primitives gallery. The dead branch in production lets Rollup drop it.
const PrimitivesGallery = import.meta.env.DEV ? lazy(() => import("./dev/primitives")) : null;
const CanvasPlayground = import.meta.env.DEV ? lazy(() => import("./dev/canvas/CanvasPlayground")) : null;

export function isDevPrimitivesRoute(): boolean {
  return import.meta.env.DEV && window.location.hash.startsWith("#/dev/primitives");
}

export default function App(): ReactElement {
  if (CanvasPlayground && import.meta.env.DEV && window.location.hash.startsWith("#/dev/canvas")) {
    return (
      <Suspense fallback={null}>
        <CanvasPlayground />
      </Suspense>
    );
  }
  if (PrimitivesGallery && isDevPrimitivesRoute()) {
    return (
      <Suspense fallback={null}>
        <PrimitivesGallery />
      </Suspense>
    );
  }
  return (
    <UiProvider>
      <Suspense fallback={null}>
        <Root />
        <WorkspaceImportDialog />
      </Suspense>
    </UiProvider>
  );
}

function Root(): ReactElement | null {
  const [info, setInfo] = useState<WindowInfo | null>(null);
  const [firstRunPending, setFirstRunPending] = useState<boolean | null>(null);
  const recheck = () => void commands.settingsFirstRunPending().then(setFirstRunPending);

  useEffect(() => {
    let live = true;
    void commands.windowsCurrent().then((r) => {
      if (live && r.status === "ok") setInfo(r.data);
    });
    void commands.settingsFirstRunPending().then((pending) => {
      if (live) setFirstRunPending(pending);
    });
    return () => {
      live = false;
    };
  }, []);

  if (!info) return null;
  if (info.container) return <ShellLayout info={info} />;
  if (firstRunPending === null) return null;
  return firstRunPending ? <Onboarding onDone={recheck} /> : <StartScreen />;
}
