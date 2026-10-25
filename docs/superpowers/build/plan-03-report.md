# Plan 03 report: Core domain and pipeline

**Status: DONE_WITH_CONCERNS.** All 17 tasks are complete, every gate passes, and the branch is rebased on main
(Plan 02 merged, no conflicts). The concerns: the new shell commands are compiled but not exercised without the GUI;
the plan's assumed schema did not match Plan 01's, so the repositories use Plan 01's columns (Ruling 2); and the
effect runners are stubs until Plans 04 and 05 land.

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-03`, branch `plan-03`, not merged.
- Commit range after the rebase: `4f1a67e`..`504bc54`, 18 commits on top of main `5781266`. That is one commit per
  task (Tasks 1–17) plus `504bc54` (ARCHITECTURE.md, and installing the shipped agents at launch).
- Ledger: `.superpowers/sdd/plan-03-progress.md`. Its per-task SHAs are from before the rebase; its last line gives
  the range after it.

## Gate results (final run on `504bc54`, after the rebase)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | exit 0; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 40 passed (40)`, `Tests 224 passed (224)` (Plan 02's suite included) |
| `cargo test --workspace` | teamcode-core `132 passed` (Plan 01 left 26), teamcode shell `4 passed; 1 ignored`, livespec-core `240 passed` (+5 live ignored), 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 925ms` |
| `npm run bindings:check` | exit 0 (bindings current) |
| `cargo fmt --check -p teamcode -p teamcode-core` | clean (not a gate) |
| lefthook pre-commit | ran on every commit; typecheck, lint and test all passed |

Per-module test counts: memory 20, pipeline 44 (the plan says 43; I added one test, Ruling 14), prompts 10, roster 5,
settings 7, views and db::views 6, db 19.

## Rulings

1. Ruling: test ids use Plan 01's prefixes and constructors (`p-1a-1`, `t-1a-1`, `s-1a-1`, `ws-1a-1`, `pane-1`;
   `TaskId::parse(..)`, and `TaskId::new()` instead of `new_id("task")`). The plan's `proj-…`, `task-…` and `sess-…`
   ids fail Plan 01's prefix check. The plan says to prefer Plan 01's names. Cost if wrong: none.
2. Ruling: the repositories use Plan 01's `0001_initial` columns, not the plan's target schema. The `tasks` table
   keeps the full `Task` in `json`, beside the query columns and `state` (JSON AgentState); there is no `state_json`
   or per-field columns. `projects.memory_location` is JSON, not `memory_kind` plus `memory_path`. The timeline uses
   `detail`, not `detail_json`. Views use `"order"` and `layout`, not `ord` and `layout_json`. So `0003_core` only
   adds A11's `state_entered_at` and `links_bound_at` (plus a backfill) and three indexes. The plan's tests that
   name columns (`SELECT state_json`, and the column list in `migrations_apply_twice_without_error`) now use Plan
   01's names. Why: adding parallel columns would duplicate data that is already stored. Cost if wrong: raw SQL
   written against the plan's column names will fail. Plan 10's text says `workspace_projects.ord`, but the real
   column is `"order"`.
3. Ruling: `PROJECT_PALETTE` is `indigo, cyan, lime, orange, fuchsia, yellow, emerald, rose`. The plan's `"sky"` is
   not a swatch in `styles.css`, so it would have no class. Plan 01's `db::find_or_create_project` now goes through
   `projects::find_by_root`, `next_color` and `insert`, which leaves one colour picker (least-used colour). Plan 01's
   colour test now expects indigo then cyan, instead of rose then orange. Cost if wrong: projects get different
   default colours.
4. Ruling: two Plan 01 tests asserted `user_version == 1`. They now compare against `MIGRATIONS.last().0`, because
   adding 0003 broke the hard-coded number. Cost: none.
5. Ruling: the plan's Task 7 test `dragging_into_test_without_a_command_is_refused` expected `fx[1] == RunTest` for
   Build → Test. That move skips Review, so `fx[0]` is the `Skipped` line. The test now asserts the Skipped entry and
   then `[SetStage(Test), RunTest]`. The plan text is wrong here. Cost: none.
6. Ruling: Plan, Build and Review entry, Fix failing tests and Start fresh all share one `ensure_session` step. It
   emits `CreateWorktree` before `StartSession` when the worktree is on and none exists, so a card dragged past Plan
   still gets its worktree. The plan stated this for Plan only. Cost if wrong: a skipped Plan might not want a
   worktree. I see no reason it wouldn't.
7. Ruling: `FixTests` on a Shell task is `Invalid` ("… is not available in Test"). Otherwise the fix prompt would be
   typed into a shell. Cost: none.
8. Ruling: `theme` resolves to Plan 01's `domain::ThemePreference`. I did not define the plan's `values::Theme`
   (same three variants, same JSON). Cost: none; the TS type is `ThemePreference`.
9. Ruling: `settings::validate` always accepts `null`, which means "remove the override at this level"
   (`db::settings::set` deletes the row). Unknown keys, wrong levels and wrong types are still `Invalid`, and
   nothing is stored (Review Focus 5). Why: without this, the settings screens could not reset a non-Option key to
   inherited. Cost if wrong: a caller could clear a key it meant to set. No error results, because the key falls
   back to its inherited value.
10. Ruling: one `settings_table!` macro in `settings/keys.rs` generates the markers, `KEYS`, `ResolvedSettings` and
    `resolve_all`. `ResolvedSettings` is re-exported as `settings::ResolvedSettings` and
    `settings::values::ResolvedSettings`. `KeyInfo` gained `expected: &str` for the error text. Value types are
    passed as `(Type)` token groups, because specta's derive cannot read a macro `ty` fragment. Why: adding a key
    (Plans 04 and 10) is one line, with no second struct to keep in step. Cost: none.
11. Ruling: `prompts::render` substitutes in one pass instead of one `str::replace` per placeholder. The behaviour
    is the same, except that a substituted value (test output, a brief) is never read again for placeholders.
    Cost: none.
12. Ruling: "Workspace references" are rendered inside the `{{linked.projects}}` section, built from
    `ctx.workspace_context`. They are also rendered under `(This task has no linked projects.)`. Why:
    `linked_projects_block(linked, policy)` as specified has no access to them. Cost: none until Plan 10 fills them.
13. Ruling: `apply` reads `index.md` before rewriting it, so a corrupt file is set aside as `index.md.bad` first and
    never overwritten (Review Focus 3). The extra test is `a_corrupt_index_is_set_aside_before_the_rewrite`.
14. Ruling: if the row insert fails, `create_task` removes the folder it just made. A blank given title counts as no
    title, so the title is derived and Naming runs. Cost: none.
15. Ruling: `effects::dispatch` logs with `eprintln!`, as Plan 01 does, not `log::warn!`. The crate has no `log`
    dependency. Each arm is one line marked `// STUB(plan 04)` or `// STUB(plan 05)`.
16. Ruling: `AppState` has no `db()`, `app_data()` or `app_config()` accessors. The commands use Plan 01's fields
    `db`, `data_dir` and `config_dir`. `AppState::last_test(&TaskId)` returns `None // STUB(plan 05)`.
17. Ruling: `src-tauri/src/events.rs` already existed (Plan 01 defined all ten event structs). I added
    `emit_task_changed` and a generic `emit` helper. `projects_close` also takes `State` (it needs the window
    registry) and drops the registry lock before closing the window.
18. Ruling: `memory_pick_location` returns `Result<Option<PathBuf>>`. It uses the dialog's `pick_folder` callback
    with a tokio oneshot. The chosen folder is canonicalised in `projects::update`, which also canonicalises any
    `MemoryLocation::Folder` from `projects_update`.
19. Ruling: added specta's `serde_json` feature in `src-tauri/Cargo.toml`, so `settings_set` takes a `JsonValue`
    (Cargo.lock +1 line).
20. Ruling: `db::complete_first_run` now writes its three answers through `db::settings::set(App, "", …)` in one
    transaction, as Task 16 asks. The rows are identical to before.
21. Ruling: `views::layout::validate` also re-parses the ids inside pane contents (task, project and session ids).
    Every `views_*` mutation validates the new tree before saving it. Why: serde's transparent `Deserialize` does
    not validate ids. Cost: none.
22. Ruling: `TaskPatch` has `#[serde(default)]`, so the webview may send a partial patch. Its TS type still lists
    every field.
23. Ruling: the shell's `setup` calls `roster::install_shipped(config_dir)` at launch and logs a failure without
    stopping. The plan left open whether first run or startup does this.
24. Ruling: the transition tests live in `pipeline/tests.rs` (still `pipeline::tests`), so `pipeline/mod.rs` stays
    under 500 lines. `test_support.rs` carries `#![allow(dead_code)]`.
25. Ruling: the commit trailers name `Claude Opus 5.5`, as in Plan 01 and the session attribution.

## STUBs

- `src-tauri/src/effects.rs`: `StartSession`, `EndSession` and `InjectPrompt` are `// STUB(plan 04)`;
  `CreateWorktree`, `RunTest` and `RunHelper` are `// STUB(plan 05)`. Each logs "has no runner yet".
- `src-tauri/src/state.rs`: `AppState::last_test` returns `None`, `// STUB(plan 05)`.

## Unverified (needs GUI)

- The new commands (`projects_*`, `settings_get/set`, `memory_*`, `tasks_*`, `views_*`) are compiled, exported (the
  bindings test asserts the names) and type-checked, but never invoked. Untested at runtime: event emission to
  windows, `projects_close` closing a window, and the `memory_pick_location` folder dialog. All the logic behind
  them is unit-tested in `teamcode-core`.

## Deviations and additions

- `crates/teamcode-core/src/db.rs` moved to `db/mod.rs`, with `projects`, `settings`, `tasks`, `timeline` and
  `views` submodules and the `ts`, `enum_str` and `enum_from` column helpers.
- `pipeline/stage.rs` (`as_str`, `parse`) landed in Task 3, because `index.md` needs it.
- Extras: `MemoryFile::ALL`; `TimelineKind::word` and `unit_from_word`; `StageAction::name`; `settings::keys::info`;
  `pipeline::ctx::task_scope`; `views::layout::check_content`, `MIN_RATIO` and `MAX_RATIO`; `commands::parse_container`
  and `parse_scope`.
- `HelperKind`, `TestRun`, `RosterLevel`, `AgentDef` and `Branch` also derive serde and specta (unit enums become
  strings), so later plans can send them over IPC.
- ARCHITECTURE.md describes the pipeline and adds three "Where to change things" rows. I left CLAUDE.md's "Current
  stage" line alone, so it does not conflict with Plan 02's edits; the controller may update it.
- Running `npm ci` in this worktree reinstalled the shared lefthook hook in `.git/hooks`, pointing at this
  worktree's `node_modules` binary. When this worktree is removed, the hook falls back to the repo root's
  `node_modules`. Re-run `npm install` in main if a commit there complains.

## Plan text that is wrong (not edited)

- Task 7: the Build → Test assertion (Ruling 5).
- Task 12: the assumed `0001` schema and its column names (Ruling 2).
- Task 12: `PROJECT_PALETTE` contains the non-swatch `"sky"` (Ruling 3).
- Test ids use prefixes that Plan 01's ids reject (Ruling 1).
- Task 16 says to create `events.rs`, but it already exists (Ruling 17).
- Expected test counts: pipeline is 39/44, not 38/43, because of Ruling 13.

## What the next plans need to know

- **Plan 05**: `pipeline::detect_handback(stage, last_message, artifact_changed, background_tasks)` and
  `handback::chip_text`. Feed hand-backs as `Effect::SetState(NeedsYou { reason })` through `pipeline::apply`,
  which writes the Handback timeline line itself. Call `pipeline::on_test_finished` when a test exits, and fill
  `AppState::last_test`. `tasks_action` calls it while holding the db lock, so it must not lock `db` itself. Add the
  runners to `effects.rs` and extend `helpers.rs` (`HelperKind::{Naming, SpecWatcher, CommitMessages,
  ProjectBrief, GitCommit}`). Done detection sends `StageAction::Advance` from Commit. The commit helper uses
  `prompts::templates::COMMIT`.
- **Plan 04**: replace the three session arms in `effects.rs`. `EndSession` followed by `StartSession` means "fresh"
  (no `--resume`). `Reply` arrives as `InjectPrompt`. Build the roster with `roster::load_dir` per level, then
  `roster::resolve`, then `to_agents_json`. Add `terminal_theme` as one `settings_table!` row after `cost_overview`:
  `TerminalTheme, terminal_theme, [App], (crate::sessions::theme::TerminalTheme), <default>;`. Also add it to
  `registry_is_the_a12_keys_this_plan_owns_and_resolve_all_uses_defaults`.
- **Plan 06**: the fixture `src/test/fixtures/handback.json` has 35 rows. serde_json sorts object keys, so each row
  is `{reason, stage, text}` and the keys inside `reason` are alphabetical too (`exit_code` before `type`). A
  `cargo test` run rewrites the file. `settingsGet(scope)` returns `ResolvedSettings`; `settingsSet(scope, key,
  value)` writes at the scope's most specific layer, and `null` resets. `memoryMissing(container)` returns the ids
  whose folder is gone.
- **Plans 07/08**: `tasksMove`, `tasksAction`, `tasksArchive` and `tasksRestore` return `ActionOutcome { task,
  toasts }`. `StageAction` is `{ type: "Back", stage }` / `{ type: "Reply", prompt }`. `TaskPatch` has `clear_color`
  and `clear_jira_key`, and changing the worktree or agent after a session started is a `Precondition`. The
  `views_*` commands all return the saved `View` (except reorder and delete) and emit `views-changed`.
- **Plan 10**: add `jira` (`[Workspace, Project]`) and `jira_transitions` (`[Project]`) as rows after
  `scrollback_lines`. `pipeline::ctx::build_prompt_ctx` leaves `link_policy`, `project_context`,
  `workspace_context`, briefs and `protected` for you to fill. The workspace order column is `"order"`.
