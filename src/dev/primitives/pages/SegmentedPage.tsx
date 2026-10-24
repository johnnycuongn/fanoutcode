import { LayoutGrid, LayoutList, Rows3 } from "lucide-react";
import { useState, type ReactElement } from "react";
import { Segmented, Tooltip } from "../../../components/ui";

export default function SegmentedPage(): ReactElement {
  const [liveSpec, setLiveSpec] = useState<"off" | "simple" | "heavy">("simple");
  const [detail, setDetail] = useState<"simple" | "medium" | "detailed">("medium");
  const [view, setView] = useState<"board" | "list" | "rows">("board");
  const [shortcut, setShortcut] = useState<"a" | "b">("a");
  return (
    <div className="flex max-w-md flex-col gap-6">
      <h1 className="text-lg font-semibold">Segmented</h1>
      <Segmented<"off" | "simple" | "heavy">
        aria-label="LiveSpec"
        value={liveSpec}
        onValueChange={setLiveSpec}
        options={[
          { value: "off", label: "Off" },
          { value: "simple", label: "Simple" },
          { value: "heavy", label: "Heavy" },
        ]}
      />
      <Segmented<"simple" | "medium" | "detailed">
        aria-label="Detail level"
        size="sm"
        fullWidth
        value={detail}
        onValueChange={setDetail}
        options={[
          { value: "simple", label: "Simple" },
          { value: "medium", label: "Medium" },
          { value: "detailed", label: "Detailed" },
        ]}
      />
      <Segmented<"board" | "list" | "rows">
        aria-label="Layout"
        value={view}
        onValueChange={setView}
        options={[
          { value: "board", label: <span className="sr-only">Board</span>, icon: <LayoutGrid className="size-3.5" /> },
          { value: "list", label: <span className="sr-only">List</span>, icon: <LayoutList className="size-3.5" /> },
          { value: "rows", label: <span className="sr-only">Rows</span>, icon: <Rows3 className="size-3.5" /> },
        ]}
      />
      <Tooltip content="Icon-only options need a tooltip">
        <span className="text-sm text-mute">Icon-only options expose their name to screen readers.</span>
      </Tooltip>
      <Segmented<"a" | "b">
        aria-label="With shortcuts and a disabled option"
        value={shortcut}
        onValueChange={setShortcut}
        options={[
          { value: "a", label: "First", shortcut: "cmd+1" },
          { value: "b", label: "Second", shortcut: "cmd+2", disabled: true },
        ]}
      />
    </div>
  );
}
