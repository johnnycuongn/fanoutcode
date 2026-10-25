# Fanout Code 11: Cross-Task File Awareness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Claude session stops before writing a file another active task has uncommitted changes in, the developer decides in the task's new Overlap dock tab, the other session is told, and a worktree task plans around the others and brings its base branch in knowing both tasks' intent.

**Architecture:** A new `fanoutcode_core::overlap` module owns the derived *claims* (dirty attributed files per shared repo, written as a JSON file), the per-task *grants* file, the `fanoutcode claim-check` hook that reads both and answers Claude Code's `PreToolUse` protocol like the existing guard, the `task_overlaps` and `task_footprint` tables, and every prompt text. The shell adds `overlap_sync` (rebuild claims and overlaps when attribution or git status changes), `ClaimDenied` handling in `hook_feed` with a state pin, five `overlap_*` commands and one `overlap-changed` event. The webview gets `src/features/overlap/` (store, Overlap tab, decision and integration panes) and small additions to the dock, task page, board, sidebar and settings.

**Tech Stack:** Rust (rusqlite, serde, serde_json, chrono, tempfile in tests), Tauri 2 commands and events exported by tauri-specta, React 19 + TypeScript strict, module stores on `createStore`, Vitest + Testing Library.

**Spec:** `docs/superpowers/specs/2026-10-25-cross-task-file-awareness-design.md` (this plan cites its sections as §N). The base design is `docs/superpowers/specs/2026-10-24-team-code-design.md`; the earlier plans are in this folder, `00-overview-and-contracts.md` first.

## Global Constraints

- `crates/fanoutcode-core` never depends on Tauri; anything decidable lives there with tests (`.claude/rules/core.md`).
- Generated bindings: edit Rust, run `npm run bindings`, commit `src/lib/bindings.ts`; no hand-written mirrors (ADR 0001).
- Serde: no `rename_all`; `#[serde(tag = "type")]` on data-carrying enums, unit or struct variants only; every new data-carrying enum round-trips through serde in a test.
- Migration `0011_overlap.sql` is listed in `db::MIGRATIONS` and never edited after it lands.
- Files written atomically through `memory::store::write_atomic`.
- Hooks: fail open; never wait on the developer; timeout 5 s; the app never writes into the project's `.claude/`.
- The app never runs `git merge`, `rebase`, `push` or branch deletion (§2, §6.3). Only the integrate prompt may tell the agent to merge or rebase (§6.3); prompts still never say commit, push or delete.
- Webview: primitives from `src/components/ui/` only; semantic colour tokens only; IPC only through `src/lib/bindings.ts`; features import each other only through `index.ts`; component under about 200 lines, store under 300, Rust module under 500.
- Naming: commands `domain_verb` snake_case, events kebab-case, components `PascalCase.tsx`, lib `camelCase.ts`.
- Copy fixed by the spec is used verbatim: chips *File in use* / *N files in use*, *Holding for <other>*, *Merge needs you*; tab name **Overlap**; button **Decide**; action **Bring <base> in**; sentinel `FANOUTCODE: integrate done`.
- Every commit passes `npm run typecheck`, `npm run lint`, `npx vitest run`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `npm run bindings:check` when Rust IPC types changed. Conventional Commits; never `--no-verify`.

## Review Focus

Inputs the spec implies but no task's tests would otherwise exercise; each line's test is added to the task named.

