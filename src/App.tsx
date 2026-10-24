// Placeholder root page: this window's title and the first-run state. Plan 06 replaces it with the shell, the start
// screen and onboarding.
import { useEffect, useState } from "react";
import { commands } from "@/lib/bindings";

export default function App() {
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
