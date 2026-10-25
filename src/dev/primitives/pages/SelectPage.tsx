import { useState, type ReactElement } from "react";
import {
  Field,
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "../../../components/ui";

function Priority({ size }: { size?: "sm" | "md" }): ReactElement {
  const [value, setValue] = useState("medium");
  return (
    <Select value={value} onValueChange={setValue}>
      <SelectTrigger size={size} aria-label="Priority">
        <SelectValue placeholder="Priority" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="none">None</SelectItem>
        <SelectItem value="low">Low</SelectItem>
        <SelectItem value="medium">Medium</SelectItem>
        <SelectItem value="high">High</SelectItem>
      </SelectContent>
    </Select>
  );
}

export default function SelectPage(): ReactElement {
  return (
    <div className="flex max-w-xs flex-col gap-4">
      <h1 className="text-lg font-semibold">Select</h1>
      <Priority size="sm" />
      <Priority />
      <Select>
        <SelectTrigger aria-label="Agent">
          <SelectValue placeholder="Pick an agent" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectLabel>Agents</SelectLabel>
            <SelectItem value="claude" hint="default">
              Claude Code
            </SelectItem>
            <SelectItem value="codex">Codex</SelectItem>
          </SelectGroup>
          <SelectSeparator />
          <SelectItem value="shell">Shell</SelectItem>
          <SelectItem value="disabled" disabled>
            Unavailable
          </SelectItem>
        </SelectContent>
      </Select>
      <Field label="Worktree convention" error="Choose one.">
        <Select>
          <SelectTrigger>
            <SelectValue placeholder="Choose" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="claude">Claude Code</SelectItem>
            <SelectItem value="managed">Fanout Code managed</SelectItem>
          </SelectContent>
        </Select>
      </Field>
    </div>
  );
}
