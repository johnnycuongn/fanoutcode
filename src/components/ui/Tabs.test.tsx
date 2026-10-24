import { render, screen, within } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./Tabs";

function Fixture({ value, onValueChange }: { value?: string; onValueChange?: (v: string) => void }) {
  return (
    <Tabs value={value} defaultValue="memory" onValueChange={onValueChange}>
      <TabsList aria-label="Dock">
        <TabsTrigger value="memory">Memory</TabsTrigger>
        <TabsTrigger value="team" count={2}>
          Team
        </TabsTrigger>
        <TabsTrigger value="changes" count={0}>
          Changes
        </TabsTrigger>
      </TabsList>
      <TabsContent value="memory">Memory panel</TabsContent>
      <TabsContent value="team">Team panel</TabsContent>
      <TabsContent value="changes">Changes panel</TabsContent>
    </Tabs>
  );
}

test("ArrowRight moves focus and activates the next tab", async () => {
  const on = vi.fn();
  render(<Fixture onValueChange={on} />);
  screen.getByRole("tab", { name: /Memory/ }).focus();
  await setupUser().keyboard("{ArrowRight}");
  expect(on).toHaveBeenCalledWith("team");
  expect(screen.getByRole("tab", { name: /Team/ })).toHaveFocus();
});

test("Home and End jump to the first and last tab", async () => {
  const on = vi.fn();
  render(<Fixture onValueChange={on} />);
  screen.getByRole("tab", { name: /Memory/ }).focus();
  const user = setupUser();
  await user.keyboard("{End}");
  expect(on).toHaveBeenLastCalledWith("changes");
  await user.keyboard("{Home}");
  expect(on).toHaveBeenLastCalledWith("memory");
});

test("Tab lands on the active tab only and its panel is shown", async () => {
  render(<Fixture value="team" />);
  await setupUser().tab();
  expect(screen.getByRole("tab", { name: /Team/ })).toHaveFocus();
  expect(screen.getByRole("tab", { name: /Team/ })).toHaveAttribute("tabindex", "0");
  expect(screen.getByRole("tab", { name: /Memory/ })).toHaveAttribute("tabindex", "-1");
  expect(screen.getByRole("tabpanel")).toHaveTextContent("Team panel");
});

test("count renders an amber badge and is hidden at zero", () => {
  render(<Fixture value="team" />);
  expect(within(screen.getByRole("tab", { name: /Team/ })).getByText("2")).toHaveClass("text-warn");
  expect(within(screen.getByRole("tab", { name: /Changes/ })).queryByText("0")).toBeNull();
});
