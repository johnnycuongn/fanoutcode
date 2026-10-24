import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { formatElapsed, tickStore, useElapsed } from "./elapsed";

describe("formatElapsed", () => {
  it.each([
    [0, "0m"],
    [59_000, "0m"],
    [60_000, "1m"],
    [59 * 60_000, "59m"],
    [60 * 60_000, "1h 00m"],
    [65 * 60_000, "1h 05m"],
    [24 * 3_600_000, "1d 0h"],
    [26.5 * 3_600_000, "1d 2h"],
  ])("%i ms → %s", (ms, text) => {
    expect(formatElapsed(ms)).toBe(text);
  });
  it('clamps negative and NaN input to "0m"', () => {
    // Review Focus 4
    expect(formatElapsed(-5_000)).toBe("0m");
    expect(formatElapsed(Number.NaN)).toBe("0m");
  });
});

describe("tickStore", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());
  it("starts the interval on the first subscriber and stops on the last unsubscribe", () => {
    const set = vi.spyOn(globalThis, "setInterval");
    const clear = vi.spyOn(globalThis, "clearInterval");
    const a = tickStore.subscribe(() => {});
    const b = tickStore.subscribe(() => {});
    expect(set).toHaveBeenCalledTimes(1);
    a();
    expect(clear).not.toHaveBeenCalled();
    b();
    expect(clear).toHaveBeenCalledTimes(1);
  });
});

describe("useElapsed", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-10-24T10:00:00Z"));
  });
  afterEach(() => vi.useRealTimers());
  it("re-renders only when the formatted value changes", () => {
    const renders = { n: 0 };
    function Probe() {
      renders.n++;
      return <span>{useElapsed("2026-10-24T10:00:00Z")}</span>;
    }
    const { getAllByText } = render(
      <>
        <Probe />
        <Probe />
        <Probe />
      </>,
    );
    expect(renders.n).toBe(3);
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(renders.n).toBe(3);
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(renders.n).toBe(6);
    expect(getAllByText("1m")).toHaveLength(3);
  });
  it("a future or unparsable timestamp reads 0m", () => {
    function Future() {
      return <span>{useElapsed("2026-10-24T11:00:00Z")}</span>;
    }
    function Broken() {
      return <span>{useElapsed("not a date")}</span>;
    }
    expect(render(<Future />).container.textContent).toBe("0m");
    expect(render(<Broken />).container.textContent).toBe("0m");
  });
});
