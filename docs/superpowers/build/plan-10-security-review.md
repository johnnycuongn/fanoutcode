# Plan 10 security review (Rust side, whole branch)

Scope: `crates/teamcode-core/src/{workspace,jira}/**`, the Plan 10 changes in `pipeline/ctx.rs`, `prompts`, `settings`,
and `src-tauri/src/{commands/{workspaces,context_docs,jira,settings,windows}.rs, session_service, linking.rs,
jira_transitions.rs, workspace_sync.rs, state.rs, main.rs}` at worktree HEAD `ac64743`. Read-only review;
`cargo test -p teamcode-core --lib -- workspace:: jira::` passes (128 tests).

Threat model as briefed: attacker authors a workspace file the user opens, controls a remote URL or Confluence page,
and controls file and folder names in a repo; no code execution on the machine.

## Result

| Severity | Count |
|---|---|
| High | 0 |
| Medium | 0 |
| Low | 1 |

## L1 (Low): imported URL references become trusted without being approved, so a public-looking host reaches private networks

**Where**
- `crates/teamcode-core/src/workspace/import.rs:267`: `!needs || approved_doc_ids.contains(&d.id)`. Every Url and
  Confluence reference in a confirmed preview is stored `trusted = true`. Only File and Folder references have
  `needs_approval`, and the dialog shows URL references as plain text with no checkbox
  (`WorkspaceImportDialog.tsx`, `References`).
- `crates/teamcode-core/src/workspace/url_guard.rs:124`: a trusted doc may fetch any `Private` address. The check
  runs on the address the host resolves to at fetch time, not on anything the developer saw.
- `crates/teamcode-core/src/workspace/url_guard.rs:177` with `:205`: a trusted hop to the doc's own host uses the
  system proxy, which skips the address pin (the proxy resolves the name again).
- Reached from `context_docs_fetch` through `context_fetch.rs:92`.

**Attack.** The shared workspace file lists `{"kind":"Url","target":"http://docs.vendor-wiki.example:8080/<path>","label":"API spec"}`,
and the attacker's DNS answers `192.168.1.1` (or any 10/8, 172.16/12, 100.64/10, fc00::/7 host) for that name. The
preview shows "API spec (docs.vendor-wiki.example)". The developer confirms the workspace, and that confirmation is
about roots, agents and instructions; nothing per reference is asked. The reference is now trusted. When the developer
fetches it, `validate_hop` resolves the name to the LAN address, accepts it because the doc is trusted, and sends an
attacker-chosen GET (any port, path and query) to the internal service. The response is written to the workspace's
context cache, which is `--add-dir` for every task in the workspace. The prompts name that cache path, and the same
file's stage instructions can tell the agent what to do with the text. If a system proxy is configured, the trusted
hop goes through it and the proxy resolves the name itself. With DNS rebinding and a proxy listening on the machine,
loopback becomes reachable too, which breaks the rule that loopback and link-local are always refused.

**Why this is not covered by the 6th or 8th ruling.** The 6th ruling allows private ranges for "user-added or
preview-confirmed docs", and it accepts "a crafted file could still reach an intranet host the user confirmed". What
the developer confirms here is a public-looking name, never an address class. Trust is granted for the whole file, not
per reference, and it is enforced against whatever the name resolves to later. So the developer never actually
confirms an intranet host.

**Smallest fix.** Import Url and Confluence references untrusted unless approved one by one, the same as files. In
`import.rs:267`, use `approved_doc_ids.contains(&d.id)` for every kind, and set `needs_approval: true` for every doc in
the preview (`import.rs:196, 217`). Public URLs still fetch while untrusted, with no proxy and the address pinned. An
intranet doc then needs the existing per-doc approval or `context_docs_trust`. The UI only needs to render the checkbox
for URL rows as well.

## Ledger fixes verified end to end (hold)

- **Ids into paths (1st):** `WorkspaceFile::parse` re-checks the workspace and doc ids. `default_file_path`,
  `context_dir`, `text_cache_path`, `brief_path` and `write_roots_file` all re-parse their ids. Rows get fresh `doc-` ids.
