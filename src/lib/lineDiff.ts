// A line diff for read-only display (the Overlap tab's two boxes): the common prefix and suffix are kept, the middle
// is an LCS when it is small enough, else removed then added whole, so a huge file never stalls the webview.

export type DiffLineKind = "same" | "add" | "del";
export type DiffLine = { kind: DiffLineKind; text: string };

/** Above this many LCS cells the middle is shown as removed then added. */
const MAX_CELLS = 4_000_000;

function splitLines(text: string): string[] {
  if (text === "") return [];
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

function lcs(a: string[], b: string[]): DiffLine[] {
  const n = a.length;
  const m = b.length;
  // table[i][j]: the LCS length of a[i..] and b[j..], flattened.
  const table = new Uint32Array((n + 1) * (m + 1));
  const at = (i: number, j: number) => i * (m + 1) + j;
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--)
      table[at(i, j)] =
        a[i] === b[j] ? table[at(i + 1, j + 1)] + 1 : Math.max(table[at(i + 1, j)], table[at(i, j + 1)]);
  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      out.push({ kind: "same", text: a[i] });
      i++;
      j++;
    } else if (table[at(i + 1, j)] >= table[at(i, j + 1)]) out.push({ kind: "del", text: a[i++] });
    else out.push({ kind: "add", text: b[j++] });
  }
  while (i < n) out.push({ kind: "del", text: a[i++] });
  while (j < m) out.push({ kind: "add", text: b[j++] });
  return out;
}

export function lineDiff(before: string, after: string): DiffLine[] {
  const a = splitLines(before);
  const b = splitLines(after);
  let head = 0;
  while (head < a.length && head < b.length && a[head] === b[head]) head++;
  let tail = 0;
  while (tail < a.length - head && tail < b.length - head && a[a.length - 1 - tail] === b[b.length - 1 - tail]) tail++;
  const midA = a.slice(head, a.length - tail);
  const midB = b.slice(head, b.length - tail);
  const middle =
    midA.length * midB.length > MAX_CELLS
      ? [
          ...midA.map((text): DiffLine => ({ kind: "del", text })),
          ...midB.map((text): DiffLine => ({ kind: "add", text })),
        ]
      : lcs(midA, midB);
  const same = (text: string): DiffLine => ({ kind: "same", text });
  return [...a.slice(0, head).map(same), ...middle, ...a.slice(a.length - tail).map(same)];
}

export function diffStats(lines: DiffLine[]): { added: number; removed: number } {
  let added = 0;
  let removed = 0;
  for (const l of lines) {
    if (l.kind === "add") added++;
    else if (l.kind === "del") removed++;
  }
  return { added, removed };
}
