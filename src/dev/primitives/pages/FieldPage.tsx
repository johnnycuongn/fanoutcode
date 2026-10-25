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
      <section className="flex flex-col gap-3 rounded-well bg-well p-3">
        <Field label="Test command" layout="row" hint="Run from the project root.">
          <Input mono defaultValue="npm test" />
        </Field>
        <Field label="Notes" layout="row" error="Too long.">
          <Textarea rows={2} />
        </Field>
      </section>
    </div>
  );
}
