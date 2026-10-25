import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import type { Task } from "@/lib/bindings";
import { setupUser } from "@/test/ui";
import { makeTask } from "../testing/fixtures";
import { OverflowMenu } from "./OverflowMenu";

async function openMenu(task: Task, onIntegrate = vi.fn()) {
  render(
    <OverflowMenu
      task={task}
      mayCommit={false}
      onRename={vi.fn()}
      onAction={vi.fn()}
      onStopAgent={vi.fn()}
      onLinkIssue={vi.fn()}
      onIntegrate={onIntegrate}
    />,
  );
  // Radix opens its menu on pointerdown or a key, not a synthetic click
  screen.getByRole("button", { name: "More actions" }).focus();
  const user = setupUser();
  await user.keyboard("{Enter}");
  await screen.findAllByRole("menuitem");
  return { user, onIntegrate };
}

test("Done worktree task offers Bring main in", async () => {
  const task = makeTask({ stage: "Done", worktree: "On", worktree_path: "/wt/a", base_branch: "main" });
  const { user, onIntegrate } = await openMenu(task);
  await user.click(screen.getByRole("menuitem", { name: "Bring main in" }));
  expect(onIntegrate).toHaveBeenCalledTimes(1);
});

test("an unset base reads Bring base in", async () => {
  await openMenu(makeTask({ stage: "Done", worktree: "On", worktree_path: "/wt/a", base_branch: null }));
  expect(screen.getByRole("menuitem", { name: "Bring base in" })).toBeTruthy();
});

test("no worktree, or a stage before Done, offers no Bring in", async () => {
  await openMenu(makeTask({ stage: "Done", worktree_path: null, base_branch: "main" }));
  expect(screen.queryByRole("menuitem", { name: /^Bring/ })).toBeNull();
});

test("a Build worktree task offers no Bring in", async () => {
  await openMenu(makeTask({ stage: "Build", worktree: "On", worktree_path: "/wt/a", base_branch: "main" }));
  expect(screen.queryByRole("menuitem", { name: /^Bring/ })).toBeNull();
});
