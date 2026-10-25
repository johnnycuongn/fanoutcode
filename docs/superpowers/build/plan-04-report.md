# Plan 04 report: Sessions and terminals

**Status: DONE_WITH_CONCERNS.** All 16 tasks are done, one commit each, plus one follow-up fix. Every gate passes.
`git rebase main` was a no-op: main is still `82ca780`, Plan 05's last commit.

Concerns:
- Nothing that needs the GUI was exercised: the dev-app checks in Tasks 10, 11, 12 and 15, and the whole spike pass
  bar in Task 16.
- The Background ADR records **No-go**, but by default, not by measurement.
- A few contracts had to bend to what Plans 03 and 05 actually built (Rulings 1–6).

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-04`, branch `plan-04`, not merged.
- Commit range `82ca780..c291dbd`, 17 commits: `0bab2ab` (Task 1) through `7e87b61` (Task 16), then `c291dbd`
  (follow-up fix and ARCHITECTURE.md).
- Spike branch `spike/background-driver` at `c2cdb58`, not merged (see the ADR and Ruling 16).
- Ledger: `.superpowers/sdd/plan-04-progress.md`.

## Gate results (final run on `c291dbd`)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 93 passed (93)`, `Tests 554 passed (554)` (was 90 / 534) |
| `cargo test --workspace` | teamcode-core `317 passed` (was 268), shell `13 passed; 1 ignored` (was 4), livespec-core `240 passed`, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 2.87s`, no chunk-size warning |
| `npm run bindings:check` | exit 0 |
| `cargo fmt --check -p teamcode -p teamcode-core` | clean (not a gate) |
| lefthook pre-commit | ran and passed on every commit; `--no-verify` never used |

Per-module counts. The plan's expected counts all match.
- Core: scrollback 4, batch 2, flow 3, driver 3, registry 13, stop 3, launch 12, inject 4, theme 3, db::sessions 2.
  - Registry is the plan's 9 + 2 + 1, plus one test added with the follow-up fix.
  - db::sessions is extra: the plan assumed Plan 03 had built that module.
- Shell: pty 5 (run three times, stable), session_service 4.
- Webview: terminalTheme 5, terminalStore 9, TerminalPane 6.
- Spike branch only: background 2.

## STUB(plan 04) replaced

`grep -rn "STUB(plan 04)" src src-tauri crates` is now empty.
- `src-tauri/src/effects.rs`, the `StartSession`, `EndSession` and `InjectPrompt` arms: they now collect into
  `session_service::apply_effects`.
- `src/features/views/components/TerminalPane.tsx`: the real pane, same path, same A5 props and `focus()` handle.
- `src/lib/pendingCommands.ts`, `sessionsTerminal`: deleted. `TaskPicker` calls the generated
  `commands.sessionsTerminal`.
- `src/features/settings/types.ts`: `TerminalTheme` and `TerminalPalette` are re-exported from the bindings, and
  `Settings` is now an alias of `ResolvedSettings`.
- `src/features/board/stubs/plan04.ts`: deleted. `board/ipc.ts` calls `commands.sessionsStop` through `unwrap`.

## STUBs added

- `src-tauri/src/session_service/mod.rs`:
  - `protected_deny_rules(&Task) -> Vec<String>` returns `vec![]` (`STUB(plan 10)`, Plan 10's
    `workspace::protected_deny_rules`).
  - `RosterScope.workspace` is `Vec::new()` (`STUB(plan 10)`, workspace-level agents).

## Rulings

1. Ruling: the Claude Code worktree name is Plan 05's `worktree::slug_of(task)` (the folder name without its
   `YYYY-MM-DD-` date), not the whole memory folder name.
   - Why: Plan 05's `CreateWorktree` records `worktree_path = .claude/worktrees/<slug>`, and the git watcher watches
     that path. The spec says `claude --worktree <slug>`. Using the whole folder name would make Claude create a
     different directory from the one the app watches.
   - The Task 7 test expects `--worktree x` for `2026-10-24-x`.
   - Cost if wrong: one function, `launch::worktree_name`.
2. Ruling: the plan's names were adapted to what Plans 01, 03 and 05 actually built.
   - `TaskId::from` → `TaskId::parse`. The invalid-id test deserialises `"../x"` through serde, which does not
     validate.
   - `SessionId::generate()` → `SessionId::new()`.
   - `AgentDef.tools` is `Vec<String>` and has `origin`, so `agents_json` delegates to `roster::to_agents_json`.
   - `hook_specs(&task_id, app_data)`.
   - `status_line_command(log_path, user_status_line(~/.claude/settings.json))`.
   - Cost: none.
3. Ruling: Plan 03 has no `db::sessions`, so I added `crates/teamcode-core/src/db/sessions.rs`, tested: `insert`,
   `get`, `mark_exited`, `mark_all_exited`, `delete`. It uses Plan 01's `sessions` columns. `attached_to` is never
   stored.
4. Ruling: `teamcode_core::Error` now derives `Clone, PartialEq, Eq`. The plan's tests compare errors with
   `assert_eq!`. Cost: none.
5. Ruling: `effects::dispatch` takes the stage action.
   - The new signature is `dispatch(app, state, task, action: Option<&StageAction>, external)`.
   - `commands::tasks` gains `run_with(.., action, compute)`. `run` is `run_with(.., None, ..)`, so Plan 05's callers
     are unchanged. `tasks_action`, `tasks_archive`, `tasks_restore` and `run_action` pass the action.
   - `apply_effects(app, task, action: Option<&StageAction>, effects)` reads `None` (a move, a create, a test
     result) as neutral: never fresh, never an archive.
   - Session effects run last in `dispatch`, so they get the task after `CreateWorktree`.
   - Cost if wrong: two signatures.
6. Ruling: `InjectPrompt` for a task with no live session first opens one, resuming the Claude session.
   - Why: after a relaunch, `task.primary_session` still names the dead session, so Plan 03's `ensure_session` emits
     no `StartSession`. Without this, a stage prompt would go nowhere.
   - The prompt waits in the queue until Claude's `SessionStart` hook marks it idle.
7. Ruling: the terminal channel is `Channel<TerminalBytes>`.
   - `TerminalBytes` is a shell newtype that implements `IpcResponse` as `InvokeResponseBody::Raw`, so frames travel
     as raw bytes instead of JSON arrays. Tauri's blanket impl for `Vec<u8>` would send JSON arrays.
   - Its specta type is `number[]`. `terminalStore.toBytes` accepts `ArrayBuffer`, `Uint8Array` and `number[]`.
   - Unverified (needs GUI): that WKWebView delivers an `ArrayBuffer`.
8. Ruling: the PTY spawner resolves programs on the login shell's PATH (livespec-core's `shell_env`) and passes that
   PATH to the child.
   - Why: an app started from Finder gets a minimal PATH, so `claude` in `~/.local/bin` would not be found.
   - The lookup is warmed in `setup` and before the registry lock is taken.
   - `kill` sends SIGKILL to the pid, because portable-pty's killer only sends SIGHUP.
9. Ruling: the PTY threads are a reader, a flusher and a waiter.
   - The waiter reports the exit at most 500 ms after `wait` returns, so a background job holding the PTY open
     cannot hide the exit.
   - Frames are emitted under the batch lock, so they stay in order.
   - The paused-gate test counts payload bytes, not the total. The terminal turns `\n` into `\r\n`, so the plan's
     "total reaches 5,000,000" is 7,500,000 bytes on the wire.
10. Ruling: `registry.inject` sends at once only when the agent is idle **and** the queue is empty, so prompts keep
    FIFO order.
    - `pump` puts the prompt back at the head if its write fails.
    - `live_for_task` skips a session whose PTY a stop sequence holds, and prefers the newest. This was the follow-up
      fix: without it, Start fresh session's prompt could reach the session being stopped. Its test was written with
      the fix, not before it.
11. Ruling: `SessionExited.code` stays `Option<i32>`, Plan 01's event shape. Plan 04 always sends `Some`.
12. Ruling: the shell code is a directory module, `session_service/{mod,effects,stop,quit}.rs`, re-exported, so
    `session_service::apply_effects` and the other paths match the plan and each file stays under 500 lines.
    `sessions_open` and `sessions_terminal` are async commands that run on `spawn_blocking`, off the main thread.
13. Ruling: the quit path does not use `blocking_show`, because Tauri forbids it on the main thread.
    - It holds the exit, shows the dialog with `show(callback)`, and on Quit runs `stop_all_blocking` on a thread,
      then `app.exit(0)`.
    - A static `QUITTING` flag and `code.is_some()` let that exit through.
    - With live but idle sessions it skips the dialog and still stops them gracefully.
14. Ruling: `useAppTheme()` is new in `src/lib/theme.ts`. Plan 01 never built it. It is a `useSyncExternalStore`
    over `data-theme` on `<html>`.
15. Ruling: xterm, its addons and its CSS.
    - The `@xterm/*` packages were installed in Task 13, because `terminalTheme.ts` imports xterm's `ITheme` type.
    - `useXterm` loads xterm and its addons with dynamic imports. Statically, they grew the shell chunk from 405 kB
      to 895 kB and brought back the chunk-size warning. Now xterm, webgl and canvas are separate chunks.
    - `[data-term-theme]` paints `--term-bg` in `styles.css`.
    - The placeholder gets "Bring back" from a `broughtAway` flag in `terminalStore`. React Compiler lint forbids
      setState in an effect.
    - Tests use the repo's `mockBindings()` and mock `@tauri-apps/api/core`'s `Channel` (Plan 06's Ruling 1).
16. Ruling: the spike branch `spike/background-driver` is **kept**, though the plan says to delete it on No-go.
    - Why: this No-go was not measured, and the branch lets someone run the table without rewriting the driver.
    - On it, `ExternalSession` uses per-field `serde(rename)` for Claude Code's camelCase JSON, not `rename_all`.
    - The ADR was committed on `plan-04`, not main, so the controller's merge carries it.
17. Ruling: `TerminalPlaceholder` shows "Open in <window title>" with a "Show window" button. The button calls
    Plan 01's `commands.windowsFocus`, not `WebviewWindow.getByLabel().setFocus()`, which would need a raw Tauri API
    import.
18. Ruling: Task 12's red step was the missing `insert_second` fixture, because `SessionRegistry::working` was
    already written in Task 4, where the plan's interface lists it.
19. Ruling: the commit trailers name `Claude Opus 5.5`, the model doing the work.
20. Ruling: I kept a commit helper script in `TeamCode-wt/scratch-plan04/`, outside the worktree, and deleted it at
    the end. Nothing else was written outside the worktree except the ledger and this report.

## Background spike (Task 16)

- `docs/adr/0004-background-session-driver.md` has 7 rows reading `Observed: not measured (needs GUI)` and the line
  `**Decision:** No-go (the default): OwnedDriver is the only driver for this release.`
- I measured nothing. I did not start a real `claude --bg` session, because that runs a paid, unsupervised agent.
- I did read `claude agents --json` without `--bg`. Its rows have `id, name, cwd, kind (background|interactive),
  status, state, waitingFor, sessionId, pid, startedAt (ms)`, not the plan's `started`. A reconcile must filter
  `kind == "background"`.

## Unverified (needs GUI)

- Task 10, Step 4: that the channel delivers an `ArrayBuffer` in WKWebView.
- Task 11, Step 4: in the running app, a Reply queued while the agent works is sent on `Stop`.
- Task 12, Step 4: the ⌘Q dialog, Cancel and Quit (exits within about 5 s), and `--resume <id>` in `ps` after a
  relaunch.
- Task 15, Step 5:
  - a 50 MB `yes` burst without freezing, with the webview under one core;
  - ⌘B mid-stream, then a full repaint;
  - two windows with Open in / Bring here / Bring back;
  - the theme switching live, with the WebGL and canvas renderers in WKWebView.
- Task 16: the whole pass bar.
- The window-`Destroyed` detach, the Codex ticker, the codex launch, and every event emission were compiled, but
  none ran in a window.

## Concerns for the controller and the next plans

- **`Effect::RunTest` is still `STUB(plan 05)` in `effects.rs`** (Plan 05's arm, not mine; its report says it was
  filled). Tests start only when the webview calls `tests_run` (start-or-attach). So entering Test, by drag or
  Advance, starts nothing until something opens the Test tab.
- Plan 08 must call `sessions_open(task_id)` when a task page shows its terminal. Nothing in the UI opens a task's
  session yet; the TaskPicker opens only plain terminals. When `primary_session` changes (Start fresh session), the
  page should remount `TerminalPane` with the new id.
- Cancelling the quit dialog after the last window closed leaves the app running with no window. Plan 01's lifecycle
  has no reopen handler.
- A Claude session id that `--resume` no longer knows makes `claude` exit at once. The pane shows "Session ended
  (exit N)". There is no automatic retry with Start fresh session.
- Scrollback size: new sessions read `scrollback_lines` when they start. The open xterm also takes the setting live.
  The registry ring of a running session keeps its old size.
- The main-thread commands (`sessions_attach`, `sessions_write`, …) take the registry lock that the PTY threads take
  for each frame. They hold it only for in-memory work, but a write to a child that has stopped reading its input
  could block until the PTY buffer drains.

## Plan text that is wrong (not edited)

- Task 7: `--worktree <memory folder name>` conflicts with Plan 05's slug and the spec (Ruling 1). Also,
  `AgentDef.tools` is not `Option`.
- Task 9: the paused-gate total of 5,000,000 bytes ignores `\r\n` (Ruling 9). Also, `cargo test -p teamcode pty::`
  needs portable-pty's SIGKILL caveat: its killer sends SIGHUP.
- Task 8: `cargo test … sessions::inject sessions::registry` passes two filters, which cargo rejects. I ran them
  separately.
- Task 10: the plan assumes `db::sessions` exists (Ruling 3). The contract's `Channel<Vec<u8>>` would be JSON
  arrays (Ruling 7).
- Task 12: `blocking_show` on the main thread (Ruling 13).
- Task 16: the `claude agents --json` field names. The plan also says to delete the spike branch on No-go
  (Ruling 16).
- "Names assumed from other plans": `useAppTheme` (Plan 01) did not exist. `SessionId::generate` and `TaskId::from`
  do not exist.

## What the next plans need to know

- **Webview** (`@/features/views` exports):
  - `TerminalPane({ sessionId, paneId, focused? })` with `ref.focus()`, `TerminalPlaceholder`, `terminalStore`,
    `useTerminal`, and `startTerminalEvents`. The pane calls `startTerminalEvents` itself.
  - `TerminalState.status` is one of `detached | attaching | attached | elsewhere | exited`, plus `pending`,
    `exitCode` and `broughtAway`.
- **Commands**:
  - `sessionsOpen(taskId)` and `sessionsTerminal(projectId)` return the session id.
  - `sessionsAttach` and `sessionsBring` take `(sessionId, windowId, paneId, Channel)` and return `Scrollback`.
  - Also: `sessionsDetach`, `sessionsWrite(id, number[])`, `sessionsResize`, `sessionsAck(id, bytes)`,
    `sessionsGet`, `sessionsList`, and `sessionsStop` (graceful; `session-exited` follows).
- **Events**: `session-attached`, `session-exited {session_id, code}`, and `session-queue-changed {session_id,
  pending}`.
- **Plan 08**:
  - The Reply button goes through `tasks_action(Reply { prompt })`. It is sent when the agent is idle, otherwise
    queued, and the pane shows "N prompts will send when idle".
  - ⌃` uses the `TerminalPane` ref.
- **Plan 10**:
  - Replace the two `STUB(plan 10)` items in `session_service/mod.rs`.
  - `GeneratedSettings.permission_deny` carries the deny rules. Your guard hook goes in `hooks` as a `HookSpec`.
  - The settings file is `<app_data>/claude-settings/<task-id>.json` (mode 0600, written atomically) and is
    rewritten at every launch.
