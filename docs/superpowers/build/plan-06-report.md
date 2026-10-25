# Plan 06 report: Shell (windows, sidebar, views)

**Status: DONE_WITH_CONCERNS.** All 19 tasks are complete and every gate passes. The branch is rebased on main. The
rebase was a no-op because main is still at Plan 03's `504bc54`. The concerns:

- Two prop mismatches with Plan 07 (Concerns 1–2).
- Several plan tests did not run as written against the real bindings and primitives, so I adapted them (Rulings 1–6).
- Everything that needs the GUI is unverified.

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-06`, branch `plan-06`, not merged.
- Commit range: `479ef19`..`c09568e`, 20 commits on top of main `504bc54`. That is one commit per task (Tasks 1–19)
  plus `c09568e`, which loads the shell and the start screen as separate chunks.
- Ledger: `.superpowers/sdd/plan-06-progress.md`, one line per task.

## Gate results (final run on `c09568e`)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 63 passed (63)`, `Tests 319 passed (319)` (Plans 01–03 had 231) |
| `cargo test --workspace` | livespec-core `240 passed`, teamcode-core `132 passed`, shell `4 passed; 1 ignored`, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 2.12s`, no chunk-size warning after `c09568e` (before it the main chunk was 756 kB) |
| `npm run bindings:check` | exit 0. This plan adds no Rust commands. |
| lefthook pre-commit | typecheck, lint and test passed on every commit; `--no-verify` was never used |

Per-task test counts match the plan: keyboard 7; tasksStore + format 7; settings store 4; shell store 6; layout 9;
views store 8; ViewCanvas 6; TaskPicker 3; TabStrip 4; Sidebar 4 (10 with the shell store); sidebar 7, then 10;
AttentionStrip 3; ShellLayout 3; palette 6; notify 4; start-screen 3; settings 6, then 10.

On top of the plan's tests:
- `ComboboxList.test.tsx`: 4 tests (Ruling 6).
- `App.test.tsx`: rewritten, 3 tests (Ruling 24).

## STUBs (swap points)

- `src/features/board/index.ts`, `// STUB(plan 07)`. It exports `Board(BoardProps)`, `BoardSettings` and
  `OpenTarget`, exactly as Plan 07 defines them. It has no `openNewTask` (Concern 2).
- `src/features/task-page/index.ts`, `// STUB(plan 08)`. It exports `TaskPage`, `TaskPageProps` and
  `TaskPageHandle` exactly as Plan 08 Task 19 gives them. The handle methods are no-ops.
- `src/features/canvas/index.ts`, `// STUB(plan 09)`. It exports `CanvasPane({ taskId })`. The views feature passes
  `renderTeam` to the task page once that module exports one.
- `src/features/views/components/TerminalPane.tsx`, `// STUB(plan 04)`. It exports `TerminalPane`,
  `TerminalPaneProps` and `TerminalPaneHandle`, and `@/features/views` re-exports all three.
  - Plan 04 replaces this file.
  - Plan 04 must keep the rest of `src/features/views/index.ts` and add its own exports (`TerminalPlaceholder`,
    `terminalStore`).
- `src/lib/pendingCommands.ts` holds typed wrappers marked `STUB(plan NN)`:
  - Plan 05: `costSummary(scope)` and `gitChanges(taskId)`.
  - Plan 04: `sessionsTerminal(projectId)`.
  - Plan 10: `jiraSearch`, `workspacesCreate`, `workspacesAddProject`, `workspacesSaveAs`.

  Each wrapper looks the command up on the generated `commands`, so it works unchanged once the binding exists (and
  in tests). Until then it returns an empty list or a "not available yet" error.

  The same file holds STUB types:
  - Plan 05: `CostLine`, `LiveCost`, `CostSummary`, `ChangedFile` and `RepoChanges` (only the fields read here).
  - Plan 10: `Issue`.

  When those plans merge, delete each wrapper and type and import the generated one.
- `src/features/settings/types.ts`, `// STUB(plan 04)`. It holds `TerminalTheme`, `TerminalPalette` and
  `Settings = ResolvedSettings & { terminal_theme }`. Once Plan 04 adds the key, `Settings` becomes an alias.

## Unverified (needs GUI)

