import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import type { Stage } from "@/lib/bindings";
import { setupUser } from "@/test/ui";
import type { PrimaryButton } from "../primaryAction";
import { PrimaryAction } from "./PrimaryAction";

const button = (label: string): PrimaryButton => ({
  label,
  intent: { kind: "focus-terminal" },
  disabled: false,
  hint: null,
});

async function openSplit(label: string, stage: Stage, onAction = vi.fn()) {
  render(
    <PrimaryAction primary={button(label)} stage={stage} mayCommit={false} onPrimary={vi.fn()} onAction={onAction} />,
  );
  // Radix opens its menu on pointerdown or a key, not a synthetic click
  screen.getByRole("button", { name: "Other ways to continue" }).focus();
  const user = setupUser();
  await user.keyboard("{Enter}");
  await screen.findAllByRole("menuitem");
  return { user, onAction };
}

test("while the button reads Reply, the split menu still offers the stage's forward step", async () => {
  const { user, onAction } = await openSplit("Reply", "Build");
  await user.click(screen.getByRole("menuitem", { name: "Request review" }));
  expect(onAction).toHaveBeenCalledWith({ type: "RequestReview" });
});

test("the forward step is not repeated when it is already the button", async () => {
  await openSplit("Request review", "Build");
  expect(screen.queryByRole("menuitem", { name: "Request review" })).toBeNull();
  expect(screen.getByRole("menuitem", { name: "Start fresh session" })).toBeTruthy();
});

test("in Spec a Reply button gets a split menu holding Approve spec", async () => {
  await openSplit("Reply", "Spec");
  expect(screen.getByRole("menuitem", { name: "Approve spec" })).toBeTruthy();
  expect(screen.queryByRole("menuitem", { name: "Start fresh session" })).toBeNull();
});
