# Cleanup report: remove STUB(plan NN) stand-ins

Branch `cleanup-stubs` in /Users/johnnynguyen/Personal/TeamCode-wt/cleanup. One commit: e9449db (on top of 4af8efa). Not merged.

## Removed
1. `src/features/board/stubs/plan05.ts` (folder deleted). `board/ipc.ts` now calls `commands.tasksActivity`, `commands.costSummary`, `commands.gitChanges` and `events.teamEvent.listen`, with the same `unwrap` it already had. The two eslint-disable lines and the raw `invoke`/`listen` imports went with the file. `cost.ts`, `activity.ts`, `TaskCard.tsx`, `testing/fixtures.ts`, `activity.test.ts` import `CostSummary`, `HookEvent`, `RepoChanges`, `TaskEvent` from `@/lib/bindings`. The board's `team-event` payload type is the generated `TeamEvent`.
2. `src/lib/pendingCommands.ts` deleted. `ChangesSection`, `CostsPanel`, `ViewCostBadge` call `commands.gitChanges` / `commands.costSummary` and use the generated types; `src/test/fixtures.ts` uses the generated `CostSummary`.
3. `src/features/dock/jira.ts` deleted. `ticketStore.ts` now types its ipc as `Live<"jiraIssue" | "jiraComment" | "jiraSearch" | "tasksUpdate">` and builds it with `live(...)`. `jiraKeyPatch` moved into `ticketStore.ts` (the store's `link` now passes the full TaskPatch).
4. `src-tauri/src/effects.rs`: `Effect::RunTest => {}` with the one-line comment "No-op by design: the task page's Test tab owns the run through `tests_run`." `no_runner` was deleted, since it was only used there. Nothing logs the effect any more.

## Generated shapes differing from the stubs (call sites adapted, bindings.ts untouched)
- `ChangedFile` has `status, binary, owners, shared, source`; `RepoChanges` has `base` and `other`. Only the `activity.test.ts` "applyChanges" fixture needed the extra fields.
- `Issue` has required `issue_type, description, updated`; the `ticketStore.test.ts` fixture was updated.
- `TaskActivity` and `TeamEvent` use `TaskId` instead of `string`, which is compatible.

## Tests
No test deleted; no test subject was deleted. Edited tests: `activity.test.ts` (fixture), `ticketStore.test.ts` (fixture and the `tasksUpdate` expectation now uses `jiraKeyPatch("TC-12")`). The board tests keep mocking `board/ipc` and the dock store tests keep injecting a fake ipc, as the rest of the suite does. `ChangesSection.test.tsx` and `CostsPanel.test.tsx` pass unchanged, because they mock `commands.*` via the bindings mock.

## Remaining checks
- `grep -rn 'STUB(plan' src src-tauri crates` returns nothing.
- No `teamcode/no-manual-ipc` disables and no `invoke(` outside bindings remain. The leftover `@tauri-apps/api` imports (`Channel`, `getCurrentWindow`, `getCurrentWebview`) are not `invoke`/`listen` and are untouched.

## Gates (all pass)
typecheck clean; lint clean (eslint + prettier); vitest 165 files / 921 tests passed; `cargo test --workspace` all ok (240 core, 13 shell, rest 0 or ignored); clippy `-D warnings` clean; `npm run build` ok; `npm run bindings:check` exit 0 (no diff); e2e 3 passed (WebKit browsers copied from plan-09 into this worktree's node_modules, not committed). The lefthook pre-commit hook ran on the commit.
