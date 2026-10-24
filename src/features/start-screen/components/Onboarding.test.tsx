// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/lib/theme", () => ({ applyTheme: vi.fn() }));
import { mockBindings, commands } from "@/test/mockBindings";
import { applyTheme } from "@/lib/theme";
import { Onboarding } from "./Onboarding";

describe("Onboarding", () => {
  it("explains both lifecycles, previews the theme and completes first run", async () => {
    commands.settingsFirstRunComplete.mockResolvedValue({ status: "ok", data: null });
    const onDone = vi.fn();
    render(<Onboarding onDone={onDone} />);
    expect(screen.getByText(/A clean worktree is removed when its session exits/)).toBeInTheDocument();
    expect(screen.getByText(/offered, never automatic/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Team Code-managed/ }));
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    fireEvent.click(screen.getByRole("radio", { name: "Dark" }));
    expect(applyTheme).toHaveBeenCalledWith("Dark");
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    await vi.waitFor(() => expect(commands.settingsFirstRunComplete).toHaveBeenCalledWith("TeamCodeManaged", "Dark"));
    expect(onDone).toHaveBeenCalled();
  });
});
