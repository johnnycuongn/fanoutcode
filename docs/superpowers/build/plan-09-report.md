# Plan 09 report: Team canvas

**Status: DONE_WITH_CONCERNS.** All 17 tasks are done, plus one fix commit, one perf commit and one docs commit. Every
gate passes, and the Playwright smoke suite was actually run in WebKit (3 passed). `git rebase main` was a no-op:
main is still `2d43034` (Plan 08); Plan 10 has not merged.

The concerns:
- Plan 05's `PlanStep` has no per-step `files`, `owners` or `after` (Ruling 1). Owners and dependencies are recovered
  from `PlanDoc`, but step progress is indeterminate, and file-to-step attribution relies only on dispatches.
- There is no Rive file. The working ring is a CSS ring (Ruling 7).
- Nothing ran in the real Tauri window. The suite drives the Vite dev server in WebKit with the IPC mocked.

Details:
- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-09`, branch `plan-09`, not merged.
- Commit range `2d43034..9e9d8c4`, 20 commits: `16c7967` (Task 1) … `2f2d38c` (Task 17), plus `c3a75e5` (canvas fix),
  `2340c4b` (perf) and `9e9d8c4` (ARCHITECTURE.md and CLAUDE.md).
- Ledger: `.superpowers/sdd/plan-09-progress.md`.

## Gate results (final run on `9e9d8c4`, after the rebase)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 155 passed (155)`, `Tests 888 passed (888)` (main had 128 / 762) |
| `cargo test --workspace` | livespec-core `240 passed`, teamcode-core `321 passed` (main 317, +4 `canvas::`), shell `13 passed; 1 ignored`, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run bindings:check` | exit 0 |
| `npm run build` | `✓ built in 13.01s`. Vite warns about one chunk over 500 kB: `elk.bundled-*.js`, 1.44 MB, loaded lazily (Ruling 22). `grep` finds no `__E2E_UNMOCKED__` and no playground in `dist`. |
| `npm run e2e` | `3 passed (4.6s)` in WebKit. Run 6 times in total, every run green. |
| lefthook pre-commit | passed on every commit; `--no-verify` was never used |

Per-file test counts:
- Model: events 9, attribution 12, files 10, steps 9, graph 9, model 4, replay 4, positions 6, layout 6, adapters 5,
  store 7, outline 4, lens 7, placement 5.
- Components: StepCard 7, AgentHub 2, FileCard 2, WorkingRing 2, edges 2, motionFor 1, TeamCanvas 3, Outline 1,
  DrillIn 2, ReplayScrubber 2, TeamTab 2, teamPage 2, Slider 1.

Compared with the plan's expected counts, StepCard, WorkingRing, TeamCanvas, DrillIn, adapters, outline and lens
have extra tests, and none of the plan's tests was dropped.

## STUBs

- **Replaced:** `src/features/canvas/index.ts` (`STUB(plan 09)`) is now the real public surface: `TeamTab`,
  `TeamStrip`, `CanvasPane`, `renderTeam`, `onOpenTeamPane`, `teamCanvasContent`, `teamShortcut`, `teamSummary`,
  `DOCK_GROWN_FRACTION`, `openCanvas`, `closeCanvas` and types. `grep -rn "STUB(plan 09)" src src-tauri crates` is
  empty.
- **Added:** none.

## Rulings

1. **`PlanDoc` shape.** Ruling: the canvas reads its own `CanvasStep { number, title, done, after, owners, files, repo }`
   in `types.ts`. `adapters.fromPlanDoc(doc, roots)` builds it from Plan 05's real `PlanStep` (`number, title, done,
   deps, explicit_deps, repo`) and `PlanDoc.team: TeamRole { role, steps }`:
   - `after` is `deps` only when `explicit_deps`; the flow canvas adds the implicit previous-step edge itself.
   - `owners` are the Team roles that list the step.
   - `repo` (a heading name) is mapped to a root path.
   - `files` is `[]`, because `PlanDoc` carries none.

   Why: the plan assumed `PlanStep { after, owners, files }`, which does not exist, and hand-written mirrors are
   forbidden.

   Cost if wrong: with no planned files, `progress` is null (an indeterminate bar). An orchestrator edit attributes
   to a step only through a dispatch. Fix: add `files` to Plan 05's parser; only `fromPlanDoc` changes.
2. **Type names.** Ruling: I used the generated names `TeamArrangement` (`"HubGraph" | "Flow"`, not `"FlowCanvas"`),
   `TeamPlacement`, `CanvasLayout` and `DrillInLens`. Plan 03 owns them; the plan's names do not exist. Cost: none.
3. **Fixture ids.** Ruling: `TASK_ID = "t-0c4"`, the project is `p-0a1`, and the other tasks are `t-1d2` and
   `t-2e3`, because Plan 01's `TaskId::parse` requires the `t` prefix. `ROOTS` colours are `indigo` and `cyan`
   (real swatches) instead of `blue` and `teal`. Cost: none.
4. **Task order.** Ruling: Task 9 (Rust and bindings) ran before Tasks 7 and 8. Their `CanvasPosition` comes from
   the generated bindings, and the alternative was a temporary hand-written mirror. The ledger records this. Cost:
   none.
5. **Migration and Rust tests.** Ruling: `0009_canvas.sql` is the plan's idempotent `CREATE TABLE IF NOT EXISTS`. It
   is a no-op on every database, because Plan 01's `0001_initial` already creates `canvas_positions`, with an
   `ON DELETE CASCADE` foreign key to `tasks`.
   - The core tests use `db::open_in_memory()` and insert a project and a task (the foreign key requires them). The
     plan's `db::open(":memory:")` would not do that.
   - `canvas_positions_set` calls `db::tasks::require` first.
   - Cost: none.
6. **Hub layout overlaps.** Ruling: the hub graph uses ELK stress with the plan's options, then my `layout.separate`
   pass, which pushes overlapping boxes apart along the shallower axis and falls back to a row below. Why: stress
   alone overlapped cards (the plan's own no-overlap test failed), and ELK's `sporeOverlap` throws inside elkjs
   0.9.3. Cost: a slightly less "stressed" shape.
7. **No Rive file.** Ruling: `WorkingRing` is a CSS ring: a spinning border, and a static ring (`data-ring="static"`)
   under reduced motion. `@rive-app/react-canvas` is not installed.
   - Why: no `.riv` file ships with the Rive packages, the format is binary, and I could not author one here.
     Shipping the Rive runtime with nothing to play (and it fetches its WASM from a CDN by default, which the CSP
     forbids) would be dead weight.
   - The tests drop the Rive mocks and drive reduced motion with `mockMatchMedia`.
   - Cost: spec §11.3's "Rive ring" is a CSS ring until someone authors the asset. The swap lives entirely in
     `WorkingRing.tsx`.
8. **Reduced motion.** Ruling: components read `useReducedMotion` from `@/lib/motion` (the repo rule), not
   `motion/react`, so the tests use `mockMatchMedia` instead of mocking `motion/react`.
9. **Avatar initials.** Ruling: FileCard avatars use the `Avatar` primitive with `name={role}`. They show its
   two-letter initials (`FR`, `BA`) with the role as the accessible name, not the plan's single letter; the test
   regex changed accordingly. Cost: none.
10. **Tooltip provider in tests.** Ruling: render tests that include a Tooltip (FileCard, TeamCanvas, TeamTab,
    DrillIn) use `renderUi`, because a Tooltip needs its provider.
11. **Store split.** Ruling: the store is split for the 300-line budget. `canvasState.ts` (158 lines) holds the state
    core: entries, derivation, `update`, positions. `store.ts` (226 lines) does the loading, events and actions.
    `layoutRunner.ts` does the debounces. `canvasState` and `layoutRunner` import each other, but only use each
    other at call time. Cost: none.
12. **Shared entries.** Ruling: `openCanvas` and `closeCanvas` are reference-counted. Plan 08 can render the
    `renderTeam` node in the dock and in the Swap body at once, and a pane may show the same task. Cost: none.
13. **Attributed files.** Ruling: `fromRepoChanges` reads `RepoChanges.files` (attributed) and ignores `other`.
    Plan 05's `ChangedFile` has no `attributed` flag. Cost: none.
14. **Roots.** Ruling: `rootsFor` takes names and colours from projects. A linked root with no project gets its
    folder name and `"mute"`, which `asSwatch` renders as slate.
15. **Dispatch tools.** Ruling: both `Task` and `Agent` count as dispatch tools, as in Plan 05's
    `dispatch_from_tool_input`.
16. **Task-page wiring.** Ruling: `renderTeam` and `onOpenTeamPane` take a `Task`, because Plan 08's real
    `TaskPageProps` are `renderTeam?: (task: Task) => ReactNode` and `onOpenTeamPane?(task: Task)`.
    - `onOpenTeamPane` imports `@/features/views` at call time, so its test waits for the call.
    - Plan 06's `PaneContentHost` already passes its own `openTeamPane(viewId, pane, taskId)`, so the canvas's
      `onOpenTeamPane` is exported but not used by the app.
17. **Slider primitive.** Ruling: `Slider` wraps the `radix-ui` umbrella package's `Slider`, which is already a
    dependency, instead of adding `@radix-ui/react-slider`.
    - It is the last `PRIMITIVES` entry.
    - `index.test.ts` now expects 27 entries; its name says the 26 of §15.3 plus Slider (Plan 09).
    - The example page is `src/dev/primitives/pages/SliderPage.tsx` (the repo rule's path, not the plan's), and it is
      registered in `registry.ts`.
18. **Outline test.** Ruling: steps 2 and 4 both belong to `a3`, so its edit appears under each. The test uses
    `getAllByText(...)` with a length of 2; the plan's `getByText` would throw.
19. **Drill-in.**
    - A target accepts a bare agent id or `agent:<id>`.
    - For a file target, the event stream keeps only tool calls on that file, plus its owners' hooks. This is what the
      plan's expected rows imply.
    - Agent filter chips appear only when the target has more than one agent, so a single agent's name is not
      repeated.
20. **Dev playground.** Ruling: `src/dev/canvas/CanvasPlayground.tsx` lives where the plan put it, at `#/dev/canvas`
    in `App.tsx` (dev builds only). It reaches canvas internals behind a commented
    `eslint-disable teamcode/feature-boundaries`, since a dev harness has to seed the store. `seedCanvas`,
    `updateSettings` and `pushEvent` are store exports for it.
