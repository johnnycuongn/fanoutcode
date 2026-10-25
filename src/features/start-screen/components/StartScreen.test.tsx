import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { UiProvider } from "@/components/ui";
import { mockMatchMedia } from "@/test/ui";
import { workingStore } from "../working";
import { StartScreen } from "./StartScreen";

beforeEach(() => {
  workingStore.reset();
  commands.windowsList.mockResolvedValue([]);
  commands.windowsRecent.mockResolvedValue({ status: "ok", data: [] });
});

const mark = () => screen.getByTestId("app-mark");

function renderStart(): void {
  render(
    <UiProvider>
      <StartScreen />
    </UiProvider>,
  );
}

it("shows the brand mark and the name, with no tagline", async () => {
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 0 });
  renderStart();
  expect(screen.getByRole("heading", { level: 1, name: "Fanout Code" })).toBeInTheDocument();
  expect(mark()).toHaveAttribute("width", "48");
  await vi.waitFor(() => expect(commands.tasksWorkingCount).toHaveBeenCalled());
  expect(mark()).toHaveAttribute("data-spinning", "false");
});

it("spins the mark while any agent is working", async () => {
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 2 });
  renderStart();
  await vi.waitFor(() => expect(mark()).toHaveAttribute("data-spinning", "true"));
});

it("keeps the mark still under reduced motion", async () => {
  mockMatchMedia(true);
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 2 });
  renderStart();
  await vi.waitFor(() => expect(workingStore.get().count).toBe(2));
  expect(mark()).toHaveAttribute("data-spinning", "false");
});
