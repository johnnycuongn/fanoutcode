# Final review — Team Code on `main` (205 commits, e9449db)

Read-only review of the whole after all ten plans and the stub cleanup merged. Inputs: the approved spec, the
contracts (A1–A20), `CLAUDE.md`, `.claude/rules/`, every plan report and ledger, and the code itself. The main flows
were traced end to end through the Rust and TypeScript (not through the unit tests, which mock the other side).

Gates re-run on `main` in this checkout: `cargo test --workspace` 449 + 13 + 240 pass, clippy clean,
`npm run lint` clean, `npm run bindings:check` clean. `npm run typecheck` and `npx vitest run` **fail here only because
this checkout's `node_modules` is stale** (item 0); the code itself is not at fault. `npm run e2e` was not run
(WebKit is not in this checkout's `node_modules`).

Counts: **0 Critical, 8 Important, 14 Minor.** Work the list top to bottom; items 0–8 must land before this is called
done, the rest can follow.

---

## 0. Important (environment, do first) — `main`'s `node_modules` is stale; typecheck, vitest and the pre-commit hook fail

- **Where:** `/Users/johnnynguyen/Personal/TeamCode/node_modules` (281 lockfile packages missing: `motion`,
  `@xterm/*`, `@xyflow/react`, `elkjs`, `@codemirror/merge`, `@dnd-kit/*`, `vaul`, `sonner`, `@playwright/test`,
  `@testing-library/user-event`, …).
- **What:** every plan ran `npm install` in its own worktree; the main checkout's `node_modules` dates from Plan 01.
  `tsc --noEmit` reports hundreds of `Cannot find module` errors and all 165 vitest files fail to load, so lefthook
  refuses every commit on `main` (`npm run typecheck` is a pre-commit job).
- **Fix:** `npm install` in `/Users/johnnynguyen/Personal/TeamCode`, then
  `PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install webkit`, then re-run all gates including `npm run e2e` and record
  the result. Nothing to change in the repository.

## 1. Important — The first Plan prompt of a workspace task has no Linked projects block (prompts are assembled before the links bind)

- **Where:** `src-tauri/src/commands/tasks.rs:71-92` (`run_with` builds `TransitionCtx`, which assembles every stage
  prompt from the task row) versus `src-tauri/src/session_service/mod.rs:46`
  (`linking::bind_if_first_start` runs inside `open_primary`, on the effects thread, after `apply`).
  `crates/teamcode-core/src/pipeline/ctx.rs:50-58` builds `ctx.linked` from `task.linked_roots`, which is still
  empty at that moment; `prompts/mod.rs:90-99` therefore renders `(This task has no linked projects.)`.
- **Why it matters:** spec §4.3 and §6.8 say every stage prompt carries the Linked projects section (names, roots,
  briefs, protection). For the first stage action of every workspace task (Approve spec → Plan, or a drag into
  Plan/Build) the agent gets `--add-dir` for the linked roots but is told it has none, and the `(protected: read
  only)` marks are absent. Later stage prompts are right, because the task is bound by then. Plan 10's test
  `prompts_carry_policy_protection_references_and_stage_instructions` binds first and then builds the prompt, so it
  passes while the live order is the reverse.
- **Smallest fix:** in `run_with`, inside the lock and before `db::tasks::require`, call
  `teamcode_core::workspace::launch::bind_if_first_start(&conn, &state.data_dir, &id, None)` (a no-op once bound;
  binds nothing for a project-window task) and, when it returns `needs_briefs`, start `HelperKind::ProjectBrief`
  after the lock is released. `open_primary` keeps its own call as the belt. This also matches §4.1's wording ("the
  window that issues the first stage action decides").

## 2. Important — Dragging a card into Test from the board never runs the tests; the chip pulses "working" forever

- **Where:** `src-tauri/src/effects.rs:40` (`Effect::RunTest => {}`); `crates/teamcode-core/src/pipeline/enter.rs:39-47`
  (entering Test emits `RunTest` then `SetState(Working)`); `src/features/dock/components/TestTab.tsx:22-24` is the
  only caller of `tests_run`, and it mounts only when a task page shows the Test tab.
- **Why it matters:** §6 lists the board drag as one of three equivalent ways to move a card; §6.5 says the command
  runs on entry. Without the task page open, the task sits in Test as *working*, the primary button reads "Running
  tests…" (disabled, `primaryAction.ts:71`), and nothing happens until someone opens the page. Plan 04 flagged this;
  the cleanup documented it as a no-op instead of closing it.
- **Smallest fix:** `Effect::RunTest => { observe::log_err("starting the tests", crate::commands::tests::start_or_attach(app, &task.id)); }`.
  `start_or_attach` (`commands/tests.rs:41`) is already start-or-attach with a replay buffer, so the Test tab that
  opens later attaches to the run in flight; `finished` already applies `on_test_finished`.

## 3. Important — A session that exits leaves the card "working" (dead `--resume`, missing `claude`, crash)

- **Where:** `src-tauri/src/session_service/mod.rs:129-146` (`exited` marks the registry and the row, emits
  `session-exited`, and never touches the task's `AgentState`).
- **Why it matters:** after a relaunch every Plan..Commit page calls `sessions_open`, which resumes with
  `--resume <id>` (`sessions/launch.rs:172`). When Claude Code no longer knows that id, or `claude` is not on the
  login PATH, the process exits at once; the pane says "Session ended (exit N)" but the board chip stays green and
  pulsing, the attention strip is silent, and queued prompts are dropped (`registry.on_exit` clears the queue).
  Plan 04 listed this under concerns; nobody owned it.
- **Smallest fix:** in `exited`, when `session.task_id` is `Some` and it was the task's live primary session, call
  `hook_feed::set_state(app, &state, &task_id, …)` with `NeedsYou { reason: Error { message: "Session ended (exit N)" } }`
  for a non-zero code and `Idle` for zero (`set_state` already skips archived tasks and no-ops on an equal state).
  Take the registry lock only for the lookup, release it, then call `set_state`.

## 4. Important — A failed commit-message draft leaves the Commit card "working" with no way to retry

- **Where:** `crates/teamcode-core/src/pipeline/enter.rs:49-53` (entering Commit sets `Working` and runs
  `CommitMessages`); `src-tauri/src/helpers.rs:36-55` (`run_after` only logs a helper's error);
  `src-tauri/src/helpers.rs:156-196` (`commit_messages` sets *Message drafted* only on success);
  `src/features/dock/components/CommitTab.tsx` offers Copy and Commit, no re-draft.
- **Why it matters:** the drafting is a model call (`claude -p` or the API). On any failure — rate limit, no network,
  no key — the task stays `Working` indefinitely (so it never appears under *needs you*), `commits.md` is never
  written, and the Commit tab stays empty. The developer can still press "Mark done", but the chip, the attention
  strip and the Dock badge all lie.
- **Smallest fix:** in `helpers::run_after`'s spawned job, on `Err(e)` for `HelperKind::CommitMessages` (and
  `SpecWatcher` if you like) call `hook_feed::set_state(.., NeedsYou { reason: Error { message: e.to_string() } })`
  when the task is still in that stage. Optionally give the Commit tab a "Draft again" button that calls
  `tasks_action(Back { stage: Commit })`-free re-entry: `helpers::run(app, &id, HelperKind::CommitMessages)` behind a
  tiny command.

## 5. Important — Workspace files saved outside app data: no ⌘S, no save on close, and reopening a dirty workspace silently reverts it to the file

- **Where:** `src/lib/keyboard.ts:29-62` (no save chord in `KEYMAP`); `src-tauri/src/windows.rs:21-31`
  (`CloseRequested` saves the window list only); `crates/teamcode-core/src/workspace/import.rs` `open_file`
  (`Some(record) if adds_nothing(..)` → `open::import` → `store.replace_from_file` → `set_file_clean`), reached from
  `windows_open_workspace` via the start screen's recents and `openWorkspaceFile` (`importStore.ts:50-60`).
- **Why it matters:** spec §4.2 says an exported file is written "only on an explicit save (⌘S with the sidebar
  focused, or on closing the window)" and shows a dirty dot meanwhile. Today the only save is the button in
  `WorkspaceFileSection`. Worse: a developer who closes the window with unsaved changes and later opens the
  workspace from the start screen gets `open_file`, which re-imports the stale file over the record (the file is a
  subset of the record, so `adds_nothing` passes) and clears `dirty` — the unsaved policy/fragment/agent edits are
  gone without a prompt. `open::reload` guards this case; `open_file` does not. Plan 10's report lists ⌘S and
  save-on-close as "still open" Plan 06 obligations.
- **Smallest fix:** (a) in `open_file`, `Some(record) if record.dirty => Ok(record)` before the `adds_nothing` arm (the
  record is the truth, the file its mirror); (b) in `windows::on_window_event` `CloseRequested`, for a Workspace
  container with `dirty` call `state.workspaces.save(&id)` and log a failure; (c) add `"workspace.save": { chords:
  ["⌘S"], scope: "global" }` to `KEYMAP` and register it in `useShellKeys` to `workspaceStore.save()` when the window
  shows a workspace.

## 6. Important — ⌘W (and possibly ⌘H/⌘M) is claimed by Tauri's default macOS menu, not by the app's keymap

- **Where:** `src-tauri/src/lib.rs` sets no menu, so Tauri 2 installs `Menu::default()` on macOS, whose File/Window
  menus bind **Close Window ⌘W** (and ⌘H, ⌘M); `src/lib/keyboard.ts:47` maps ⌘W to `pane.close`. The contracts and
  `ARCHITECTURE.md` list a `src-tauri/src/menu.rs` that does not exist; Plan 06's report (Concern 6) warned that a
  menu must not bind ⌘W/⌘N/⌘T/⌘D/⌘K/⌘P/⌘B/⌘E/⌘J/⌘1–9/⌘\.
- **Why it matters:** menu key equivalents are resolved by AppKit before the webview sees the keydown in the usual
  configuration, so ⌘W would close the whole window instead of the focused pane (sessions survive, but every pane in
  the window is gone). This is a GUI-verification item (no plan ran the app), but the gap is real either way: the
  spec's keymap depends on the app owning its menu.
- **Smallest fix:** add `src-tauri/src/menu.rs` building the app menu explicitly (App, Edit, View, Window) with Close
  Window on ⌘⇧W and without ⌘N/⌘T/⌘D/⌘K/⌘P/⌘B/⌘E/⌘J/⌘1–9/⌘\; set it with `.menu(..)` in `lib.rs`. Then verify every
  §10.5 chord in `npm run tauri dev`.

## 7. Important — Two Rust modules are over the 500-line budget the owner set (spec §15.1: "split, not excused")

- **Where:** `crates/teamcode-core/src/workspace/import.rs` (600 lines),
  `crates/teamcode-core/src/workspace/context_fetch.rs` (563 lines).
- **Why it matters:** the budget is an explicit engineering requirement and `.claude/rules/core.md` repeats it. Both
  files grew during the ten security-fix rounds. Every other file is within budget (largest component
  `TaskPage.tsx` 199, largest store `canvas/store.ts` 226).
- **Smallest fix:** move `import.rs`'s `#[cfg(test)] mod tests` (≈240 lines) to `workspace/import_tests.rs` with
  `#[path]`, as `store_tests.rs` and `guard/tests.rs` already do; move `context_fetch.rs`'s HTML-to-text and
  streaming-cap helpers (`html_to_text`, the chunked reader) into `workspace/context_text.rs`.

## 8. Important — The rules and `CLAUDE.md` have drifted from the code they describe (spec §15.2)

- `CLAUDE.md:95-97` still says "Current stage: Plan 01 Foundation complete; Plan 02 UI primitives next." All ten plans
  are merged. Write the real state (all plans merged; GUI verification and item list above pending).
- `.claude/rules/prompts.md` says "Placeholders are exactly these six; any other `{{…}}` is a bug", but the shipped
  `TEST` template uses `{{test.exit_code}}` and `{{test.output}}` (`prompts/templates.rs:1-4, 117-136`;
  `prompts/mod.rs:124-131` substitutes them). Add the two to the rule.
- `.claude/rules/ui.md` says "never import from `@tauri-apps/api/core` or `@tauri-apps/api/event`", but the lint rule
  (`eslint/rules/noManualIpc.js`) deliberately allows `import { Channel } from "@tauri-apps/api/core"`, used in
  three stores, and `@tauri-apps/api/window|webview` are imported in four files. State the rule as the lint enforces
  it.
- `ARCHITECTURE.md` "Repository layout" lists `src-tauri/src/menu.rs` (absent), `commands/{projects,…,cost}.rs`
  (there are 18 command files), and `src/livespec/ … tabSession logic` (never copied, by Plan 08 Ruling 1); "The task
  page and dock" still says Ticket "goes through `dock/jira.ts` until Plan 10's bindings land" (deleted in the
  cleanup); "UI primitives" says twenty-six (there are 27 families plus `ComboboxList`). Bring the map up to date.
