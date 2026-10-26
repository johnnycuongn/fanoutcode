// The forward step asks first when it would hide something: "Mark done" while the task's files are still uncommitted
// (Fanout Code may commit off); "Approve changes" while review.md lists blockers (the count also shows on the button),
// and, with no test command yet, whether to use the one the project's files suggest. A detected command runs only
// once the developer accepts it here or in Settings: it is the repository's own script.
import { useEffect, useState, type ReactNode } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  toast,
} from "@/components/ui";
import { commands, events, type DetectedTest, type StageAction, type Task } from "@/lib/bindings";
import { uncommittedCount } from "@/lib/uncommitted";
import { parseReviewNotes } from "@/features/changes";

type Choice = "cancel" | "confirm" | "secondary";
interface Ask {
  title: string;
  body: string;
  confirm: string;
  /** A second way forward ("Skip tests"), between Cancel and the confirm button. */
  secondary?: string;
  resolve(choice: Choice): void;
}

/** The tests the guard may offer at Approve changes: the project's saved command, and whether tests are on. */
export interface GuardTests {
  command: string | null;
  enabled: boolean;
}

const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

/** The blocker findings ("critical") in the task's review.md; [] when there is none or it cannot be read. */
async function reviewBlockers(taskId: string): Promise<string[]> {
  try {
    const r = await commands.memoryRead(taskId, "review.md");
    const md = r?.status === "ok" ? r.data : null;
    return md
      ? parseReviewNotes(md)
          .filter((n) => n.severity === "critical")
          .map((n) => n.text)
      : [];
  } catch {
    return [];
  }
}

/** The blockers review.md lists while the task is in Review, kept fresh as the task changes. */
function useBlockers(task: Pick<Task, "id" | "stage"> | null): string[] {
  const [blockers, setBlockers] = useState<string[]>([]);
  const id = task?.id ?? null;
  const inReview = task?.stage === "Review";
  useEffect(() => {
    if (!id || !inReview) return;
    let live = true;
    const load = () => void reviewBlockers(id).then((b) => live && setBlockers(b));
    load();
    const off = events.taskChanged
      .listen((e) => {
        if (e.payload.task.id === id) load();
      })
      .catch(() => () => {});
    return () => {
      live = false;
      void off.then((fn) => fn());
    };
  }, [id, inReview]);
  return inReview ? blockers : [];
}

async function detectTest(projectId: string): Promise<DetectedTest | null> {
  try {
    const r = await commands.projectsDetectTest(projectId);
    return r?.status === "ok" ? r.data : null;
  } catch {
    return null;
  }
}

export function useForwardGuard(
  task: Pick<Task, "id" | "stage" | "project_id"> | null,
  tests: GuardTests = { command: null, enabled: false },
): {
  /** Blockers review.md lists, while in Review. */
  blockers: number;
  /** Resolves true when `action` may go ahead. */
  guard(action: StageAction): Promise<boolean>;
  /** Runs `go` once `action` may go ahead. */
  ifAllowed(action: StageAction, go: () => Promise<unknown>): void;
  dialog: ReactNode;
} {
  const [ask, setAsk] = useState<Ask | null>(null);
  const blockers = useBlockers(task);
  const choose = (a: Omit<Ask, "resolve">) => new Promise<Choice>((resolve) => setAsk({ ...a, resolve }));
  const confirmWith = async (a: Omit<Ask, "resolve">) => (await choose(a)) === "confirm";
  const { command: testCommand, enabled: testsOn } = tests;
  /** No saved test command but the project suggests one: use it (saved), skip tests, or stop. */
  const offerTests = async (projectId: string): Promise<boolean> => {
    if (!testsOn || testCommand?.trim()) return true;
    const found = await detectTest(projectId);
    if (!found) return true;
    const choice = await choose({
      title: `Run the tests with ${found.command}?`,
      body: `${found.source} has a test script, and this project has no test command yet. Using it saves ${found.command} as the project's test command and runs it in the Test stage. It runs this repository's own script, so use it only if you trust the code.`,
      confirm: `Use ${found.command}`,
      secondary: "Skip tests",
    });
    if (choice === "cancel") return false;
    if (choice === "secondary") return true;
    const r = await commands.settingsSet(
      { workspace: null, project: projectId, task: null },
      "test_command",
      found.command,
    );
    if (r?.status === "error") toast.error(r.error);
    return r?.status === "ok";
  };
  const guard = async (action: StageAction): Promise<boolean> => {
    if (!task) return true;
    if (task.stage === "Commit" && action.type === "Advance") {
      const n = await uncommittedCount(task.id);
      if (n === 0) return true;
      return confirmWith({
        title: "Mark done with uncommitted changes?",
        body: `${n} ${plural(n, "file", "files")} from this task ${plural(n, "is", "are")} not committed yet. Commit ${plural(n, "it", "them")} from your terminal first, or mark the task done now and its card will say it is not committed.`,
        confirm: "Mark done anyway",
      });
    }
    if (task.stage === "Review" && action.type === "ApproveChanges") {
      const found = await reviewBlockers(task.id);
      const n = found.length;
      if (
        n > 0 &&
        !(await confirmWith({
          title: `Approve with ${n} ${plural(n, "blocker", "blockers")} in review?`,
          body: `review.md lists ${n} ${plural(n, "blocker", "blockers")}, starting with: ${found[0]} Ask the agent to fix ${plural(n, "it", "them")} first, or approve anyway.`,
          confirm: "Approve anyway",
        }))
      )
        return false;
      return offerTests(task.project_id);
    }
    return true;
  };
  const settle = (choice: Choice) => {
    ask?.resolve(choice);
    setAsk(null);
  };
  const ifAllowed = (action: StageAction, go: () => Promise<unknown>) =>
    void guard(action).then((ok) => (ok ? go() : undefined));
  const dialog = ask ? (
    <Dialog open onOpenChange={(open) => !open && settle("cancel")}>
      <DialogContent size="sm" hideClose>
        <DialogHeader>
          <DialogTitle>{ask.title}</DialogTitle>
          <DialogDescription>{ask.body}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button onClick={() => settle("cancel")}>Cancel</Button>
          {ask.secondary && <Button onClick={() => settle("secondary")}>{ask.secondary}</Button>}
          <Button variant="primary" onClick={() => settle("confirm")}>
            {ask.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  ) : null;
  return { blockers: blockers.length, guard, ifAllowed, dialog };
}
