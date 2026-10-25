# Plan 10 report: Workspaces and integrations

**Status: DONE_WITH_CONCERNS.** All 20 tasks are done, plus 11 security fix commits that the coordinator asked for
during the run (ten review rounds). Every gate passes after the rebase on main (`2d43034`, which includes Plan 08).
Not merged.

Concerns:
- Nothing that needs the GUI ran: the import dialog, the settings screen, pickers, drag-drop into Docs, the watcher's
  reload banner in a window, a real keychain prompt, and any live Jira or Confluence (all clients are tested only
  against wiremock or a local chunked TCP server; no real Jira was contacted).
- Several plan interfaces changed because of the security rulings (listed under Deviations). Plan 08 runs against
  the final shapes already, through its own runtime-lookup stub.
- Red steps: Task 2 and Task 11 were written test-and-implementation together (the first security interruption
  landed mid-Task 2), so their failing runs were not observed separately. Every other task recorded its failing run.

- Worktree `/Users/johnnynguyen/Personal/TeamCode-wt/plan-10`, branch `plan-10`.
- Commit range after the rebase: `ca95465`..`ac64743`, 32 commits on top of main `2d43034` (20 task commits, 11
  `fix(workspace|integrations)` commits, 1 ARCHITECTURE.md commit).
- Ledger: `.superpowers/sdd/plan-10-progress.md` (SHAs in it are from before the rebase).