21. **Fixes found by WebKit screenshots of the playground** (commit `c3a75e5`). This was Task 12's "check by eye",
    done with Playwright screenshots instead of the GUI.
    - Every node now has handles on all four sides, and edges pick the sides that face each other. Depends edges run
      right to left; before this, edges looped through the top handles.
    - The focus `fitView` is capped at zoom 1; it had zoomed to 2× on two nodes.
    - The done stack, and its "N done" badge, appear only in the hub graph, and the top card is the one labelled.
22. **Lazy elkjs.** Ruling: elkjs loads on the first layout. This took the canvas chunk from 1.69 MB to small, and
    elk is its own 1.44 MB chunk. Vite's chunk-size warning remains for that chunk; I did not raise the global limit.
    Cost: a first layout waits for the chunk (a local file in the app).
23. **Smoke-suite locators.** Ruling: the suite's locator block matches the real DOM; only that block (and one exact
    "Toolbar" match) differs from the plan.
    - The board is `getByTestId("board")`.
    - A column is a heading with the exact stage name.
    - A card is `role="article"`, filtered by its text, because cards have no accessible name.
    - The task title is the `Rename <title>` button.
    - The stepper has six chips: Done marks all six done.
24. **IPC mock.**
    - The mock keeps the views the shell creates in page memory (`views_create`, `views_update_layout`,
      `views_rename`, `views_reorder`), because opening a task creates a view.
    - `plugin:*` calls (window, notification, event) get null and are not counted as unmocked.
    - `settings_first_run_pending` and `memory_missing` are also handled.