- `docs/adr/0001-generated-bindings.md:32` and `src/lib/bindings.ts:2`: the `// @ts-nocheck` was justified "until a
  command takes a Channel"; several do now (`sessions_attach`, `tests_run`, `livespec_analyze`). Drop the line (or
  teach the generator to omit the unused import) so errors inside the generated file are reported again.

---

## Minor

9. **Automatic Done shows no toast.** Done detection (`git_watcher.rs:on_settled` → `run_action(Advance)`) logs the
   "Task done" toast to stderr (Plan 05 Ruling 6); spec §6.6 wants a toast. Fix: in the webview, toast on a
   `task-changed` whose stage became `Done` (`tasksStore.applyTaskChanged`), or add a `toast` event.
10. **Dock badge reads 0 after a relaunch** until some task changes (`tasksStore.needsYouByContainer` fills only from
    `needs-you` events; Plan 06 Concern 4). Fix: emit `needs-you` for the window's container in `windows_current`, or
    derive the count from `tasks_list`.
11. **`Notification` hook events never move the chip** (`hooks/feed.rs:14-32`); spec §7.3 lists them beside
    `PermissionRequest` for *Permission needed*. Low impact while `PermissionRequest` fires; map
    `Notification { message }` containing a permission prompt to `PermissionNeeded`.
