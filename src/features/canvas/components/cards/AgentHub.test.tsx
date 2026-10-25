import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { AgentHub } from "./AgentHub";
import { ROOTS } from "../../fixtures";

const agent = {
  agentId: "a3",
  role: "ui-polish",
  adhoc: true,
  working: true,
  steps: [2, 4],
  description: null,
  startedSeq: 17,
  stoppedSeq: null,
  files: [],
  costUsd: 0.4,
};
const view = { detail: "Simple" as const, showCost: false, showRepo: false, roots: ROOTS, reduced: false };

describe("AgentHub", () => {
  it("shows the role, the general-purpose tag and the ring while working", () => {
    render(<AgentHub data={agent} view={view} />);
    expect(screen.getByText("ui-polish")).toBeTruthy();
    expect(screen.getByText("general-purpose")).toBeTruthy();
    expect(document.querySelector("[data-ring]")).toBeTruthy();
  });
  it("drops the ring when stopped and shows cost only with the overview on", () => {
    const { rerender } = render(<AgentHub data={{ ...agent, working: false }} view={view} />);
    expect(document.querySelector("[data-ring]")).toBeNull();
    expect(screen.queryByText("$0.40")).toBeNull();
    rerender(<AgentHub data={{ ...agent, working: false }} view={{ ...view, showCost: true }} />);
    expect(screen.getByText("$0.40")).toBeTruthy();
  });
});
