// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/features/shell", () => ({ ShellLayout: () => <div data-testid="shell" /> }));
import { mockBindings, commands } from "@/test/mockBindings";
import App from "./App";

const windowInfo = (container: unknown) => ({
  status: "ok",
  data: { window_id: "w-2", title: "Acme", container, project_ids: container ? ["p-1"] : [] },
});

beforeEach(() => commands.settingsFirstRunPending.mockResolvedValue(true));

it("shows the shell in a project window", async () => {
  commands.windowsCurrent.mockResolvedValue(windowInfo({ type: "Project", id: "p-1" }));
  render(<App />);
  expect(await screen.findByTestId("shell")).toBeInTheDocument();
});

it("shows this window's title and the first-run note on the start screen", async () => {
  commands.windowsCurrent.mockResolvedValue(windowInfo(null));
  render(<App />);
  expect(await screen.findByRole("heading", { name: "Acme" })).toBeTruthy();
  expect(await screen.findByText("Onboarding pending.")).toBeTruthy();
});