1. A file outside every repo of the task (a path under `/tmp`, or the task's memory folder) → the hook allows; nothing is claimed there. Test in Task 5 (`allows_a_path_outside_every_repo`).
2. A claim whose task is archived, or whose state is recorded but whose file is no longer dirty → not a claim; the hook allows. Tests in Task 3 (`archived_tasks_never_claim`, `a_committed_file_drops_its_claim`).
3. A `Write` with a relative `file_path` → resolved against the hook's `cwd` before the lookup. Test in Task 5 (`resolves_a_relative_path_against_cwd`).
4. An `AllFiles` grant between A and B must not cover a claim held by a third task C. Test in Task 4 (`all_files_grant_covers_only_the_named_task`).
5. "Start fresh session" or archiving while decisions are pending → the pending decisions are cleared and the pin released, so the new session is not born needs-you. Test in Task 10 (`fresh_session_clears_pending_and_unpins`).

---

## File structure

**Core, `crates/fanoutcode-core/src/overlap/`** (new; each file under 500 lines):

| File | Responsibility |
|---|---|
| `mod.rs` | Public types: `Relation`, `OverlapRow`, `PendingDecision`, `Decision`, `DecisionScope`; re-exports |
| `claims.rs` | `Claim`, `ClaimsFile`, `repo_key`, `claims_path`, `build_claims`, `write_claims`, `read_claims` |
| `grants.rs` | `Grant`, `GrantScope`, `GrantsFile`, `grants_path`, `read_grants`, `write_grants`, `GrantsFile::covers`, `prune` |
| `check.rs` | The hook: `CheckInput`, `CheckDecision`, `evaluate`, `run_check`, `check_hook_spec` |
| `git_rules.rs` | `evaluate_git`: which shell commands commit or discard claimed paths |
| `notes.rs` | The per-task note inbox the hook drains into `additionalContext`: `push_note`, `drain_notes`, `notes_path` |
| `store.rs` | `task_overlaps` CRUD |
| `footprint.rs` | `## Files` parser and `task_footprint` CRUD |
| `compute.rs` | `overlaps_for`: rows from claims, footprints and change sets |
| `prompts.rs` | Every prompt text: `others_working_here`, `go_ahead`, `told`, `soft_note`, `integrate`, `INTEGRATE_SENTINEL` |

Also touched: `domain.rs`, `db/mod.rs`, `db/tasks.rs`, `db/timeline.rs`, `memory/index.rs`, `hooks/mod.rs`, `hooks/parse.rs`, `changes/mod.rs`, `changes/repos.rs`, `changes/attributor.rs`, `settings/keys.rs`, `settings/values.rs`, `pipeline/handback.rs`, `prompts/templates.rs`, `workspace/launch.rs`, `git/mod.rs`, `worktree.rs`, `lib.rs`; new `migrations/0011_overlap.sql`.

**Shell, `src-tauri/src/`:** new `overlap_sync.rs` (claims and overlaps rebuild, hookless attribution, notes delivery), new `commands/overlap.rs`; touched `main.rs`, `hook_feed.rs`, `git_watcher.rs`, `events.rs`, `specta.rs`, `state.rs`, `session_service/` (worktree base recording).

**Webview, `src/features/overlap/`** (new): `types.ts`, `store.ts`, `components/OverlapTab.tsx`, `components/OverlapFileList.tsx`, `components/DecisionPane.tsx`, `components/IntegratePane.tsx`, `components/BaseBranchDialog.tsx`, `index.ts`. Touched: `features/dock/{types,tabRules}.ts`, `features/dock/components/{Dock,DockTabStrip}.tsx`, `features/task-page/{primaryAction.ts,components/OverflowMenu.tsx,components/TaskPage.tsx}`, `features/board/components/BoardCard.tsx`, `features/shell/components/TabItem.tsx`, `features/sidebar/components/ChangesSection.tsx`, `features/settings/components/ProjectSettings.tsx`, `features/canvas` (event rows), `lib/format.ts`.

---

### Task 1: Domain, migration and settings

**Files:**
- Modify: `crates/fanoutcode-core/src/domain.rs` (HandbackReason, Task, new enums)
- Modify: `crates/fanoutcode-core/src/changes/mod.rs` (ChangeSource)
- Modify: `crates/fanoutcode-core/src/db/tasks.rs` (base_branch column), `crates/fanoutcode-core/src/db/mod.rs` (MIGRATIONS)
- Create: `crates/fanoutcode-core/migrations/0011_overlap.sql`
- Modify: `crates/fanoutcode-core/src/settings/keys.rs`, `crates/fanoutcode-core/src/settings/values.rs`
- Modify: `crates/fanoutcode-core/src/pipeline/handback.rs` (chip_text)
- Test: existing serde round-trip tests in `domain.rs` and `handback.rs`; `settings/mod.rs` tests

**Interfaces:**
- Produces: `HandbackReason::FileInUse { pending: u32 }`, `HandbackReason::HoldingFor { other: String }` (chip `Holding for <other>`, set when every pending decision of the task is a Hold), `HandbackReason::MergeNeedsYou { repo: PathBuf, paths: Vec<String> }`; `Task.base_branch: Option<String>`; `ChangeSource::Observed`; `settings::values::Integration { Merge, Rebase }`; settings keys `ask_before_other_tasks_files: bool = true` [Project], `guard_git_for_other_tasks: bool = true` [Project], `worktree_integration: Integration = Merge` [Project]; tables `task_overlaps`, `task_footprint`; `chip_text` returns `"File in use"` for `pending == 1`, `"N files in use"` otherwise, `"Merge needs you"`.

- [ ] **Step 1: Write the failing tests**

In `domain.rs` tests, extend the HandbackReason round-trip list with `FileInUse { pending: 2 }` and `MergeNeedsYou { repo: "/r".into(), paths: vec!["a.ts".into()] }`, asserting the JSON carries `"type":"FileInUse"`. In `handback.rs` tests:

```rust
#[test]
fn chip_text_for_overlap_reasons() {
    assert_eq!(chip_text(&HandbackReason::FileInUse { pending: 1 }, Stage::Build), "File in use");
    assert_eq!(chip_text(&HandbackReason::FileInUse { pending: 3 }, Stage::Build), "3 files in use");
    assert_eq!(chip_text(&HandbackReason::HoldingFor { other: "Login".into() }, Stage::Build), "Holding for Login");
    assert_eq!(chip_text(&HandbackReason::MergeNeedsYou { repo: "/r".into(), paths: vec![] }, Stage::Done), "Merge needs you");
}
```

In `settings/mod.rs` tests: `resolve` yields `ask_before_other_tasks_files == true`, `guard_git_for_other_tasks == true`, `worktree_integration == Integration::Merge` by default, and `validate("worktree_integration", ScopeKind::App, json!("Merge"))` is `Err(Invalid)` (project scope only). In `db/tasks.rs` tests: a task saved with `base_branch: Some("main")` reads back with it.

- [ ] **Step 2: Run them**

Run: `cargo test -p fanoutcode-core domain:: handback:: settings:: db::tasks::`
Expected: compile errors for the missing variants, field, key and value.

- [ ] **Step 3: Implement**

`0011_overlap.sql`:

```sql
-- 0011_overlap (Plan 11). Never edited after merge.
ALTER TABLE tasks ADD COLUMN base_branch TEXT;
CREATE TABLE task_overlaps (
    task_id TEXT NOT NULL, other_task TEXT, repo_root TEXT NOT NULL, path TEXT NOT NULL,
    relation TEXT NOT NULL,            -- Claimed | GrantedOnTop | TakenOver | PlannedBy | Committed
    pending_json TEXT,                 -- PendingDecision while a decision is open, else NULL
    held INTEGER NOT NULL DEFAULT 0,   -- the developer chose Hold; pending stays until the claim is released
    noted_out INTEGER NOT NULL DEFAULT 0, noted_in INTEGER NOT NULL DEFAULT 0,
    decided_at TEXT, updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, repo_root, path, other_task));
CREATE TABLE task_footprint (
    task_id TEXT NOT NULL, repo_root TEXT NOT NULL, path TEXT NOT NULL,
    source TEXT NOT NULL,              -- Plan | ChangeSet
    note TEXT, added_at TEXT NOT NULL,
    PRIMARY KEY (task_id, repo_root, path, source));
```

Add `(11, "overlap", include_str!("../../migrations/0011_overlap.sql"))` to `MIGRATIONS`. Add the variants, the `base_branch` field (read and written in `db/tasks.rs` beside `links_bound_at`), `ChangeSource::Observed`, `values::Integration` (derive like `DockPosition`), the three `settings_table!` rows, and the `chip_text` arms.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p fanoutcode-core` — Expected: all pass, including the existing enum round-trips.

- [ ] **Step 5: Commit**

```bash
git add crates/fanoutcode-core
git commit -m "feat(core): overlap domain types, settings and migration 0011"
```

---

### Task 2: Hook events and timeline kinds for overlaps

**Files:**
- Modify: `crates/fanoutcode-core/src/hooks/mod.rs` (HookEvent), `crates/fanoutcode-core/src/hooks/parse.rs`
- Modify: `crates/fanoutcode-core/src/memory/index.rs` (TimelineKind), `crates/fanoutcode-core/src/db/timeline.rs`
- Test: `hooks/parse.rs` tests, `memory/index.rs` tests, `db/timeline.rs` tests

**Interfaces:**
- Produces: `HookEvent::ClaimDenied { repo: PathBuf, path: Option<String>, command: Option<String>, other_task: Option<TaskId> }`, `HookEvent::OverlapDecided { repo: PathBuf, path: String, other_task: Option<TaskId>, decision: String }`, `HookEvent::OverlapNoted { repo: PathBuf, path: String, other_task: TaskId, direction: String }`; log line first fields `ClaimDenied`, `OverlapDecided`, `OverlapNoted` parsed from a JSON payload with those keys; `TimelineKind::Overlap { detail: String }` with word `"overlap"`, stored in `task_timeline.detail` as `{"detail": …}`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn parses_claim_denied_and_overlap_lines() {
    let line = "ClaimDenied\t2026-10-25T10:00:00Z\t{\"repo\":\"/r\",\"path\":\"src/a.ts\",\"other_task\":\"task-b2\"}";
    let ev = parse_line(&TaskId::parse("task-a1").unwrap(), 1, line).unwrap();
    assert!(matches!(ev.event, HookEvent::ClaimDenied { ref path, .. } if path.as_deref() == Some("src/a.ts")));
    // OverlapDecided and OverlapNoted likewise; an unknown first field still parses to Unknown
}
```

Timeline: `append` then `list` round-trips `TimelineKind::Overlap { detail: "on top of Login in src/a.ts".into() }`; `IndexEntry` renders as `· overlap: on top of Login in src/a.ts` in `index.md` (follow how `Spec { reason }` renders).

- [ ] **Step 2: Run them** — Expected: FAIL, unknown variants.
- [ ] **Step 3: Implement** the variants, the three `parse.rs` arms, the serde round-trip entries, `TimelineKind::Overlap` with `word()`, `unit_from_word` unchanged (it carries detail), `db/timeline.rs::append` detail mapping and the reader's inverse.
- [ ] **Step 4: Run** `cargo test -p fanoutcode-core hooks:: memory:: db::timeline::` — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(core): ClaimDenied, OverlapDecided and OverlapNoted hook events; overlap timeline kind`

---

### Task 3: Claims file

**Files:**
- Create: `crates/fanoutcode-core/src/overlap/mod.rs`, `crates/fanoutcode-core/src/overlap/claims.rs`
- Modify: `crates/fanoutcode-core/src/lib.rs` (`pub mod overlap;`)
- Test: `overlap/claims.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `changes::AttributedFile`, `git::StatusEntry`, `domain::{Task, AgentState, Stage, TaskId}`, `memory::store::write_atomic`.
- Produces:
  ```rust
  pub struct Claim { pub path: String, pub task_id: Option<TaskId>, pub title: String, pub stage: Option<Stage>,
                     pub state: Option<AgentState>, pub last_edit: DateTime<Utc> }
  pub struct ClaimsFile { pub repo_root: PathBuf, pub written_at: DateTime<Utc>, pub claims: Vec<Claim> }
  pub fn repo_key(root: &Path) -> String;                 // 16 hex chars of a stable hash of the canonical path
  pub fn claims_path(app_data: &Path, root: &Path) -> PathBuf;   // <app_data>/claims/<repo_key>.json
  pub struct TaskFiles<'a> { pub task: &'a Task, pub files: &'a [AttributedFile] }
  pub fn build_claims(root: &Path, tasks: &[TaskFiles<'_>], status: &[StatusEntry], now: DateTime<Utc>) -> ClaimsFile;
  pub fn write_claims(app_data: &Path, file: &ClaimsFile) -> Result<PathBuf>;
  pub fn read_claims(path: &Path) -> Option<ClaimsFile>;  // None when missing or malformed (fail open)
  ```
  A claim exists for a path when some task in `tasks` has an `AttributedFile` for `root` with that path **and** the path appears in `status` (dirty); archived tasks are skipped; `last_edit` is the attribution's `added_at`. An `AttributedFile` with `source == Observed` and no task (see Task 7) yields a claim with `task_id: None, title: "another session"`.

- [ ] **Step 1: Write the failing tests**

`claims_only_dirty_attributed_files_of_active_tasks`, `archived_tasks_never_claim`, `a_committed_file_drops_its_claim` (same attribution, empty status → no claim), `repo_key_is_stable_and_path_safe`, `write_then_read_round_trips`, `read_returns_none_for_malformed`.

- [ ] **Step 2: Run** `cargo test -p fanoutcode-core overlap::claims` — Expected: FAIL to compile.
- [ ] **Step 3: Implement** `claims.rs` and a `mod.rs` that declares `pub mod claims;` (the other modules follow in later tasks).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(core): claims file per shared repo`

---

### Task 4: Grants file

**Files:**
- Create: `crates/fanoutcode-core/src/overlap/grants.rs`
- Test: in-file

**Interfaces:**
- Produces:
  ```rust
  #[serde(tag = "type")] pub enum GrantScope { File { repo: PathBuf, path: String }, AllFiles }
  pub struct Grant { pub other_task: Option<TaskId>, pub scope: GrantScope, pub granted_at: DateTime<Utc> }
  pub struct GrantsFile { pub grants: Vec<Grant> }
  pub fn grants_path(app_data: &Path, task_id: &TaskId) -> PathBuf;   // <app_data>/grants/<task-id>.json
  pub fn read_grants(path: &Path) -> GrantsFile;                       // empty when missing or malformed
  pub fn write_grants(app_data: &Path, task_id: &TaskId, file: &GrantsFile) -> Result<PathBuf>;
  impl GrantsFile {
      pub fn covers(&self, repo: &Path, path: &str, other: Option<&TaskId>) -> bool;
      pub fn add(&mut self, grant: Grant);
      /// Drops grants whose claims are gone: a File grant whose path is no longer claimed, an AllFiles grant whose
      /// other task holds no claim in any of `claims`.
      pub fn prune(&mut self, claims: &[&ClaimsFile]);
  }
  ```

- [ ] **Step 1: Tests:** `file_grant_covers_exactly_its_path`, `all_files_grant_covers_only_the_named_task` (grant on B does not cover C's claim), `a_grant_with_no_other_task_matches_an_unattributed_claim`, `prune_drops_grants_without_claims`, `read_is_empty_for_missing_file`, serde round-trip of `GrantScope`.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p fanoutcode-core overlap::grants` — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(core): per-task grants file`

---

### Task 5: The claim-check hook, git rules and the note inbox

**Files:**
- Create: `crates/fanoutcode-core/src/overlap/check.rs`, `crates/fanoutcode-core/src/overlap/git_rules.rs`, `crates/fanoutcode-core/src/overlap/notes.rs`
- Modify: `crates/fanoutcode-core/src/workspace/launch.rs` (`LaunchExtras.claim_check`), `src-tauri/src/main.rs` (`claim-check` branch), `crates/fanoutcode-core/src/sessions/launch.rs` only if `LaunchExtras.guard` is merged there (add `claim_check` the same way)
- Test: in-file tests; `workspace/launch.rs` test `launch_extras_adds_the_claim_check_hook`

**Interfaces:**
- Consumes: `ClaimsFile`, `GrantsFile`, `hooks::{HookSpec, sh_single_quote}`, `workspace::guard::shell_words` (make it `pub(crate)` if needed for tokenising the Bash command).
- Produces:
  ```rust
  pub struct CheckInput { pub tool_name: String, pub tool_input: serde_json::Value, pub cwd: PathBuf }
  pub enum CheckDecision { Allow, Deny { repo: PathBuf, path: Option<String>, command: Option<String>, other: Option<Claim>, reason: String } }
  pub const EDIT_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];
  pub fn evaluate(input: &CheckInput, own_task: &TaskId, claims: &[ClaimsFile], grants: &GrantsFile, guard_git: bool) -> CheckDecision;
  pub fn deny_reason(path: &str, other_title: &str) -> String;   // the §5.3 sentence, verbatim
  pub fn run_check(stdin: &mut dyn Read, stdout: &mut dyn Write, args: &CheckArgs) -> i32;
  pub struct CheckArgs { pub task_id: TaskId, pub claims_paths: Vec<PathBuf>, pub grants_path: PathBuf, pub notes_path: PathBuf, pub log_path: PathBuf, pub guard_git: bool }
  pub fn check_hook_spec(exe: &Path, args: &CheckArgs) -> HookSpec;  // event PreToolUse, matcher "Edit|Write|MultiEdit|NotebookEdit|Bash", timeout 5
  // git_rules.rs
  pub enum GitRule { CommitAll, AddAll, CommitPath, AddPath, CheckoutPath, RestorePath, Stash, ResetHard, Clean }
  pub fn evaluate_git(command: &str, cwd: &Path, repo: &Path, claimed: &[&Claim]) -> Option<(GitRule, Vec<String>)>;
  // notes.rs
  pub fn notes_path(app_data: &Path, task_id: &TaskId) -> PathBuf;   // <app_data>/notes/<task-id>.json
  pub fn push_note(path: &Path, note: &str) -> Result<()>;
  pub fn drain_notes(path: &Path) -> Vec<String>;                     // empties the file
  ```
  `run_check` reads the payload, resolves `tool_input.file_path` (or `notebook_path`) against `cwd`, picks the claims file whose `repo_root` is a prefix of the path, and: own task's claim or no claim → allow; covered by a grant → allow; else deny and append `ClaimDenied\t<rfc3339>\t{repo,path,other_task}` to `log_path`. For `Bash` with `guard_git`, `evaluate_git` against the claims of other tasks in the repo containing `cwd`; a hit denies with the files named and logs `ClaimDenied` with `command`. On allow, if `notes_path` has notes, print `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","additionalContext":"<notes joined by blank lines>"}}` and drain. Always exit 0; unreadable files allow.

