# Re-review: final-fixes wave (items 0–8)

I reviewed branch `final-fixes` (`b6d50b4..0135ef7`, 8 commits on `main` e9449db) in
`/Users/johnnynguyen/Personal/TeamCode-wt/final-fixes`. The inputs were `final-review.md` items 0–8, `final-fixes-report.md`,
`final-fixes-package.diff` and the spec. I changed nothing.

**What I re-ran:** `cargo test -p teamcode-core -- pipeline:: sessions::exit workspace::open workspace::store` (73 passed);
`cargo test -p teamcode --lib -- menu` (2 passed); `npx vitest run src/lib/keyboard.test.ts src/features/dock src/features/shell`
(16 files, 82 tests passed). I diffed the moved test modules against `e9449db`: the only changes are a header comment and
rustfmt rewrapping.

**Summary:** all nine items are ADDRESSED. Item 0's environment step is still pending, and item 3 leaves two
"working forever" paths the review did not name. The fix wave introduces no new Critical or Important breakage.

---

## Per item

### 0. Stale `node_modules` on main: ADDRESSED for the repository, but the environment step is still pending
- **Evidence:** `CLAUDE.md:24-34` adds the `npm ci` + `PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install webkit` step
  and describes what a stale install looks like. The report records every gate green in the worktree, including
  `npm run e2e` (3 passed).
- **Reasoning:** the ruling is sound. The finding said "nothing to change in the repository", and the worktree gates
  were re-run and recorded as the item asked. Main itself is not fixed yet: `/Users/johnnynguyen/Personal/TeamCode/node_modules`
  still lacks `motion`, `@xyflow/react` and the WebKit build. The controller must run `npm ci` and the WebKit install in
  main before it relies on lefthook there.

### 1. First Plan prompt lacks Linked projects: ADDRESSED
- **Evidence:** `crates/teamcode-core/src/pipeline/prepare.rs:43-57` binds through `workspace::launch::bind_if_first_start`
  when the computed effects contain `StartSession` and the task is unbound, then recomputes against the bound task.
  `src-tauri/src/commands/tasks.rs:64-85` uses it inside the lock and starts `ProjectBrief` after the lock is released.
- **Coverage:**
  - `tasks_move` goes through `run`, so the drag path is covered too (`tasks.rs:188-189`).
  - The test `the_first_plan_prompt_of_a_workspace_task_lists_its_linked_projects` reproduces the live order.
- **Ruling:** binding only when the action starts a session is right. Spec §4.1 (spec line 64) says the links are
  "fixed when its session first starts".
- **Item 12:** it is unchanged rather than regressed. `open_primary` already passed `None` for the issuing window
  (`session_service/mod.rs:47`).

### 2. A board drag into Test never runs the tests: ADDRESSED
- **Evidence:**
  - `src-tauri/src/effects.rs:40-45` dispatches `Effect::RunTest` to `commands::tests::start_or_attach`.
  - `commands/tests.rs:72-77` emits `test-started` after the lock is dropped.
  - `testStore.ts:71-73` and `TestTab.tsx:25-27` make an open tab attach.
- **Lock order:** dispatch runs with the database lock released (`tasks.rs:32`, after the `run_with` block), so the
  existing test_runs → db lock order is kept.
- **`test-started` ruling:** sound. Without the event, an open tab whose store already holds a finished run would never
  follow a new run.
- **Duplicate-run ruling:** acceptable, because both calls go through start-or-attach.

### 3. A session exit leaves the card "working": ADDRESSED for the exit path, with two gaps the review did not name
- **Evidence:**
  - `src-tauri/src/session_service/mod.rs:124-162` reads `is_stopping` and `on_exit` under one registry lock, then calls
    `hook_feed::set_state` through `card_after_exit`.
  - `crates/teamcode-core/src/sessions/exit.rs:9-24` maps a clean exit to Idle and a non-zero exit to
    `NeedsYou{Error "Session ended (exit N)"}`. It does nothing for a stop, a non-primary session, an archived task, or
    Test/Commit.
  - `registry.rs:293-298`: `is_stopping` is true exactly while `stop_blocking` holds the PTY. That covers Start fresh,
    where the old session is still the primary when it exits, and quit (`quit.rs` → `stop_all_blocking`).
- **What it fixes:** the dead `--resume` and crash cases the review named.
- **Gaps** (same symptom, not regressions):
  - **(a) `claude` not on PATH:** this case never reaches `exited`. `pty.rs:110` `resolve(..)?` fails at spawn, so
    `session_service::start` returns `Err` and `apply_effects` only logs it (`session_service/effects.rs` `apply_effects`).
    The card keeps the `SetState(Working)` from `enter::prompt` (`enter.rs:83`). The review assumed the process "exits at
    once", which is wrong for this cause.
  - **(b) Board/page "Stop agent":** this goes through `stop_blocking`, so `state_after_exit` returns `None`
    (`exit.rs:13`) and the card stays "working". The fixer chose this deliberately, but it is the same stuck-chip symptom.
  - **Suggested fix for both:** set Idle after a user-initiated `sessions_stop`, and `NeedsYou{Error}` when
    `open_primary` fails from `apply_effects`.

### 4. A failed commit draft leaves Commit "working": ADDRESSED
- **Evidence:**
  - `src-tauri/src/helpers.rs:46-50, 64-71` calls `pipeline::on_helper_failed` (`pipeline/mod.rs:237-246`) on any job
    error, and the card needs you only for `CommitMessages` while the task is still in Commit.
  - The guard released before `set_state` is a let-statement temporary, so there is no self-deadlock.
