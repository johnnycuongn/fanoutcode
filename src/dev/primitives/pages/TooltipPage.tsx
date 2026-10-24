import { Archive } from "lucide-react";
import type { ReactElement } from "react";
import { Button, IconButton, Tooltip } from "../../../components/ui";

export default function TooltipPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Tooltip</h1>
      <section className="flex items-center gap-3 p-8">
        {(["top", "right", "bottom", "left"] as const).map((side) => (
          <Tooltip key={side} content={`Opens on the ${side}`} side={side}>
            <Button>{side}</Button>
          </Tooltip>
        ))}
        <Tooltip content="Archive task" shortcut="cmd+shift+a">
          <IconButton label="Archive" icon={<Archive className="size-3.5" />} />
        </Tooltip>
        <Tooltip content="Locked once the session has started">
          <Button disabled>Worktree</Button>
        </Tooltip>
      </section>
    </div>
  );
}
