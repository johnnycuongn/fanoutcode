import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { TouchesEdge, DependsEdge } from "./edges";

const geom = {
  id: "e",
  source: "a",
  target: "b",
  sourceX: 0,
  sourceY: 0,
  targetX: 100,
  targetY: 100,
  sourcePosition: "bottom",
  targetPosition: "top",
} as unknown as Omit<Parameters<typeof TouchesEdge>[0], "data">;
describe("edges", () => {
  it("adds the flowing class only when working", () => {
    const { container, rerender } = render(
      <svg>
        <TouchesEdge {...geom} data={{ working: true }} />
      </svg>,
    );
    expect(container.querySelector(".edge-flowing")).toBeTruthy();
    rerender(
      <svg>
        <TouchesEdge {...geom} data={{ working: false }} />
      </svg>,
    );
    expect(container.querySelector(".edge-flowing")).toBeNull();
  });
  it("labels a depends edge", () => {
    const { container } = render(
      <svg>
        <DependsEdge {...(geom as Parameters<typeof DependsEdge>[0])} />
      </svg>,
    );
    expect(container.querySelector("title")?.textContent).toBe("depends on");
  });
});
