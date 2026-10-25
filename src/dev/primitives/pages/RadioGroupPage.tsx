import { useState, type ReactElement } from "react";
import { RadioGroup } from "../../../components/ui";

type Choice = "top" | "all" | "replace" | "hold";

export default function RadioGroupPage(): ReactElement {
  const [choice, setChoice] = useState<Choice>("top");
  return (
    <div className="flex max-w-md flex-col gap-6">
      <h1 className="text-lg font-semibold">RadioGroup</h1>
      <RadioGroup<Choice>
        aria-label="What to do with the file"
        value={choice}
        onValueChange={setChoice}
        options={[
          { value: "top", label: "Proceed on top", description: "Keep the other changes and add yours around them." },
          { value: "all", label: "Proceed for every file", description: "No further files are asked about." },
          { value: "replace", label: "Replace their changes", description: "A destructive choice.", tone: "bad" },
          { value: "hold", label: "Hold", disabled: true },
        ]}
      />
    </div>
  );
}