- **Id collision, silent write scope, unvalidated fields (2nd):** an id owned by a record at another path gets a fresh
  id (`confirm_open`). Every import goes through `preview_open` → `confirm_open`, or through `open_file` /
  `relocate_file` gated by `adds_nothing`. `windows_open_workspace`, `workspaces_open` and `workspaces_reload` all go
  through `open_file`. No other path parses a workspace file (`find_or_create_workspace` has no caller left). Sanitize
  drops bad agent names, non-stage fragments, project-only settings, `jira`/`jira_transitions` and non-absolute file
  targets. The only workspace-scope settings left are `workspace_board` and `workspace_project_layer`, which are
  harmless, so `adds_nothing` not comparing settings is fine.
- **Roots and preview buffer (3rd):** `root_problem` refuses `/`, ancestors of home, and anything equal to, inside or
  containing app data or app config. `confirm_open` imports the cached buffer, re-resolves the roots and re-runs
  `root_problem`. The token is 128-bit, single use and expires after 10 minutes. `adds_nothing` compares agents and
  fragments by body, docs by kind, target, label and page id, refuses loosened protection or policy, and refuses a new
  root. `DocTrust::Existing` matches trust on the same fields, so a reopen cannot move trust onto another target.
- **Protection by id (4th):** `protected_roots` fails closed for unknown, non-member and duplicate links. A task whose
  workspace is gone keeps every link protected. A protected root whose name has rule syntax refuses the launch. Deny
  rules cover both the canonical and the original form. A trusted reload that removes a protected member leaves bound
  tasks protected (non-member → protected).
- **Confluence slicing, streamed cap (5th):** `html_to_text` slices only at `<`, `>` and `;` boundaries, and the
  entity lookahead is bounded. URL bodies are capped by Content-Length and while streaming. Files are read through
  `take()`.
- **SSRF (6th, 8th):** every fetch goes through `guarded_get` or `validate_hop`; there is no other reqwest path for
  context docs. Each hop is checked, no redirect is followed automatically, untrusted hops use no proxy and pin the
  address, and trust is scoped to the original host. The gap left is L1.
- **Per-container cache, guarded fetch, contained file reads (7th, 9th):** a task's `--add-dir` gets only its home
  project's cache and its bound workspace's cache. File and Folder docs are read only when trusted, and only when the
  canonical path is inside a root or equals the confirmed path, so a symlink swapped in by a later `git pull` is
  refused. Untrusted docs are only counted in prompts, never named. Imported File and Folder docs need per-doc
  approval, and a root the file declares grants nothing.
- **Jira token (10th):** the keychain account comes from the scope id and is never a setting. The stored credential
  is bound to the site origin and checked on every use (`token_for_site`). Only `jira_link`, `jira_link_workspace`
  and `jira_unlink` write `jira`: `settings_set` and `workspaces_set_setting` call `check_settable`, `upsert_settings`
  and `clear_settings` skip `jira`, and import strips it. Confluence fetches send the token only when the doc's origin
  equals the linked site, over a client pinned to that site with no redirects. reqwest 0.12 removes `Authorization`
  on redirects that change host or port, so the Jira client's default redirects do not leak the token. Issue keys
  are validated before they go into URLs, and transition ids are alphanumeric.

## Considered, not reported

- **Bash guard bypasses** (`python -c`, `$HOME/...`, `$(...)`): documented as best effort. The `Edit` deny rules are
  the stated hard guarantee.
- **Case or symlink aliases of a protected root reaching `Edit`:** whether Claude Code's `Edit(//path/**)` matching
  folds case on APFS or resolves symlinks is outside this codebase and I could not verify it. If it does neither, a
  case-variant path or a repo symlink into a protected root would get past the deny rule. Worth one manual test
  against the installed `claude` before release.
- **Preview lists agent names and fragment stages, not their text:** the instructions come from a file the developer
  confirmed, and the agent's actions are still gated by Claude Code permissions. This is a UX choice, not an exploit.
- **Unusual roots that are neither refused nor warned about** (for example `/usr/local/bin`, `~/Documents`): they are
  shown in the preview and need confirmation, as the 3rd and 9th rulings intend.
- **Team Code's own Commit action reaching protected linked repos:** the `commits.md` drafts are writable by the
  agent, but the commit needs the developer's click and "may commit", and it can only commit the developer's own
  changes there, because the agent cannot write in a protected root.