- [ ] **Step 1: Tests** (`check.rs`): `allows_when_no_claim`, `allows_own_claim`, `denies_other_tasks_claim_with_reason_and_logs_claim_denied`, `allows_when_a_grant_covers`, `allows_a_path_outside_every_repo`, `resolves_a_relative_path_against_cwd`, `drains_notes_into_additional_context_on_allow`, `malformed_input_allows`, `non_edit_tool_without_git_guard_allows`; (`git_rules.rs`): `commit_a_with_a_claimed_dirty_path_is_caught`, `commit_naming_only_own_paths_is_not_caught`, `add_dot_and_add_all_are_caught`, `checkout_and_restore_of_a_claimed_path_are_caught`, `stash_reset_hard_and_clean_are_caught_when_any_claim_exists`, `git_log_and_status_are_never_caught`; (`notes.rs`): `push_then_drain_returns_in_order_and_empties`. In `workspace/launch.rs`: `launch_extras_adds_the_claim_check_hook` (a Claude Code task gets `claim_check: Some(spec)` whose command starts with `<exe> claim-check` when `ask_before_other_tasks_files` is on, `None` when it is off and `guard_git_for_other_tasks` is off).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.** `LaunchExtras` gains `pub claim_check: Option<HookSpec>`; `launch_extras` takes the resolved project settings it needs (add parameters `ask_before: bool, guard_git: bool`, read by the caller from `settings::resolve`), and builds `CheckArgs` with `claims_paths` for every repo of `observe::repos` shape (`task.worktree_path` or home root, then `linked_roots`), `grants_path`, `notes_path`, `event_log_path`. `main.rs`: `fanoutcode claim-check <task-id> <grants> <notes> <log> <guard-git:0|1> <claims…>` → `run_check`. Wherever `extras.guard` is pushed into the settings file's hooks, push `extras.claim_check` too.
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 5: Commit** — `feat: fanoutcode claim-check hook, git rules against other tasks' claims, note inbox`

