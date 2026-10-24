// The window's root: the shell for a project or workspace window, otherwise the start screen (or onboarding on
// first run). Dev builds also serve the primitives gallery at `#/dev/primitives`.
import { lazy, Suspense, useEffect, useState, type ReactElement } from "react";
import { UiProvider } from "@/components/ui";
import { commands, type WindowInfo } from "@/lib/bindings";
import { ShellLayout } from "@/features/shell";

// Dev builds only: `#/dev/primitives` shows the primitives gallery. The dead branch in production lets Rollup drop it.
const PrimitivesGallery = import.meta.env.DEV ? lazy(() => import("./dev/primitives")) : null;

export function isDevPrimitivesRoute(): boolean {
  return import.meta.env.DEV && window.location.hash.startsWith("#/dev/primitives");
}

export default function App(): ReactElement {
  if (PrimitivesGallery && isDevPrimitivesRoute()) {
    return (
      <Suspense fallback={null}>
        <PrimitivesGallery />
      </Suspense>
    );
  }
  return (
    <UiProvider>
      <Root />
    </UiProvider>
  );
}

function Root(): ReactElement | null {
  const [info, setInfo] = useState<WindowInfo | null>(null);
  const [firstRunPending, setFirstRunPending] = useState(false);

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
  return (
    <main className="grid h-full place-items-center bg-page text-ink">
      <div className="text-center">
        <h1 className="text-lg font-semibold">{info.title}</h1>
        <p className="text-mute">{firstRunPending ? "Onboarding pending." : "Start screen."}</p>
      </div>
    </main>
  );
}
