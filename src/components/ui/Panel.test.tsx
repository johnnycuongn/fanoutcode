import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Button } from "./Button";
import { Panel, PanelBody, PanelFooter, PanelHeader } from "./Panel";

test("card variant renders card tokens and radius", () => {
  render(<Panel variant="card" aria-label="p" />);
  expect(screen.getByLabelText("p")).toHaveClass("bg-card", "border-line", "rounded-card");
});

test("well variant", () => {
  render(<Panel variant="well" aria-label="p" />);
  expect(screen.getByLabelText("p")).toHaveClass("bg-well", "rounded-well");
});

test("bar variant", () => {
  render(<Panel variant="bar" aria-label="p" />);
  expect(screen.getByLabelText("p")).toHaveClass("bg-bar", "border-b");
});

test("header renders title and actions", () => {
  render(<PanelHeader title="Changes" actions={<Button>Tidy</Button>} />);
  expect(screen.getByText("Changes")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Tidy" })).toBeInTheDocument();
});

test("as renders the given element", () => {
  render(<Panel as="aside" />);
  expect(screen.getByRole("complementary")).toBeInTheDocument();
});

test("body scrolls on request and the footer renders", () => {
  render(
    <Panel>
      <PanelBody scroll>Body</PanelBody>
      <PanelFooter>Footer</PanelFooter>
    </Panel>,
  );
  expect(screen.getByText("Body")).toHaveClass("overflow-auto", "min-h-0");
  expect(screen.getByText("Footer").tagName).toBe("FOOTER");
});
