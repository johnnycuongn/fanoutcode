import type { ReactElement } from "react";
import { Field, Input, Textarea } from "../../../components/ui";

export default function FieldPage(): ReactElement {
  return (
    <div className="flex max-w-xl flex-col gap-6">
      <h1 className="text-lg font-semibold">Field</h1>
      <Field label="Title" hint="Short and specific.">
        <Input placeholder="Add rate limiting" />
      </Field>
      <Field label="Intent" required hint="What should the agent do?">
        <Textarea rows={3} />
      </Field>
      <Field label="Branch" error="That branch already exists.">
        <Input mono defaultValue="fo/rate-limit" />
      </Field>
      <p className="text-xs text-mute">
        layout=&quot;row&quot; stacks until an ancestor is an @container at least 28rem wide.
      </p>
      <section className="@container w-64 rounded-well bg-well p-3" aria-label="Narrow container">
        <div className="flex flex-col gap-3">
          <Field label="Test command" layout="row" hint="Run from the project root.">
            <Input mono defaultValue="npm test" />
          </Field>
          <Field label="Notes" layout="row" error="Too long.">
            <Textarea rows={2} />
          </Field>
        </div>
      </section>
      <section className="@container rounded-well bg-well p-3" aria-label="Wide container">
        <div className="flex flex-col gap-3">
          <Field label="Test command" layout="row" hint="Run from the project root.">
            <Input mono defaultValue="npm test" />
          </Field>
          <Field label="Notes" layout="row" error="Too long.">
            <Textarea rows={2} />
          </Field>
        </div>
      </section>
    </div>
  );
}
