# Plan 07 report: Board

**Status: DONE_WITH_CONCERNS.** All 18 tasks are complete and every gate passes. The concerns are everything that needs the
GUI (drag with a real pointer, animation, Lottie playback), four backend names that are not in the generated bindings yet
and are reached through marked stubs, and a task-cache shape that Plan 06 will own.

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-07`, branch `plan-07`, not merged.
- Commit range `504bc54` (main, Plan 03 final) .. `873922f`: 20 commits (one per task 1-18, plus `ff45ba2` a build fix and
  `873922f` ARCHITECTURE.md). `git rebase main` was a no-op: main had not advanced and Plan 06 had not merged when I finished.
- Ledger: `.superpowers/sdd/plan-07-progress.md`, one line per task.
- Rust and the generated bindings are untouched. Files changed outside `src/features/board/`: `vite.config.ts` (Ruling 15),
  `ARCHITECTURE.md`, `package.json` / `package-lock.json` (dnd-kit), and the two Plan 06 stubs `src/lib/format.ts`,
  `src/lib/stores/tasksStore.ts`.

## Gate results (final run on `873922f`)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0, no output |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 66 passed (66)`, `Tests 435 passed (435)` (Plans 01-03 contribute 224; the board adds 211 in 26 files) |
| `cargo test --workspace` | all `ok`: livespec-core 240, teamcode-core 132, shell 4 (+1 ignored keychain), 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run bindings:check` | exit 0 |
| `npm run build` | `built in 885ms` |
| lefthook `pre-commit` | typecheck, lint and test ran green on all 20 commits; no `--no-verify` |

`npm run build` does not include the board: nothing imports it until Plan 06 mounts it, so it is tree-shaken. I checked the
real bundling with a temporary `import { Board } from "@/features/board"` in `main.tsx` (reverted, not committed): the build
emitted `DoneLottie-*.js` and `EmptyStateLottie-*.js` as their own chunks, `dotlottie-player-*.wasm` as a bundled asset, and
(after Ruling 15) `empty-column-*.lottie` and `done-moment-*.lottie` as separate files.

Per-file test counts: chipText 15, filters 11, columns 8, store 13, actions 11, elapsed 12, activity 18, CardParts 9, TaskCard 15,
menu 9, RenameInline 3, JiraKeyDialog 3, useBoardShortcuts 3, CardMenu 4, ConfirmDialog 2, dnd 14, Board 6, Empty 12, Motion 5,
lanes 7, Swimlanes 8, WorkspaceModes 9, cost 9, BoardCost 2, KeyboardDrag 3, index 1.

## Rulings

1. Ruling: the `tasksStore` STUB uses the shape Plan 06's own plan documents (`{ container, byId, order, needsYouByContainer, loaded }`,
   built with `createStore`), not the `Record<string, Task>` that Plan 07's Interfaces assume. The board touches the cache only through
   `src/features/board/taskCache.ts` (`taskMap`, `putTask`, `dropTask`, `replaceTasks`). Why: Plan 06 owns the file and its text fixes the
   shape, so the stub can be replaced by theirs without touching board logic. The tests' `tasksStore.get()['t-1']` became `taskMap()['t-1']`.
   Cost if wrong: if Plan 06 ships a different shape, only `taskCache.ts` changes.
2. Ruling: `src/lib/format.ts` STUB exports `chipText(reason, stage)` and `handbackLabel(task)`, and the board's `chipFor` calls
   `chipText`, not `handbackLabel`. Why: Plan 06's `handbackLabel` takes a `Task` while the board has a `Pick<Task, 'stage' | 'state'>`;
   `chipText` has the same signature in both. Cost: none.
3. Ruling: Plan 02's primitives differ from what the plan assumed, so I used the real ones. `transitionFor(reduced, full?)` (not
   `(kind, reduced)`); `StatusDot` takes `tone`/`pulse` and I pass `data-testid="status-dot"`; `EmptyState` takes `title`, `description`,
   `animation` (a `.lottie` URL), so an empty column puts its hint in `title` and is wrapped in a `data-testid="empty-state"` element; no
   `Lottie` primitive exists, so the done moment is a board-local lazy `DoneLottie.tsx` over `@lottiefiles/dotlottie-react` (same wasm
   setup as `EmptyStateLottie`); `toast` comes from the barrel. Cost if wrong: Plan 02 may lift `DoneLottie` into a primitive later.
4. Ruling: `tasks_activity`, `cost_summary`, `git_changes`, `team-event` and `sessions_stop` are not in `bindings.ts` (Plans 05 and 04).
   `stubs/plan05.ts` and `stubs/plan04.ts` call them by name with `invoke`/`listen` behind `eslint-disable-next-line
   teamcode/no-manual-ipc -- STUB(plan NN)` and define the types with the field names of Plan 05's plan (`TaskActivity`, `TeamRow`,
   `CostSummary`, `CostLine`, `LiveCost`, `RepoChanges`, `ChangedFile { path, added, removed }`, `TaskEvent`, `HookEvent`). Hand-written
   mirrors are normally forbidden (ADR 0001); these are temporary. Cost if wrong: a name mismatch would fail at runtime only, until the
   swap (see "What the next plans need to know").
5. Ruling: `makeCost` defaults `live: null`, not `live: false`, because Plan 05's `CostSummary.live` is `Option<LiveCost>`.
6. Ruling: the plan's `WorkspaceProjectLayer` is `ProjectLayer` in the generated bindings (`"MultiRoot" | "Rail"`); `WorkspaceBoard` is as
   planned. `TaskPatch` in the bindings has every field required plus `clear_color`/`clear_jira_key`, so `ipc.tasksUpdate` takes a board-side
   `TaskPatchInput` (`color: null` clears the colour, `jira_key: null` unlinks) and builds the full `TaskPatch`. The mock-facing call is
   still `tasksUpdate('t-1', { jira_key: 'TC-12' })` as the plan's tests say.
7. Ruling: `tasks_move` and the other task commands return `ActionOutcome { task, toasts }` (Plan 03), not nothing. `moveTask` writes the
   returned task into the cache and shows each core toast with `toast.info`. Cost: none.
8. Ruling: plan inconsistency between Tasks 12 and 13. Task 12's tests wait for `column-grid` on an empty board, but Task 13 replaces the
   columns with `BoardEmpty` when there are no tasks. I kept Task 13's behaviour and gave those three Task 12 tests one card.
9. Ruling: `BoardProps.focusedProject` is `string | null`, but Plan 06's `boardProject()` returns `ProjectId | 'all'`. `configureBoard` treats
   `'all'` as `null`. Cost: none.
10. Ruling: `configureBoard` is idempotent for equal props, and a project the user picked by hand in One-project mode survives a re-render
    with the same `focusedProject` (it resets only when the focus value changes or the mode newly becomes `one_project`). Without this, a host that
    passes a fresh `settings` object each render would snap the picker back to All.
11. Ruling: `CardMenu` is split in two. `CardMenu({ items, children })` wraps the card in a context menu (a `div.contents` trigger) and
    `CardMenuButton({ items })` is the `…` button inside the card, both rendering one `cardMenuItems(task, ctx)` list. The plan's single
    `CardMenu({ task, ctx, children })` cannot put a button inside the card and wrap the card in the same component. Display shortcuts such as
    `⌘E` are shown as the item's right-hand hint (the primitive's `shortcut` takes `cmd+e` chords).
12. Ruling: keyboard. `KeyboardSensor` starts on Space only, drops on Space or Enter, cancels on Escape, so Enter stays free to open a card
    (`TaskCard` opens on Enter unless a drag is on). The card is also the dnd activator node, so keys typed in the rename box never start a
    drag, and a card being renamed is not draggable. Arrow keys step between columns of the same lane (`nextColumnTarget`).
13. Ruling: cost loading is lazier than the plan. `trackTaskCost` runs for Detailed cards and, for other levels, on the first hover of a
    card, not for all 200 cards at load. Each `cost-updated` still refetches the board total (no throttle), as the plan's test requires.
    Cost if wrong: a burst of status-line events means one `cost_summary` per event. `CostSummary.by_task` already carries per-task lines,
    so a later change could drop per-task fetches entirely.
14. Ruling: `store.ts` was split. The selectors and `modeOf` live in `selectors.ts` (re-exported from `store.ts`, so `import { selectColumns }
    from './store'` still works), keeping both files under the 300-line budget.
15. Ruling: I changed Plan 01's `vite.config.ts`: `build.assetsInlineLimit` returns `false` for `.lottie`. Vite inlined both small `.lottie` files as
    `data:application/octet-stream;base64` URLs, and dotLottie fetches its source, which the CSP (`default-src 'self'`) blocks. This also fixes
    Plan 02's `EmptyState` art. Cost if wrong: one config line.
16. Ruling: tests that render cards use `mountUi` (`src/features/board/testing/render.tsx`, `render` inside `UiProvider` with a `rerender` that
    keeps it), because the card's tooltip needs `TooltipProvider`. `Select` and the combobox are driven by keyboard (Radix, not a native
    `<select>`). The Meta-click test uses `userEvent.setup()` so the modifier persists across calls. Tests that mount full columns mock
    `@/components/ui/EmptyStateLottie` (jsdom has no WebAssembly), as Plan 02's registry test does.
17. Ruling: `CardChip` passes the dot through `Chip`'s `icon` slot, with the model tone on `data-tone`; `Chip`'s own `status` prop cannot show a
    custom tone or pulse.
18. Ruling: a coloured card paints only its left edge: `border-l-2` plus `swatchClass(color, 'edge')` (which sets all four border colours) plus
    `border-t-line border-r-line border-b-line`. Cost: none.
19. Ruling: I added `ARCHITECTURE.md` "The board" section and a table row (repo rule: kept current). It sits next to Plan 02's section, so it can
    conflict with Plan 06's edits; keep both sides.
20. Ruling: the red step was skipped when I wrote implementation and test together in Task 1 and Task 12; I confirmed afterwards that removing the
    module makes the test fail with `Failed to resolve import`. Every other task has a recorded failing run before the implementation.
21. Ruling: commit trailers say `Co-Authored-By: Claude Sonnet 5.5`, my own model name, as the brief says.

## STUBs

- `src/lib/stores/tasksStore.ts` `// STUB(plan 06)`: state shape only, no loading, events or selectors. Replace with Plan 06's file.
- `src/lib/format.ts` `// STUB(plan 06)`: `chipText`, `handbackLabel`. Replace with Plan 06's file (same `chipText(reason, stage)`).
- `src/features/board/stubs/plan05.ts` `// STUB(plan 05)`: `tasksActivity`, `costSummary`, `gitChanges`, `listenTeamEvent`, plus local copies of
  Plan 05's types.