## Gate results (final run on `ac64743`, after the rebase)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0 |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 138 passed (138)`, `Tests 795 passed (795)` |
| `cargo test --workspace` | teamcode-core `445 passed`, shell `13 passed; 1 ignored`, livespec-core `240 passed`, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 4.22s` (the existing "settings/index.ts is dynamically imported … also statically imported" Rollup note is Plan 06's SettingsSection; not new) |
| `npm run bindings:check` | exit 0 |
| Guard smoke | `printf '{…rm -rf /w/api…}' \| cargo run -q -p teamcode -- guard /dev/null` → one stderr line, no stdout, `exit=0` |

New tests by area: workspace::file 7, store 7, open 9, import 9, watcher 2 (stable over 4 runs), link 13, policy 8,
guard 13 (incl. tokeniser), fragments 5, briefs 1, context_docs 6, context_fetch 11, url_guard 3, launch 5, convert 4;
jira:: client 8, adf 4, intent 1, cache 1, settings 6, confluence 6; webview: workspace feature 10 files / 33 tests.

## Rulings

SHAs inside rulings are from before the rebase. The fix commits after it: `9a5874c` (ids), `459b1a5` (import review),
`3ff2a1c` (buffer, ancestors), `75d1aba` (protection by id), `f04002c` + `8200a48` (HTML, caps), `9100243` (SSRF),
`3d365a3` (per-container cache, file reads), `520b8ab` (trust per host, proxy), `3e839bc` (per-doc approval),
`86d5b4f` (Jira token binding). Where an early ruling's signature was later changed, the later ruling and the
Deviations section are current.

- Ruling (first review, id sites checked): every place an id from a file, the webview or a log becomes a path is
  re-parsed: workspace file id (`WorkspaceFile::parse`, `default_file_path`), context doc ids (`parse`,
  `text_cache_path`, plus fresh ids on import), container ids in cache dirs (`context_dir`), guard roots file task id
  (`write_roots_file`), brief path project id (`brief_path`), Jira keychain accounts (`account_for`), Jira issue keys
  in URL paths (`valid_issue_key`), Confluence page ids (digits only), every command id via `parse`. The Jira issue
  cache never builds a path. Cost if wrong: none.
- Ruling: security fix 4555ab2 (coordinator request) — WorkspaceFile::parse re-validates the workspace id and every context doc id; default_file_path returns Result and refuses ids failing valid_id.
- Ruling: test ids use Plan 01's prefixes and `parse` (`p-1`, `ws-a`, `t-1`); the plan's `WorkspaceId::from`, `proj-1`, `ws-ext`, `task-1` do not exist or fail the prefix/hex check.
- Ruling: `notify = "8"` (already in Cargo.lock via the shell), not "7".
- Ruling: `db::Db = Arc<Mutex<Connection>>` is new (Plan 03 has no handle type); AppState.db becomes this type.
- Ruling: 0010 adds `workspaces.dirty` (Plan 01's table had no column for it) and drops/recreates 0001's never-written `jira_links` with the issue-cache shape.
- Ruling (controller security rulings 1–3, commit after ff4a519): workspace import is `preview_open(path) -> WorkspaceImportPreview` + `confirm_open(path, token)`; `open_file(path)` survives only as the trusted reopen of the record already at that canonical path when the file adds nothing (no new root, agent, instruction text, reference; no loosened policy/protection). Id collision with a record elsewhere → fresh WorkspaceId, record marked dirty so the next save writes the new id, nothing inherited (jira not carried). Roots not an existing dir, `/`, home itself, inside app_data/app_config block confirm. sanitize() drops bad agent names (store's lower-case/digit/dash rule; roster has no name rule), non-prompting stage keys, settings failing validate(Workspace) plus `jira`, relative File/Folder docs; name cleaned (fallback: file stem). Trusted reopen skips root checks so a vanished folder shows as Missing instead of refusing the app's own workspace. reload/relocate_file refuse when the file adds anything (UI then runs preview/confirm). Cost if wrong: an extra review step for the developer's own files edited by hand; a trusted reopen still trusts a file the app wrote earlier even if another user's identical content arrives at the same path.
- Ruling (third review, HIGH 1): a root is refused when it is `/`, equal to or an ancestor of $HOME, app_data or app_config, or inside app_data/app_config; folders inside $HOME pass. Cost if wrong: a developer who keeps a repo at an ancestor of app data cannot add it (none plausible).
- Ruling (third review, 2): confirm_open(token) imports the exact buffer preview_open captured (keyed by a random 128-bit getrandom token, 10-minute TTL, single use) and refuses when any root re-canonicalises differently. Signature is now `confirm_open(store, token, env)` (the plan/controller's `path` argument dropped: the path is captured too). Cost if wrong: a preview older than ten minutes must be redone.
- Ruling (third review, 3): the no-dialog trust check compares docs on kind+target+label+page_id (not id/fetched_at/order), agents name+body, fragments stage+text; preview exposes `docs: Vec<ImportDoc{kind,target,label,page_id}>` (replaces `doc_targets`). Imported docs get fresh ids. Cost if wrong: more dialogs after a teammate renames a reference.
- Ruling (third review, 4): FNV `token_of` removed; nothing uses a content hash for integrity, so no sha2/ADR needed. Cost if wrong: none.
- Ruling (fourth review, 1): `LinkBinding` gains `links: Vec<BoundLink { project_id: Option<ProjectId>, root }>`, persisted in a new `task_links` table (added to my unmerged 0010 rather than a new `Task` field, so Plan 08's TS Task fixtures are untouched). `protected_roots(home, workspace, bound: &[BoundLink])` (the plan's `projects`/`bound_roots` parameters replaced) decides by id and fails closed. `links_for(task.linked_roots, stored)` pairs roots with stored ids (`same_root`: canonical, case-insensitive on macOS). Cost if wrong: a task bound before task_links existed has every linked root protected until a fresh binding; acceptable since none exist yet.
- Ruling (fourth review, 2): `deny_rules` is `Result<Vec<String>>`; `* ? [ ] { } ( ) \\ newline` or non-UTF-8 refuses the session with the controller's Precondition text; rules emitted for given and canonical forms. Cost if wrong: a repo named like `api (old)` cannot be a protected target until renamed.
- Ruling (fifth review, 1): html_to_text slices only at `find` byte indices (char boundaries); an unterminated `<tag` swallows the rest; entity lookup limited to a `;` within 12 bytes found by `find`. Tests: é, emoji after `&`, CJK, unterminated tag. Cost if wrong: an unterminated `<` drops trailing text.
- Ruling (fifth review, 2): fetch_text refuses Content-Length > 2 MB (Precondition), streams with `chunk()` stopping one byte past the cap, reads files via `take(cap+1)`, and "marks the doc truncated" by appending `TRUNCATED_NOTE` to the cached text (no schema column). Truncation test uses a raw chunked tokio server (wiremock always sends Content-Length). Cost if wrong: a reference over 2 MB with an honest Content-Length is refused rather than truncated.
- Ruling (sixth review): the html_to_text entity lookahead is a 13-byte byte scan (`as_bytes().take(13).position(';')`), linear on a page of `&`; test converts 1 MB of `&` under 2 s. Cost if wrong: none.
- Ruling (Task 15): the issuing window is not threaded through Plan 04's effect chain (open_primary has no window). `linking::bind_if_first_start(app, state, window_label: Option<&str>, task_id)` uses the window's container when given, else the container the task was created in (`Task.workspace_id` is set by tasks_create in a workspace window). open_primary calls it with None. Cost if wrong: a workspace task first started from a project window still binds to its workspace.
- Ruling (Task 15): prompts are filled in core (`workspace::launch::fill_prompt_ctx`, called by Plan 03's build_prompt_ctx) into Plan 03's existing PromptCtx fields (link_policy, protected, briefs, project_context/workspace_context lines from `reference_line`, and a new `workspace_fragments` map picked per stage by build_stage_prompts) rather than the plan's PromptFragments strings, because Plan 03's assemble already renders the Linked projects block. `fragments::prompt_fragments`/`linked_projects_fragment` exist and are tested but the live path renders through Plan 03. Cost if wrong: the prompt wording is Plan 03's, not this plan's exact block.
- Ruling (Task 15): briefs come from Plan 05's LiveSpec BriefStore (what `RunHelper(ProjectBrief)` writes) first, `workspace::briefs` second; the brief helper is started when a bound link lacks one.
- Ruling (Task 15): the context cache folder is added to `--add-dir` only when the task's project or workspace has a fetched URL/Confluence doc; appended after Plan 04's claude_launch args (ClaudeLaunch unchanged).
- Ruling (Task 15): `AppState.db` is now `teamcode_core::db::Db` (Arc<Mutex<Connection>>); every `state.db.lock()` call site is unchanged. The stores lock per call, so no command holds `state.db` while calling one.
- Ruling (SSRF review): `workspace::url_guard` — forbidden (loopback, link-local incl. 169.254.169.254 and fe80::/10, unspecified, multicast, broadcast, 0/8, IPv4-mapped forms) always refused; private (10/8, 172.16/12, 192.168/16, 100.64/10, fc00::/7) only when `ContextDoc.trusted`. Redirects are followed manually (Policy::none, max 5), each hop re-validated and pinned via ClientBuilder::resolve. `trusted` is set by `context_docs_add` and by `confirm_open` for previewed docs; sanitize forces false. add/sanitize use a lexical check (no DNS at add time; fetch re-validates). Cost if wrong: an internal wiki link from a shared file needs one confirmation (or re-adding it) before it can be fetched.
- Ruling (seventh review, 1): cache is `<app_data>/context/project-<id>/` / `workspace-<id>/` (ids re-parsed); launch_extras adds only the home project's folder and the bound workspace's (when it exists). No migration: no cache existed before this branch; a reference whose cache file is missing reads "not fetched yet" and is re-fetched on demand. Cost if wrong: none beyond a re-fetch.
- Ruling (seventh review, 2): `fetch_text(doc, allowed_roots, atlassian, guard: &NetGuard)` is the only context-doc network path; `NetGuard` has a crate-private field, so outside teamcode-core it can only be `NetGuard::system()`. The shell's `state.http` is not used for context docs. Cost if wrong: none.
- Ruling (seventh review, 3): imported File/Folder targets are canonicalised in preview_open; `ImportDoc.outside_projects` + a problems line "…: Outside the workspace's projects."; `confirm_open(store, token, allow_outside_docs, env)` / `workspaces_confirm_open(token, allow_outside_docs)` trusts those only when true. fetch_text refuses untrusted file docs and any path whose canonical form is outside the container's roots unless it equals the stored (confirmed/added) canonical path. App-added docs are canonicalised by `context_docs_add` and trusted. Cost if wrong: a reference file outside the projects needs the extra checkbox in the import dialog.
- Ruling (eighth review, 1): `hop_trusted(trusted, origin, hop)` — trust only on the doc's normalised original host; any other hop host is validated untrusted. Cost if wrong: an intranet wiki that redirects to a sibling intranet host fails until that host's page is added directly.
- Ruling (eighth review, 2): `client_plan`: untrusted hop → `no_proxy()` + pinned address; trusted hop to the vetted host → system proxy allowed, pin still set (effective only without a proxy; through a proxy, the proxy resolves the name and the pin does not apply). Tested through the `client_plan` builder seam. Cost if wrong: an untrusted public doc fails behind a strict proxy until confirmed.
- Ruling (ninth review, 1): `confirm_open(store, token, approved_doc_ids: &[String], env)` / `workspaces_confirm_open(token, approved_doc_ids)` — every imported File/Folder doc is untrusted unless its preview id is approved (replaces the seventh review's `allow_outside_docs`); `ImportDoc { id, needs_approval, outside_projects, host, … }`; duplicate doc ids are dropped. `ImportRoot.warning` (non-blocking, also in `problems`) for dot-folder roots and roots under ~/Library, ~/.ssh, ~/.aws, ~/.config, ~/.gnupg, ~/.kube, ~/.docker, ~/.azure, ~/.gcloud, ~/.password-store. Cost if wrong: a shared workspace with many file references needs each ticked; dotfile repos show a warning.
- Ruling (ninth review, 2): prompt references (`reference_lines`) include only `is_visible` docs (trusted; files/folders pass `readable_path` against the container's roots); untrusted ones are omitted and counted as "N context docs are waiting for confirmation in Team Code". A trusted file doc whose file is missing is omitted silently. Cost if wrong: an agent is not told about a reference until the developer confirms it.
- Ruling (Task 16): a project's Jira scope is itself plus the first workspace (by id) containing it that links a site, since project-only commands carry no workspace. Cost if wrong: a project in two workspaces with different sites uses the first.
- Ruling (Task 16): transition failures are logged (`eprintln!`), not toasted: there is no shell→webview toast event in the contracts (Plan 05 made the same call). Listener uses `TaskChanged::listen_any`.
- Ruling (Task 16): jira_link normalises the site URL and refuses local/link-local hosts (url_guard lexical check); search JQL escaping (`search_jql`) lives in core.
- Ruling: added `context_docs_trust(container, id)` so a reference that came from a file can be confirmed later (the refusal messages tell the developer to confirm it).
- Ruling (tenth review, PRIORITY): `JiraSite { site_url, email }` (no `account`; deviates from the plan's/A19 shape). Account derived by `jira::settings::account_for(kind, id)` from the layer `resolve_with_account` found the site in. Keychain value is `Credential { site: <origin>, token }` JSON written by `credential_text`; `token_for_site` refuses a token whose origin differs (or a legacy plain token). `settings_set` calls `check_settable` (refuses `jira`); imports drop `jira`/`jira_transitions` with "Jira links aren't imported; link Jira in Team Code". Cost if wrong: a site URL edited by hand needs relinking; workspace files never carry a Jira link.
- Ruling (Task 18): the import review lives in `features/workspace` (`importStore`, `openWorkspaceFile`, `WorkspaceImportDialog` lazily mounted once in App.tsx); the start screen's Open workspace… and recents call `openWorkspaceFile`. Added command `workspaces_set_setting(id, key, value)` so workspace options go through the file mirror (plain `settings_set` would not mark the file). The ContextDocsEditor has no drop zone of its own: Plan 06's Docs section already forwards drops (`onAddPath` → `addDroppedPath`); `context_docs_add` decides File vs Folder on disk.
- Ruling: Tasks 1–14's plan tests are used verbatim except for ids (Plan 01 prefixes, `parse` instead of `from`),
  `&[x.clone()]` → `std::slice::from_ref` (clippy `cloned_ref_to_slice_refs`), and `set_body_raw` in the URL test
  (wiremock's `set_body_string` forces `text/plain` over an inserted content type). Cost if wrong: none.
- Ruling: modules were split to stay under 500 lines: `workspace/{rows, import, import_checks, launch, url_guard,
  context_fetch, store_tests}.rs`, `workspace/guard/{mod, rules, shell_words, protocol, tests}.rs`.
- Ruling: `GuardRule` displays as `rm`, `git commit`, `sed -i`, `<tool> package change`, `output redirection`,
  `find -delete/-exec`; `cd -` keeps the running cwd (conservative); `sh|bash|zsh|dash -c` unwrapped up to depth 8;
  `(`, `)`, `$(` and backticks split segments. Interpreters running scripts are the documented gap.
- Ruling: `bind_if_first_start` and `link_context` live in core (`workspace::launch`, used with the db lock held);
  `src-tauri/src/linking.rs` is the thin glue the plan names.

## STUBs

- Filled: both `STUB(plan 10)` in `src-tauri/src/session_service/mod.rs` (deny rules + guard hook via
  `workspace::launch::launch_extras`; workspace agents via `workspace_agents`).
- Removed: my `jiraSearch`, `workspacesCreate`, `workspacesAddProject`, `workspacesSaveAs` wrappers and `Issue` type
  from `src/lib/pendingCommands.ts`; callers (TicketSection, start screen, palette) use the generated commands. The
  Plan 05 wrappers in that file (`costSummary`, `gitChanges`) are left for their owner.
- Left for the controller: `src/features/dock/jira.ts` is Plan 08's `STUB(plan 10)` (runtime lookup of
  `jiraIssue/jiraComment/jiraSearch`). It works unchanged with the real bindings; I did not edit Plan 08's folder.
  Delete it after merge and use the generated `Issue`.
- None added.

## Unverified (needs GUI or a real service)

- The workspace settings screen, import dialog, pickers (open/save/folder), Docs drag-drop, the reload banner on a
  real `workspace-file-changed`, and window conversion (`workspaces_convert`, `workspaces_create_from_windows`).
- Keychain read/write of Jira credentials; any real Jira/Confluence call; transitions on stage change.
- A real Claude Code session honouring the generated `permissions.deny` rules and the `PreToolUse` guard hook.

## Deviations from the plan (interfaces other plans see)

- Workspace import: `workspaces_preview_open(path) -> WorkspaceImportPreview`, `workspaces_confirm_open(token,
  approved_doc_ids)`; `workspaces_open(path)` and `windows_open_workspace(path)` only reopen the record's own
  unchanged file and otherwise return `Precondition("Review this workspace file before opening it.")`. Webview:
  `openWorkspaceFile(path)` from `@/features/workspace` runs the whole flow.
- `JiraSite` is `{ site_url, email }` (no `account`).
- `LinkBinding` has `links: Vec<BoundLink>`; `policy::protected_roots(home, workspace, bound: &[BoundLink])`;
  `policy::deny_rules(..) -> Result<Vec<String>>`; `fragments::references_fragment(heading, container, docs, roots,
  app_data)`; `fragments::prompt_fragments(project, project_roots, docs, link, app_data)`; `LinkContext` gains
  `workspace` and `workspace_roots`; `context_docs::{context_dir, text_cache_path}` take a container;
  `fetch_text(doc, allowed_roots, atlassian, guard: &NetGuard)`; `ContextDoc.trusted`.
- `PromptCtx` gains `workspace_fragments` (Plan 03's type), filled by `fill_prompt_ctx` from `build_prompt_ctx`.
- Added commands: `workspaces_preview_open`, `workspaces_confirm_open`, `workspaces_set_setting`, `context_docs_trust`.
- `0010_workspaces.sql` also adds `workspaces.dirty`, `context_docs.trusted`, `task_links`, and recreates the unused
  0001 `jira_links` as the issue cache.
- `CostsPanel` is in `@/features/workspace` (plan's API); Plan 06's simpler `settings/components/CostsPanel.tsx` still
  serves project settings.

## Plan text that is wrong (not edited)

- Task 1/2: `WorkspaceId::from`, `ProjectId::from("proj-1")`, `"ws-ext"` (not hex), `workspace_projects.ord` (the
  column is `"order"`), no `dirty` column in 0001, and `Db` (no such type existed).
- Task 3's `open_file_with_known_id_reuses_record…` is unsafe as written (superseded by the id-collision ruling).
- Task 6's path-matching `protected_roots(.., projects, bound_roots)` (superseded by id matching).
- Task 9's `references_fragment` path `/app/context/doc-2.md` (now per container, and only for an existing cache).
- Task 14: wiremock `set_body_string` + `insert_header("content-type", …)` does not send HTML.
- Task 15 assumes Plan 04 calls `bind_if_first_start` with the issuing window; Plan 04's `open_primary` has no window.
- Tasks 17–20 test literals mock commands with bare values; the bindings return `Result`.

## What the next plans need to know

- **Controller / merge**: expect conflicts only in ARCHITECTURE.md, Cargo.lock and `src-tauri/src/lib.rs` (keep both).
  After merge, delete Plan 08's `src/features/dock/jira.ts` stub.
- **Plan 08 (Ticket tab)**: `commands.jiraIssue/jiraComment/jiraSearch/jiraTransitions/jiraTransition/jiraCached`
  with shapes as in the plan; or `jiraStore` from `@/features/workspace` (`issue()` returns cached then refreshes).
- **Plan 06 obligations**: the shell already passes Docs rows and the Workspace settings tab; still open: ⌘S →
  `workspaces_save` when the sidebar is focused, save-on-close when dirty, and the "Reload?" toast on
  `workspace-file-changed` (the settings screen shows its own banner).
- **Plan 09**: nothing from this plan beyond `CostSummary.by_agent` (Plan 05's).

## Addendum (2026-10-25): whole-branch review Low and rebase onto Plan 09

- Ruling (whole-branch review, Low): every imported context doc, whatever its kind, is approved one by one.
  `ImportDoc.needs_approval` is always true (the dialog shows a checkbox per doc, with the host for links), and
  `confirm_open` trusts only ids in `approved_doc_ids`. Test: an imported URL doc that is not approved stays untrusted;
  once approved it is trusted. Commit `a354424`. Cost if wrong: a shared workspace's intranet links need one tick each.
- Rebased onto main `9e9d8c4` (Plan 09 merged). Conflicts were in `db::MIGRATIONS` (kept 0009_canvas then
  0010_workspaces), `commands/mod.rs` and `specta.rs` (kept both canvas and Plan 10 modules), and ARCHITECTURE.md
  (kept both sections and both table rows). `npm run bindings` regenerated with no diff.
- Plan 09's smoke suite needed two changes (commit `4af8efa`): `e2e/fixtures/mockTauri.ts` answers `context_docs_list`,
  and `e2e/fixtures/seed.ts` adds `jira: null` and `jira_transitions`, now that `ResolvedSettings` has those keys. The
  worktree's `npm ci` had removed the local WebKit, so I copied
  `node_modules/playwright-core/.local-browsers` from the plan-09 worktree. That is a local copy, not a global install.
- Commit range now `bb243de`..`4af8efa`, 34 commits on top of main `9e9d8c4`.
- Gates on `4af8efa`:
  - typecheck exit 0
  - lint clean (Prettier OK)
  - vitest `Test Files 165 passed (165)`, `Tests 921 passed (921)`
  - cargo test teamcode-core 449, shell 13 (1 ignored), livespec-core 240, 0 failed
  - clippy clean
  - build `✓ built in 13.67s`
  - bindings:check exit 0
  - `PLAYWRIGHT_BROWSERS_PATH=0 npm run e2e` → `3 passed`