None of these were exercised in a real window:
- the native title and the `•` dirty mark
- the Dock badge (`setBadgeCount`)
- OS notifications and the permission prompt
- the cross-window focus tracking
- dropping files onto the Docs section (`onDragDropEvent`)
- pane drag-to-edge moves and tab drag-reorder (dnd-kit pointer sensors; the logic under them, `movePane` and
  `reorderViews`, is unit-tested)
- divider drags
- the folder and file dialogs (open, save, relocate, re-export)
- whether WKWebView delivers every §10.5 chord

`cargo check` accepts the new capability permissions.

## Rulings

1. Ruling: tests import the typed `commands` and `events` from `@/test/mockBindings`, not from `@/lib/bindings`.
   They are the same objects the `vi.mock` factory returns.
   - Why: the generated command types have no `.mockResolvedValue`, so the plan's test code fails
     `npm run typecheck`.
   - Cost if wrong: one import line per test file.
2. Ruling: I adjusted test literals for strict TypeScript and lint.
   - Partial settings are cast `as Settings`.
   - `as any` became typed casts, because `no-explicit-any` is on.
   - `viewsStore.set((s) => ({ ...s, … }))`, because Plan 01's function form of `set` replaces the whole state.
   - Store updates in WorkspaceSidebar are wrapped in `act()`.
   - Cost if wrong: none; every assertion is unchanged.
3. Ruling: tests import other features only through their `index.ts`, so `@/features/views/layout` became
   `@/features/views` and `…/shell/components/ProjectRail` became `@/features/shell`.
   - Why: `teamcode/feature-boundaries` lints test files too.