- `src/features/board/stubs/plan04.ts` `// STUB(plan 04)`: `sessionsStop`.
- Not a stub but a gap: `src/features/board/assets/empty-column.lottie` is Plan 02's placeholder ring (copied), and `done-moment.lottie` is a
  hand-written ring and check mark (CC0), authored as a dotLottie v1 zip. Neither has been opened by a real player.

## Unverified (needs the GUI or the real backend)

- A pointer drag in a real webview (the 4 px activation distance, the drag overlay, the layout spring, the destination glow, reduced motion).
  The keyboard drag is covered end to end in jsdom (`KeyboardDrag.test.tsx`, with stubbed column rectangles); the pure drop rules are unit tested.
- 60 fps with 200 cards (spec §16). Cards are memoised, selectors are stable, the clock re-renders only text leaves, but nothing was profiled.
- Lottie playback and the wasm being served from the bundle in the real webview; the still frame under macOS Reduce motion.
- The context menu, tooltip delay (600 ms) and the `…` menu in WKWebView.
- `tasks_activity`, `cost_summary`, `git_changes`, `team-event`, `sessions_stop` against real Rust: the stubs use the command and event names
  from the plans and have never run.
- `tasks_move` refusal (for example Test with no test command) through Tauri: tested with a rejected mock, and `moveTask` rolls back and toasts.