---

### Task 6: Overlap and footprint stores

**Files:**
- Create: `crates/fanoutcode-core/src/overlap/store.rs`, `crates/fanoutcode-core/src/overlap/footprint.rs`
- Modify: `crates/fanoutcode-core/src/overlap/mod.rs` (types)
- Test: in-file, with `test_support::test_conn`

**Interfaces:**
- Produces (in `mod.rs`):
  ```rust
  #[derive(Serialize, Deserialize, specta::Type)] pub enum Relation { Claimed, GrantedOnTop, TakenOver, PlannedBy, Committed }
  #[serde(tag = "type")] pub enum PendingDecision { Edit { tool: String, input: serde_json::Value }, Command { command: String } }
  #[serde(tag = "type")] pub enum Decision { OnTopFile, OnTopAll, Replace, Hold }
  pub struct OverlapRow { pub task_id: TaskId, pub other_task: Option<TaskId>, pub repo_root: PathBuf, pub path: String,
      pub relation: Relation, pub pending: Option<PendingDecision>, pub held: bool, pub noted_out: bool, pub noted_in: bool,
      pub decided_at: Option<DateTime<Utc>>, pub updated_at: DateTime<Utc> }
  ```
  `store.rs`: `upsert(conn, &OverlapRow)`, `list(conn, &TaskId) -> Vec<OverlapRow>`, `pending(conn, &TaskId) -> Vec<OverlapRow>`, `set_pending(conn, &TaskId, repo, path, other: Option<&TaskId>, Option<PendingDecision>)`, `set_relation(conn, &TaskId, repo, path, other, Relation, decided_at)`, `clear_pending(conn, &TaskId) -> u32`, `taken_over_paths(conn, &TaskId, repo) -> Vec<String>` (paths where this task is the *other* and relation is `TakenOver`), `mark_noted(conn, &TaskId, repo, path, other, outgoing: bool)`, `remove_absent(conn, &TaskId, keep: &[(PathBuf, String, Option<TaskId>)])`.
  `footprint.rs`: `pub struct FootprintEntry { pub path: String, pub note: Option<String> }`, `pub fn parse_files_section(md: &str) -> Vec<FootprintEntry>` (lines under `## Files` until the next `## `, each `- path` or `- path — note`, backticks stripped), `pub enum FootprintSource { Plan, ChangeSet }`, `pub struct FootprintRow { pub task_id, pub repo_root, pub path, pub source, pub note, pub added_at }`, `replace_plan(conn, &TaskId, repo, &[FootprintEntry], now)`, `add_change_set(conn, &TaskId, repo, paths: &[String], now)`, `footprint(conn, &TaskId) -> Vec<FootprintRow>`, `footprints_of(conn, tasks: &[TaskId]) -> Vec<FootprintRow>`.

- [ ] **Step 1: Tests:** `upsert_then_list_round_trips_every_field`, `pending_lists_only_rows_with_a_decision`, `clear_pending_returns_the_count`, `taken_over_paths_reads_the_other_side`, `parse_files_section_reads_paths_and_notes_and_stops_at_next_heading`, `parse_files_section_is_empty_without_the_heading`, `replace_plan_replaces_only_plan_rows`; serde round-trips of `Relation`, `PendingDecision`, `Decision`.
- [ ] **Step 2: Run** — FAIL. **Step 3: Implement.** **Step 4: Run** `cargo test -p fanoutcode-core overlap::` — PASS.
- [ ] **Step 5: Commit** — `feat(core): task_overlaps and task_footprint stores, ## Files parser`

---

### Task 7: Computing overlaps, taken-over exclusion, observed attribution

**Files:**
- Create: `crates/fanoutcode-core/src/overlap/compute.rs`
- Modify: `crates/fanoutcode-core/src/changes/repos.rs` (`done_paths`), `crates/fanoutcode-core/src/changes/attributor.rs` (`observe_dirty`), callers of `done_paths` (`src-tauri/src/git_watcher.rs`)
- Test: in-file

**Interfaces:**
- Produces:
  ```rust
  pub struct OtherTask<'a> { pub task: &'a Task, pub files: &'a [AttributedFile], pub footprint: &'a [FootprintRow], pub branch: Option<String> }
  /// Rows for `me`: Claimed when another active task's claim exists for a path in my change set or footprint
  /// (shared tree only); PlannedBy when another task's footprint names a path in mine (separate trees); Committed when
  /// the other task's attribution for the path is no longer dirty. Existing GrantedOnTop/TakenOver rows are kept.
  pub fn overlaps_for(me: &OtherTask<'_>, others: &[OtherTask<'_>], claims: &[ClaimsFile], status_by_repo: &[(PathBuf, Vec<StatusEntry>)], existing: &[OverlapRow], now: DateTime<Utc>) -> Vec<OverlapRow>;
  pub fn shares_tree(a: &Task, b: &Task, repo: &Path) -> bool;   // same working tree for `repo`: both without a worktree on the home root, or a linked root
  ```
  `changes::done_paths(repos, attributed, last, taken_over: &[(PathBuf, Vec<String>)])` excludes taken-over paths. `Attributor::observe_dirty(&mut self, repo: &Path, status: &[StatusEntry], now: DateTime<Utc>) -> Vec<AttributedFile>` attributes every dirty path not already attributed with `source: Observed, agent_id: MAIN_AGENT`.

