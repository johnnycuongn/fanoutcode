# Final fixes report: Team Code, branch `final-fixes`

**Status:** DONE_WITH_CONCERNS. Items 0–8 are fixed. The concerns are the GUI checks that can't run here.
**Worktree:** `/Users/johnnynguyen/Personal/TeamCode-wt/final-fixes`, branch `final-fixes`, based on `main` e9449db (main has not moved since).
**Commits:** `b6d50b4..0135ef7`, 8 commits, not merged.

```
b6d50b4 fix: bind a workspace task's links before its first stage prompt is assembled        (item 1)
7cf0ef5 fix: start the tests when a task enters Test, whichever way it got there             (item 2)
9b7d135 fix: move the card off "working" when its primary session ends on its own            (item 3)
2aaf0b0 fix: hand back a failed commit-message draft and offer Draft again                   (item 4)
8ff65ba fix: keep unsaved workspace changes: save on ⌘S and on close, never revert on reopen (item 5)
dd5312b fix: build the native menu explicitly so it claims no chord of the keymap            (item 6)
462a6c7 refactor: move the import and context-fetch tests out to stay under the module budget (item 7)
0135ef7 docs: bring CLAUDE.md, the rules, ARCHITECTURE.md and ADR 0001 in line with the code (items 0 and 8)
```

## Gates (all green in the worktree, after the last commit)

