import { describe, it, expect } from "vitest";
import { line, summary } from "./costTestSupport";
import { formatTokens, formatUsd, rowsFor } from "./costFormat";

describe("costFormat", () => {
  it("formatUsd prefixes a tilde for estimates and floors tiny values", () => {
    expect(formatUsd(1.234, false)).toBe("$1.23");
    expect(formatUsd(1.234, true)).toBe("~$1.23");
    expect(formatUsd(0.004, false)).toBe("<$0.01");
    expect(formatUsd(0, false)).toBe("$0.00");
  });
  it("formatTokens abbreviates", () => {
    expect(formatTokens(950)).toBe("950");
    expect(formatTokens(12_340)).toBe("12.3k");
    expect(formatTokens(1_200_000)).toBe("1.2M");
  });
  it("rowsFor sorts by cost and days by key", () => {
    const s = summary({
      by_task: [line("t-1", 0.5), line("t-2", 2)],
      by_day: [line("2026-10-23", 1), line("2026-10-24", 0.1)],
    });
    expect(rowsFor(s, "task").map((r) => r.key)).toEqual(["t-2", "t-1"]);
    expect(rowsFor(s, "day").map((r) => r.key)).toEqual(["2026-10-24", "2026-10-23"]);
  });
});
