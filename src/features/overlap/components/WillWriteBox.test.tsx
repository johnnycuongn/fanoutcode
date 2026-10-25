import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { commands, mockBindings } from "@/test/mockBindings";
import type { PendingDecision } from "@/lib/bindings";
import { WillWriteBox } from "./WillWriteBox";

const FILE = "import x from 'x';\n\nconst a = 1;\nconst b = 2;\nexport { a, b };\n";

function renderBox(otherTask: string | null = "task-c3") {
  return render(
    <WillWriteBox taskTitle="Checkout" taskId="task-b2" otherTask={otherTask} repo="/r/alpha" path="src/a.ts" />,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commands.gitFileRead.mockResolvedValue({ status: "ok", data: FILE });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("WillWriteBox", () => {
  it("asks for the decision against the row's other task", async () => {
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: null });
    renderBox(null);
    await act(async () => {});
    expect(commands.overlapPendingInput).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts", null);
  });

  it("labels an Edit with the lines its old string falls on in the current file", async () => {
    const edit: PendingDecision = {
      type: "Edit",
      tool: "Edit",
      input: { file_path: "/r/alpha/src/a.ts", old_string: "const a = 1;\nconst b = 2;", new_string: "const a = 3;" },
    };
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: edit });
    renderBox();
    expect(await screen.findByText("lines 3–4")).toBeInTheDocument();
    expect(screen.getByText("const a = 3;")).toBeInTheDocument();
    expect(commands.gitFileRead).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts");
    expect(commands.overlapPendingInput).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts", "task-c3");
  });

  it("a one-line Edit says one line, and an old string not in the file shows no range", async () => {
    const one: PendingDecision = {
      type: "Edit",
      tool: "Edit",
      input: { old_string: "const b = 2;", new_string: "const b = 5;" },
    };
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: one });
    const { unmount } = renderBox();
    expect(await screen.findByText("line 4")).toBeInTheDocument();
    unmount();
    const gone: PendingDecision = { type: "Edit", tool: "Edit", input: { old_string: "nope", new_string: "yes" } };
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: gone });
    renderBox();
    expect(await screen.findByText("yes")).toBeInTheDocument();
    expect(screen.queryByText(/^lines? \d/)).toBeNull();
  });

  it("says the tool call has not landed after 10 seconds instead of loading forever", async () => {
    vi.useFakeTimers();
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: null });
    renderBox();
    await act(async () => {});
    expect(screen.getByText("Loading what Checkout will write…")).toBeInTheDocument();
    await act(async () => {
      vi.advanceTimersByTime(9_999);
    });
    expect(screen.queryByText(/hasn't landed yet/)).toBeNull();
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.getByText("The tool call hasn't landed yet; it shows here when it does.")).toBeInTheDocument();
    expect(screen.queryByText("Loading what Checkout will write…")).toBeNull();
  });
});
