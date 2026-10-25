import { render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => ({ events: (await import("@/test/taskPageFixtures")).fakeEvents }));
import { makeTask } from "@/test/taskPageFixtures";
import { createChangesStore } from "../store";
import { fakeIpc } from "../testing/fixtures";
import { ChangesTab } from "./ChangesTab";

test("opens on the first changed file and mounts its note under the line in the editor", async () => {
  const s = createChangesStore(fakeIpc(), { read: () => null, write: () => {} });
  const { container } = render(<ChangesTab task={makeTask()} store={s} homeName="api" />);
  await screen.findByRole("option", { name: /src\/a\.ts/, selected: true });
  await waitFor(() => expect(container.querySelector(".cm-editor")).toBeTruthy());
  await waitFor(() => expect(container.querySelector('[data-note-id="api/src/a.ts:1:0"]')).toBeTruthy());
  expect(screen.getByText("x")).toBeTruthy();
});