25. **Browser location.** Ruling: WebKit is installed into `node_modules/playwright-core/.local-browsers`
    (`PLAYWRIGHT_BROWSERS_PATH=0`). `npm run e2e` sets the variable, and CI installs with the same variable. Nothing
    is installed globally.
26. **Type-checking the suite.** Ruling: `tsconfig.json` now includes `e2e` and `playwright.config.ts`, so the seed's
    `satisfies` checks actually run under `npm run typecheck`.
27. **React Flow attribution.** Ruling: the attribution badge is hidden (`proOptions.hideAttribution`), because it is
    an external link inside the app webview. React Flow logs a console warning asking for a Pro subscription. The
    MIT licence allows hiding it. Cost: one line to restore if the owner prefers to keep it.
28. **Default detail level.** Ruling: the canvas's own default for an unloaded `detail_level` is `Medium` (spec
    §13). Cost: none.

## Unverified (needs GUI)

- Everything in the real Tauri window (WKWebView):
  - `canvas_positions_get` and `canvas_positions_set` over real IPC;
  - `team-event`, `task-changed`, `git-changes` and `cost-updated` arriving live;
  - drag and persist across a relaunch;
  - ⌘⇧J in each placement;
  - macOS "Reduce motion" on the edge dash and the springs.
- The Rust commands are compiled, exported and clippy-clean, and their core logic is unit-tested.
- What did run in WebKit: the board, the task page and the Team tab, through the smoke suite with the IPC mocked; and
  the playground in both arrangements, with the full sample Build played and no page errors.

