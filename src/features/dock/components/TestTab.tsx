// The Test tab (§6.5): the test command's output streamed in JetBrains Mono, following the tail while it runs, the
// exit code, Run again, Fix failing tests, and adding a linked project's tests to the run.
import { useEffect, useRef, type ReactElement } from "react";
import { Button, Chip, Menu, MenuContent, MenuItem, MenuTrigger, StatusDot } from "@/components/ui";
import { events, type Project, type Task } from "@/lib/bindings";
import { testStore, type TestStore } from "../testStore";

export function TestTab({
  task,
  projects,
  store = testStore,
}: {
  task: Task;
  projects: Project[];
  store?: TestStore;
}): ReactElement {
  const run = store.store.useSelector((s) => s[task.id]);
  const pre = useRef<HTMLPreElement | null>(null);
  const recorded = run !== undefined;

  useEffect(() => {
    if (task.stage === "Test" && !recorded) void store.attach(task.id);
  }, [store, task.id, task.stage, recorded]);
  useEffect(() => {
    const off = events.testFinished.listen((e) => {
      if (e.payload.task_id === task.id) store.onTestFinished(task.id, e.payload.code);
    });
    return () => void off.then((fn) => fn());
  }, [store, task.id]);
  useEffect(() => {
    if (run?.running && pre.current) pre.current.scrollTop = pre.current.scrollHeight;
  }, [run?.lines, run?.running]);

  const linked = projects.filter((p) => task.linked_roots.includes(p.root));
  const failed = run?.code !== null && run?.code !== undefined && run.code !== 0;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-1">
        <Button size="sm" variant="secondary" disabled={run?.running} onClick={() => void store.run(task.id)}>
          Run again
        </Button>
        {failed && (
          <Button size="sm" onClick={() => void store.fix(task.id)}>
            Fix failing tests
          </Button>
        )}
        {linked.length > 0 && (
          <Menu>
            <MenuTrigger asChild>
              <Button size="sm" variant="ghost">
                Add linked project&apos;s tests
              </Button>
            </MenuTrigger>
            <MenuContent>
              {linked.map((p) => (
                <MenuItem key={p.id} onSelect={() => void store.addLinked(task.id, p.id)}>
                  {p.name}
                </MenuItem>
              ))}
            </MenuContent>
          </Menu>
        )}
        <span className="flex-1" />
        {run?.running && <StatusDot status="working" pulse label="Running" />}
        {run && run.code !== null && (
          <Chip size="sm" tone={run.code === 0 ? "ok" : "bad"}>
            {`exit ${run.code}`}
          </Chip>
        )}
      </div>
      {run?.error && (
        <p role="alert" className="px-3 py-1 text-xs text-bad">
          {run.error}
        </p>
      )}
      <pre ref={pre} className="min-h-0 flex-1 overflow-auto px-3 py-2 font-mono text-[12px] leading-snug">
        {run ? [...run.lines, run.partial].join("\n") : "No test run yet."}
      </pre>
    </div>
  );
}