## Deviations from the plan (all additive or forced)

- Extra files: `listen.ts` (`track`/`all`, so an unlisten is safe whether or not the listener has resolved), `taskCache.ts`, `selectors.ts`,
  `stubs/`, `testing/render.tsx`, `components/{boardContext.ts,BoardBody.tsx,BoardCard.tsx,BoardCost.tsx,DoneLottie.tsx,FilterPopover.tsx}`,
  `assets/`.
- Extra behaviour, each with a test: a task-changed while a `tasks_list` is in flight cannot overwrite a newer container (`loadBoard` race);
  a rejected move restores only the stage, so a rename that arrived meanwhile is kept; `lane` reorder indexes are validated; a snapshot from
  `tasks_activity` never overwrites live activity; a failing `git_changes` or `cost_summary` keeps the last numbers; archived-only boards
  show columns, not "No tasks yet"; Mixed keeps its chip row when the chosen project has no cards.
- `Column` keeps the plan's `projectsById` prop; `BoardCard` receives `project` from it. `DraggableCard` takes an extra `lane` prop (for the
  droppable data) and a render-prop `children`.
- No "New task" button in the header (the plan lists none); the empty board and the host's `onNewTask` cover it.

## Plan text that is wrong (not edited)

- Task 12 vs Task 13: see Ruling 8.
- Task 11: the test helper `drop(overId, laneOf, readOnly, t = task)` is called with `undefined` to mean "no task", but a default parameter
  replaces `undefined`, so that case would test a real task. I call `resolveDrop({ task: undefined, ... })` directly.
