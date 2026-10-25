// @vitest-environment jsdom
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => (await import("@/test/mockBindings")).mockBindings());
import { commands } from "@/test/mockBindings";
import { makeTask } from "@/test/fixtures";
import { setupUser } from "@/test/ui";
import { BaseBranchDialog } from "./BaseBranchDialog";

const task = makeTask({ stage: "Done", worktree: "On", worktree_path: "/wt/a", base_branch: null });

beforeEach(() => {
  commands.overlapBaseSuggestion.mockReset().mockResolvedValue({ status: "ok", data: "main" });
  commands.overlapSetBase.mockReset().mockResolvedValue({ status: "ok", data: { ...task, base_branch: "main" } });
});

test("confirms the suggestion and calls overlapSetBase", async () => {
  const onConfirmed = vi.fn();
  render(<BaseBranchDialog task={task} open onClose={vi.fn()} onConfirmed={onConfirmed} />);
  const input = screen.getByLabelText("Base branch");
  await waitFor(() => expect(input).toHaveProperty("value", "main"));
  expect(commands.overlapBaseSuggestion).toHaveBeenCalledWith("task-b2");
  await setupUser().click(screen.getByRole("button", { name: "Bring main in" }));
  expect(commands.overlapSetBase).toHaveBeenCalledWith("task-b2", "main");
  await waitFor(() => expect(onConfirmed).toHaveBeenCalledWith(expect.objectContaining({ base_branch: "main" })));
});

test("an edited branch is sent, and an Invalid answer shows inline", async () => {
  commands.overlapSetBase.mockResolvedValue({ status: "error", error: "Not a safe branch name." });
  const onConfirmed = vi.fn();
  render(<BaseBranchDialog task={task} open onClose={vi.fn()} onConfirmed={onConfirmed} />);
  const input = screen.getByLabelText("Base branch");
  await waitFor(() => expect(input).toHaveProperty("value", "main"));
  const user = setupUser();
  await user.clear(input);
  await user.type(input, "bad..name{Enter}");
  expect(commands.overlapSetBase).toHaveBeenCalledWith("task-b2", "bad..name");
  expect(await screen.findByText("Not a safe branch name.")).toBeTruthy();
  expect(onConfirmed).not.toHaveBeenCalled();
});

test("no suggestion leaves the field empty and the confirm disabled", async () => {
  commands.overlapBaseSuggestion.mockResolvedValue({ status: "ok", data: null });
  render(<BaseBranchDialog task={task} open onClose={vi.fn()} onConfirmed={vi.fn()} />);
  await waitFor(() => expect(commands.overlapBaseSuggestion).toHaveBeenCalled());
  expect(screen.getByLabelText("Base branch")).toHaveProperty("value", "");
  expect(screen.getByRole("button", { name: "Bring base in" })).toHaveProperty("disabled", true);
});