4. Ruling: `windowsList()` returns `WindowInfo[]` directly (Plan 01's binding), so the palette test mocks
   `mockResolvedValue([])`, not a `Result`. The code accepts either shape.
5. Ruling: `projects_update` is sent the full `ProjectPatch`
   (`{ name: null, color: null, memory_location }`), because the generated type requires every key. The plan's test
   expected the bare `{ memory_location }`; it now uses `expect.objectContaining`.
6. Ruling: I added `ComboboxList` to the Combobox primitive family: `src/components/ui/ComboboxList.tsx`, its test,
   a `PRIMITIVES.Combobox` member and a section on the gallery page.
   - Why: Plan 02's `Combobox` is a popover trigger that cannot be typed into. The picker and palette tests need an
     always-open `role=combobox` input with `role=option` rows.
   - It searches labels and keywords only, never values, so `tc-7` finds a task by its Jira key.
   - The PRIMITIVES order test is unaffected.
   - I wrote this primitive's test after its implementation, so its red step was skipped. Every plan task had a
     recorded failing run.
7. Ruling: `task.new` (⌘T) has `global` scope.
   - Why: the plan's "`task.*` are pane-scoped" would make ⌘T dead on the Board, which shows "⌘T starts a task".
   - Cost if wrong: ⌘T would also fire from the Board, which is what the spec wants.
8. Ruling: the keyboard scope falls back to `pane` when no `[data-scope]` element has focus and a view is the active
   tab, so ⌘D, ⌘W and ⌘P work right after switching tabs. On the Board the fallback is `global`.
9. Ruling: `TaskPicker` is mounted once by `ShellLayout`, not by `ViewCanvas`.
   - Why: a modal inside `ViewCanvas` would aria-hide the view, which breaks Task 7's test, and every hidden view
     would mount its own picker. Task 14 already lists it as a ShellLayout consumer.
10. Ruling: `openTaskInCurrentView` lives in `src/features/shell/openTask.ts` and is exported from the shell's
    `index.ts`, not from `store.ts`. That keeps the store free of a views import.
    - If the active view already shows the task, it focuses that pane.
    - I added `openTaskInNewView` for ⌘-click.
11. Ruling: view CRUD, loading and events live in `src/features/views/viewsCrud.ts`, as the plan allows. The store
    test imports `loadViews` from there.
12. Ruling: `loadViews` restores the saved focused panes without validating them, because the plan's restore test
    requires it. The effective focus always resolves through `focusedPaneOf`, which falls back to the first pane.
    - A `views-changed` reload keeps only panes that still exist.
    - It keeps the local layout of a view with an unsaved edit, so a debounced divider drag is not clobbered.
13. Ruling: I added `closePane(viewId, pane)`. The pane header's close button closes its own pane, not the focused
    one; ⌘W uses `closeFocused`.
14. Ruling: the pending-command wrappers sit in one file, `src/lib/pendingCommands.ts`, not in each feature.
    - Why: `cost_summary` is used by both the shell and the settings feature, and one file makes the swap a single
      edit.
15. Ruling: additive store APIs:
    - `loadProjects(ids?)`, which keeps the window's projects in workspace order.
    - `setContainerProjects(ids)`, so `task-changed` places a workspace's tasks correctly.
    - `upsertProject`, `selectProjectList`, `selectTaskList` and `sameContainer`.
    - `tasksStore` keeps the plan's shape exactly; see Concern 3.
16. Ruling: settings APIs are slightly wider than the plan:
    - `useSetting` returns `T | undefined` and `loadSettings` returns `Settings | undefined` (the value is absent
      until it is loaded).
    - An app-scope write refreshes the narrower scopes already loaded.
    - Added `taskScope`, `useSettings(scope = app)` (Plan 04 needs it), and `settingsDialogStore` with
      `openSettings` and `closeSettings`, created in Task 15 because the palette needs them.
17. Ruling: `createView` returns `Promise<ViewId | null>`, which is null on failure, and the call sites use `void`.
18. Ruling: inside a workspace project root, the Tasks section is a flat list in needs-you, running, idle order,
    without group headings.
    - Why: the root header carries the needs-you count, and repeated group counts made the plan's own
      `getByText("1")` ambiguous.
19. Ruling: notifications.
    - Toasts appear only for tasks of this window's projects.
    - Window focus is shared between windows through `localStorage["tc:focused-window"]` (same origin), so the
      owning window (the smallest id) can tell whether any window has focus.
    - The previous state is recorded before any `await`, so a storm of `task-changed` events gives one notice.
20. Ruling: Plan 03's `memory_pick_location` already writes the location. The field still calls `projects_update`
    with the chosen path, as the plan says. It is idempotent and refreshes `projectsStore` from the returned
    project.
21. Ruling: `CostSummary.live` is `LiveCost | null` (Plan 05's shape), not the plan's `false`; `makeCostSummary`
    uses `null`.
22. Ruling: I added the `dialog:allow-save` capability (for Re-export…) on top of the plan's list.
    `notification:default`, `core:window:allow-set-badge-count` and `allow-is-focused` are in as planned.
23. Ruling: additive helpers:
    - `keyboard.ts`: `isAppChord(e)` (Plan 04 needs it) and `chordHint(label)` (for KeyHint's `cmd+shift+a` form).
    - `format.ts`: `agentStatus`, `stageToken`, `formatUsd` and `asSwatch`.
    - `handbackLabel` takes `Pick<Task, "stage" | "state">`, as Plan 07 calls it.
24. Ruling: `src/App.tsx` routes:
    - a container → `ShellLayout`
    - first run pending → `Onboarding`
    - otherwise → `StartScreen`

    It waits for both answers before rendering, so the start screen never flashes before onboarding. The shell and
    the start screen are lazy chunks. Plan 01's `App.test.tsx` was rewritten for this routing.
25. Ruling: the onboarding has no Lottie art, because no first-run asset exists; it uses an `EmptyState` without
    animation.
26. Ruling: under reduced motion the tab strip turns off dnd-kit's CSS transition. The attention chips animate with
    Motion through `transitionFor(reduced)`.
27. Ruling: a single-project window lists one `New terminal` entry; a workspace window lists one per project.

## Concerns for the controller and the next plans

1. **Board `focusedProject`.**
   - Plan 06's test pins `focusedProject: "all"` when there is no Rail focus.
   - Plan 07 types it `string | null` and treats null as "all".
   - `'all'` type-checks as a string, but Plan 07's `configureBoard` would treat it as a project id.
   - Fix: either Plan 07 maps `"all"` to null, or both plans switch to null. One line either side.
2. **⌘T / `onNewTask`.**
   - The shell calls `openNewTask()` from `@/features/board` when that module exports it.
   - Plan 07's public surface does not list `openNewTask`, so ⌘T and `onNewTask` do nothing until Plan 07 exports
     an opener for its new-task sheet.
3. **`tasksStore` shape.**
   - Plan 07 assumes `createStore<Record<string, Task>>`.
   - Plan 06's state is `{ container, byId, order, needsYouByContainer, loaded }`, and it already loads `tasks_list`
     and applies `task-changed`.
   - Plan 07 anticipates this case in its own notes (line 1913): use `tasksStore.get().byId` and drop the duplicate
     load and event wiring.
4. **The Dock badge after a relaunch.** `needsYouByContainer` fills only from `needs-you` events, so the badge reads
   0 until a task changes. Plan 03 emits per project and per workspace on every change, which is enough while the
   app runs. A startup count command (or a `needs-you` emit at window load) would close the gap.
5. **Detail-level "Inherit".** `ResolvedSettings` does not say which layer a value came from. So a project's
   Detail level shows "Inherit" only when the resolved value is null, which it never is for `detail_level`.
   Choosing Inherit still writes null and clears the override. A `settings_get` variant that returns per-layer
   values would fix the display.
6. **Native menu.** There is no `menu.rs` yet. Whoever adds one must not bind ⌘N, ⌘W, ⌘T, ⌘D, ⌘K, ⌘P, ⌘B, ⌘E,
   ⌘J, ⌘1–⌘9 or ⌘\ (Close Window becomes ⌘⇧W), or the webview never receives those chords.
7. **Expected merge conflicts.**
   - `package.json` and `package-lock.json`: `@dnd-kit/*` was added here and is also added by Plan 07.
   - `src/features/board/index.ts`: take Plan 07's file.
   - `src/features/task-page/index.ts` and `src/features/canvas/index.ts`: take Plans 08 and 09's files.
   - `src/features/views/components/TerminalPane.tsx`: take Plan 04's file.
   - `src/features/views/index.ts`: keep both sides' exports.

## Plan text that is wrong (not edited)

- The test code across Tasks 2–19 does not type-check or lint against the real bindings, `createStore` and the lint
  rules (Rulings 1–5).
- Task 8 says Plan 02's `Combobox` supplies a typeable `role=combobox` input; it does not (Ruling 6).
- Task 1 makes ⌘T pane-scoped through "`task.*`" (Ruling 7).
- Task 8 says "mounted once by ViewCanvas" (Ruling 9).
- `RecentEntry.last_opened` is `opened_at` in Plan 01; this plan reads only `kind`, `name`, `path` and `missing`.
- Task 16 says `src-tauri/Cargo.lock`; the lock file is the workspace root's `Cargo.lock`.
- The conventions section names `values::Theme`; the binding is `ThemePreference` (Plan 03, Ruling 8).

## What the next plans need to know

- **Plan 04**:
  - `isAppChord(e)` is in `src/lib/keyboard.ts`.
  - `useSettings()` (app scope by default) is exported from `@/features/settings`.
  - Replace `src/features/views/components/TerminalPane.tsx` and keep its export names.
  - After `sessions_terminal` lands, delete the wrapper in `src/lib/pendingCommands.ts`; `TaskPicker` imports it
    from there.
  - The pane hands focus to the terminal through `setPaneHandle(pane, { kind: "terminal", handle })`, which
    `PaneContentHost` already does with the pane's ref.
- **Plan 05**:
  - Delete `costSummary`, `gitChanges` and their types from `src/lib/pendingCommands.ts`.
  - The Changes section reads `RepoChanges.files[].{path, added, removed}`.
  - The cost badge and the costs panel read `cost_usd`, `estimated`, `input_tokens`, `output_tokens` and `by_*`.
- **Plan 07**:
  - Import `tasksStore`, `projectsStore`, `handbackLabel`, `chipText` and the container helpers as listed.
  - The shell passes `settings: BoardSettings` built in `src/features/shell/boardSettings.ts`.
  - Resolve Concerns 1–3.
- **Plan 08**:
  - `TaskPage` receives `taskId`, `paneId`, `container`, `projects`, `onBack`, `onOpenTeamPane` and a `ref`.
  - Call `consumeDockTab(taskId)` from `@/features/shell` when the page mounts.
  - Set `data-scope="spec-page"` on the Spec page root so ⌃` is routed (the pane frame already sets
    `data-scope="pane"`).
- **Plan 09**:
  - `CanvasPane({ taskId })` renders `PaneContent::TeamCanvas`.
  - Export `renderTeam` from `@/features/canvas` and the shell passes it to the task page.
  - `splitFocusedPane` and `openTeamPane` are exported from `@/features/views`.
- **Plan 10**:
  - Supply `Sidebar`'s `docs` and `onAddPath`, and `SettingsDialog`'s `workspaceTab`.
  - `CostsPanel({ scope })` is reusable.
  - Delete the `jiraSearch` and `workspaces*` wrappers when the bindings land.
  - `newWorkspaceFromOpenWindows` could switch to your `workspaces_create_from_windows`.