| Gate | Result |
|---|---|
| `npm ci` | ok (WebKit copied from plan-09's `node_modules/playwright-core/.local-browsers`) |
| `npm run typecheck` | exit 0 |
| `npm run lint` | exit 0, "All matched files use Prettier code style!" |
| `npx vitest run` | Test Files 165 passed (165), Tests 925 passed (925) |
| `cargo test --workspace` | livespec-core 240 passed; teamcode_lib 15 passed, 1 ignored; teamcode_core 457 passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `npm run build` | exit 0, "✓ built in 13.38s" (the usual chunk-size warning) |
| `npm run bindings:check` | exit 0 |
| `PLAYWRIGHT_BROWSERS_PATH=0 npm run e2e` | 3 passed (13.0s) |

One pre-commit vitest run failed once and passed on the retry (the `git commit --amend` of the docs commit). It happened
while nothing else was running. I couldn't tell which test failed. The likely suspects are the timing-heavy suites
(`registry.test.tsx` takes about 8 s, plus the fake-timer stores). It didn't happen again in three later full runs.

## Important items

### 0. Stale `node_modules` on main: fixed in the docs (0135ef7)
I didn't touch the main checkout. `CLAUDE.md` → Commands now tells a fresh clone, or any checkout whose lockfile
moved, to run `npm ci` and then `PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install webkit`. It says nothing runs these
automatically (there is no `postinstall`), and it describes what a stale `node_modules` looks like. The `npm run e2e`
line points back to that install. There is no README in the repo, so CLAUDE.md is the place. The controller still
has to refresh main's `node_modules` after the merge.

### 1. The first stage prompt had no Linked projects block: fixed (b6d50b4)
- I added `pipeline::prepare_action` in `crates/teamcode-core/src/pipeline/prepare.rs`. It reads the task, builds the
  transition context and computes the effects. If those effects contain `StartSession` and the links aren't bound
  yet, it calls `workspace::launch::bind_if_first_start` and computes again. `commands/tasks.rs::run_with` now uses
  it and starts `HelperKind::ProjectBrief` once the lock is released when `needs_briefs`. `open_primary` keeps its
  own call as a fallback.
- `run`/`run_with` now take `Fn` instead of `FnOnce`, because the effects may be computed twice. Two callers changed
  to match: `action.clone()` and `state_now.clone()`.
- **Tests** (`pipeline::prepare::tests`):
  - `the_first_plan_prompt_of_a_workspace_task_lists_its_linked_projects` checks that the prompt has
    `## Linked projects` and the api root, and doesn't have "This task has no linked projects". It failed with the
    binding step turned off: `panicked at crates/teamcode-core/src/pipeline/prepare.rs:92:9`.
  - `an_action_that_starts_no_session_leaves_the_links_unbound`: Archive doesn't bind.
  - Output: `test result: ok. 2 passed; 0 failed`.

### 2. A board drag into Test never ran the tests: fixed (7cf0ef5)
- `effects.rs` now handles `Effect::RunTest` by calling `commands::tests::start_or_attach(app, &task.id)` and logging
  any failure. This also fixes the task header's and the palette's "Run tests", which only sent `RunTests` before.
- When it starts a new run, `start_or_attach` emits a new event, `test-started {task_id}`.
- `testStore.onTestStarted` attaches unless the store is already following a run. `TestTab` listens for the event, so
  an open tab follows a run started elsewhere. A tab that opens later attaches on mount, as before.
- **Tests:**
  - `testStore.test.ts` › "test-started from elsewhere attaches once; a run already followed is left alone". It failed
    before the change (`TypeError: s.onTestStarted is not a function`); now 6/6 pass.
  - `specta::tests::bindings_export_every_contract_event_and_the_first_commands` now also checks `test-started`
    (`1 passed`).
- The dispatch line itself is shell glue with no unit test: no mock Tauri runtime is wired up. **Unverified: needs
  GUI.**

### 3. A session exit left the card "working": fixed (9b7d135)
- `sessions::state_after_exit(session, task, stopping, code)` lives in `crates/teamcode-core/src/sessions/exit.rs`.
  When the task's primary session exits in Plan, Build or Review, a clean exit sets `Idle`. Any other exit code sets
  `NeedsYou { Error { "Session ended (exit N)" } }`.
- It returns `None` in four cases:
  - The app asked for the exit. `SessionRegistry::is_stopping` reports this: the stop sequence holds the PTY. That
    covers Start fresh, Archive, the stop button and quit.
  - The session is no longer the task's primary.
  - The task is archived.
  - The task is in Test or Commit, where the test run or the draft owns the state.
- `session_service::exited` reads `is_stopping` and `on_exit` under the same lock, releases it, then calls
  `hook_feed::set_state`.
- **Tests:**
  - `sessions::exit::tests::a_primary_session_that_dies_on_its_own_needs_you_with_the_exit_code`
  - `sessions::exit::tests::an_asked_for_stop_another_session_or_a_test_stage_leaves_the_card_alone`
  - `sessions::registry::tests::a_session_whose_pty_was_taken_for_a_stop_is_stopping_until_it_exits`. Before the
    change this didn't compile (`no method named is_stopping`).
  - Output: `test result: ok. 52 passed` (the `sessions` filter).

### 4. A failed commit draft left Commit "working": fixed (2aaf0b0)
- `pipeline::on_helper_failed(task, kind, message)`: when the commit-message draft fails and the task is still in
  Commit (and not archived), the card becomes `NeedsYou { Error { "Couldn't draft the commit messages: …" } }`.
  Failures of other helpers change nothing.
- `helpers::run_after` calls it, through `hand_back_failure`, whenever a job returns an error.
- **Draft again:** `pipeline::redraft_commits(task)` emits `[RunHelper(CommitMessages), SetState(Working)]` and is
  refused outside Commit. The new command `tasks_draft_commits` calls it. The Commit tab shows the error and a
  **Draft again** button, disabled while the draft runs (`commitStore.redraft`).
- **Tests:**
  - `pipeline::tests::a_failed_commit_draft_needs_you_and_can_be_drafted_again`. It failed before the change
    (`cannot find function on_helper_failed`); now `1 passed`.
  - `CommitTab.test.tsx` › "a failed draft shows its error and Draft again runs the helper once more" and
    "Draft again is disabled while the draft runs". Both failed before the change (2 failed); now the dock suite
    passes, 54/54.

### 5. Workspace saves: no ⌘S, no save on close, reopen reverted changes: fixed (8ff65ba)
- **(a) Reopen.** `import::open_file` now returns a dirty record as it is. `open::reload` with `discard_unsaved` calls
  the new `import::reopen(store, path, keep_unsaved: false)`, so discarding still re-reads the file.
- **(b) Close.** `windows::on_window_event(CloseRequested)` reads the window's container before closing it. For a
  workspace it calls the new `WorkspaceStore::save_if_dirty` and then emits `workspace-changed`.
- **(c) ⌘S.** `KEYMAP["workspace.save"] = ⌘S` in a new `sidebar` scope. It matches only when
  `ActiveScope.sidebarFocused` is true, which `useShellKeys` sets when focus is inside `[aria-label="Sidebar"]`.
  The handler saves when the store reports an explicit-save workspace that is dirty.
- **Tests:**
  - `workspace::open::tests::reopening_a_dirty_workspace_keeps_its_unsaved_changes`. It failed before the change
    (`panicked … open.rs:264:9: still unsaved`).
  - `store_tests::save_if_dirty_writes_only_unsaved_changes`.
  - `keyboard.test.ts` › "claims ⌘S for the workspace file only while the sidebar has focus". It failed before the
    change (1 failed).
  - The existing `reload_refuses_when_dirty_unless_discard` still passes. Output: `test result: ok. 110 passed`
    (the `workspace` filter); vitest keyboard plus shell, 28 passed.
- Saving on close is shell glue. **Unverified: needs GUI.**

### 6. Tauri's default menu claimed ⌘W: fixed (dd5312b)
- `src-tauri/src/menu.rs` builds these menus, set with `.menu(menu::build)` and `.on_menu_event(menu::on_menu_event)`:
  - **App:** About, Services, Hide ⌘H, Hide Others ⌘⌥H, Show All, Quit ⌘Q.
  - **Edit:** Undo, Redo, Cut, Copy, Paste, Select All.
  - **View:** Full Screen ⌃⌘F.
  - **Window:** Minimize ⌘M, Zoom, **Close Window ⌘⇧W** (a custom item that closes the focused window, so its
    `CloseRequested` runs), Bring All to Front.
  - **Help:** empty.
- There is no File menu, because its default item is Close Window on ⌘W.
- `MENU_CHORDS` lists every accelerator the menu takes. Test `menu::tests::the_menu_claims_no_chord_of_the_keymap`
  reads `src/lib/keyboard.ts` with `include_str!` and fails on any overlap. I injected ⌘W to check it fails
  (`panicked at src-tauri/src/menu.rs:141:13`). `close_window_is_command_shift_w` also passes. Output: `2 passed`.
- **GUI check of every §10.5 chord against the menu: unverified, needs GUI.** In particular:
  - ⌘W must close the pane, not the window.
  - ⌘` must still reach `window.next` or AppKit's window cycling.
  - ⌘C/⌘V/⌘A must still work in xterm and in inputs. The standard Edit items send them to the webview, as Tauri's
    default menu did.

### 7. Two Rust modules over 500 lines: fixed (462a6c7)
- `workspace/import.rs` went from 607 to 298 lines and `workspace/context_fetch.rs` from 563 to 152. Each one's test
  module moved to `import_tests.rs` and `context_fetch_tests.rs` through `#[path]`, as `store_tests.rs` does. The
  code didn't change.
- The same tests pass before and after: import 10, context_fetch 9.
- I didn't move the context_fetch helpers (`folder_listing`, `capped_text`). With its tests moved out, the module is
  152 lines, so a second file would add nothing.
- The largest Rust module now is `memory/index.rs` at 473 lines.

### 8. Rules and docs had drifted: fixed (0135ef7)
- **CLAUDE.md:**
  - "Current stage" now says all ten plans are built and merged, the Important fixes have landed, nothing has run in
    the GUI yet, and the Minor items are still open.
  - New "How to add: a ⌘ chord" entry.
  - The fresh-clone commands (item 0). CLAUDE.md is 110 lines.
- **`.claude/rules/`:**
  - `prompts.md` allows the eight placeholders, adding `{{test.exit_code}}` and `{{test.output}}`.
  - `ui.md` states the IPC rule the way the lint enforces it: `Channel` from core, nothing from event, and the
    window and webview handles are allowed. It also gains the keymap/menu rule.
  - `ipc.md` has the menu rule.
  - `core.md` allows sibling `*_tests.rs` modules.
- **ARCHITECTURE.md:**
  - The real shell, command (17 domains) and core layout, including `menu.rs`.
  - The `src/livespec` contents, without the tabSession copy.
  - 27 primitive families plus `ComboboxList`.
  - Ticket on the generated `jira_*` commands.
  - The new flows: test-start, session-exit, Draft again, workspace save and the bind-before-prompt.
  - A "keyboard chord" row in "Where to change things".
- **`// @ts-nocheck`:** dropped from the generated bindings. I took it out of `HEADER` in `src-tauri/src/specta.rs`
  and regenerated. `tsc --noEmit` is clean: `Channel` is used now that commands take one, so `noUnusedLocals` no
  longer fires. ADR 0001 says so.

## Minor items 9–22: all deferred

The review's triage lists only items 0–8 as must-fix. It names item 9 optional ("if you want it in this round"). I
left it, because a correct fix needs either a new toast event or a way to tell the automatic Done apart from Mark done
(which already toasts).

| # | Deferred item |
|---|---|
| 9 | Automatic Done (`run_action(Advance)` from the git watcher) still logs "Task done" to stderr; no toast in the webview. |
| 10 | The Dock badge reads 0 after a relaunch until some task changes (`needs-you` only from events). |
| 11 | `Notification` hook events don't map to *Permission needed*. |
| 12 | Links bind to the creating container, not the issuing window. Item 1 binds in `run_with`, but `tasks_action` doesn't pass the window label yet. |
| 13 | `prompts::templates::COMMIT` is dead code; the commit helper uses its own prompt. |
| 14 | A Build hand-back can race the 500 ms git debounce for worktree tasks. |
| 15 | Spec §14's "types mirrored by hand" sentence contradicts §15.1. Spec edits are out of scope for this branch. |
| 16 | Settings can't show "Inherit" (no `settings_layers`). |
| 17 | The Rive ring is CSS; the dotLottie files are placeholders; there is no first-run art (assets). |
| 18 | Canvas progress bars are indeterminate (no per-step files in `PlanDoc`). |
| 19 | The launch replay blocks `setup`; the git watcher recurses into `node_modules`/`target`. |
| 20 | Reviewer notes for worktree tasks may land "outside the change set" (folder-name mismatch). |
| 21 | Opening a Commit- or Review-stage page resumes the Claude session even when no prompt follows. |
| 22 | Deny-rule matching for case variants and symlinks of a protected root, checked against the real `claude`: manual test before release. |

## Rulings

- **Bind only when the effects start a session.** `prepare_action` binds when the computed effects contain
  `StartSession`, not on every stage action. That keeps "links are fixed at the first session start" (§4.1): Archive,
  Back to Spec and the like leave an unstarted task unbound. *If wrong:* a first prompt that reaches a task without
  `StartSession` (a resume after relaunch, sent through `InjectPrompt`) is already bound by `open_primary`, so nothing
  is lost.
- **`test-started` event.** I added this event so a Test tab that is already open attaches to a run started from the
  board or the header. Without it the tab ignores runs after its first, because it attaches only when nothing is
  recorded. It is a new entry in the contract's event list. *If wrong:* one more event in the bindings.
- **Duplicate-run window.** `testStore.run` keeps calling `tests_run` alongside `tasks_action(RunTests)`. Both go
  through start-or-attach, so they share one run. A second run starts only if a test command finishes between the two
  calls (a few milliseconds). *If wrong:* a very fast suite runs twice, and the second result is logged as "not
  expected in Commit" after advance-on-green.
- **Session exit leaves Test and Commit alone.** `state_after_exit` changes nothing outside Plan, Build and Review,
  because in Test and Commit the card's state belongs to the test run and the draft.
- **⌘S needs the sidebar focused.** ⌘S is sidebar-scoped, as §4.2 says ("with the sidebar focused"), rather than
  global as the review suggested. The keyboard listener runs in the capture phase and calls `preventDefault`, so a
  global ⌘S would stop CodeMirror's own ⌘S in the diff editor and the memory tab: CodeMirror checks `defaultPrevented`
  (`@codemirror/view` `runHandlers`).
- **New command `tasks_draft_commits`.** Draft again uses a new command instead of a new `StageAction` variant, to
  keep A1's action set unchanged.
- **Standard Edit items kept.** The menu keeps the standard Edit items (⌘C/⌘V/⌘A/⌘X/⌘Z). They aren't in §10.5, and
  without them WKWebView text fields can't copy and paste. ⌘C/⌘V/⌘A are in `TERMINAL_RESERVED`; the menu sends them
  to the webview, as Tauri's default menu did.

## Unverified: needs GUI

- A board drag into Test starts the run, and the Test tab streams it.
- A dead `--resume` turns the card red.
- Draft again after a real provider failure.
- Save on window close.
- ⌘S in the sidebar.
- Every §10.5 chord against the new native menu: ⌘W closes the pane; ⌘`, ⌘N, ⌘T, ⌘D, ⌘K, ⌘P, ⌘B, ⌘E, ⌘J, ⌘1–9 and
  ⌘\ reach the webview; ⌘⇧W closes the window.
- Everything else in the review's "Needs the GUI before release" list.

## For the controller

- After merging, run `npm ci` and `PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install webkit` in the main checkout
  (item 0). Then re-run the gates.
- New public names:
  - Core: `pipeline::{prepare_action, Prepared, on_helper_failed, redraft_commits}`, `sessions::state_after_exit`,
    `SessionRegistry::is_stopping`, `WorkspaceStore::save_if_dirty`, `workspace::import::reopen` (crate-only).
  - Shell: the command `tasks_draft_commits`, the event `test-started`, and the `menu` module.
  - Keymap: `KEYMAP["workspace.save"]` and `Scope = "sidebar"`.