- Task 17: the `cost-updated` test refetches the board total without ever calling `loadBoardCost`; it only works if an earlier test left the
  container in module state. Mine calls `loadBoardCost` first and `resetCostStore()` between tests.
- Task 9: `Avatar` cannot render `CX` for Codex (it derives initials from the name), so the agent badge is a small span with the plan's
  `CC`/`CX`/`SH` text and `aria-label`.
- Task 8 / 9 / 13 / 14: the primitive APIs assumed there (`StatusDot` testid, `EmptyState` `illustration`/`hint`, `Lottie`, `transitionFor(kind, ...)`) do
  not match Plan 02 (Ruling 3). Task 16's `selectOptions` cannot drive a Radix Select (Ruling 16).
- Plan 06 text: `boardProject()` returns `'all'` where `BoardProps.focusedProject` is `string | null` (Ruling 9); `openNewTask()` from this module is
  optional there and this plan does not export one.

## What the next plans need to know

- **Merging with Plan 06 (the controller):** delete my `src/lib/stores/tasksStore.ts` and `src/lib/format.ts` and take Plan 06's. Check that
  `tasksStore` is a `createStore` whose state has `byId` and `order` (and `loaded`), because `taskCache.ts` calls `tasksStore.get()/set()/subscribe()/reset()`.
  Plan 06 also loads tasks and applies `task-changed` for the same container, so the cache is written by both (harmless, idempotent). Plan 06's
  `src/features/board/index.ts` stub, if it created one, loses to mine. Mount: `<Board container projects settings focusedProject currentWindowId
  onOpenTask onNewTask />` inside a `UiProvider`; `settings` is `BoardSettings` `{ detailLevel, detailLevelByProject, costOverview, workspaceBoard,
  projectLayer }` read from `ResolvedSettings` (`detail_level`, `cost_overview`, `workspace_board`, `workspace_project_layer`). The board fills its
  parent (`h-full`), so give it a sized container. `ARCHITECTURE.md` will conflict textually; keep both sides.
