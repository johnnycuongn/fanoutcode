import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SWATCHES, swatchClass } from "@/lib/theme";
import { mountUi } from "../testing/render";
import { emptyActivity } from "../activity";
import { WORKING, makeCost, makeProject, makeTask } from "../testing/fixtures";
import { TaskCard } from "./TaskCard";

const activity = {
  lastLine: "Editing store.ts",
  currentFile: { path: "/r/src/store.ts", added: 12, removed: 3 },
  team: [{ agentId: "a", role: "frontend", working: true }],
};
const base = {
  activity,
  showProject: false,
  costOverview: false,
  cost: null,
  focused: false,
  onOpen: vi.fn(),
  onFocus: vi.fn(),
  menu: null,
};
const swatch = SWATCHES[0];
const task = makeTask({
  id: "t-1",
  title: "Add login",
  priority: "High",
  jira_key: "TC-12",
  color: swatch,
  state: WORKING,
  stage: "Build",
});

describe("TaskCard", () => {
  it("card shows the overlap badge only when overlaps exist", () => {
    const { rerender } = mountUi(<TaskCard {...base} task={task} level="Simple" />);
    expect(screen.queryByText(/overlap/)).toBeNull();
    rerender(<TaskCard {...base} task={task} level="Simple" overlaps={0} />);
    expect(screen.queryByText(/overlap/)).toBeNull();
    rerender(<TaskCard {...base} task={task} level="Simple" overlaps={1} />);
    expect(screen.getByText("1 overlap")).toBeInTheDocument();
    rerender(<TaskCard {...base} task={task} level="Simple" overlaps={3} />);
    expect(screen.getByText("3 overlaps")).toBeInTheDocument();
  });
  it("Simple shows title, priority flag, Jira key, chip, agent avatar and elapsed time, and nothing else", () => {
    mountUi(<TaskCard {...base} task={task} level="Simple" />);
    expect(screen.getByText("Add login")).toBeInTheDocument();
    expect(screen.getByLabelText("Priority High")).toBeInTheDocument();
    expect(screen.getByText("TC-12")).toBeInTheDocument();
    expect(screen.getByText("Working")).toBeInTheDocument();
    expect(screen.getByLabelText("Claude Code")).toHaveTextContent("CC");
    expect(screen.getByTestId("elapsed")).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: /Stage/ })).toBeNull();
    expect(screen.queryByText("store.ts")).toBeNull();
  });
  it.each([
    ["Codex", "CX"],
    ["Shell", "SH"],
  ] as const)("the %s agent reads %s", (kind, text) => {
    mountUi(<TaskCard {...base} task={makeTask({ agent_kind: kind })} level="Simple" />);
    expect(screen.getByLabelText(kind)).toHaveTextContent(text);
  });
  it("Simple hides the priority flag when priority is None", () => {
    mountUi(<TaskCard {...base} task={makeTask({ priority: "None" })} level="Simple" />);
    expect(screen.queryByLabelText(/Priority/)).toBeNull();
  });
  it("paints the 2 px edge with the swatch edge class and no edge without a colour", () => {
    const { rerender } = mountUi(<TaskCard {...base} task={task} level="Simple" />);
    expect(screen.getByRole("article")).toHaveClass("border-l-2", swatchClass(swatch, "edge"));
    rerender(<TaskCard {...base} task={makeTask({ color: null })} level="Simple" />);
    expect(screen.getByRole("article")).not.toHaveClass("border-l-2");
  });
  it("Medium adds the stepper and the current file with +/-", () => {
    mountUi(<TaskCard {...base} task={task} level="Medium" />);
    expect(screen.getByRole("list", { name: "Stage Build, 3 of 6" })).toBeInTheDocument();
    expect(screen.getByText("store.ts")).toBeInTheDocument();
    expect(screen.getByText("+12")).toHaveClass("text-ok");
    expect(screen.getByText("−3")).toHaveClass("text-bad");
    expect(screen.queryByTestId("team-member")).toBeNull();
  });
  it("Medium shows no file line when the agent is not touching a file", () => {
    mountUi(<TaskCard {...base} task={task} level="Medium" activity={emptyActivity} />);
    expect(screen.queryByTestId("current-file")).toBeNull();
  });
  it("Detailed adds the team row, and cost only when the overview is on", () => {
    const cost = makeCost({ cost_usd: 0.42, input_tokens: 12_300, output_tokens: 800, estimated: true });
    const { rerender } = mountUi(<TaskCard {...base} task={task} level="Detailed" cost={cost} />);
    expect(screen.getAllByTestId("team-member")).toHaveLength(1);
    expect(screen.queryByText("~$0.42")).toBeNull();
    rerender(<TaskCard {...base} task={task} level="Detailed" cost={cost} costOverview />);
    expect(screen.getByText("~$0.42")).toBeInTheDocument();
  });
  it("shows the project badge only when asked", () => {
    const project = makeProject({ id: "p-1", name: "Alpha" });
    const { rerender } = mountUi(<TaskCard {...base} task={task} level="Simple" project={project} />);
    expect(screen.queryByText("Alpha")).toBeNull();
    rerender(<TaskCard {...base} task={task} level="Simple" project={project} showProject />);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
  });
  it("hover tooltip carries the last agent line", async () => {
    mountUi(<TaskCard {...base} task={task} level="Simple" />);
    await userEvent.hover(screen.getByRole("article"));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Editing store.ts");
  });
  it("the tooltip says so when the agent has produced nothing yet", async () => {
    mountUi(<TaskCard {...base} task={task} level="Simple" activity={emptyActivity} />);
    await userEvent.hover(screen.getByRole("article"));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("No agent output yet");
  });
  it("⌘-click opens in a new view, plain click in the current view", async () => {
    const onOpen = vi.fn();
    mountUi(<TaskCard {...base} task={task} level="Simple" onOpen={onOpen} />);
    await userEvent.click(screen.getByRole("article"));
    expect(onOpen).toHaveBeenLastCalledWith({ kind: "current" });
    const user = userEvent.setup();
    await user.keyboard("{Meta>}");
    await user.click(screen.getByRole("article"));
    await user.keyboard("{/Meta}");
    expect(onOpen).toHaveBeenLastCalledWith({ kind: "new_view" });
  });
  it("Enter on the focused card opens it", async () => {
    const onOpen = vi.fn();
    mountUi(<TaskCard {...base} task={task} level="Simple" onOpen={onOpen} />);
    screen.getByRole("article").focus();
    await userEvent.keyboard("{Enter}");
    expect(onOpen).toHaveBeenCalledWith({ kind: "current" });
  });
  it("clicking inside the menu slot does not open the task", async () => {
    const onOpen = vi.fn();
    mountUi(<TaskCard {...base} task={task} level="Simple" onOpen={onOpen} menu={<span>menu</span>} />);
    await userEvent.click(screen.getByText("menu"));
    expect(onOpen).not.toHaveBeenCalled();
  });
  it("an archived card wears an Archived badge and a hidden card is dimmed", () => {
    mountUi(<TaskCard {...base} task={makeTask({ archived: true, hidden: true })} level="Simple" />);
    expect(screen.getByText("Archived")).toBeInTheDocument();
    expect(screen.getByRole("article")).toHaveAttribute("data-hidden", "true");
  });
});
