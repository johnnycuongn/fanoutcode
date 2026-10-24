import { Search, X } from "lucide-react";
import type { ReactElement } from "react";
import { Input } from "../../../components/ui";

export default function InputPage(): ReactElement {
  return (
    <div className="flex max-w-md flex-col gap-4">
      <h1 className="text-lg font-semibold">Input</h1>
      <Input size="sm" placeholder="Small" />
      <Input placeholder="Medium" />
      <Input leading={<Search />} placeholder="Search tasks" trailing={<X />} />
      <Input invalid defaultValue="Invalid" />
      <Input mono defaultValue="TC-42" />
      <Input disabled placeholder="Disabled" />
    </div>
  );
}
