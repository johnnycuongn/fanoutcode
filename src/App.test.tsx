// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@/lib/bindings", () => ({
  commands: {
    windowsCurrent: vi.fn(async () => ({
      status: "ok",
      data: { window_id: "w-2", title: "Acme", container: { type: "Project", id: "p-1" }, project_ids: ["p-1"] },
    })),
    settingsFirstRunPending: vi.fn(async () => true),
  },
}));

it("shows this window's title and the first-run note", async () => {
  render(<App />);
  expect(await screen.findByRole("heading", { name: "Acme" })).toBeTruthy();
  expect(await screen.findByText("Onboarding pending.")).toBeTruthy();
});
