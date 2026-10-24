import { useState, type ReactElement } from "react";
import { Checkbox } from "../../../components/ui";

export default function CheckboxPage(): ReactElement {
  const [on, setOn] = useState<boolean | "indeterminate">("indeterminate");
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold">Checkbox</h1>
      <Checkbox label="Unchecked" />
      <Checkbox label="Checked" defaultChecked />
      <Checkbox label="Indeterminate (click to cycle)" checked={on} onCheckedChange={setOn} />
      <Checkbox label="Disabled" disabled />
      <Checkbox label="Disabled and checked" disabled defaultChecked />
      <Checkbox label="With description" description="Shows a flag on the card and in the attention strip." />
      <Checkbox size="sm" label="Small" />
      <Checkbox aria-label="No visible label" />
    </div>
  );
}
