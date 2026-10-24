import { useRef, type KeyboardEvent, type ReactElement } from "react";
import { Input } from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { renameTask } from "../actions";

/** The title as a text box: Enter or blur saves through `tasks_rename`, Escape cancels. */
export function RenameInline({ task, onDone }: { task: Pick<Task, "id" | "title">; onDone(): void }): ReactElement {
  const settled = useRef(false);
  const finish = (commit: boolean, value: string) => {
    if (settled.current) return;
    settled.current = true;
    if (commit) void renameTask(task.id, value);
    onDone();
  };
  const keyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    e.stopPropagation(); // typing must not reach the card's drag and open handlers
    if (e.key === "Enter") {
      e.preventDefault();
      finish(true, e.currentTarget.value);
    } else if (e.key === "Escape") {
      e.preventDefault();
      finish(false, "");
    }
  };
  return (
    <Input
      size="sm"
      aria-label="Task title"
      autoFocus
      defaultValue={task.title}
      onFocus={(e) => e.currentTarget.select()}
      onKeyDown={keyDown}
      onBlur={(e) => finish(true, e.currentTarget.value)}
      onClick={(e) => e.stopPropagation()}
      className="w-full"
    />
  );
}
