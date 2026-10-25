import { expect, test } from "vitest";
import { nextOnGrowShortcut, type GrowState } from "./placement";

const s: GrowState = { open: false, grown: false, body: "terminal" };
test("DockGrows opens the dock on Team and toggles grown", () => {
  expect(nextOnGrowShortcut("DockGrows", s)).toEqual({
    open: true,
    grown: true,
    body: "terminal",
    tab: "team",
    openTeamPane: false,
  });
  expect(nextOnGrowShortcut("DockGrows", { ...s, open: true, grown: true })).toMatchObject({
    open: true,
    grown: false,
    tab: "team",
  });
});
test("Swap flips the body and leaves the dock alone", () => {
  expect(nextOnGrowShortcut("Swap", s)).toEqual({
    open: false,
    grown: false,
    body: "team",
    tab: null,
    openTeamPane: false,
  });
});
test("OwnPane asks the shell for a team pane and changes nothing else", () => {
  expect(nextOnGrowShortcut("OwnPane", s)).toEqual({ ...s, tab: null, openTeamPane: true });
});