12. **Links bind to the creating container, not the issuing window** (`linking.rs:18-37`, Plan 10 Task 15 ruling):
    a task created in a workspace window but first started from a project window still binds to the workspace.
    Spec §4.1 says the issuing window decides. Acceptable for v1; if item 1's fix moves binding into `run_with`, pass
    the calling `Window` label through `tasks_action` and the deviation disappears.
13. **`prompts::templates::COMMIT` is dead code**: Plan 05's helper uses its own single-turn block format
    (`helpers/commit_messages.rs`, Ruling 19). Delete the template or make the helper use it, so there is one Commit
    prompt.
14. **Build hand-back can race the git debounce for worktree tasks** (`hook_feed.rs:files_changed`): a `Stop` with the
    sentinel that arrives inside the 500 ms debounce, with no attributed edits, reads as *Replied*. Fix: on a `Stop`
    in Build, await `git_watcher::refresh` before `artifact_changed`.
15. **Spec §14 contradicts §15.1** ("Types are mirrored by hand … `AGENTS.md`" vs generated bindings). The code follows
    §15.1 and ADR 0001; strike the §14 sentence in the spec.
16. **Settings "Inherit" display** (Plan 06 Concern 5): `ResolvedSettings` cannot say which layer a value came from, so
    a project's Detail level never shows "Inherit". Add a `settings_layers(scope)` command later.