## Not wired (outside this plan's files)

- `TeamStrip` exists, but Plan 08's `TaskBody` does not render it under Swap while the terminal is shown; it shows
  only its own terminal strip when the team is shown. Fix: one line in `task-page/components/TaskBody.tsx`.
- `teamShortcut` exists and is tested, but Plan 08 already handles ⌘⇧J with `dock/placement.ts`'s
  `nextOnGrowShortcut`, so the app does not call it.
- There is no `settings-changed` event. The canvas re-reads settings on open and on `task-changed`, so a settings
  change shows the next time the Team tab opens.

## Plan text that is wrong (not edited)

- **Shapes from other plans:**
  - Plan 05's `PlanStep` has no `after`, `owners` or `files` (Ruling 1).
  - The arrangement is `"Flow"`, not `"FlowCanvas"`; the placement type is `TeamPlacement`.
  - `TaskPageProps.renderTeam` and `onOpenTeamPane` take a `Task`, not a `TaskId`.
  - `ChangedFile` has no `attributed` flag.
- **Ids and the database:** test ids `task-…` and `project-…` fail Plan 01's prefixes. `db::open(":memory:")` with
  no task row violates the 0001 foreign key.
- **Layout:** the stress layout alone overlaps, and SPOrE throws (Ruling 6).
- **Test code:**
  - The `motion/react` and Rive mocks (Rulings 7 and 8).
  - Single-letter avatars (Ruling 9).
  - Outline's `getByText` on a row that appears twice (Ruling 18).
  - TeamCanvas's mocked `useCanvas` returns a new object per call, which loops the position spring forever; the
    test now returns a stable state.
- **Dependency:** `@radix-ui/react-slider` is unnecessary (Ruling 17).
- **Smoke suite:** the locator block (`region "Board"`, card `button`, task `heading`, seven stepper stages) does not
  match Plans 06–08 (Ruling 23). Task 17 Step 1's `npx playwright install webkit` without
  `PLAYWRIGHT_BROWSERS_PATH=0` installs into the global cache.

## What the next plans need to know

- **Merging Plan 10.** Expect textual conflicts in these files; keep both sides:
  - `package.json` and `package-lock.json`;
  - `crates/teamcode-core/src/db/mod.rs` (`MIGRATIONS`: keep `(9, "canvas", …)` before 10);
  - `crates/teamcode-core/src/lib.rs`;
  - `src-tauri/src/commands/mod.rs` and `src-tauri/src/specta.rs`;
  - `src/lib/bindings.ts` (re-run `npm run bindings`);
  - `ARCHITECTURE.md` and `CLAUDE.md`.

  `commands/canvas.rs` uses `state.db.lock().expect(..)`, which also compiles against Plan 10's
  `db::Db = Arc<Mutex<Connection>>`.
- **Any plan** that adds a command the board, task page or Team tab calls at load must add a handler to `HANDLERS`
  in `e2e/fixtures/mockTauri.ts`. The suite fails and names the command otherwise. Run `npm run e2e`; WebKit lives
  in `node_modules` and `npm ci` removes it, so reinstall with `PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install
  webkit`.
- **Plan 05 (follow-up):** add per-step `files` (and step owners, if the Team section names them per step) to
  `PlanStep`. Then only `canvas/adapters.ts` `fromPlanDoc` changes, and progress bars become real.
- **Rive:** to use a real ring, add `src/assets/rive/agent-working.riv`, install `@rive-app/react-canvas`, pin its WASM
  to the bundled file (CSP), and change only `components/cards/WorkingRing.tsx`.
