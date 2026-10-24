// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { createStore } from "./createStore";

it("returns the initial state and shallow-merges patches", () => {
  const s = createStore({ n: 1, name: "a" });
  s.set({ n: 2 });
  expect(s.get()).toEqual({ n: 2, name: "a" });
  s.set((st) => ({ ...st, name: "b" }));
  expect(s.get().name).toBe("b");
});

it("notifies subscribers once per change and not for an identical state", () => {
  const s = createStore({ n: 1 });
  const seen = vi.fn();
  const off = s.subscribe(seen);
  s.set({ n: 2 });
  s.set((st) => st);
  expect(seen).toHaveBeenCalledTimes(1);
  off();
  s.set({ n: 3 });
  expect(seen).toHaveBeenCalledTimes(1);
});

it("use() re-renders a component on change", () => {
  const s = createStore({ n: 1 });
  const { result } = renderHook(() => s.use());
  act(() => s.set({ n: 5 }));
  expect(result.current.n).toBe(5);
});

it("useSelector re-renders only when the selected value changes", () => {
  const s = createStore({ n: 1, other: 0 });
  let renders = 0;
  const { result } = renderHook(() => {
    renders++;
    return s.useSelector((st) => st.n);
  });
  act(() => s.set({ other: 9 }));
  expect(renders).toBe(1);
  act(() => s.set({ n: 2 }));
  expect(result.current).toBe(2);
  expect(renders).toBe(2);
});

it("reset returns to the initial state", () => {
  const s = createStore({ n: 1 });
  s.set({ n: 7 });
  s.reset();
  expect(s.get()).toEqual({ n: 1 });
});