- **Merging with Plan 05:** in `stubs/plan05.ts` re-export the types from `@/lib/bindings`, call `commands.tasksActivity`, `commands.costSummary`,
  `commands.gitChanges` and `events.teamEvent.listen`, and delete the two Tauri imports and the eslint-disable lines. Check the field names the board
  reads: `TaskActivity { task_id, current_file, plus, minus, team: { name, state }[] }`, `RepoChanges { repo_root, files: { path, added, removed }[] }`,
  `CostSummary { cost_usd, input_tokens, output_tokens, estimated }`, and `team-event` payload `{ task_id, event: TaskEvent }` with `event.event` the
  `HookEvent` (`type`-tagged). Then delete `makeCost`'s local type import if it moves.
- **Merging with Plan 04:** `commands.sessionsStop(sessionId)` replaces `stubs/plan04.ts`.
- **Plan 09:** import `CardChip`, `CardStepper`, `CardTeamRow`, `chipFor` from `@/features/board`; `CardChip` takes an `indicator` slot for the Rive ring.
  Plan 08 may use `ElapsedTime`, `formatElapsed`, `formatCost`, `formatTokens`.
- **Plan 06 keyboard map:** the board handles ⌘E (rename) and ⌘⇧A (archive/restore) itself, only while focus is inside the board and a card is
  focused, and stops only those keys. If Plan 06 binds them window-wide, the board's handler wins inside the board.
- Swimlane order and collapsed rows persist in localStorage under `teamcode.board.lanes.<Project|Workspace>.<id>`.
- The board adds no setting keys and no migrations.

## Rebase onto main (Plan 06 merged, c09568e)

Branch `plan-07` is now `c09568e..ebd4319` (21 commits: the 20 above, replayed, plus `ebd4319`). Gates on `ebd4319`: typecheck exit 0; lint clean; vitest
`Test Files 90 passed`, `Tests 534 passed`; `cargo test --workspace` 376 passed, 0 failed; clippy clean; `bindings:check` exit 0; `npm run build` ok (now includes
the board through the shell; `.lottie` files emitted as separate assets).

- Conflicts resolved: `src/lib/format.ts` and `src/lib/stores/tasksStore.ts` taken from Plan 06 (my stubs dropped); `package.json` keeps one `@dnd-kit/*` set (main had
  core, sortable and utilities; I added `modifiers`) and the lock file was regenerated; `src/features/board/index.ts` is mine (Plan 06's STUB(plan 07) replaced, same props
  and `BoardSettings` / `OpenTarget` exports); `views/index.ts` needed no change. `taskCache.ts` already matched Plan 06's real `{container, byId, order, needsYouByContainer, loaded}`
  shape, so it did not change. The board still loads `tasks_list` and `task-changed` itself, as well as Plan 06 (idempotent).
- `focusedProject: "all"` is mapped to null inside `configureBoard` (already so; the shell's tests are untouched).
- `openNewTask()` is exported from `src/features/board/index.ts`. Plan 06 reads it through a namespace lookup (`ShellLayout.tsx`, palette `commands.ts`), so it has no stub to remove
  and none of Plan 06's files changed. The plan had no new-task flow, so I added a minimal sheet (`newTask.ts`, `components/NewTaskDialog.tsx`, 4 tests): description, optional Jira key,
  a project picker in workspaces, then `tasks_create(project, intent, jira_key, workspace_id)`. The sheet is rendered by the Board, so ⌘T pressed on a task tab opens it when the
  Board tab is next shown. Needs the GUI to confirm; `tasks_create` itself is Plan 03's and is untested at runtime.
- Plan 05 wrappers: `cost_summary` and `git_changes` now come from Plan 06's `src/lib/pendingCommands.ts` (no second copy; `stubs/plan05.ts` re-exports its types). I kept
  `tasks_activity`, the `team-event` listener (`stubs/plan05.ts`) and `sessions_stop` (`stubs/plan04.ts`), because `pendingCommands.ts` has none of them. Note that until Plan 05's
  bindings exist, `cost_summary` answers "not available yet" and the board just shows no cost.
