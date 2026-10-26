// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/lib/theme", () => ({ applyTheme: vi.fn() }));
import { mockBindings, commands } from "@/test/mockBindings";
import { applyTheme } from "@/lib/theme";
import { Onboarding } from "./Onboarding";

describe("Onboarding", () => {
  it("says what the app does, checks the agents, recommends worktrees, previews the theme and completes", async () => {
    commands.agentsCheck.mockResolvedValue({
      status: "ok",
      data: [
        { kind: "ClaudeCode", program: "claude", path: "/usr/local/bin/claude", version: "2.1.310 (Claude Code)" },
        { kind: "Codex", program: "codex", path: null, version: null },
      ],
    });
    commands.settingsFirstRunComplete.mockResolvedValue({ status: "ok", data: null });
    const onDone = vi.fn();
    render(<Onboarding onDone={onDone} />);
    expect(screen.getByText(/runs AI coding agents from a board/)).toBeInTheDocument();
    expect(await screen.findByText("2.1.310 (Claude Code)")).toBeInTheDocument();
    expect(screen.getByText("npm install -g @openai/codex")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    // the recommended choice is first and already picked
    expect(screen.getByRole("button", { name: /Its own worktree \(recommended\)/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    fireEvent.click(screen.getByRole("radio", { name: "Dark" }));
    expect(applyTheme).toHaveBeenCalledWith("Dark");
    fireEvent.click(screen.getByRole("button", { name: "Start" }));
    await vi.waitFor(() => expect(commands.settingsFirstRunComplete).toHaveBeenCalledWith("FanoutCodeManaged", "Dark"));
    expect(onDone).toHaveBeenCalled();
  });

  it("with no agent CLI found it says so but still lets you continue", async () => {
    commands.agentsCheck.mockResolvedValue({
      status: "ok",
      data: [
        { kind: "ClaudeCode", program: "claude", path: null, version: null },
        { kind: "Codex", program: "codex", path: null, version: null },
      ],
    });
    render(<Onboarding onDone={vi.fn()} />);
    expect(await screen.findByText(/Install Claude Code or Codex to run tasks/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continue" })).toBeEnabled();
  });
});
