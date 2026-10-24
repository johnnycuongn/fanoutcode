import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { SPRING, prefersReducedMotion, transitionFor, useReducedMotion } from "./motion";
import { mockMatchMedia } from "@/test/ui";

test("prefersReducedMotion is false when matchMedia is absent", () => {
  const saved = window.matchMedia;
  Object.defineProperty(window, "matchMedia", { value: undefined, configurable: true });
  expect(prefersReducedMotion()).toBe(false);
  Object.defineProperty(window, "matchMedia", { value: saved, configurable: true });
});

test("useReducedMotion reads the media query", () => {
  mockMatchMedia(true);
  expect(renderHook(() => useReducedMotion()).result.current).toBe(true);
});

test("useReducedMotion flips when the media query changes", () => {
  const media = mockMatchMedia(false);
  const { result } = renderHook(() => useReducedMotion());
  expect(result.current).toBe(false);
  act(() => media.set(true));
  expect(result.current).toBe(true);
});

test("transitionFor returns the 150 ms ease under reduced motion and the spring otherwise", () => {
  expect(transitionFor(true)).toEqual({ duration: 0.15, ease: "easeOut" });
  expect(transitionFor(false)).toBe(SPRING);
});