- [ ] **Step 1: Tests:** `claimed_when_sharing_a_tree_and_the_other_has_a_dirty_attributed_file`, `planned_by_when_trees_differ_and_the_footprint_names_my_file`, `committed_once_the_other_file_is_clean`, `keeps_granted_and_taken_over_rows`, `shares_tree_rules` (worktree vs none; linked roots always shared), `done_paths_skips_taken_over_paths`, `observe_dirty_attributes_only_new_dirty_paths_as_observed`.
- [ ] **Step 2–4:** FAIL → implement → `cargo test --workspace` PASS (update `git_watcher.rs` to pass `&[]` for now; Task 10 fills it).
- [ ] **Step 5: Commit** — `feat(core): overlap computation, taken-over exclusion from Done detection, observed attribution`

---

### Task 8: Prompt texts, the `## Files` section, the rule exception

**Files:**
- Create: `crates/fanoutcode-core/src/overlap/prompts.rs`
- Modify: `crates/fanoutcode-core/src/prompts/templates.rs` (PLAN), `.claude/rules/prompts.md`
- Test: in-file; `prompts/templates.rs` tests

**Interfaces:**
- Produces:
  ```rust
  pub struct OtherSummary { pub title: String, pub stage: Stage, pub state: AgentState, pub branch: Option<String>,
      pub intent: String, pub planned: Vec<String>, pub touched: Vec<String>, pub added_files: Vec<String>, pub exported: Vec<String> }
  pub fn others_working_here(others: &[OtherSummary]) -> String;   // "" when empty; ends with the §6.1 instruction, verbatim
  pub fn go_ahead(decision: Decision, other_title: &str, path: &str) -> String;       // §5.4 sentences
  pub fn told(decision: Decision, my_title: &str, path: &str) -> String;              // §5.5 sentences
  pub fn soft_note(other_title: &str, path: &str, branch: &str) -> String;            // §6.2 sentence
  pub fn new_task_note(title: &str, branch: &str, planned: &[String]) -> String;      // §6.1 last paragraph
  pub struct ConflictCtx { pub path: String, pub other_title: String, pub spec_excerpt: String, pub plan_excerpt: String }
  pub fn integrate(base: &str, mode: Integration, conflicts: &[ConflictCtx]) -> String;  // §6.3 rule verbatim; ends with the sentinel instruction
  pub const INTEGRATE_SENTINEL: &str = "FANOUTCODE: integrate done";
  pub fn export_names(source: &str) -> Vec<String>;  // names after `export (default )?(function|class|const|let|var|type|interface|enum|async function) `, and Rust `pub fn|struct|enum|trait NAME`
  ```
  PLAN template: after the `## Team` instructions add: *"List every file you intend to change under a `## Files` heading, one path per line as `- path — why`, before the `## Team` section."* and keep the eight placeholders unchanged.

- [ ] **Step 1: Tests:** `others_working_here_lists_each_task_and_ends_with_the_instruction`, `others_working_here_is_empty_with_no_others`, `go_ahead_and_told_name_the_file_and_the_task` (assert the exact §5.4/§5.5 sentences for `OnTopFile` and `Replace`), `integrate_carries_the_rule_and_the_sentinel`, `export_names_reads_ts_and_rust_exports`; in `templates.rs`: `plan_template_asks_for_a_files_section`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5:** Edit `.claude/rules/prompts.md` last bullet to: *"Prompts never tell the agent to commit, push or delete branches; the Commit stage drafts a message. The one prompt that says merge or rebase is the integrate prompt of Plan 11 (`overlap::prompts::integrate`), sent only from a Done card by the developer."*
- [ ] **Step 6: Commit** — `feat(core): overlap prompt texts, ## Files in the Plan prompt, integrate prompt rule exception`

---

### Task 9: Git helpers and the base branch

**Files:**
- Modify: `crates/fanoutcode-core/src/git/mod.rs`, `crates/fanoutcode-core/src/worktree.rs` (doc only), `src-tauri/src/session_service/` where `ManagedCreate` is run (record base), `src-tauri/src/commands/tasks.rs` or a new command in Task 11 for confirming the base
- Test: `git/mod.rs` async tests with `git::test_support`

**Interfaces:**
- Produces:
  ```rust
  pub async fn current_branch(repo: &Path) -> Result<Option<String>>;         // None when detached
  pub async fn default_branch(repo: &Path) -> Result<Option<String>>;         // origin/HEAD target, else main, else master, else None
  pub async fn has_conflicts(repo: &Path) -> Result<bool>;                    // MERGE_HEAD/REBASE_HEAD exists or status has UU/AA/DD/AU/UA/DU/UD
  pub async fn merge_base(repo: &Path, a: &str, b: &str) -> Result<String>;
  ```
  Shell: when a managed worktree is created, `current_branch(repo_root)` is stored in `tasks.base_branch`; for the Claude Code convention nothing is stored at creation (§6.3).

- [ ] **Step 1: Tests:** `current_branch_reads_head`, `default_branch_prefers_origin_head_then_main_then_master`, `has_conflicts_is_true_mid_merge`, `merge_base_of_two_branches`.
- [ ] **Step 2–4:** FAIL → implement with `git::run` → PASS.
- [ ] **Step 5:** In the shell's managed-worktree creation path, after `git worktree add`, write `base_branch` through `db::tasks` and emit `task-changed`. Test by the existing worktree creation test if one exists; otherwise rely on Task 12's command test.
- [ ] **Step 6: Commit** — `feat: git branch helpers; record the base branch of a managed worktree`

---

### Task 10: Shell sync: claims, overlaps, pin, hookless attribution

**Files:**
- Create: `src-tauri/src/overlap_sync.rs`
- Modify: `src-tauri/src/hook_feed.rs`, `src-tauri/src/git_watcher.rs`, `src-tauri/src/state.rs`, `src-tauri/src/events.rs`, `src-tauri/src/lib.rs` (startup rebuild), `src-tauri/src/session_service/` (fresh session / archive clear)
- Test: core-level logic stays in core; shell has no unit tests by rule. Add to core: `overlap::pin::pinned(pending: u32, derived: Option<AgentState>) -> Option<AgentState>` in `overlap/mod.rs` with tests `pin_overrides_working_and_replied_while_pending`, `no_pin_without_pending`; and `fresh_session_clears_pending_and_unpins` as a `store.rs` test of `clear_pending` plus the `pinned(0, …)` identity.

**Interfaces:**
- Produces (shell): 
  ```rust
  pub fn shared_repos_of(state: &AppState, task: &Task) -> Vec<PathBuf>;               // repos another active task also writes
  pub async fn rebuild_claims(app: &AppHandle, repo: &Path) -> Result<()>;             // build_claims over active tasks' attributions + git::status; write_claims; prune every affected task's grants
  pub async fn rebuild_overlaps(app: &AppHandle, task_id: &TaskId) -> Result<()>;      // overlaps_for → store::remove_absent + upsert; emit OverlapChanged
  pub fn on_claim_denied(app: &AppHandle, state: &AppState, ev: &TaskEvent) -> Result<()>;  // set_pending + pin to NeedsYou{FileInUse{pending}}
  pub fn deliver_notes_if_idle(app: &AppHandle, state: &AppState, task_id: &TaskId) -> Result<()>;  // on Stop: drain notes and inject
  pub async fn observe_hookless(app: &AppHandle, repo: &Path) -> Result<()>;           // exactly one working hookless session on this shared repo → attributor.observe_dirty; else Observed claims without task
  ```
  `events.rs`: `pub struct OverlapChanged { pub task_id: TaskId, pub count: u32, pub pending: u32 }` (`overlap-changed`). `AppState` gains `pub pinned: Mutex<HashMap<TaskId, AgentState>>` holding the derived state kept aside while pinned.
  `hook_feed.rs`: before `feed::next_state`, `ClaimDenied` → `on_claim_denied`; the computed `next` passes through `overlap::pinned(pending_count, next)`; after `save_attributions` on an edit → `rebuild_claims` for the file's repo when shared, then `rebuild_overlaps` for every active task on that repo. `git_watcher.rs::on_settled`: `rebuild_claims` for shared repos, `observe_hookless`, and `done_paths(…, &taken_over)` from `store::taken_over_paths`. Startup: rebuild claims for every shared repo of every active task. Fresh session and archive: `store::clear_pending`, drop the pin, delete the grants file.

