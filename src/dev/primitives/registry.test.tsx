import { cleanup, render, screen } from "@testing-library/react";
import { Suspense } from "react";
import { expect, test } from "vitest";
import * as ui from "../../components/ui";
import { PRIMITIVES } from "../../components/ui";
import { pages } from "./registry";

test("every name in PRIMITIVES has exactly one example page", () => {
  expect(pages.map((p) => p.name).sort()).toEqual(Object.keys(PRIMITIVES).sort());
});

test("every member PRIMITIVES lists is exported from the barrel", () => {
  for (const [name, members] of Object.entries<readonly string[]>(PRIMITIVES)) {
    for (const m of members) expect(ui, `${name} → ${m}`).toHaveProperty(m);
  }
});

test("each example page renders an h1 named after its primitive", async () => {
  for (const { name, Page } of pages) {
    render(
      <Suspense fallback={null}>
        <Page />
      </Suspense>,
    );
    expect(await screen.findByRole("heading", { level: 1, name })).toBeInTheDocument();
    cleanup();
  }
});
