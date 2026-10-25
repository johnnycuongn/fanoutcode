# Plan 08 report: Task page and dock

**Status: DONE_WITH_CONCERNS.** All 19 tasks are done, one commit each, plus one ARCHITECTURE.md commit. Every gate
passes. `git rebase main` was a no-op: main is still `c291dbd` (Plan 10 has not merged).

Concerns:
- Nothing ran in the GUI: the real terminal and dock layout, CodeMirror in WKWebView, the opener plugin, and the
  live events are untested outside jsdom.
- The Ticket tab goes through a `STUB(plan 10)` wrapper (`src/features/dock/jira.ts`). It does nothing useful until
  Plan 10's `jira_*` bindings land.
- Several plan shapes did not match what Plans 01, 03 and 05 built. I adapted the code and the plan's tests to the
  real bindings (Rulings 1–9).

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-08`, branch `plan-08`, not merged.
- Commit range `c291dbd..2d43034`: 20 commits, `600a73e` (Task 1) through `16c773a` (Task 19), then `2d43034`
  (ARCHITECTURE.md).
- Ledger: `.superpowers/sdd/plan-08-progress.md`.

## Gate results (final run on `16c773a`/`2d43034`)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 128 passed (128)`, `Tests 762 passed (762)` (main had 94 / 557) |
| `cargo test --workspace` | livespec-core 240, teamcode-core 317, shell 13 (+1 ignored), 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 4.02s`, no chunk-size warning (CodeMirror languages load lazily) |
| `npm run bindings:check` | exit 0 (no Rust commands added) |
| lefthook pre-commit | passed on every commit; `--no-verify` never used |

## STUB(plan 08) replaced

`src/features/task-page/index.ts` (Plan 06's stub) now exports the real `TaskPage`, `TaskPageProps` and
`TaskPageHandle`, at the same path and with the same props (A5). `grep -rn "STUB(plan 08)" src` is empty.

## STUBs added

- `src/features/dock/jira.ts`, `STUB(plan 10)`:
  - `jiraIpc.jiraIssue`, `jiraComment` and `jiraSearch` look up `commands.jiraIssue`, `jiraComment` and
    `jiraSearch` at call time.
  - Until Plan 10 lands, issue and comment reject with "Jira is not available yet." and search returns `[]`.
  - It also defines a STUB `Issue` type with Plan 10's field names. `key`, `summary`, `status`, `assignee` and `url`
    are required; `issue_type`, `description` and `updated` are optional.
  - Swap: delete the file and use `live("jiraIssue", "jiraComment", "jiraSearch")` with the generated `Issue`.
    Keep `jiraKeyPatch`, or move it elsewhere.

## Rulings

1. Ruling: Plan 01 never copied LiveSpec's webview files into `src/livespec/`. I copied them from the LiveSpec
   checkout at `c55834b`, the commit `livespec-core` came from (`~/Projects | Software/margin`).
   - Copied: `MiniMarkdown`, `DraftPane`, `NotesPane`, `sections`, `lanes`, `checks`, `clipboard`,
     `analysisState` and `saver`, with their tests.
   - Not copied: `tabSession`, `useTabSession`, `history` and `tabStatus`. Task 3 deletes them, so they were never
     brought in.
   - Cost if wrong: none.
2. Ruling: the generated commands return `Result<T, string>`. They do not reject, which is what the plan
   assumed.
   - I added `src/lib/ipcResult.ts` with `unwrap`, `errorMessage` and `live(...names)`. `live` builds a command set
     over the generated `commands`, looked up at call time, that resolves to the value and rejects with the error
     string.
   - Every store's `ipc` parameter has that shape, so the plan's `vi.fn` fakes work unchanged.
   - Cost if wrong: one file.
3. Ruling: I adapted the code and tests to the real shapes.
   - `LiveSpecEvent` has capitalised variants: `Started`, `Text`, `Done { text, partial }`, `Check { result }`.
   - `FileDiff` is `{ repo_root, path, status, before, after, binary }`, not `{ base, working, binary }`.
   - `ChangedFile` has more fields than the plan lists.
   - The placement type is `TeamPlacement`, not `TeamViewPlacement`.
   - `TaskPatch` needs every field. The live wrappers build the full patch; the stores still call
     `tasksUpdate(id, { jira_key })`.
4. Ruling: `RepoChanges` has no `home` flag. The home repo is the first entry, because Plan 05's `repos_of` puts
   the worktree or project root first.
   - Plan 03's Review prompt writes `<project folder>/<path>` for every repo. So the Dock passes `homeName` (the
   project root's folder name), which stands in for a worktree's own folder name.
   - Cost if wrong: notes on a worktree task show under "outside the change set".
5. Ruling: Plan 03's Review prompt writes `blocker`, `major`, `minor` and `nit`, sometimes in backticks.
   - The parser maps these onto the plan's five severities: blocker → critical, major → high, minor → medium,
     nit → low.
   - It also accepts backticked lines.
   - One extra test covers this.
6. Ruling: the Commit tab reads `git_drafts`, Plan 05's Rust parser of `commits.md` and the same one `git_commit`
   uses. It does not run a TypeScript `parseCommits`.
   - The real file uses absolute repo roots, `### subject` headings and backticked `Files:`, so the plan's grammar
     was wrong.
   - `commits.ts` now holds `draftsFrom` and `commitSummary`, and the tests were adapted to them.
7. Ruling: a missing memory folder makes `memory_read` return `null`, not a `Precondition` error.
   - On `null`, the Spec store and the Memory tab ask `memory_missing({Project})`.
   - If the task is listed, they show `Task memory folder is missing: <memory_dir>` (the Rust wording) and turn
     editing off.
   - A rejection is still shown verbatim. This keeps Review Focus 3.
