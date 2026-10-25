// Workspace-level subagents (markdown with front matter), added to every task's roster in this workspace; an empty
// definition removes the role. A box remounts (by key) when the saved text changes.
import { useState, type ReactElement } from "react";
import { Button, Field, Input, Textarea } from "@/components/ui";
import { workspaceStore } from "../store";
import { Section } from "./Section";

const TEMPLATE = (name: string) => `---\nname: ${name}\ndescription: \n---\n`;

function AgentBox({ name, saved }: { name: string; saved: string }): ReactElement {
  const [text, setText] = useState(saved);
  return (
    <Field label={name}>
      <Textarea
        rows={4}
        mono
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => {
          if (text.trim() !== saved) void workspaceStore.setAgent(name, text.trim() || null);
        }}
      />
    </Field>
  );
}

export function WorkspaceAgents(): ReactElement {
  const agents = workspaceStore.useSelector((s) => s.agents);
  const [name, setName] = useState("");
  const add = () => {
    const n = name.trim().toLowerCase();
    if (!n) return;
    void workspaceStore.setAgent(n, TEMPLATE(n));
    setName("");
  };
  return (
    <Section title="Agents">
      {Object.entries(agents).map(([n, md]) => (
        <AgentBox key={`${n}:${md}`} name={n} saved={md} />
      ))}
      <div className="flex gap-1">
        <Input
          size="sm"
          aria-label="New role name"
          placeholder="role-name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <Button size="sm" variant="secondary" onClick={add}>
          Add role
        </Button>
      </div>
    </Section>
  );
}
