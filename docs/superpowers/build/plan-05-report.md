# Plan 05 report: Observation (hooks, git, helpers, cost)

**Status: DONE_WITH_CONCERNS.** All 19 tasks are complete, plus one docs commit. The branch is rebased on main
(`ebd4319`, which now has Plans 06 and 07) and every gate passes. The concerns are below: nothing in the shell has run
against a GUI, two plan mechanisms had to change to fit what Plans 01 and 03 built (Rulings 4, 13 and 20), and toasts
from transitions the app makes on its own have no window to reach (Ruling 6).

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-05`, branch `plan-05`, not merged.
- Commit range after the rebase: `d259271..82ca780`, 20 commits on top of main `ebd4319` (one per task, then the
  ARCHITECTURE.md commit). The ledger's per-task SHAs are from before the rebase; its last line gives the range after
  it.
- Ledger: `.superpowers/sdd/plan-05-progress.md`.

## Gate results (final run on `82ca780`, after the rebase)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | exit 0; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 90 passed (90)`, `Tests 534 passed (534)` (Plans 02, 06 and 07 suites included) |
| `cargo test --workspace` | teamcode-core `268 passed` (Plan 03 left 132), livespec-core `240 passed`, teamcode shell `4 passed; 1 ignored`, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 2.86s` |
| `npm run bindings:check` | exit 0 (bindings current) |
| `cargo fmt --check -p teamcode-core -p teamcode` | clean (not a gate) |
| lefthook pre-commit | ran on every commit and passed |

Per-module test counts, with the plan's expected count in brackets where the two differ:
- hooks: specs 8 [7], parse 11, plan_steps 8, attribution 5, feed/artifact/cursor/subscribers 9.
- git 11; worktree 9 [7]; changes 11 [9].
- testrun 10; helpers 2; naming and briefs 5; spec_watcher 6; commit_messages 5; livespec_level 5; livespec_run 8 [7].
- cost 14 [13]; activity 7; timeline spec kind 2.

Every extra test is an addition. None of the plan's tests was dropped or weakened.

## Rulings

1. Ruling: the plan's tests call `TaskId::new("t-1")`, but Plan 01's `new()` takes no arguments, so the tests use
   `TaskId::parse`. Cost if wrong: none.
2. Ruling: in Task 1 I wrote the tests and the code in one pass, so I never saw Task 1's tests fail. From Task 2 on,
   each task's failing compile was run and seen before the code was written. Cost: Task 1 had no red step.
3. Ruling: `watch_filter` ignores `*.lock` only inside `.git`, because `Cargo.lock` and `package-lock.json` are real
   changes. I added `git::git_dir_hit`. The watcher also watches a worktree's own git directory and the common
   `refs`, which sit outside the worktree, so a commit made inside a worktree is noticed. Cost if wrong: extra
   refreshes.
4. Ruling: Done detection checks `changes::done_paths`, which is every attributed path plus the files in the task's
   last cached change set. The plan checked "repos with a non-empty `files` list", but a committed file drops out of
   `git status`, so after a commit every list is empty and that check passes for nothing. Auto-Done needs at least one
   such path. Cost if wrong: a task with no attributions and no cached set never moves to Done on its own; the
   developer drags it.
5. Ruling: the plan's `GitWatcher::watch(task_id, repos)` is `git_watcher::watch(&app, &task_id).await`. It works out
   the repos itself and does nothing when they have not changed. Plan 04 calls it after the launch. Cost: a different
   call shape for Plan 04.
6. Ruling: when the app takes a stage action itself (Done detection, a test result), the toasts are logged, because
   the contracts have no toast event. `commands::tasks::run` and `run_action` are now `pub(crate)`. Cost: the "Task
   done" toast is not shown after an automatic Done. Fix: add an event, or let Plan 06 react to `task-changed`.
7. Ruling: `HookEvent::PreToolUse` carries no `tool_use_id`, and `SubagentStop` carries no transcript path. The
   tailer reads both from the raw payload of the log line, and the contract shapes are unchanged.
8. Ruling: only main-agent events and `PermissionRequest` (from any agent) drive the task's chip. During the launch
   replay no Bash-window git probes run, because today's status says nothing about a call that ended long ago.
9. Ruling: `Effect::CreateWorktree` is implemented; it was a `STUB(plan 05)` arm.
   - The settings come from the new `worktree::settings_for`. When `worktree_convention` is unset the convention is
     Team Code-managed. A relative `worktrees_dir` resolves against the root.
   - Under Team Code-managed, `worktree::prepare` runs `git worktree add` on its own thread, so it is safe inside or
     outside a runtime.
   - Under Claude Code's own convention, only the path `.claude/worktrees/<slug>` is recorded.
   - Cost if wrong: the default convention.
10. Ruling: `effects::dispatch` passes the updated task to the arms after `CreateWorktree`. `tasks.rs` `finish` now
    re-reads the row after dispatch, so `task-changed` carries the new worktree path.
11. Ruling: the base is recorded when a task enters Build with an empty `base` (`observation::on_task_applied`). Each
    repo whose `HEAD` resolves gets one.
12. Ruling: the same hook keeps every non-archived task's repos watched and unwatches a task when it is archived. At
    launch every non-archived task is watched.
13. Ruling: Fix failing tests uses Plan 03's layered Test prompt template. It is fed by `TestResult::as_test_run`
    through `AppState::last_test`, which is no longer a stub. The plan's `TransitionCtx.fix_tests_prompt` does not
    exist; Plan 03 chose `StagePrompts.fix_tests`. `testrun::fix_prompt` exists and is tested but is not wired. Cost:
    a project's Test prompt override applies, and linked commands are joined with ` ; ` on the template's Command line.
14. Ruling: the `AppState` field is `last_tests`, plural, because `last_test()` is Plan 03's method name.
    `tests_stop` applies the result as exit code -1, so the card shows Tests failed.
15. Ruling: `provider.rs` is written against livespec-core's API, because LiveSpec's shell file was not on this
    machine to copy.
    - Team Code's `model_provider` picks the provider kind. Models, efforts, agent counts and Jev switches come from
      LiveSpec's own `Settings` at `<app_config>/livespec.json`, with defaults when the file is missing.
    - Keychain accounts are `anthropic-api-key` and `typesafe-api-key`. The TypeSafe key also comes from the
      `TYPESAFE_API_KEY` environment variable.
    - `provider::resolve`, `jev` and `typesafe_key_present` are `async`, because keychain reads block.
    - Cost if wrong: Plan 06's settings screens must write `livespec.json` for model choices.
16. Ruling: the naming helper renames a task only while its title is still Plan 03's `title_from_intent(intent)` (or
    `naming::fallback`). Plan 03's derived title is not the plan's six-word fallback. If no answer is accepted, the
    title is not changed. `naming::run_with_usage` and `spec_watcher::run_with_usage` exist so that helper usage
    reaches the task's cost.
17. Ruling: Plan 03's `build_prompt_ctx` now fills `LinkedProject.brief` from `briefs::cached`
    (`<app_data>/briefs`).
18. Ruling: the spec watcher has a new `TimelineKind::Spec { reason }`. It renders as `· spec: <reason>` in
    `index.md`, and the db stores `{"reason"}`.
    - The event struct is `events::SpecWatcher`, so the event name is `spec-watcher`; the core payload type is
      `SpecWatcherEvent`.
    - The app writes `spec.md` only if the file is unchanged since the watcher read it.
    - The diffs show untracked files whole.
    - Changes in the tree during Review start it after 2 s through `helpers::run_after`; a newer change replaces the
      pending run.
19. Ruling: commit drafts use the plan's single-turn `COMMIT_SYSTEM` block format, not Plan 03's
    `prompts::templates::COMMIT`. That template is agentic, has a different format, and writes the file itself.
    - Added the command `git_drafts(task_id) -> Vec<RepoCommits>`.
    - `commit_files` commits with a pathspec, so other staged changes are not swept in.
    - `HelperKind::GitCommit` (the pipeline's Commit action) commits every draft in order through the same guarded
      `commit_one`.
20. Ruling: Plan 01's `cost_buckets.hour` column is `TEXT`. It stores the integer hour as a decimal string, and
    queries read it with `CAST`. The stage is stored as the lower-case stage word. Every breakdown is ordered by key,
    ascending.
21. Ruling: Plan 01's test `a_gap_in_versions_still_applies_the_later_file` used version 5, which `0005_observation`
    now takes. It uses the last version plus 2.
22. Ruling: `hooks_tailer::subscribe(f: HookSubscriber)` takes the type alias. It is the same type as
    `Box<dyn Fn(&TaskId, &HookEvent) + Send + Sync>`, and clippy's `type_complexity` rejects the spelled-out form.
23. Ruling: `specta.rs` imports `commands::tests` as `test_runs`, because `tests` collides with its own test module.
24. Ruling: dependencies added.
    - teamcode-core: `tokio`, specta's `serde_json` feature; dev-dependencies `tokio` with `net` and `reqwest` (the
      same version and features as livespec-core).
    - shell: `notify = "8"`, `livespec-core`, `reqwest`.
    - Added `From<livespec_core::Error> for Error`.
25. Ruling: the commit script did not stop on a clippy error, so Task 15 first landed with a test lint. I amended
    that commit, my own and unpushed, before going on, and the script now stops on clippy failures.
26. Ruling: these additions back the shell and are tested where they hold logic: `changes::{repos_of, done_paths}`,
    `db::tasks::list_all`, `worktree::{settings_for, prepare, default_worktrees_dir}`, `cost::ledger::{ingest,
    register_transcript}`, `Attributor::{with_files, repos, open_windows}`, `testrun::TestResult::as_test_run`, and
    `hooks::parse::normalize_agent_id`.

## STUBs

I added none. Every `STUB(plan 05)` that Plan 03 left is filled:
- In `effects.rs`, the `CreateWorktree`, `RunTest` and `RunHelper` arms.
- `AppState::last_test`.

`StartSession`, `EndSession` and `InjectPrompt` are still `STUB(plan 04)`.

## Unverified (needs GUI)

These are compiled, clippy-clean and exported, but they have never run in the app:
- the `notify` watchers (FSEvents) and the event-log tailer, including the launch replay, which runs through
  `block_on` in `setup`;
- event emission to windows;
- every new command invoked from the webview;
- keychain reads;
- the Local model server;
- real `claude -p` helper calls.

All the logic behind them is unit-tested in `teamcode-core`, against fixtures, temporary git repos and a local SSE
server.

## Concerns

- Toasts from transitions the app makes itself are lost (Ruling 6).
- The launch replay blocks `setup` until every log is read from its cursor. A very long log written while the app was
  closed delays the first window.
- The watcher reacts to every change in a watched repo, `node_modules` and `target` included. Ignored files trigger a
  cheap `git status` refresh. A path filter would cut this.
- The launch replay restores the last `StatusLine` as live cost. That figure may be stale until the next turn.

## What the next plans need to know

- **Plan 04**
  - Hook types and helpers: `teamcode_core::hooks::{HookSpec, HookEvent, TaskEvent, hook_specs, hook_command,
    status_line_command, user_status_line, sh_single_quote, event_log_path}`.
  - The status line is `{type: "command", command: status_line_command(log, user_status_line(user settings json))}`.
  - Register `note_hook_event` with `teamcode_lib::hooks_tailer::subscribe(Box::new(..))` in `setup` before
    `HooksTailer::start`. A comment in `lib.rs` marks the line.
  - After a launch, call `git_watcher::watch(&app, &task_id).await`.
  - Use `worktree::settings_for(conn, task, root)` with `worktree::launch_plan`.
  - Plan 05 already runs `CreateWorktree`, so your `StartSession` arm gets the task with its worktree path.
  - The tailer sets `claude_session_id` on `SessionStart`.
- **Plans 06 and 07**: delete the `STUB(plan 05)` wrappers `costSummary` and `gitChanges` in
  `src/lib/pendingCommands.ts`. The generated bindings have the same names and shapes (`RepoChanges` and
  `ChangedFile` carry more fields). Board cards read `tasksActivity(container)`. The live events are `team-event`,
  `git-changes`, `cost-updated`, `test-finished` and `spec-watcher`.
- **Plan 08**
  - git: `gitChanges`, `gitDiff`, `gitFileRead`, `gitFileWrite`, `gitIncludeOther`, `gitDrafts`,
    `gitCommit(taskId, repo, index)`.
  - tests: `testsRun(taskId, Channel<number[]>)` (start-or-attach with replay), `testsStop`,
    `testsAddLinked(taskId, projectId) -> TestCommand`.
  - LiveSpec: `livespecAnalyze(taskId, Channel<LiveSpecEvent>)` and `livespecCancel`. A project with LiveSpec Off gets
    `Precondition("LiveSpec is off for this task.")`.
  - The Memory tab shows `· spec: <reason>` timeline lines.
- **Plan 09**: `hooksList` fills in `SubagentStart.description`; `hooksPlanSteps` returns a `PlanDoc`. The `by_agent`
  keys are agent ids, `main` and `helpers`.
- **Plan 10**: `RunHelper(ProjectBrief)` builds the briefs of a task's linked roots through `briefs::ensure`. Put
  your guard hook in `HookSpec`. The secret accounts are in `provider.rs`.
