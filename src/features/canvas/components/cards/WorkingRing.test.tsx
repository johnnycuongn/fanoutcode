import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { mockMatchMedia } from "@/test/ui";
import { WorkingRing } from "./WorkingRing";

describe("WorkingRing", () => {
  it("renders the static ring under reduced motion", () => {
    mockMatchMedia(true);
    const { container } = render(<WorkingRing />);
    expect(container.querySelector('[data-ring="static"]')).toBeTruthy();
    expect(container.querySelector('[data-ring="loop"]')).toBeNull();
  });
  it("loops with full motion", () => {
    const { container } = render(<WorkingRing />);
    expect(container.querySelector('[data-ring="loop"]')).toBeTruthy();
  });
});
