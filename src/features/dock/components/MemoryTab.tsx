// The Memory tab (§8): the task's memory files rendered, the stage's file first, an Edit switch that saves through
// memory_write (500 ms after typing, and when switched off or on ⌘S), live re-reads when an agent writes the file
// on screen, and the spec watcher's log. A missing memory folder shows its message and turns editing off.
import { useEffect, useState, type ReactElement } from "react";
import { EmptyState, Segmented, Switch, Textarea } from "@/components/ui";
import { events, type Task } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";
import { MiniMarkdown } from "@/livespec/MiniMarkdown";
import {
  MEMORY_FILES,
  createMemorySaver,
  isEditable,
  memoryFileFor,
  shouldReload,
  type MemoryFile,
  memoryKey,
  type MemoryIpc,
} from "../memory";
import { WatcherLog } from "./WatcherLog";

const memoryIpc: MemoryIpc = live("memoryRead", "memoryWrite", "memoryMissing");

async function read(ipc: MemoryIpc, task: Task, file: MemoryFile): Promise<string> {
  const text = await ipc.memoryRead(task.id, file);
  if (typeof text === "string") return text;
  const missing = (await ipc.memoryMissing?.({ type: "Project", id: task.project_id })) ?? [];
  if (missing.includes(task.id)) throw `Task memory folder is missing: ${task.memory_dir}`;
  return "";
}

export function MemoryTab({ task, ipc = memoryIpc }: { task: Task; ipc?: MemoryIpc }): ReactElement {
  const [file, setFile] = useState<MemoryFile>(memoryFileFor(task.stage));
  const [stage, setStage] = useState(task.stage);
  const [doc, setDoc] = useState<{ file: MemoryFile; text: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [editing, setEditing] = useState(false);
  const [reload, setReload] = useState(0);
  const [saver] = useState(() =>
    createMemorySaver(
      ipc,
      () => setDirty(false),
      (e) => setError(errorMessage(e)),
    ),
  );

  // A new stage opens on its file, unless the developer is in the middle of editing one (Review Focus 5).
  if (task.stage !== stage) {
    setStage(task.stage);
    if (!dirty && !editing) setFile(memoryFileFor(task.stage));
  }

  useEffect(() => {
    let live = true;
    read(ipc, task, file).then(
      (text) => {
        if (!live) return;
        setDoc({ file, text });
        setError(null);
      },
      (e) => live && setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
    // task.id and the reload counter, not the task object: a task-changed alone must not re-read
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ipc, task.id, file, reload]);

  useEffect(() => {
    const offs = [
      events.teamEvent.listen((e) => {
        if (e.payload.task_id === task.id && shouldReload(e.payload.event, file, dirty)) setReload((n) => n + 1);
      }),
      events.taskChanged.listen((e) => {
        if (e.payload.task.id === task.id && file === "index.md") setReload((n) => n + 1);
      }),
    ];
    return () => {
      for (const off of offs) void off.then((fn) => fn());
    };
  }, [task.id, file, dirty]);

  const text = doc?.file === file ? doc.text : null;
  const key = memoryKey(task.id, file);
  const switchFile = (next: MemoryFile) => {
    if (dirty) void saver.flush(key);
    setEditing(false);
    setDirty(false);
    setFile(next);
  };
  const toggleEdit = (on: boolean) => {
    if (!on) void saver.flush(key);
    setEditing(on);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-3 border-b border-line px-3 py-1.5">
        <Segmented
          aria-label="Memory file"
          size="sm"
          value={file}
          onValueChange={switchFile}
          options={MEMORY_FILES.map((f) => ({ value: f.file, label: f.label }))}
        />
        <span className="flex-1" />
        {error === null && isEditable(file) && (
          <Switch size="sm" label="Edit" checked={editing} onCheckedChange={toggleEdit} />
        )}
        {error === null && !isEditable(file) && <span className="text-xs text-mute">Written by Team Code</span>}
      </div>
      <div className="min-h-0 flex-1 overflow-auto px-4 py-3">
        {error !== null && (
          <p role="alert" className="text-[13px] text-bad">
            {error}
          </p>
        )}
        {error === null && text !== null && editing && (
          <Textarea
            mono
            aria-label={file}
            className="h-full min-h-48 resize-none"
            value={text}
            onChange={(e) => {
              setDoc({ file, text: e.target.value });
              setDirty(true);
              saver.edit(key, e.target.value);
            }}
            onKeyDown={(e) => {
              if (e.key === "s" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                void saver.flush(key);
              }
            }}
          />
        )}
        {error === null && text !== null && !editing && text.trim() !== "" && <MiniMarkdown text={text} />}
        {error === null && text !== null && !editing && text.trim() === "" && (
          <EmptyState size="sm" title={`${file} is empty`} />
        )}
        <WatcherLog taskId={task.id} />
      </div>
    </div>
  );
}