- **Draft again:** `tasks_draft_commits` (`commands/tasks.rs:221-225`) runs `pipeline::redraft_commits`
  (`mod.rs:248-256`). `CommitTab.tsx:46-68` shows the error and a Draft again button, disabled while Working.
- **Ruling:** a new command instead of a new `StageAction` variant is sound. It keeps A1's action set fixed, and the
  review itself suggested "a tiny command".

### 5. Workspace saves: ADDRESSED
- **(a) Reopen:** `workspace/import.rs:281-285` returns a dirty record as it is, and `open.rs:113` keeps Reload-discard
  re-reading the file.
- **(b) Close:** `src-tauri/src/windows.rs:24-36, 39-53` reads the window's container before `close` and calls
  `WorkspaceStore::save_if_dirty` (`store.rs:309-314`).
- **(c) ⌘S:** `src/lib/keyboard.ts:70, 135` and `useShellKeys.ts:20, 39-41`.
- **Ruling:** making ⌘S sidebar-scoped is correct, and the review's "global" suggestion was wrong. Spec §4.2 (line 69)
  says "⌘S with the sidebar focused". A capture-phase `preventDefault` on a global ⌘S would also take ⌘S from the
  CodeMirror editors.
- **GUI risk:** `Sidebar.tsx:16-21` (`<aside aria-label="Sidebar">`) has no `tabIndex`. In WKWebView a mouse click on a
  `<button>` does not focus it, so after clicking a sidebar item `document.activeElement` is usually `body`, and ⌘S only
  matches once focus is in a sidebar input. `tabIndex={-1}` on the aside would fix it. Check this in `tauri dev`.

### 6. The default macOS menu claims ⌘W: ADDRESSED
- **Evidence:**
  - `src-tauri/src/menu.rs:35-91` builds App, Edit, View, Window and Help, with no File menu and Close Window on
    `CmdOrCtrl+Shift+W` (`menu.rs:15`).
  - `lib.rs:36-37` installs the menu with `.menu(menu::build).on_menu_event(..)`.
- **Test:** `the_menu_claims_no_chord_of_the_keymap` (`menu.rs:136`) reads `keyboard.ts` through `include_str!`. It
  normalises the modifier order and fails on any overlap, so the menu and keymap cannot silently diverge again.
- **Edit-items ruling:** keeping the standard Edit items is right. It matches Tauri's default, and WKWebView needs them
  for copy and paste.
- **Still needs the GUI:** the per-chord check, as the report says.

### 7. Modules over 500 lines: ADDRESSED
- **Evidence:** `workspace/import.rs` is now 298 lines and `workspace/context_fetch.rs` 152. Their tests moved to
  `import_tests.rs` and `context_fetch_tests.rs` through `#[path]`.
- **Verification:** I diffed the moved code against `e9449db`, and it is identical apart from rustfmt rewrapping.
- **Helper move skipped:** not moving the context_fetch helpers is fine, because the budget is met.

### 8. Rules and docs drift: ADDRESSED
- **`CLAUDE.md:104-111`:** the real current stage.
- **`.claude/rules/prompts.md:14-16`:** the eight placeholders.
- **`.claude/rules/ui.md:20-22`:** the IPC rule as the lint enforces it.
- **`ARCHITECTURE.md`:**
  - the layout now includes `menu.rs`;
  - 17 command domains plus `mod.rs`, which matches the 18 files;
  - `livespec` without `tabSession`;
  - 27 primitive families + `ComboboxList`;
  - Ticket on the generated `jira_*` commands.
- **`@ts-nocheck`:** removed from `specta.rs:23` HEADER and from `src/lib/bindings.ts:1-2`, and ADR 0001 is updated.
  Typecheck and `bindings:check` are green per the report.
- **Leftovers:** two small ones, listed under Minor below.

---

## New Critical/Important breakage introduced by the diff

**None.** Lower-severity notes on the touched code:

- **Minor:** `src-tauri/src/specta.rs:21-22`. The doc comment above `HEADER` still justifies `@ts-nocheck`, which has
  been removed.
- **Minor:** the contracts doc (`docs/superpowers/plans/2026-10-24-team-code-00-overview-and-contracts.md`) does not
  list the new `test-started` event or the `tasks_draft_commits` command. The report calls `test-started` "a new entry
  in the contract's event list", but that list was not edited.
- **Minor:** `sessions/exit.rs:16-17`. An exit with code 0 replaces `NeedsYou{StageDone}` with Idle. For example, a
  developer who types `/exit` after a Plan hand-back loses the "needs you" signal. This follows the review's own
  prescription; consider keeping an existing NeedsYou.
- **Minor:** `windows.rs:24-36`. Save-on-close writes even while a `workspace-file-changed` reload offer is still open,
  so an external edit to the file is overwritten silently. The spec sanctions saving on close, and the record is the
  source of truth, but the open offer is never resolved.
- **Minor:** `TestTab.tsx:21-23`. A tab that mounts while the store already holds an old finished run (`recorded`)
  does not attach to a newer run started while the page was closed. `test-started` only reaches mounted tabs.
- **Minor:** `lib.rs:36`. `.menu(..)` now also gives Windows and Linux windows a menu bar. This is acceptable under
  spec §1, "macOS first".

## Out of scope (not touched by the diff)

- Item 3's two gaps (spawn failure and Stop agent) are in code the diff did not change: `pty.rs`,
  `session_service/effects.rs`, `commands/sessions.rs`.
- Minor items 9–22 are deferred as recorded. Item 12 still binds to the creating container, now from `prepare_action`
  as well.
- Quitting with a dirty workspace does not save; only `CloseRequested` does. No data is lost, because fix 5(a) keeps the
  dirty record on reopen.
