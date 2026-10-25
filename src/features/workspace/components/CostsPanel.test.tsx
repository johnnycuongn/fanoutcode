// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings } from "@/test/mockBindings";
import type { CostSummary } from "@/lib/bindings";
import { costsStore } from "../costsStore";
import { line, summary } from "../costTestSupport";
import { CostsPanel } from "./CostsPanel";

function mockCosts(s: CostSummary, enabled = true) {
  costsStore.set({ scope: { workspace: "ws-1", project: null, task: null }, summary: s, group: "project", enabled });
}

describe("CostsPanel", () => {
  beforeEach(() => vi.spyOn(costsStore, "load").mockResolvedValue());

  it("shows the total from cost_usd, a Live chip and rows by key or name", () => {
    mockCosts(
      summary({
        cost_usd: 1.5,
        estimated: true,
        live: { cost_usd: 0.2, context_used_pct: 10, at: "x" } as CostSummary["live"],
        input_tokens: 12_340,
        output_tokens: 950,
        by_project: [line("proj-1", 1.5)],
      }),
    );
    render(<CostsPanel scope={{ workspace: "ws-1" }} names={{ "proj-1": "api" }} />);
    expect(screen.getAllByText("~$1.50").length).toBeGreaterThan(0);
    expect(screen.getByText("Live")).toBeInTheDocument();
    expect(screen.getByText("api")).toBeInTheDocument();
    expect(screen.queryByText("proj-1")).toBeNull();
  });

  it("explains how to turn costs on and when nothing is recorded", () => {
    mockCosts(summary({}), false);
    const { unmount } = render(<CostsPanel scope={{ workspace: "ws-1" }} />);
    expect(screen.getByText("Turn on Cost overview in app settings to collect costs.")).toBeInTheDocument();
    unmount();
    mockCosts(summary({}), true);
    render(<CostsPanel scope={{ workspace: "ws-1" }} />);
    expect(screen.getByText("No cost recorded yet.")).toBeInTheDocument();
  });
});
