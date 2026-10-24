import type { ReactElement } from "react";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui";
import { setPickedProject, useBoard } from "../store";

/** One project at a time: pick a project, or All for a read-only overview. */
export function ProjectPicker(): ReactElement {
  const picked = useBoard((s) => s.pickedProject);
  const projects = useBoard((s) => s.projects);
  const order = useBoard((s) => s.projectOrder);
  return (
    <Select value={picked ?? "all"} onValueChange={(value) => setPickedProject(value === "all" ? null : value)}>
      <SelectTrigger size="sm" aria-label="Project" className="w-44">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">All</SelectItem>
        {order.map((id) => (
          <SelectItem key={id} value={id}>
            {projects[id]?.name ?? id}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
