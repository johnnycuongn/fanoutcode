import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { makeSettings, makeTask } from "@/test/taskPageFixtures";
import { createDockStore } from "../store";
import { mem } from "../testing/fakes";
import { Dock } from "./Dock";

test("renders only the visible tabs for the stage and the team slot on Team", () => {
  const d = createDockStore(mem());
  d.selectTab("Build", "team");
  render(
    <Dock
      task={makeTask({ stage: "Build" })}
      settings={makeSettings()}
      open
      teamSlot={<div data-testid="team" />}
      store={d}
    />,
  );
  expect(screen.getAllByRole("tab").map((t) => t.textContent)).toEqual(["Memory", "Team", "Changes", "Ticket"]);
  expect(screen.getByTestId("team")).toBeTruthy();
});
test("renders nothing when closed and marks the right position", () => {
  const d = createDockStore(mem());
  const { container, rerender } = render(
    <Dock task={makeTask()} settings={makeSettings()} open={false} teamSlot={null} store={d} />,
  );
  expect(container.firstChild).toBeNull();
  rerender(
    <Dock task={makeTask()} settings={makeSettings({ dockPosition: "Right" })} open teamSlot={null} store={d} />,
  );
  expect(container.querySelector('[data-position="right"]')).toBeTruthy();
});