- [ ] **Step 1: Core tests** named above. Run — FAIL. Implement `overlap::pinned`. Run — PASS.
- [ ] **Step 2: Shell:** implement `overlap_sync.rs` and the wiring. Register the event in `specta::builder()`; run `npm run bindings`.
- [ ] **Step 3: Verify:** `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run bindings:check` — all pass. Manual: with two tasks on one checkout in `npm run tauri dev`, the second's `Edit` of a file the first changed shows the denial in its terminal and the card goes *File in use*.
- [ ] **Step 4: Commit** — `feat(shell): claims and overlaps sync, ClaimDenied hand-back with state pin, hookless attribution`

---

### Task 11: Overlap commands and the decision flow

**Files:**
- Create: `src-tauri/src/commands/overlap.rs`
- Modify: `src-tauri/src/specta.rs`, `src-tauri/src/commands/mod.rs`, `src/lib/bindings.ts` (generated)
- Test: core: `overlap::decide::outcome` pure function (new small file `overlap/decide.rs`) with tests

**Interfaces:**
- Core (`overlap/decide.rs`):
  ```rust
  pub struct DecisionOutcome { pub grant: Option<Grant>, pub relation: Relation, pub my_prompt: Option<String>, pub other_prompt: Option<String>,
      pub my_timeline: String, pub other_timeline: Option<String>, pub taken_over: bool, pub hold: bool }
  pub fn outcome(decision: Decision, row: &OverlapRow, my_title: &str, other_title: &str, now: DateTime<Utc>) -> DecisionOutcome;
  ```
  Tests: `on_top_file_grants_the_path_and_prompts_both`, `on_top_all_grants_all_files`, `replace_marks_taken_over`, `hold_grants_nothing_and_keeps_pending`. Add `pin_for(rows: &[OverlapRow], titles: &HashMap<TaskId, String>) -> Option<HandbackReason>` in `overlap/mod.rs` beside `pinned`, with tests `all_held_pins_holding_for`, `any_unheld_pins_file_in_use_with_the_count`, `none_pending_pins_nothing`.
- Shell commands (all `#[tauri::command] #[specta::specta]`, ids re-parsed with `TaskId::parse`):
  ```rust
  pub fn overlap_list(state, task_id: TaskId) -> Result<Vec<OverlapRow>>;
  pub async fn overlap_decide(app, task_id: TaskId, repo: PathBuf, path: String, other_task: Option<TaskId>, decision: Decision) -> Result<()>;
  pub fn overlap_pending_input(state, task_id: TaskId, repo: PathBuf, path: String) -> Result<Option<PendingDecision>>;
  ```
  `overlap_decide`: `outcome` → write grant (`read_grants` + `add` + `write_grants`), `set_relation`/`set_pending(None)` (Hold keeps the row pending with `held = true`; when every pending row of the task is held the pin becomes `NeedsYou { HoldingFor { other } }`, else it stays `FileInUse`), timeline rows (`TimelineKind::Overlap`) and `OverlapDecided` log lines in both tasks' event logs, taken-over path moved (`changes::store` of the other task loses it, mine gains it with `source: Manual`), `events::emit_task_changed`, inject `my_prompt` to my session and push `other_prompt` to the other task's notes inbox plus `deliver_notes_if_idle`. When `pending` reaches 0 the pin is released and the kept-aside state applied. Hold: when `rebuild_claims` later finds the claim gone, the shell clears that pending row and injects *"the file is now free"* with the `OnTopFile` go-ahead.

- [ ] **Step 1: Core tests** → FAIL → implement `decide.rs` → PASS.
- [ ] **Step 2: Shell:** implement the commands; register in `collect_commands!`; `npm run bindings`.
- [ ] **Step 3: Verify:** `cargo test --workspace`, clippy, `npm run bindings:check`.
- [ ] **Step 4: Commit** — `feat: overlap_list, overlap_decide and overlap_pending_input; the decision flow`

---

### Task 12: Integration from the Done card

**Files:**
- Modify: `src-tauri/src/commands/overlap.rs`, `src-tauri/src/hook_feed.rs`, `src-tauri/src/state.rs` (`integrating: Mutex<HashSet<TaskId>>`), `src-tauri/src/specta.rs`
- Core: `crates/fanoutcode-core/src/overlap/integrate.rs` (pure pieces) with tests

**Interfaces:**
- Core:
  ```rust
  pub fn conflict_ctx(path: &str, other: &Task, spec_md: &str, plan_md: &str) -> ConflictCtx;   // first `## ` section of spec.md; `## Files` entry and steps mentioning the path from plan.md
  pub fn integration_result(message: &str, has_conflicts: bool) -> IntegrationResult;         // enum { Done, Conflicted, StillWorking }
  ```
  Tests: `conflict_ctx_excerpts_spec_and_plan`, `integration_result_rules` (sentinel + clean → Done; conflicts → Conflicted regardless of sentinel; neither → StillWorking).
- Shell commands:
  ```rust
  pub async fn overlap_integrate(app, task_id: TaskId) -> Result<()>;      // Precondition "Confirm the base branch first." when base_branch is None; else build ConflictCtx per overlapping path, set Working, mark integrating, inject prompts::integrate
  pub async fn overlap_set_base(app, task_id: TaskId, branch: String) -> Result<Task>;
  pub fn overlap_base_suggestion(state, task_id: TaskId) -> Result<Option<String>>;   // git::default_branch of the home root
  pub async fn overlap_resume(app, task_id: TaskId, instruction: String) -> Result<()>;
  pub async fn overlap_abort(app, task_id: TaskId) -> Result<()>;           // injects "Run `git merge --abort` or `git rebase --abort`, leave the branch as it was, then say FANOUTCODE: integrate done"
  ```
  `hook_feed.rs`: for a task in `integrating`, a main-agent `Stop` runs `integration_result(last_assistant_message, git::has_conflicts(worktree).await?)`: `Done` → Idle, timeline `Overlap { detail: "integrated <base>" }`, if `worktree_integration == Rebase` re-record `RepoBase` for the home repo as `merge_base(base, HEAD)`, unmark; `Conflicted` → `NeedsYou { MergeNeedsYou { repo, paths } }` with the conflicted paths from status; `StillWorking` → Working. The normal feed is skipped for that event.

- [ ] **Step 1: Core tests** → FAIL → implement → PASS.
- [ ] **Step 2: Shell:** commands, state, feed branch; register; `npm run bindings`.
- [ ] **Step 3: Verify:** `cargo test --workspace`, clippy, `npm run bindings:check`. Manual: a Done worktree task, `Bring main in` with a seeded conflict → card shows *Merge needs you*; `Abort` clears it.
- [ ] **Step 4: Commit** — `feat: bring the base branch into a worktree task from its Done card`

---

### Task 13: "Others working here", soft notes, new-task notes

**Files:**
- Modify: `crates/fanoutcode-core/src/workspace/launch.rs::fill_prompt_ctx` or the shell's Plan-prompt assembly (wherever `PromptCtx.task_additions` is filled), `src-tauri/src/hook_feed.rs` (soft notes on edits), `src-tauri/src/overlap_sync.rs` (new-task note), `src-tauri/src/commands/tasks.rs` (plan.md footprint parse on Plan `Stop`)
- Core: `overlap::others::summaries(conn, me: &Task, others: &[Task], files_of: impl Fn(&TaskId) -> Vec<AttributedFile>, footprints: &[FootprintRow], branches: &HashMap<TaskId, String>, read_file: impl Fn(&Path) -> Option<String>) -> Vec<OtherSummary>` in a new `overlap/others.rs` with tests

**Interfaces:**
- Consumes: `prompts::others_working_here`, `soft_note`, `new_task_note`, `footprint::parse_files_section`, `notes::push_note`.
- Behaviour: entering Plan or Build appends `others_working_here(summaries)` to `task_additions` when non-empty (§6.1: the section is kept current for Build too). On a Plan-stage main `Stop` with the sentinel, parse `plan.md`'s `## Files` → `footprint::replace_plan` for the home repo (paths relative to it). On a `PostToolUse` edit by task A whose path is in active task B's footprint and `!shares_tree(A, B, repo)`: once per direction (`noted_out`/`noted_in`), push `soft_note` to A's inbox and the mirror to B's, `deliver_notes_if_idle` both. When a task enters Plan, push `new_task_note` to every other active task's inbox on the same project.

