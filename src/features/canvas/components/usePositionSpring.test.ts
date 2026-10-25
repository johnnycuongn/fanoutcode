import { describe, it, expect } from "vitest";
import { motionFor } from "./usePositionSpring";

describe("motionFor", () => {
  it("is a 300 ms spring normally and a 150 ms ease under reduced motion", () => {
    expect(motionFor(false)).toEqual({ type: "spring", duration: 0.3, bounce: 0.2 });
    expect(motionFor(true)).toEqual({ duration: 0.15, ease: "easeOut" });
  });
});
