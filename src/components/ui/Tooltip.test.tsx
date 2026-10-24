import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { renderUi, setupUser } from "@/test/ui";
import { Button } from "./Button";
import { Tooltip } from "./Tooltip";

test("shows on focus and hides on Escape", async () => {
  const user = setupUser();
  renderUi(
    <Tooltip content="Archive task" delay={0}>
      <Button>A</Button>
    </Tooltip>,
  );
  await user.tab();
  expect(await screen.findByRole("tooltip")).toHaveTextContent("Archive task");
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("tooltip")).not.toBeInTheDocument());
});

test("renders the shortcut as a KeyHint", async () => {
  renderUi(
    <Tooltip content="Archive" shortcut="cmd+shift+a" delay={0}>
      <Button>A</Button>
    </Tooltip>,
  );
  await setupUser().tab();
  expect(await screen.findByRole("tooltip")).toHaveTextContent("⌘⇧A");
});

test("disabled renders only the child", () => {
  renderUi(
    <Tooltip content="x" disabled>
      <Button>A</Button>
    </Tooltip>,
  );
  expect(screen.getByRole("button")).toBeInTheDocument();
  expect(document.querySelector("[data-state]")).toBeNull();
});

test("shows for a disabled child by wrapping it in a focusable span", async () => {
  renderUi(
    <Tooltip content="Session already started" delay={0}>
      <Button disabled>Worktree</Button>
    </Tooltip>,
  );
  await setupUser().tab();
  expect(await screen.findByRole("tooltip")).toHaveTextContent("Session already started");
  expect(screen.getByRole("button").parentElement).toHaveAttribute("tabindex", "0");
});