- [ ] **Step 1: Core tests** (`others.rs`): `summaries_skip_me_and_archived`, `summaries_carry_intent_first_line_planned_touched_added_and_exports`. FAIL → implement → PASS.
- [ ] **Step 2: Shell wiring.** Verify with `cargo test --workspace` and clippy; manual: a second task's Plan prompt (visible in its terminal) ends with the "Others working here" section.
- [ ] **Step 3: Commit** — `feat: others working here in the Plan prompt, footprints from plan.md, soft notes between worktree tasks`

---

### Task 14: Webview overlap store

**Files:**
- Create: `src/features/overlap/types.ts`, `src/features/overlap/store.ts`, `src/features/overlap/store.test.ts`, `src/features/overlap/index.ts`

**Interfaces:**
- Consumes: `commands.overlapList`, `commands.overlapDecide`, `commands.overlapPendingInput`, `commands.overlapIntegrate`, `commands.overlapSetBase`, `commands.overlapBaseSuggestion`, `commands.overlapResume`, `commands.overlapAbort`, `events.overlapChanged`; types `OverlapRow`, `Relation`, `Decision`, `PendingDecision` from bindings.
- Produces:
  ```ts
  export type OverlapState = { byTask: Record<TaskId, OverlapRow[]>; counts: Record<TaskId, { count: number; pending: number }> };
  export const overlapStore: Store<OverlapState>;
  export async function refreshOverlaps(taskId: TaskId): Promise<void>;
  export function startOverlapEvents(): () => void;             // overlap-changed → counts + refresh when the task is loaded
  export async function decide(taskId: TaskId, row: OverlapRow, decision: Decision): Promise<void>;
  export function overlapCount(s: OverlapState, taskId: TaskId): number;
  export function pendingRows(rows: OverlapRow[]): OverlapRow[];
  export function viewOverlapCount(view: View, tasksById: Record<TaskId, Task>, s: OverlapState): number;
  ```

- [ ] **Step 1: Tests** (`store.test.ts`, `vi.mock("@/lib/bindings", () => mockBindings())`): `refresh loads rows and counts`, `overlap-changed updates counts and refetches a loaded task`, `decide calls overlapDecide with the row's keys and refreshes`, `viewOverlapCount sums distinct tasks of the view`.
- [ ] **Step 2: Run** `npx vitest run src/features/overlap` — FAIL. **Step 3: Implement.** **Step 4:** PASS; `npm run typecheck`, `npm run lint`.
- [ ] **Step 5: Commit** — `feat(ui): overlap store`

---

### Task 15: The Overlap dock tab plumbing

**Files:**
- Modify: `src/features/dock/types.ts` (`DockTab` adds `"overlap"`), `src/features/dock/tabRules.ts` (`visibleTabs(stage, hasTestCommand, hasOverlap)`, `DOCK_TABS` entry `{ id: "overlap", label: "Overlap" }`), `src/features/dock/components/DockTabStrip.tsx` (`tones?: Partial<Record<DockTab, Tone>>` passed as `countTone`), `src/features/dock/components/Dock.tsx` (body case, counts, tone warn when pending > 0, open-on-pending once), `src/features/shell/types.ts` if it mirrors `DockTab`
- Test: `tabRules.test.ts`, `Dock.test.tsx`

**Interfaces:**
- Consumes: `overlapStore`, `overlapCount`, `pendingRows` (Task 14); `OverlapTab` (Task 16, stub it as `EmptyState` in this task and replace in the next).
- Produces: the tab appears whenever `overlapCount > 0`, in every stage; its count is the overlap count; `countTone` is `"warn"` when any row is pending, else `"neutral"`; when `pending` goes from 0 to >0 the dock selects `"overlap"` once (like Commit's default tab, without overwriting the remembered tab permanently).

- [ ] **Step 1: Tests:** `tabRules`: `overlap tab shows only with overlaps, in every stage`; `Dock.test.tsx`: `shows Overlap with the count and amber while pending`, `selects Overlap once when a decision becomes pending`.
- [ ] **Step 2–4:** FAIL → implement → PASS; typecheck, lint.
- [ ] **Step 5: Commit** — `feat(ui): Overlap dock tab plumbing`

---

### Task 16: The Overlap tab: file list and decision pane

**Files:**
- Create: `src/features/overlap/components/OverlapTab.tsx`, `OverlapFileList.tsx`, `DecisionPane.tsx`, `OverlapTab.test.tsx`
- Modify: `src/features/dock/components/Dock.tsx` (use the real tab), `src/features/overlap/index.ts`

**Interfaces:**
- Consumes: `commands.gitDiff(taskId, repo, path)` for the other task's changes (call it with the *other* task's id so the diff is against its base), `commands.overlapPendingInput`, `DiffEditor` from `@/features/changes` is not reused directly (its props are editor-bound); render both sides with `MiniMarkdown` fenced diff blocks or a read-only two-column `<pre>` through a small `DiffText` primitive if none exists (check `src/components/ui` first; add one under the primitive rules if needed).
- Produces: `OverlapTab({ task, projects })`: left `OverlapFileList` grouped *Needs you* / *Also touched by others*, each row `path`, other title, stage, state dot, relation label (`claimed`, `granted on top`, `taken over`, `planned by`, `committed`; `holding` when `row.held`); right `DecisionPane({ task, row })` with the heading *"<Other> is also changing <path>"*, the two boxes (*<Other>'s changes · +a −r*, *What <Task> will write*), the four choices as a `RadioGroup` primitive (use it if `src/components/ui` has one; otherwise add it under the primitive rules, with its test, barrel entry, `PRIMITIVES` entry and example page) with the §5.4 labels and sub-lines, the footer line *"Whatever you choose, <Other>'s session is told."*, **Open in Changes** (`requestDockTab(task.id, "changes")`) and **Continue** (calls `decide`). Rows without a pending decision show the settled relation and the decision history (from `OverlapRow.decided_at` and relation).

- [ ] **Step 1: Tests:** `lists pending first with the other task's name`, `renders both sides and the four choices for a pending row`, `Continue calls overlapDecide with the selected decision`, `a settled row shows its relation and no choices`, `Replace is shown in the danger colour and needs no dialog`.
- [ ] **Step 2–4:** FAIL → implement (each component under 200 lines) → PASS; typecheck, lint.
- [ ] **Step 5: Commit** — `feat(ui): Overlap tab with the file list and the decision pane`

---

### Task 17: Integration UI: Done card action, merge hand-back, base confirmation

**Files:**
- Create: `src/features/overlap/components/IntegratePane.tsx`, `BaseBranchDialog.tsx`, tests
- Modify: `src/features/task-page/primaryAction.ts` (+ test), `src/features/task-page/components/OverflowMenu.tsx`, `src/features/task-page/components/TaskPage.tsx`, `src/lib/format.ts` (`handbackLabel`)

**Interfaces:**
- `primaryAction.ts`: `PrimaryIntent` gains `{ kind: "dock-tab"; tab: DockTab }`; `FileInUse` → `button("Decide", { kind: "dock-tab", tab: "overlap" })`; `MergeNeedsYou` → `button("Resolve merge", { kind: "dock-tab", tab: "overlap" })`; Done stage keeps **Archive** as primary. `OverflowMenu` gains **Bring <base> in** (label uses `task.base_branch ?? "base"`) when `task.stage === "Done" && task.worktree_path !== null`, calling `onIntegrate`. `TaskPage` handles the `dock-tab` intent (open dock, `requestDockTab`) and `onIntegrate`: `overlapBaseSuggestion` → if `task.base_branch` is null open `BaseBranchDialog` (confirm or edit, then `overlapSetBase`) → `overlapIntegrate`. `IntegratePane({ task })` renders inside `OverlapTab` when the task state is `MergeNeedsYou`: each conflicted path with both tasks' intent excerpts from a new command `overlap_integration_context(task_id) -> Vec<ConflictCtx>` (the shell keeps the `ConflictCtx` list it built for the integrate prompt in `AppState.integrating: Mutex<HashMap<TaskId, Vec<ConflictCtx>>>`, replacing Task 12's `HashSet`), **Resume** with a `Textarea` instruction, **Abort**. `handbackLabel` in `src/lib/format.ts` maps `FileInUse`, `HoldingFor` and `MergeNeedsYou` to the chip texts.

