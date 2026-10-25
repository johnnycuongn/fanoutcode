// @vitest-environment jsdom
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => (await import("@/test/mockBindings")).mockBindings());
import { commands } from "@/test/mockBindings";
import { makeTask } from "@/test/fixtures";
import { setupUser } from "@/test/ui";
import type { ConflictCtx, Task } from "@/lib/bindings";
import { IntegratePane, mergeNeedsYou } from "./IntegratePane";

const merging = (paths = ["src/auth.ts", "src/db.ts"]): Task =>
  makeTask({ stage: "Done", state: { type: "NeedsYou", reason: { type: "MergeNeedsYou", repo: "/r/alpha", paths } } });

const CTX: ConflictCtx[] = [
  {
    path: "src/auth.ts",
    other_title: "Login",
    spec_excerpt: "## Summary\nLet people **sign in**.",
    plan_excerpt: "- src/auth.ts: add the session check",
  },
];

beforeEach(() => {
  commands.overlapIntegrationContext.mockReset().mockResolvedValue({ status: "ok", data: CTX });
  commands.overlapResume.mockReset().mockResolvedValue({ status: "ok", data: null });
  commands.overlapAbort.mockReset().mockResolvedValue({ status: "ok", data: null });
});

test("lists each conflicted path with the other task's spec and plan excerpts", async () => {
  render(<IntegratePane task={merging()} />);
  expect(await screen.findByText("Login")).toBeTruthy();
  expect(commands.overlapIntegrationContext).toHaveBeenCalledWith("task-b2");
  expect(screen.getByText("src/auth.ts")).toBeTruthy();
  expect(screen.getByText("src/db.ts")).toBeTruthy();
  expect(screen.getByText("sign in").tagName).toBe("STRONG"); // through MiniMarkdown, not innerHTML
  expect(screen.getByText("src/auth.ts: add the session check", { exact: false })).toBeTruthy();
});

test("Resume sends the instruction", async () => {
  render(<IntegratePane task={merging()} />);
  const user = setupUser();
  const resume = screen.getByRole("button", { name: "Resume" });
  expect(resume).toHaveProperty("disabled", true);
  await user.type(screen.getByRole("textbox", { name: "Instructions for the session" }), "Keep both checks");
  await user.click(resume);
  expect(commands.overlapResume).toHaveBeenCalledWith("task-b2", "Keep both checks");
  await waitFor(() =>
    expect(screen.getByRole("textbox", { name: "Instructions for the session" })).toHaveProperty("value", ""),
  );
});

test("Abort calls overlapAbort", async () => {
  render(<IntegratePane task={merging()} />);
  await setupUser().click(screen.getByRole("button", { name: "Abort" }));
  expect(commands.overlapAbort).toHaveBeenCalledWith("task-b2");
});

test("renders nothing unless the merge needs you", () => {
  const { container } = render(<IntegratePane task={makeTask({ stage: "Done" })} />);
  expect(container.firstChild).toBeNull();
  expect(commands.overlapIntegrationContext).not.toHaveBeenCalled();
  expect(mergeNeedsYou(makeTask())).toBeNull();
  expect(mergeNeedsYou(merging(["a.ts"]))).toEqual({ repo: "/r/alpha", paths: ["a.ts"] });
});
