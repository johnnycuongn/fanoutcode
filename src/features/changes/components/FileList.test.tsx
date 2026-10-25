import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { parseReviewNotes } from "../reviewNotes";
import { repos } from "../testing/fixtures";
import { FileList } from "./FileList";

test("groups files by repo name, shows change bars and the include control for Other changes", () => {
  const onInclude = vi.fn();
  const notes = parseReviewNotes("## security reviewer\n- api/src/a.ts:1 · high · x");
  render(<FileList repos={repos()} notes={notes} selected={null} onSelect={() => {}} onInclude={onInclude} />);
  expect(screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent)).toEqual(["api", "web"]);
  expect(screen.getByRole("option", { name: /src\/a\.ts/ }).querySelector("[data-bar]")).toBeTruthy();
  expect(screen.getByRole("option", { name: /src\/a\.ts/ }).textContent).toContain("security");
  fireEvent.click(screen.getByRole("button", { name: "Show other changes" }));
  fireEvent.click(screen.getByRole("button", { name: "Include src/stray.ts" }));
  expect(onInclude).toHaveBeenCalledWith({ repo: "/w/api", path: "src/stray.ts" });
});
test("lists notes on files outside the change set", () => {
  const notes = parseReviewNotes("- web/src/missing.ts:3 · low · gone");
  render(<FileList repos={repos()} notes={notes} selected={null} onSelect={() => {}} onInclude={() => {}} />);
  expect(screen.getByText("Notes on files outside the change set")).toBeTruthy();
  expect(screen.getByText("web/src/missing.ts:3")).toBeTruthy();
});
