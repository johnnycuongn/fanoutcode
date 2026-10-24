import { describe, expect, it } from "vitest";
import * as board from "./index";

describe("board public surface", () => {
  it("exports exactly the agreed names", () => {
    expect(Object.keys(board).sort()).toEqual(
      [
        "Board",
        "CardChip",
        "CardStepper",
        "CardTeamRow",
        "ElapsedTime",
        "chipFor",
        "formatCost",
        "formatElapsed",
        "formatTokens",
        "selectNeedsYouCount",
        "useBoard",
      ].sort(),
    );
  });
});
