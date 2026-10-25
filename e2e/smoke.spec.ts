import { test, expect, type Page } from "@playwright/test";
import { STAGES, BUILD_TASK, NEEDS_YOU_TASK } from "./fixtures/seed";

// The only DOM assumptions about Plans 06–08. If a plan names these differently, change this block only.
const board = (p: Page) => p.getByTestId("board");
const column = (p: Page, stage: string) => board(p).getByRole("heading", { name: stage, exact: true });
const card = (p: Page, title: string) => board(p).getByRole("article").filter({ hasText: title });
const taskTitle = (p: Page, title: string) => p.getByRole("button", { name: `Rename ${title}` });
const stepper = (p: Page) => p.getByRole("list", { name: "Stages" });
const teamTab = (p: Page) => p.getByRole("tab", { name: "Team" });
const teamPanel = (p: Page) => p.getByRole("tabpanel", { name: "Team" });
// Plan 08's stepper shows six chips; Done marks all six done.
const STEPPER_STAGES = STAGES.filter((s) => s !== "Done");

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await expect(board(page)).toBeVisible();
});

test.afterEach(async ({ page }) => {
  const unmocked = await page.evaluate(
    () => (window as unknown as { __E2E_UNMOCKED__?: string[] }).__E2E_UNMOCKED__ ?? [],
  );
  expect(unmocked, "add these commands to HANDLERS in e2e/fixtures/mockTauri.ts").toEqual([]);
});

test("board renders seven columns and a needs-you card", async ({ page }) => {
  for (const stage of STAGES) await expect(column(page, stage)).toBeVisible();
  await expect(board(page).getByRole("heading", { name: /^(Spec|Plan|Build|Review|Test|Commit|Done)$/ })).toHaveCount(
    7,
  );
  await expect(card(page, NEEDS_YOU_TASK.title)).toContainText("Plan written, go?");
});

test("opening a task shows the task page with its stepper", async ({ page }) => {
  await card(page, BUILD_TASK.title).click();
  await expect(taskTitle(page, BUILD_TASK.title)).toBeVisible();
  for (const stage of STEPPER_STAGES) await expect(stepper(page).getByText(stage, { exact: true })).toBeVisible();
  await expect(stepper(page).locator('[aria-current="step"]')).toContainText("Build");
});

test("team tab renders the canvas with an agent hub and a step card from the fixture log", async ({ page }) => {
  await card(page, BUILD_TASK.title).click();
  await teamTab(page).click();
  const canvas = teamPanel(page).locator(".react-flow");
  await expect(canvas.locator('[data-node-kind="agent"]')).toHaveCount(4); // main, a1, a2, a3
  await expect(canvas.locator('[data-node-kind="agent"]').filter({ hasText: "ui-polish" })).toBeVisible();
  await expect(canvas.locator('[data-node-kind="step"]')).toHaveCount(6);
  await expect(
    canvas
      .locator('[data-node-kind="step"][data-status="active"]')
      .filter({ has: page.getByText("Toolbar", { exact: true }) }),
  ).toBeVisible();
  await expect(canvas.locator('[data-shared="true"]')).toHaveCount(1); // a.ts, edited by a1 and a2
  await teamPanel(page).getByRole("radio", { name: "Outline" }).click();
  await expect(teamPanel(page).getByRole("checkbox")).toHaveCount(6);
});
