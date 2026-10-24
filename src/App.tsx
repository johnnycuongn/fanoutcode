// Placeholder root page: this window's title and the first-run state. Plan 06 replaces it with the shell, the start
// screen and onboarding.
import { lazy, Suspense, useEffect, useState } from "react";
import { commands } from "@/lib/bindings";

// Dev builds only: `#/dev/primitives` shows the primitives gallery. The dead branch in production lets Rollup drop it.
const PrimitivesGallery = import.meta.env.DEV ? lazy(() => import("./dev/primitives")) : null;

export function isDevPrimitivesRoute(): boolean {
  return import.meta.env.DEV && window.location.hash.startsWith("#/dev/primitives");
}

export default function App() {
  if (PrimitivesGallery && isDevPrimitivesRoute()) {
    return (
      <Suspense fallback={null}>
        <PrimitivesGallery />
      </Suspense>
    );
  }
  return <Home />;
}

function Home() {
  const [title, setTitle] = useState("Team Code");
  const [firstRunPending, setFirstRunPending] = useState(false);

  useEffect(() => {
    let live = true;
    void commands.windowsCurrent().then((r) => {
      if (live && r.status === "ok") setTitle(r.data.title);
    });
    void commands.settingsFirstRunPending().then((pending) => {
      if (live) setFirstRunPending(pending);
    });
    return () => {
      live = false;
    };
  }, []);

  return (
    <main className="grid h-full place-items-center bg-page text-ink">
      <div className="text-center">
        <h1 className="text-lg font-semibold">{title}</h1>
        <p className="text-mute">
          {firstRunPending ? "Onboarding pending." : "Start screen and onboarding arrive with Plan 06."}
        </p>
      </div>
    </main>
  );
}
