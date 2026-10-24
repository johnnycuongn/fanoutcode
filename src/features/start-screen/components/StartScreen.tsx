// The start window: recent projects and workspaces, open buttons, and a new workspace from the open windows.
import { useEffect, useState, type ReactElement } from "react";
import { Button, Panel, Tooltip } from "@/components/ui";
import { commands } from "@/lib/bindings";
import { newWorkspaceFromOpenWindows, openProjectDialog, openWorkspaceDialog } from "../actions";
import { loadRecents, startStore } from "../store";
import { RecentList } from "./RecentList";

function useHasProjectWindows(): boolean {
  const [has, setHas] = useState(false);
  useEffect(() => {
    let live = true;
    const check = () =>
      void Promise.resolve(commands.windowsList()).then(
        (w) => live && setHas(Array.isArray(w) && w.some((x) => x.container?.type === "Project")),
      );
    check();
    window.addEventListener("focus", check);
    return () => {
      live = false;
      window.removeEventListener("focus", check);
    };
  }, []);
  return has;
}

export function StartScreen(): ReactElement {
  const recents = startStore.useSelector((s) => s.recents);
  const hasProjects = useHasProjectWindows();
  useEffect(() => void loadRecents(), []);
  const newWorkspace = (
    <Button variant="secondary" disabled={!hasProjects} onClick={() => void newWorkspaceFromOpenWindows()}>
      New workspace from open windows
    </Button>
  );
  return (
    <main className="grid h-full place-items-center bg-page p-6 text-ink">
      <Panel variant="card" padding="md" className="flex w-full max-w-2xl flex-col gap-5">
        <h1 className="text-lg font-semibold">Team Code</h1>
        <div className="flex flex-wrap gap-2">
          <Button onClick={() => void openProjectDialog()}>Open project…</Button>
          <Button variant="secondary" onClick={() => void openWorkspaceDialog()}>
            Open workspace…
          </Button>
          {hasProjects ? (
            newWorkspace
          ) : (
            <Tooltip content="Open at least one project window first">
              <span tabIndex={0}>{newWorkspace}</span>
            </Tooltip>
          )}
        </div>
        <RecentList title="Recent projects" entries={recents.filter((r) => r.kind === "Project")} />
        <RecentList title="Recent workspaces" entries={recents.filter((r) => r.kind === "Workspace")} />
      </Panel>
    </main>
  );
}
