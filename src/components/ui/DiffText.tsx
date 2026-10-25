import { useMemo, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { lineDiff, type DiffLine } from "@/lib/lineDiff";
import { cx } from "./cx";

export type DiffTextProps = Omit<ComponentProps<"section">, "children"> & {
  before: string;
  after: string;
  /** Unchanged lines kept around each change; longer unchanged runs collapse to a count. */
  context?: number;
  /** Shown when the two sides are equal. */
  empty?: ReactNode;
  "aria-label": string;
};

type Shown = DiffLine | { kind: "gap"; count: number };

const LINE: Record<DiffLine["kind"], { mark: string; cls: string }> = {
  same: { mark: " ", cls: "text-mute" },
  add: { mark: "+", cls: "bg-ok/10 text-ok" },
  del: { mark: "−", cls: "bg-bad/10 text-bad" },
};

function collapse(lines: DiffLine[], context: number): Shown[] {
  const near = lines.map(() => false);
  lines.forEach((l, i) => {
    if (l.kind === "same") return;
    for (let k = Math.max(0, i - context); k <= Math.min(lines.length - 1, i + context); k++) near[k] = true;
  });
  const out: Shown[] = [];
  lines.forEach((l, i) => {
    if (near[i]) out.push(l);
    else {
      const last = out[out.length - 1];
      if (last?.kind === "gap") last.count++;
      else out.push({ kind: "gap", count: 1 });
    }
  });
  return out;
}

/**
 * A read-only line diff of two texts in a `<pre>`, added lines green and removed lines red. Text is rendered as text,
 * so agent output is never parsed as markup.
 */
export function DiffText({
  before,
  after,
  context = 3,
  empty = "No changes",
  className,
  ...props
}: DiffTextProps): ReactElement {
  const shown = useMemo(() => collapse(lineDiff(before, after), context), [before, after, context]);
  const changed = shown.some((l) => l.kind === "add" || l.kind === "del");
  return (
    <section className={cx("min-w-0 overflow-auto rounded-well border border-line bg-well", className)} {...props}>
      {changed ? (
        <pre className="min-w-max py-1 font-mono text-xs leading-5">
          {shown.map((l, i) =>
            l.kind === "gap" ? (
              <div key={i} data-kind="gap" className="px-2 text-mute italic">
                {`${l.count} unchanged ${l.count === 1 ? "line" : "lines"}`}
              </div>
            ) : (
              <div key={i} data-kind={l.kind} className={cx("flex px-2", LINE[l.kind].cls)}>
                <span aria-hidden className="w-4 shrink-0 select-none">
                  {LINE[l.kind].mark}
                </span>
                <span className="whitespace-pre">{l.text}</span>
              </div>
            ),
          )}
        </pre>
      ) : (
        <p className="px-2 py-1.5 text-xs text-mute">{empty}</p>
      )}
    </section>
  );
}
