// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/features/shell", () => ({ ShellLayout: () => <div data-testid="shell" /> }));
vi.mock("@/features/start-screen", () => ({
  StartScreen: () => <div data-testid="start" />,
  Onboarding: () => <div data-testid="onboarding" />,
}));
import { mockBindings, commands } from "@/test/mockBindings";
import App from "./App";

// App lazy-loads each screen's chunk; under machine load that takes longer than findBy's 1 s default.
const LAZY_CHUNK = { timeout: 5000 };

const windowInfo = (container: unknown) => ({
  status: "ok",
  data: { window_id: "w-2", title: "Acme", container, project_ids: container ? ["p-1"] : [] },
});

it("shows the shell in a project window", async () => {
  commands.settingsFirstRunPending.mockResolvedValue(false);
  commands.windowsCurrent.mockResolvedValue(windowInfo({ type: "Project", id: "p-1" }));
  render(<App />);
  expect(await screen.findByTestId("shell", {}, LAZY_CHUNK)).toBeInTheDocument();
});

it("shows onboarding on first run and the start screen after it", async () => {
  commands.windowsCurrent.mockResolvedValue(windowInfo(null));
  commands.settingsFirstRunPending.mockResolvedValue(true);
  render(<App />);
  expect(await screen.findByTestId("onboarding", {}, LAZY_CHUNK)).toBeInTheDocument();
});

it("shows the start screen once first run is complete", async () => {
  commands.windowsCurrent.mockResolvedValue(windowInfo(null));
  commands.settingsFirstRunPending.mockResolvedValue(false);
  render(<App />);
  expect(await screen.findByTestId("start", {}, LAZY_CHUNK)).toBeInTheDocument();
});
