// Vitest setup for every webview test: jest-dom matchers, cleanup, and the jsdom gaps Radix, cmdk and Vaul touch.
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";
import { mockMatchMedia } from "./ui";

if (typeof window !== "undefined") {
  class ResizeObserverStub {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  window.ResizeObserver ??= ResizeObserverStub;
  Element.prototype.scrollIntoView ??= () => {};
  Element.prototype.hasPointerCapture ??= () => false;
  Element.prototype.setPointerCapture ??= () => {};
  Element.prototype.releasePointerCapture ??= () => {};
  if (!window.DOMRect.fromRect) {
    window.DOMRect.fromRect = (r = {}) => new DOMRect(r.x ?? 0, r.y ?? 0, r.width ?? 0, r.height ?? 0);
  }
  beforeEach(() => {
    mockMatchMedia(false); // every test starts with full motion and a light colour scheme
  });
  afterEach(cleanup);
}
