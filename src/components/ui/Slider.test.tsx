import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Slider } from "./Slider";

describe("Slider", () => {
  it("steps with the keyboard and exposes label and value text", () => {
    const onChange = vi.fn();
    render(
      <Slider value={5} min={1} max={10} onChange={onChange} label="Replay position" formatValue={(v) => `t${v}`} />,
    );
    const thumb = screen.getByRole("slider", { name: "Replay position" });
    expect(thumb.getAttribute("aria-valuetext")).toBe("t5");
    fireEvent.keyDown(thumb, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith(6);
    fireEvent.keyDown(thumb, { key: "End" });
    expect(onChange).toHaveBeenCalledWith(10);
  });
});
