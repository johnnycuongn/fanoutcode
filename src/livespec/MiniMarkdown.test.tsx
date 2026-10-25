import { render } from "@testing-library/react";
import { expect, test } from "vitest";
import { MiniMarkdown } from "./MiniMarkdown";

test("renders a fenced block as one pre element with its text verbatim", () => {
  const { container } = render(<MiniMarkdown text={"```\nfeat(x): a\n\nbody\n```"} />);
  expect(container.querySelector("pre")?.textContent).toBe("feat(x): a\n\nbody");
  expect(container.querySelectorAll("p")).toHaveLength(0);
});

test("renders checkbox steps with their state and keeps the step text", () => {
  const { container } = render(<MiniMarkdown text={"- [ ] 1. Add store\n- [x] 2. Wire header (after 1)"} />);
  const items = container.querySelectorAll("li");
  expect(items).toHaveLength(2);
  expect(items[0].getAttribute("data-checked")).toBe("false");
  expect(items[1].getAttribute("data-checked")).toBe("true");
  expect(items[1].textContent).toContain("2. Wire header (after 1)");
});

test("never injects markup from text", () => {
  const { container } = render(<MiniMarkdown text={"<img src=x onerror=alert(1)>"} />);
  expect(container.querySelector("img")).toBeNull();
  expect(container.textContent).toContain("<img");
});
