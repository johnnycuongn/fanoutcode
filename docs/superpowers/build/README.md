# Team Code build record (2026-10-25)

All ten implementation plans in `docs/superpowers/plans/` were built by one agent per plan, in dependency order (01 | 02, 03 | 05, 06, 07 | 04 | 08, 10 | 09), each in its own git worktree, verified by the controller and fast-forward merged to `main`. A cleanup pass removed every temporary stub, a whole-codebase review found 0 Critical / 8 Important / 14 Minor issues, one fix wave fixed all 8 Important, and a scoped re-review confirmed them.

Final state of `main`: typecheck, ESLint + Prettier, Vitest 925/925, cargo test 712 passed / 0 failed, clippy `-D warnings`, `npm run build`, `bindings:check`, and the Playwright WebKit smoke suite 3/3 all pass. Nothing has been run in the real Tauri window.

## Files here

| File | What it holds |
|---|---|
| `controller-ledger.md` | The controller's running record: every merge, every verification run, and the controller's own Rulings, including all security rulings |
| `plan-NN-report.md` | Each plan executor's report: commits, gates, Rulings (decisions made where the plan was wrong or silent), stubs, deviations |
| `plan-10-security-review.md` | Independent whole-branch security review of workspaces and integrations |
| `final-review.md` | Whole-codebase review: fix list 0–22 and triage of every plan's deferred items |
| `final-fixes-report.md`, `final-fixes-rereview.md` | The fix wave for items 0–8 and its scoped re-review |
| `cleanup-report.md` | Removal of the cross-plan stubs |

## Decisions most worth the owner's review

1. **Parallelism followed dependencies**, not ten-at-once, because every plan imports earlier plans' code.
2. **Background sessions are No-go by default** (ADR 0004): not measurable without the GUI; Owned PTY is the only driver; branch `spike/background-driver` is kept unmerged for later measurement.
3. **Workspace files are untrusted input.** Ten security review rounds produced: id validation, no record hijack by id collision, a two-step preview → confirm import that imports the exact previewed bytes, refusal of roots that are `/`, `$HOME`, an ancestor of either, or inside app data; per-doc approval for every imported file, folder, URL and Confluence doc; warnings for credential-folder roots; protection decided by project id and failing closed; refusing sessions for protected folders whose names cannot be expressed safely in a deny rule.
4. **Context fetching**: loopback, link-local (incl. cloud metadata), unspecified and multicast always refused; private ranges only for user-added or confirmed docs; trust scoped to the vetted host; untrusted fetches bypass proxies and pin the resolved address; streamed size cap; per-container caches.
5. **Jira**: the token is never named by a setting, is bound to the site it was verified against, and only Link Jira can write the Jira setting; Jira links are not carried in workspace files.
6. **Settings live in SQLite** (ADR 0002), not JSON files as spec §14 said.
7. **Tasks are stored as a JSON blob plus indexed columns** (Plan 01's schema), which changed Plan 03's migration.
8. **Entering Test starts the test run** (final review reversed an earlier no-op ruling).

## Deferred (not fixed)

Minor items 9–22 in `final-review.md`, plus from the re-review: a card stays "working" when `claude` is not on PATH or after a deliberate Stop agent; ⌘S for workspace save may only fire with focus inside a sidebar input; save-on-close overwrites an unanswered reload offer; a Test tab holding an old run does not attach to a newer one; the native menu also adds a menu bar on Windows/Linux; the contracts doc does not list `test-started` or `tasks_draft_commits`; a stale `@ts-nocheck` comment in `src-tauri/src/specta.rs`.

## Not verified (needs the running app)

`tauri dev` itself; terminal bytes and resize in the webview; quit dialog and `--resume` after relaunch; a 50 MB output burst; moving a session between windows; live hook delivery and replay; file watchers and Done detection; keychain reads and writes; real model calls; real Jira/Confluence; pointer drag on the board and canvas at 60 fps; Lottie playback; native title, Dock badge, OS notifications and file dialogs; every §10.5 keyboard chord against the native menu; whether Claude Code's `Edit(//root/**)` deny rules fold case and resolve symlinks (test against the installed `claude` before release); CI on GitHub (no remote).
