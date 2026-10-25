import type { ReactElement } from "react";
import { DiffText } from "../../../components/ui";

const BEFORE = ["import { a } from './a';", "", "export function f() {", "  return a + 1;", "}", ""].join("\n");
const AFTER = ["import { a, b } from './a';", "", "export function f() {", "  return a + b;", "}", ""].join("\n");

export default function DiffTextPage(): ReactElement {
  return (
    <div className="flex max-w-xl flex-col gap-6">
      <h1 className="text-lg font-semibold">DiffText</h1>
      <DiffText aria-label="A small change" before={BEFORE} after={AFTER} />
      <DiffText aria-label="A tight context" before={BEFORE} after={AFTER} context={0} />
      <DiffText aria-label="No change" before={BEFORE} after={BEFORE} empty="The two sides are the same" />
    </div>
  );
}