17. **Rive ring is a CSS ring; the dotLottie files are hand-made placeholders; no first-run art** (Plan 02, 07, 09
    rulings). Assets, not code; the swap points are `canvas/components/cards/WorkingRing.tsx` and the two `.lottie`
    files.
18. **Canvas progress bars are indeterminate** because `PlanDoc.steps[]` carries no per-step files (Plan 09 Ruling 1).
    Extend `hooks/plan_steps.rs` to collect the indented file lines under each step; only `canvas/adapters.ts`
    changes.
19. **Launch replay blocks `setup`** until every event log is read from its cursor (`hooks_tailer.rs:HooksTailer::start`,
    `block_on`), and the git watcher recurses into `node_modules`/`target` (Plan 05 concerns). Move the replay to a
    spawned task after the first window, and add an ignore list to `git::watch_filter`.
20. **Reviewer notes for worktree tasks** may land under "outside the change set" because `review.md` paths use the
    project folder name while the home repo is the worktree folder (Plan 08 Ruling 4). Teach the Review prompt to use
    the home project's name for the worktree repo, or match on `repoName(project.root)` first.
21. **Opening a Commit- or Review-stage page resumes the Claude session** (`task-page/body.ts:wantsSession`) even when
    no prompt will be sent. Harmless but spends a process; consider resuming lazily on the first inject.