8. Ruling: Plan 05's `team-event` never carries `StatusLine`. The context meter also reads
   `CostSummary.live.context_used_pct` on each `cost-updated`.
9. Ruling: I adapted some test details. Every assertion is the same.
   - Radix menus open with a key, not a synthetic click, so the TaskHeader test uses `userEvent` Enter.
   - MemoryTab fakes return `contents of <file>`, because MiniMarkdown renders `# x` as a heading `x`.
   - Two TaskPage tests wait for the async spec and settings loads before acting.
   - Cross-realm `Uint8Array`: the Test store uses `ArrayBuffer.isView`.
   - `vi.hoisted` holds the mocked commands.
10. Ruling: shared test builders live in `src/test/taskPageFixtures.ts`, because feature boundaries forbid
    importing another feature's internals. `task-page/testing/fixtures.ts` re-exports them under the plan's path.
11. Ruling: the plan's `Dock` has no body, so `DockSplit({ stage, position, open, body, dock })` holds the body and
    the dock in one `Resizable`.
    - A drag sets the height. `toggleGrow` and `enterStage` animate it with motion's `animate` and
      `transitionFor(reduced)`.
    - `DockProps` gained an optional `projects`, which the Test and Changes tabs need.
12. Ruling: the plan says Plan 04 needs the page to open the session.
    - `TaskPage` calls `sessions_open` (idempotent) for Plan through Commit on non-archived tasks.
    - It refetches the task if the id it gets back differs from `primary_session`.
    - `TerminalPane` is keyed by `primary_session`.
13. Ruling: `dock/memory.ts` and `spec-editor/store.ts` behaviour.
    - A stage change while the Memory tab is dirty or editing keeps the open file and buffer (Review Focus 5). A
      test covers it.
    - The spec store's `open` keys off store state, not its session map, so `specStore.store.reset()` reopens
      cleanly.
14. Ruling: I added `tauri-plugin-opener` (Rust and JS), its `init()` in `lib.rs`, and the capability
    `{ "identifier": "opener:allow-open-url", "allow": [{ "url": "https://*" }] }` (contract addition 9). They are
    for "Open in Jira".
15. Ruling: the `Changes` tab count badge refreshes when the dock opens and while the Changes tab listens to
    `git-changes`. It does not refresh from a window-wide listener.
16. Ruling: theme additions. The `animate-fill` (notes rule) and `animate-breathe` (current stage chip) loops are
    in `src/styles.css`. Reduced motion stops them after one pass.
17. Ruling: these behaviours are additive.
    - The OverflowMenu's worktree item works through `tasks_update` before a session starts.
    - The primary button's split menu is named "Other ways to continue".
    - `TaskPageHandle.archive` restores an archived task.
18. Ruling: the red step was skipped for `body.ts` (Task 19), `ipcResult.ts` and the ported LiveSpec tests. I
    wrote those before running them. Every plan test file had a recorded failing run.

## Unverified (needs GUI)

- The full page in a real window: the terminal through `TerminalPane`, the dock split and its animated grow, and
  the right inspector.
- CodeMirror's merge view, highlighting and note portals in WKWebView. jsdom builds the editor and mounts the note
  card; the visual result is unchecked.
- `openUrl` through the opener plugin and its capability scope.
- Every `live` command against real Rust: `livespec_analyze` streaming, the `tests_run` channel framing (number
  arrays versus raw bytes; both are handled), `git_file_write`, `git_drafts`, and `sessions_open`.
- Events in a window: `team-event`, `git-changes`, `spec-watcher`, `test-finished` and `cost-updated`.

## Plan text that is wrong (not edited)

- Conventions: "a call resolves to its value and rejects": the bindings return `Result` (Ruling 2).
- Task 1/2/3: assume Plan 01 copied `src/livespec` (Ruling 1). Task 3's tests use LiveSpec's lowercase event types.
- Task 11: `# plan.md` fakes cannot be found as text after MiniMarkdown renders them (Ruling 9).
- Task 12: the severities and the location form differ from Plan 03's Review prompt (Ruling 5).
- Task 13: the `FileDiff` and `RepoChanges.home` shapes (Rulings 3 and 4).
- Task 18: the `commits.md` grammar does not match Plan 05's writer (Ruling 6).
- Review Focus 3: `memory_read` does not reject for a missing folder (Ruling 7).
- Task 7: StatusLine through `team-event` (Ruling 8).
- Task 8: Radix menus do not open on `fireEvent.click`.

## What the next plans need to know

- **Plan 09**: `TaskPage` renders `renderTeam?.(task)` in the dock's Team tab, and in the body under Swap.
  `onOpenTeamPane` is called for OwnPane (without it, ⌘⇧J falls back to DockGrows). Leave both props as they are.
  `@/features/dock` exports `Dock`, `DockSplit`, `dockStore`, `nextOnGrowShortcut` and `DOCK_TABS`.
- **Plan 10**: swap `src/features/dock/jira.ts` for the generated bindings (see STUBs). The Ticket tab reads
  `key, summary, status, assignee, url`. Expect merge conflicts in `package.json`, `package-lock.json`,
  `Cargo.toml`, `Cargo.lock`, `src-tauri/src/lib.rs` (opener plugin), `src-tauri/capabilities/default.json` and
  `ARCHITECTURE.md`. Keep both sides.
- **Plans 06/07**: `primaryActionFor`, `forwardActionFor` and `freshSessionAllowed` are exported from
  `@/features/task-page`. The page calls `consumeDockTab(taskId)` on open and sets `data-scope="spec-page"` on the
  Spec page root.
- `src/lib/ipcResult.ts` (`live`, `unwrap`, `errorMessage`) is available to any store that wants a fakeable command
  set.