- [ ] **Step 1: Tests:** `primaryAction.test.ts`: `file in use gives Decide opening the overlap tab`, `merge needs you gives Resolve merge`; `OverflowMenu` test: `Done worktree task offers Bring main in`; `IntegratePane.test.tsx`: `Resume sends the instruction`, `Abort calls overlapAbort`; `BaseBranchDialog.test.tsx`: `confirms the suggestion and calls overlapSetBase`.
- [ ] **Step 2:** Add `overlap_integration_context` to the shell (Task 12's module), register, `npm run bindings`.
- [ ] **Step 3–5:** FAIL → implement → PASS; typecheck, lint, `npm run bindings:check`.
- [ ] **Step 6: Commit** — `feat(ui): Bring base in from the Done card, merge hand-back pane, base branch confirmation`

---

### Task 18: Counts everywhere

**Files:**
- Modify: `src/features/board/components/BoardCard.tsx` (+ test in `CardParts.test.tsx` or a new `BoardCard.test.tsx`), `src/features/shell/components/TabItem.tsx` and `TabStrip.tsx` (+ test), `src/features/sidebar/components/ChangesSection.tsx` (+ test), `src/features/changes/components/FileList.tsx` (+ test)

**Interfaces:**
- Consumes: `overlapStore`, `overlapCount`, `viewOverlapCount`, `startOverlapEvents` (started once in `ShellLayout` beside `startTasksEvents`).
- Produces: board card shows a small neutral `Badge` *"N overlaps"* (singular *1 overlap*) beside the state chip when N > 0; the view tab tooltip appends *" · N overlaps"*; the sidebar Changes section and the Changes tab's `FileList` show a relation `Chip` (`size="sm"`) on files that have an `OverlapRow` for that task and path.

- [ ] **Step 1: Tests:** `card shows the overlap badge only when overlaps exist`, `view tab tooltip carries the overlap count`, `changes section marks a file another task touches`, `FileList shows the relation chip`.
- [ ] **Step 2–4:** FAIL → implement → PASS; typecheck, lint.
- [ ] **Step 5: Commit** — `feat(ui): overlap counts on cards, view tabs and change lists`

---

### Task 19: Settings controls

**Files:**
- Modify: `src/features/settings/components/ProjectSettings.tsx` (+ `ProjectSettings.test.tsx`)

**Interfaces:**
- Consumes: settings keys from Task 1 through `settingsStore` / `commands.settingsSet(scope, key, value)` as the other project settings do.
- Produces: a **Working together** group with `Switch` *Ask before editing another task's files*, `Switch` *Guard git against other tasks' uncommitted work*, `Segmented` *Integration* Merge | Rebase, each saving its key at project scope.

- [ ] **Step 1: Tests:** `renders the Working together group with defaults`, `toggling saves the key at project scope`.
- [ ] **Step 2–4:** FAIL → implement → PASS; typecheck, lint (component stays under 200 lines; split a `WorkingTogetherSection.tsx` if it would not).
- [ ] **Step 5: Commit** — `feat(ui): Working together project settings`

---

### Task 20: Team view rows for overlap events, docs

**Files:**
- Modify: `src/features/canvas/` event-stream model where `HookEvent` variants map to rows (find the switch on `event.type`; `Notification` rows are the model), plus its test
- Modify: `ARCHITECTURE.md` (overlap module and sync, one paragraph each layer), `CLAUDE.md` "Current stage" (Plan 11 built), `docs/superpowers/brainstorm/2026-10-24-decisions.md` (the §10 calls and Johnny's three decisions: warn-then-decide, two grains, commit-first-then-integrate)

- [ ] **Step 1: Test:** `event stream shows OverlapDecided and OverlapNoted as muted rows with the path and the other task`; `ClaimDenied` as a muted row *"edit held: <path>"*.
- [ ] **Step 2–4:** FAIL → implement → PASS; typecheck, lint.
- [ ] **Step 5:** Docs edits; `npm run lint` (Prettier covers markdown if configured).
- [ ] **Step 6: Commit** — `feat(ui): overlap events in the Team view; docs for Plan 11`

---

## Self-review notes

- Spec coverage: §5.1 Task 3; §5.2 Task 4; §5.3 Task 5; §5.4 Tasks 6, 10, 11, 15, 16; §5.5 Tasks 5 (inbox), 11; §5.6 Task 5; §5.7 accepted, no task; §6.1 Tasks 8, 13; §6.2 Task 13; §6.3 Tasks 9, 12, 17; §7 Tasks 15–18, 20; §8 Tasks 1, 19; §9 Tasks 1, 2, 6, 10, 11.
- Names used across tasks: `OverlapRow`, `Relation`, `Decision`, `PendingDecision`, `ClaimsFile`, `GrantsFile`, `CheckArgs`, `OtherSummary`, `ConflictCtx`, `overlapStore`, `refreshOverlaps`, `decide`, `requestDockTab`, `OverlapChanged`; the commands `overlap_list`, `overlap_decide`, `overlap_pending_input`, `overlap_integrate`, `overlap_set_base`, `overlap_base_suggestion`, `overlap_resume`, `overlap_abort`, `overlap_integration_context`.
- Open to the builder (spec §11): `additionalContext` support per hook event (Task 5 delivers notes on `PreToolUse`; Task 10 falls back to idle injection), export-name extraction detail, the two-column diff rendering in Task 16, exact Rust test fixtures.
