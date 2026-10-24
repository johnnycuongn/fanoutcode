import type { ReactElement } from "react";
import { Field, Switch } from "../../../components/ui";

export default function SwitchPage(): ReactElement {
  return (
    <div className="flex max-w-md flex-col gap-4">
      <h1 className="text-lg font-semibold">Switch</h1>
      <Switch label="Label at the end" />
      <Switch label="Label at the start" labelPosition="start" defaultChecked />
      <Switch size="sm" label="Small" />
      <Switch label="Disabled" disabled />
      <Switch label="With description" description="Applies to every new task in this project." />
      <Field label="Team Code may commit" layout="row" hint="Off by default.">
        <Switch aria-label="Team Code may commit" />
      </Field>
    </div>
  );
}
