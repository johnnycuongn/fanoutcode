import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { initialAnalysis } from "./analysisState";
import { NotesPane } from "./NotesPane";

test("Off shows the switch hint and no sections", () => {
  render(<NotesPane analysis={initialAnalysis} stale={false} level="Off" onRerun={() => {}} />);
  expect(screen.getByText("LiveSpec is off. Turn on Simple or Heavy to get notes as you write.")).toBeTruthy();
});
test("renders only the sections present in the markdown", () => {
  const analysis = {
    ...initialAnalysis,
    status: "done" as const,
    analyzedDraft: "x",
    markdown: "## Conflicts\n- a\n## Gaps\n- b",
  };
  render(<NotesPane analysis={analysis} stale={false} level="Simple" onRerun={() => {}} />);
  expect(screen.getByText("Conflicts with code")).toBeTruthy();
  expect(screen.queryByText("Intent")).toBeNull();
});
test("a stale draft says so and dims the notes", () => {
  const analysis = { ...initialAnalysis, status: "done" as const, analyzedDraft: "x", markdown: "## Gaps\n- b" };
  render(<NotesPane analysis={analysis} stale level="Simple" onRerun={() => {}} />);
  expect(screen.getByText("Your draft moved on. Pause to refresh.")).toBeTruthy();
});
