// The workspace's instructions per stage, appended to that stage's prompt for every task in the workspace. Saved on
// blur; an empty box removes them.
import { useState, type ReactElement } from "react";
import { Field, Textarea } from "@/components/ui";
import { workspaceStore } from "../store";
import { FRAGMENT_STAGES, type FragmentStage } from "../types";
import { Section } from "./Section";

const title = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

function StageBox({ stage, saved }: { stage: FragmentStage; saved: string }): ReactElement {
  const [text, setText] = useState(saved);
  return (
    <Field label={title(stage)}>
      <Textarea
        rows={2}
        value={text}
        placeholder={`Appended to every ${title(stage)} prompt in this workspace`}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => {
          if (text.trim() !== saved) void workspaceStore.setFragment(stage, text.trim() || null);
        }}
      />
    </Field>
  );
}

export function PromptFragments(): ReactElement {
  const fragments = workspaceStore.useSelector((s) => s.fragments);
  return (
    <Section title="Prompt fragments">
      {FRAGMENT_STAGES.map((stage) => (
        <StageBox key={`${stage}:${fragments[stage] ?? ""}`} stage={stage} saved={fragments[stage] ?? ""} />
      ))}
    </Section>
  );
}