22. **Deny-rule matching for case variants and symlinks of a protected root is unverified against the installed
    `claude`** (security review, "Considered, not reported"). One manual test before release:
    `Edit` under `/Users/j/W/API/..` with `Edit(//Users/j/w/api/**)` denied.

---

## What held up (no issue found)

- **Hook event log end to end.** `hooks/specs.rs` writes `EventName<TAB>rfc3339<TAB>json` with the A3 buffered
  single write; `GeneratedSettings::to_json` emits Claude Code's `hooks`/`statusLine`/`permissions.deny` shape; the
  tailer (`hooks_tailer.rs`) reads from saved cursors, `parse_line` maps payload fields, `hook_feed::handle` sets the
  chip through `pipeline::apply`, feeds attribution and cost, emits `team-event`, then publishes to Plan 04's
  `note_hook_event`, which pumps the inject queue. Board (`activity.ts`, `chipText`), task page and canvas
  (`canvas/events.ts`) all read the generated `HookEvent` shape.
- **Sentinel and hand-back.** `detect_handback` accepts the sentinel wrapped in `` ` ``/`*`/`_`, rejects it mid-message
  or for the wrong stage, requires the artifact where one exists, and treats running background tasks as working;
  `chip_text` matches A2 and the 35-row fixture mirrors it in `src/lib/format.ts`.
- **Single attach.** `SessionRegistry::attach` refuses a second pane with `attached-elsewhere:<w>:<p>`, `bring` moves
  it, `detach_window` runs on `Destroyed`, `useXterm` detaches on unmount, and the placeholder offers Show window /
  Bring here / Bring back.
- **Settings.** The registry has exactly the 29 A12 keys with spec §13 levels and defaults; `resolve` walks task →
  project → workspace → app → default; `validate` rejects unknown keys, wrong levels and wrong types; `jira` is
  writable only by `jira_link*`.
- **Migrations** 0001, 0003, 0005, 0009, 0010 apply in order on a fresh database (each in its own transaction);
  0010 drops the never-written 0001 `jira_links` cleanly.
- **Workspace open.** `preview_open` → dialog → `confirm_open(token, approved_doc_ids)` → `windows_open_workspace` →
  `open_file` (trusted reopen) works for a freshly confirmed file; `launch_extras` yields `Edit(//root/**)` deny rules
  for both spellings, the `teamcode guard` PreToolUse hook on Bash, and `--add-dir` for linked roots and only the
  task's own context caches.
- **Standards.** No raw controls outside `src/components/ui/` (lint and grep); every feature has an `index.ts` and no
  feature reaches into another's internals (the only exception is the dev-only canvas playground behind an
  explicit `eslint-disable`); no hand-written mirrors of Rust types remain after the cleanup; all components are
  under 200 lines and stores under 300.

---

## Triage of the plans' deferred items and rulings

**Must be fixed before "done":** items 0–8 above. Of the reports' own lists, that covers Plan 04's "nothing starts
the tests on Test entry" and "a dead `--resume` leaves the chip", Plan 05's lost toasts (as item 9 if you want it
in this round), Plan 06's missing native menu (item 6), Plan 10's open ⌘S / save-on-close obligations (item 5), the
size-budget scan (item 7) and Plans 01/02/03's untouched `CLAUDE.md` "Current stage" line (item 8).

**Can wait (accepted as recorded):** the Background driver spike ADR 0004 "No-go by default, not measured" — Owned
is the chosen v1 driver and the spike branch is kept; the swatch palette being Plan 01's ten names; `index.md`
front matter and timeline wording; Plan 08's severity mapping (blocker/major/minor/nit → critical/high/medium/low);
Plan 09's CSS ring and lazy elkjs chunk; Plan 10's per-doc approval of imported references and the Jira-token
binding (both tightened the spec, deliberately); the `--agents` roster and `-w/-r` resume flags; Plan 03's ruling
that every stage entry (not just Plan) creates the worktree when the flag is on.

**Needs the GUI before release (never run by any plan):** `npm run tauri dev` end to end: a real `claude` session
with the generated settings (hooks landing in the log, the status line forwarding, the deny rules and guard being
honoured), the ⌘ chords against the native menu (item 6), the WKWebView `ArrayBuffer` channel frames, two windows
with Bring here / Bring back, the quit dialog, dotLottie playback under the CSP, and the keychain prompt for Jira.
