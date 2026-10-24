import { useState, type ReactElement } from "react";
import { Textarea } from "../../../components/ui";

export default function TextareaPage(): ReactElement {
  const [text, setText] = useState("Grows with its lines.\nTry adding more.");
  return (
    <div className="flex max-w-md flex-col gap-4">
      <h1 className="text-lg font-semibold">Textarea</h1>
      <Textarea autoGrow maxRows={6} value={text} onChange={(e) => setText(e.target.value)} />
      <Textarea mono rows={3} defaultValue="const a = 1;" />
      <Textarea size="sm" invalid rows={2} defaultValue="Invalid" />
    </div>
  );
}
