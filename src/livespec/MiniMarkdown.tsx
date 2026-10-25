// Renders the small Markdown subset models produce here: headings, bullets, numbered items, task checkboxes
// (`- [ ] 1. Step`), fenced code blocks, `code` and **bold**. React elements only, no innerHTML, so model output can
// never inject markup. Copied from LiveSpec (src/components/MiniMarkdown.tsx) and moved to Fanout Code's tokens.
import type { ReactNode } from "react";
import { cx } from "@/components/ui";

function inline(text: string): ReactNode[] {
  return text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g).map((part, i) => {
    if (part.startsWith("`") && part.endsWith("`") && part.length > 1)
      return (
        <code key={i} className="rounded bg-well px-1 py-px font-mono text-[0.85em]">
          {part.slice(1, -1)}
        </code>
      );
    if (part.startsWith("**") && part.endsWith("**") && part.length > 3)
      return (
        <strong key={i} className="font-semibold">
          {part.slice(2, -2)}
        </strong>
      );
    return part;
  });
}

const FENCE = /^\s*```\s*[\w+-]*\s*$/;
const BULLET = /^\s*(?:[-*•]|\d+[.)])\s+(.*)$/;
const CHECKBOX = /^\[([ xX])\]\s+(.*)$/;
const HEADING = /^#{1,4}\s+(.*)$/;
const PRE = "mb-2 overflow-x-auto rounded bg-well p-2 font-mono text-[12px]";

function item(text: string, key: number): ReactNode {
  const box = CHECKBOX.exec(text);
  if (!box) return <li key={key}>{inline(text)}</li>;
  const checked = box[1] !== " ";
  return (
    <li key={key} data-checked={String(checked)} className="-ml-4 list-none">
      <span aria-hidden className={cx("mr-1.5", checked ? "text-ok" : "text-mute")}>
        {checked ? "☑" : "☐"}
      </span>
      {inline(box[2])}
    </li>
  );
}

/** `className` replaces the default text size (e.g. a drawer that reads larger). */
export function MiniMarkdown({ text, className }: { text: string; className?: string }) {
  const blocks: ReactNode[] = [];
  let items: ReactNode[] = [];
  let fence: string[] | null = null;
  const flush = () => {
    if (items.length)
      blocks.push(
        <ul key={`ul${blocks.length}`} className="mb-1.5 list-disc space-y-1 pl-4 marker:text-mute">
          {items}
        </ul>,
      );
    items = [];
  };

  text.split("\n").forEach((raw, i) => {
    const line = raw.trimEnd();
    if (FENCE.test(line)) {
      if (fence === null) {
        flush();
        fence = [];
      } else {
        blocks.push(
          <pre key={i} className={PRE}>
            {fence.join("\n")}
          </pre>,
        );
        fence = null;
      }
      return;
    }
    if (fence !== null) {
      fence.push(raw);
      return;
    }
    const bullet = BULLET.exec(line);
    const heading = HEADING.exec(line);
    if (bullet) items.push(item(bullet[1], i));
    else {
      flush();
      if (heading)
        blocks.push(
          <h3 key={i} className="mt-4 mb-1 font-semibold first:mt-0">
            {inline(heading[1])}
          </h3>,
        );
      else if (line.trim())
        blocks.push(
          <p key={i} className="mb-1.5">
            {inline(line)}
          </p>,
        );
    }
  });
  // An unclosed fence still shows what it holds.
  const open = fence as string[] | null; // assigned inside the callback, so TypeScript narrows it to null
  if (open !== null)
    blocks.push(
      <pre key="open-fence" className={PRE}>
        {open.join("\n")}
      </pre>,
    );
  flush();
  return <div className={cx("leading-relaxed", className ?? "text-[13.5px]")}>{blocks}</div>;
}
