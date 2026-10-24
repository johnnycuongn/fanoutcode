import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
vi.mock("@/lib/motion", async (orig) => ({
  ...(await orig<typeof import("@/lib/motion")>()),
  useReducedMotion: vi.fn(() => false),
}));
import { useReducedMotion } from "@/lib/motion";
import { CardChip } from "./CardChip";
import { CardStepper } from "./CardStepper";
import { CardTeamRow } from "./CardTeamRow";

describe("CardChip", () => {
  it("renders the chip text and detail", () => {
    render(<CardChip model={{ tone: "needs_you", text: "Tests failed", detail: "exit 1" }} />);
    expect(screen.getByText("Tests failed", { exact: false }).closest("[data-tone]")).toHaveTextContent(
      "Tests failed · exit 1",
    );
  });
  it("maps the model tone onto the chip root", () => {
    render(<CardChip model={{ tone: "needs_you", text: "Spec ready" }} />);
    expect(screen.getByText("Spec ready").closest("[data-tone]")).toHaveAttribute("data-tone", "needs_you");
  });
  it("pulses only when working and not under reduced motion", () => {
    const { rerender } = render(<CardChip model={{ tone: "working", text: "Working" }} />);
    expect(screen.getByTestId("status-dot")).toHaveAttribute("data-pulse", "true");
    rerender(<CardChip model={{ tone: "idle", text: "Idle" }} />);
    expect(screen.getByTestId("status-dot")).toHaveAttribute("data-pulse", "false");
    vi.mocked(useReducedMotion).mockReturnValue(true);
    rerender(<CardChip model={{ tone: "working", text: "Working" }} />);
    expect(screen.getByTestId("status-dot")).toHaveAttribute("data-pulse", "false");
    vi.mocked(useReducedMotion).mockReturnValue(false);
  });
  it("lets an indicator replace the dot", () => {
    render(<CardChip model={{ tone: "working", text: "Working" }} indicator={<i data-testid="ring" />} />);
    expect(screen.getByTestId("ring")).toBeInTheDocument();
    expect(screen.queryByTestId("status-dot")).toBeNull();
  });
});

describe("CardStepper", () => {
  it("renders six steps with done, current and upcoming states", () => {
    render(<CardStepper stage="Build" />);
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(6);
    expect(items.map((i) => i.dataset.state)).toEqual(["done", "done", "current", "upcoming", "upcoming", "upcoming"]);
    expect(screen.getByRole("list")).toHaveAttribute("aria-label", "Stage Build, 3 of 6");
  });
  it("marks every step done in Done", () => {
    render(<CardStepper stage="Done" />);
    expect(screen.getAllByRole("listitem").every((i) => i.dataset.state === "done")).toBe(true);
  });
});

describe("CardTeamRow", () => {
  it("renders one avatar per member with its working state", () => {
    render(
      <CardTeamRow
        team={[
          { agentId: "a", role: "frontend", working: true },
          { agentId: "b", role: "security reviewer", working: false },
        ]}
      />,
    );
    expect(screen.getAllByTestId("team-member")).toHaveLength(2);
    expect(screen.getAllByTestId("team-member")[0]).toHaveAttribute("data-working", "true");
    expect(screen.getAllByTestId("team-member")[1]).toHaveAttribute("data-working", "false");
  });
  it("keys seeded members (no agent id) by role without a duplicate-key problem", () => {
    render(
      <CardTeamRow
        team={[
          { agentId: null, role: "frontend", working: false },
          { agentId: null, role: "backend", working: false },
        ]}
      />,
    );
    expect(screen.getAllByTestId("team-member")).toHaveLength(2);
  });
  it("renders nothing for an empty team", () => {
    expect(render(<CardTeamRow team={[]} />).container).toBeEmptyDOMElement();
  });
});
