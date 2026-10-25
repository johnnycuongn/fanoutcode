# Cross-task file awareness: design

Date: 2026-10-25. Status: draft for Johnny's approval. Extends the Fanout Code design
(`2026-10-24-team-code-design.md`, §6, §7.3, §9, §10.3); where this document and that one differ, this one wins.

## 1. Intent

Several tasks run their own Claude Code sessions against the same project at once. Today each session knows nothing
about the others: in a shared checkout one can overwrite another's uncommitted edits, or commit or discard them with a
stray `git` command; with a worktree per task two sessions can rewrite the same module independently and the merge
that follows drops one feature while resolving the conflict.

The promise this design adds: **no task's work disappears without the developer being told first**, whatever the
worktree setting. Each session learns, before it writes, that another task has its hands on the file; the developer
decides how to proceed; the other session is told what happened; and a merge is done knowing both tasks' intent.

## 2. Goals, non-goals, anti-goals

**Goals**

- A Claude session that is about to edit a file another active task has uncommitted changes in stops and asks the
  developer first, in the app, with the other task's changes beside what this session wants to write.
- The developer's answer comes in two grains: this file only (ask again at the next contested file), or every file
  the other task touches (no more questions between these two tasks this session). Replacing the other task's changes
  and holding off are also answers.
- The session whose file was edited on top of or replaced is told, so it does not carry on with a stale picture.
- The shell commands that can commit or discard another task's uncommitted work (`git commit -a`, `git add -A`,
  `git checkout -- …`, `git restore`, `git stash`, `git reset --hard`, `git clean`) are caught the same way.
- With per-task worktrees, a task planning its work sees what the other active tasks intend to touch and have
  touched, and plans around them; an edit inside another task's footprint earns a soft note, not a stop.
- When a worktree task brings the base branch in and hits a conflict, its session resolves it knowing both tasks'
  spec and plan and under the rule *keep both behaviours*; when it cannot, the task hands back with both sides shown.
- Every overlap and every decision is visible: an **Overlap** dock tab per task, counts on the board card and the
  view tab, entries in the task's timeline.

**Non-goals**

- Making two sessions edit one file "safely" at the same time. The design serialises through the developer; it does
  not merge live edits.
- Stopping Codex or shell tasks before they write. They have no hook mechanism, so they cannot be asked; their edits
  are attributed best effort (§4, *Claim*) so that a Claude session is still stopped before writing over them.
- Landing a task branch on `main`. §9 stands: the app never merges, rebases, pushes or deletes branches. The task's own
  session merges or rebases the base into its branch; landing stays the developer's.
- Perfect detection. Writes made by a shell command (a code generator, `sed -i`) are caught only as far as the
  existing Bash guard sees them, and two edits inside the claim's write lag (§5.7) can both pass.
- Integrating before the task's work is committed. Git will not merge over uncommitted changes; the developer commits
  first and integration runs from the Done card (§6.3).

**Anti-goals** (failure even if it technically works)

- A change of one task silently lost to another. The whole point.
- A session blocked with no way for the developer to say "go on". Every stop has a decision in the app, and the
  developer can always proceed.
- A dialog storm: being asked at every edit of a contested file. Hence the two grains.
- A server, daemon or socket the developer has to keep alive. The hook transport stays plain files, as §7.3 decided.
- Writing into the project's `.claude/` folder or the developer's own Claude settings.

## 3. Constraints

- Claude Code hooks: `PreToolUse` with a synchronous stdout decision (`permissionDecision: deny` and a reason Claude
  reads), as the read-only guard already uses; `PostToolUse` for `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, as
  attribution already uses. Hooks are per-task through the generated settings file (`--settings`). Hook commands must
  return within their timeout, so they read files, never wait on the developer.
- A session takes a typed prompt only when idle (`sessions/inject.rs`); a working session is reached by the
  `additionalContext` field a hook may return. The builder verifies which hook events honour `additionalContext` in
  the installed Claude Code and falls back to the idle injection where one does not.
- `fanoutcode-core` never depends on Tauri; everything decidable lives there with tests. The CLI subcommand that runs
  hooks is the app binary itself (`fanoutcode guard …` today, `src-tauri/src/main.rs`).
- The app runs git through the `git` binary; it never merges, rebases, pushes or deletes branches (§9).
- Budgets: Rust module under 500 lines, component under 200, store under 300. Generated bindings; no hand-written
  mirrors of Rust types (ADR 0001). Internally tagged enums with unit or struct variants only.
- Serde and naming rules of CLAUDE.md; a new `HandbackReason` variant needs its chip text in `handback::chip_text`
  and a serde round-trip test.

## 4. Concepts

- **Claim.** A file in a repo that one task has uncommitted changes in. Derived, not declared: a file is claimed by
  task T while it is in T's attributed change set *and* dirty in that working tree. A commit of the file releases the
  claim. Claims only matter when two tasks share a working tree (home repo without a worktree, or any linked repo).
  Attribution today comes from hook events only (`PostToolUse` edits and Bash windows), which hookless sessions never
  produce. New rule for them: a file that becomes dirty in a shared root while exactly one hookless session (Codex or
  shell) is active there and producing output is attributed to that task with source `Observed`; when more than one
  hookless session is active, the file is claimed by *another session* without a task, which still stops a Claude
  session (the hand-back names "another session" and shows the diff) and shows in Other changes until the developer
  assigns it. **(call)**
- **Pending decision.** A stop waiting on the developer. It lives in the overlap record (§9), not in `AgentState`;
  the task's state is pinned to `NeedsYou { FileInUse }` while any pending decision exists, whatever the chip feed
  derives from the session's hooks in the meantime, and reverts to the feed's state when the last one is decided.
  One turn may stop on several files; each is its own pending decision. **(call)**
- **Grant.** The developer's standing answer for one task about another task's claims: *on top* for one path, or
  *on top* for every path the other task claims. A grant lives for the session and clears when the claims it covers
  are released or either task is archived.
- **Footprint.** The files a task intends to touch (from the `## Files` section of its `plan.md`) plus the files it
  has touched (its attributed change set). Used in worktree mode, where there is no shared tree to claim in.
- **Overlap.** A file in this task's footprint or change set that another active task also claims or has in its
  footprint. Overlaps are what the Overlap tab lists; a pending decision is an overlap with a question attached.
- **Related tasks.** The pair (this task, that task) a grant is between. "Proceed for every file Login touches" is a
  grant on the pair.

## 5. Shared checkout: stop, ask, tell

### 5.1 Claims file

For every repo root that more than one task's session can write (the home root of tasks without a worktree; every
linked root), the app maintains `<app_data>/claims/<repo-key>.json`: for each dirty, attributed path, the owning task
id, its title, stage, agent state, and the time of its last edit. `<repo-key>` is a stable hash of the canonical repo
root. The file is rewritten atomically whenever attribution changes (the attributor already runs on every
`PostToolUse` edit and Bash window, `changes/attributor.rs`) or `git status` of that repo changes. It is also
rewritten on launch from the stored `task_files` and a fresh status, so a stale file never survives a restart.

A task's own settings file points its claim-check hook at the claims files of its repos (one path per repo) and at
its grants file.

### 5.2 Grants file

`<app_data>/grants/<task-id>.json`: the grants this task holds, each `{ other_task, scope: File { repo, path } |
AllFiles }` with the time granted. Written by the app when the developer decides; read by the hook; cleared by the
rules in §4.

### 5.3 The claim-check hook

A `PreToolUse` hook on `Edit`, `Write`, `MultiEdit` and `NotebookEdit` (matcher on the tool name), run as
`fanoutcode claim-check <task-id> <claims-paths…> --grants <grants-file>`, beside the existing guard in the generated
settings. It reads the tool input's path, canonicalises it, finds the repo it belongs to, and:

- No claim by another active task, or a grant covering it → allow (exit 0, no output).
- A claim and no grant → `permissionDecision: deny` with a reason written for Claude:
  *"Fanout Code: another task, <title>, has uncommitted changes in <path>. The developer is being asked how to
  proceed. Do not edit this file or work around the block; finish anything unrelated you were doing, then stop and
  wait for the developer's instruction."*
  It also appends one line to this task's event log, `ClaimDenied<TAB>…<TAB>{path, repo, other_task}`, so the app
  learns of the stop through the transport it already tails.

An unreadable claims file, or malformed input, is reported on stderr and allowed (fail open, like the guard): without
claims there is nothing to protect, and a broken file must not stop every session. An unreadable grants file counts as
no grants, so a write over another task's claim is denied and the developer is asked again: a grant only ever widens
what a session may do, so losing one must never allow more than the developer said. The hook never waits on the
developer. Timeout 5 s, as the other hooks.

### 5.4 The hand-back

On `ClaimDenied`, the app records a pending decision in the overlap record (the file, the other task, the tool input
snapshot) and pins the task to `NeedsYou { reason: FileInUse { pending } }` (new variant carrying the count; chip
*File in use*, *3 files in use*). The pin matters: the chip feed (`hooks/feed.rs::next_state`) would otherwise set
`Working` on the session's next `PreToolUse` and `Replied` on its `Stop` within seconds; while a pending decision
exists those derived states are kept aside and applied only once the last decision is made. The task page's primary
button reads **Decide**; the dock opens on the Overlap tab with the first pending file selected (the dock remembers
tabs per stage; a pending decision overrides the remembered tab once, as Commit does). The attention strip, board
card, view tab and Dock badge count it as any needs-you.

The Overlap tab shows, for the selected file: the other task's title, stage, state and last edit time; its changes to
the file (the diff against base the Changes tab already computes, narrowed to this path); what this session was about
to write (the tool input: for `Edit` the old and new strings and the lines they fall on, for `Write` the whole new
content, diffed against the current file); and the four answers:

1. **Proceed on top of <other>'s changes, check the next file** — grant `File { repo, path }`.
2. **Proceed on top for every file <other> touches** — grant `AllFiles` on the pair.
3. **Replace <other>'s changes in this file** — grant `File`, and mark the path *taken over*: it leaves the other
   task's change set (so its Done detection no longer waits on it) and joins this task's, with the relation shown in
   both tasks' Overlap and Changes tabs and a timeline entry in each. The app runs no git here: it does not restore
   the file from HEAD. Replace is an instruction to this session (*"the developer wants your version of this file;
   <other>'s uncommitted changes in it may go; rewrite it as your task needs"*), so for an `Edit` whose `old_string`
   no longer matches, the session re-reads and writes the file. Red text; no confirmation dialog beyond the Continue
   button, since the diff is on screen. **(call)**
4. **Hold** — no grant; the task stays needs-you with the chip *Holding for <other>*. When the file is clean (the
   other task commits it; §5.6) the task is notified and the prompt below is sent with "the file is now free".

Continue writes the grant, records an `IndexEntry` in the task's timeline (`Overlap decided: on top of Login in
src/auth/session.ts`), returns the task to `Working`, and injects into its session:

*"Fanout Code: the developer says go ahead with <path>, editing on top of <other>'s uncommitted changes (keep them;
add yours around them)."* or, for Replace, *"… replacing <other>'s uncommitted changes in <path>; the developer has
seen them."* With grain 2 the message names the other task and says no further files will be asked about.

The next edit passes the hook because the grant is in the file. If the session meanwhile ended its turn (it was told
to stop), the injection is what wakes it; if it is still working on unrelated files, the prompt queues as any stage
action does.

### 5.5 Telling the other session

Whenever a grant is written against task O's claim, O is told. The message names the file, the task and the grain:
*"Fanout Code: task Checkout is now editing src/auth/session.ts on top of your uncommitted changes (the developer
approved it). Re-read the file before your next edit to it."* For Replace: *"… has replaced your uncommitted changes
in <path> with the developer's approval; your change set marks it as taken over. Re-read it before touching it
again."*

Delivery: through `additionalContext` on O's next hook event that supports it, so a working session hears it mid-turn;
otherwise as an idle injection through the existing queue. Either way the message is logged in O's timeline.

### 5.6 The git guard

The existing Bash guard (`workspace/guard`) gains a second rule set that runs whenever the task's repo is shared: a
command that would commit, stage, restore, stash, reset or clean paths another active task claims is denied with a
reason naming the files and the task, and `ClaimDenied` is logged with `command` instead of `path`, so the same
hand-back opens with the command shown in place of the pending write. Rules, best effort as today:

- `git commit -a` / `--all`, `git commit <path>` naming a claimed path, `git add -A` / `--all` / `.` / a claimed path,
  when any claimed path is dirty in that repo.
- `git checkout -- <path>`, `git checkout <path>`, `git restore <path>` for a claimed path; `git checkout .`,
  `git restore .` when any claim exists.
- `git stash` (any form but `list`/`show`), `git reset --hard`, `git clean` when any claim exists.

The hand-back's answers for a command are **Proceed** and **Hold**. Proceed grants a single-use
`Command { repo, command, paths }` on each task named (`paths`: that task's files the command touched when the
developer answered), never `AllFiles` (an edit grant would switch the guard off), then injects "go ahead"; the
claim-check hook spends the grant, under the grants file's OS lock, before the command runs, so the next run of the
same command is checked again. A grant has a lifetime: the spend fails when the other task's claimed files the command
touches now differ from `paths`, or when the grant is older than ten minutes, and the command is then denied and asked
about again. An unspent `Command` grant is dropped at the session's next main-agent `Stop` after the go-ahead was
typed. A command that touches two tasks' files waits for both answers. The decision shows the other task's files the
command touches and whether it commits them or discards them.

The other task is told only when the command runs, not when it is granted: on spending a grant the hook appends
`CommandRan { repo, command, kind, paths, other_task }` to this task's event log. Event-log lines are agent-writable,
so the app never trusts one alone: it acts on `CommandRan` only when it matches a grant it recorded itself (same repo,
command, other task and paths), and only then queues the note for the other task (*"… ran `<command>` with the
developer's approval; your uncommitted changes in <paths> were committed into its commit / staged into its index /
stashed / discarded"*) and writes both timeline entries. A line with no matching record does nothing. `git stash pop`
and `git stash apply` are not caught: git refuses to overwrite dirty files, so they discard nothing. A Hold, on a command or an edit, is released only once the file is clean in `git status` (the
other task committed it, or no uncommitted changes are left); a claimant that merely drops out of the claims file (a
lost session, an archived task) releases nothing. Commits that name only the task's own files are never caught. A task's commit through the app's own Commit
button already stages only the task's files (§6.6) and is unaffected.

### 5.7 Lag and races

The claim appears after the owner's edit lands in its event log and the attributor runs: on the order of a hundred
milliseconds. Two edits to one file inside that window both pass. Accepted; Claude Code's own stale-read check on
`Edit` and the Changes view remain the backstop. The claims file is written atomically so the hook never reads a torn
file.

## 6. Per-task worktrees: plan around, note, merge knowing both

### 6.1 "Others working here" in the Plan prompt

When a task enters Plan, its prompt gains a section listing every other active task of the same project and of every
linked project the task can write: title, stage and state, branch (the current branch of its worktree or of the
shared tree, read from git, which covers both worktree conventions), its one-line
intent (first line of `intent.md`, else the title), its footprint, and the new files and exported names it has added
so far (from its change set: added files, and `export`-level symbols of added or changed files, extracted by the
builder's choice of lightweight parsing; a plain list of added files is the floor). The section ends with an
instruction: *"Prefer files none of them touch. Where you must touch one, say so in the plan and keep the change
narrow. Reuse what they added rather than duplicating it. List every file you intend to change under `## Files`."*

The Plan stage prompt therefore requires a `## Files` section, one path per line, optionally with a note after `—`.
`hooks/plan_steps.rs` grows a sibling parser for it. A plan without the section is still accepted; the footprint is
then the change set alone, and the Overlap tab says so.

The same section, kept current, is available to the Build prompt and on demand through a `/others` style reminder the
app injects when a new task becomes active in the same project (a one-line note: *"Fanout Code: task <title> has
started on <branch>; its planned files: …"*).

### 6.2 Soft notes during Build

An edit (seen on `PostToolUse`) to a file inside another active task's footprint, when the two tasks do not share a
working tree, is not stopped. The app tells the editing session once per file, through `additionalContext` or idle
injection: *"Fanout Code: task <other> also changes <path> on <branch>. Keep your change narrow and expect a merge."*
The overlap is recorded and shown (§7). The other task gets the mirror note once per file.

### 6.3 Bring the base in

Git refuses to merge or rebase over uncommitted changes, so integration comes after the task's work is committed:
the developer commits as today (or through the Commit button when "Fanout Code may commit" is on), the card moves to
Done, and the **Done** card's task page offers **Bring <base> in** in its Overlap tab and its primary-action menu
(Johnny's decision). The session is still alive at Done (it ends only on archive, Stop agent or Start fresh). The
action is available whatever the "may commit" setting is, because the app runs no git: it injects a prompt. This is
the one prompt that tells the agent to merge or rebase, a narrow exception to the stage-prompt rule in
`.claude/rules/prompts.md`, which otherwise stands (it still never says commit, push or delete); the rule and the
prompt tests change with it, and the integrate prompt is assembled from data rather than rendered from the
eight-placeholder template set. The base branch
is stored on the task (new nullable column). For a Team Code-managed worktree it is the branch the worktree was
created from, recorded at creation. For Claude Code's own convention the branch is created by `claude --worktree`
from the user's `worktree.baseRef`, which the app does not read; the base defaults to the repo's default branch
(`origin/HEAD`'s target, else `main`, else `master`) and is shown for confirmation the first time the action is used
on that task, then stored. **(call)** After a Rebase the task's recorded `RepoBase` no longer bounds its diff; the
app re-records it as the new merge-base with the base branch once the session reports the integration done. **(call)**
The prompt tells the session to merge (or rebase, the project setting `worktree_integration: Merge | Rebase`, default
Merge) the base into its branch and lists:

- the files that conflict or overlap, each with the other task that touched them on the base side (from the overlap
  record, else from `git log <base> --not HEAD -- <path>` attribution to tasks by branch name);
- for each such task, its spec and plan excerpts (`spec.md` summary section and the `## Files` and relevant steps of
  `plan.md`) from its memory folder;
- the rule: *"Keep both behaviours. Never resolve a conflict by taking one side wholesale. Where both sides changed
  the same lines, write the version that does what each task's spec asks. Run the tests. If you cannot keep both,
  stop, leave the conflict markers in place, and say which files and why."*

When the session's `Stop` comes without the sentinel `FANOUTCODE: integrate done` and the tree has conflict markers
or an in-progress merge, the task hands back as `NeedsYou { reason: MergeNeedsYou { repo, paths } }` (chip *Merge
needs you*; a Done card can be needs-you, state and stage being independent), and the Overlap tab shows each
conflicted file with both sides and both tasks' intent, plus **Resume** (inject "continue the integration with these
instructions: …" with a free-text field) and **Abort** (inject "`git merge --abort`, leave the branch as it was").
The sentinel with a clean tree returns the task to Idle and records the integration in the timeline. Landing the
branch on the base stays with the developer, as does archiving.

## 7. Showing it: the Overlap tab and counts

- **Overlap** is a new `DockTab` (`"overlap"`), visible in every stage whenever this task has at least one overlap,
  with the overlap count on the tab; the count turns amber (and the tab opens) while a decision is pending. Hidden
  when there is no overlap, so single-task projects never see it. Left: the files, grouped *Needs you*, *Also touched
  by others*, each row with the other task, its stage and state, and the relation (*claimed*, *granted on top*,
  *taken over*, *planned by*, *committed*). Right: the detail for the selected file as §5.4 and §6.3 describe; for a
  settled overlap, the two diffs and the decision history. **Open in Changes** jumps to the file there.
- The Changes tab's file rows carry the same relation chip; the sidebar's Changes section marks files touched by two
  tasks.
- The board card shows an overlap count beside the needs-you chip; the view tab's status dot already reflects
  needs-you, and the hover tooltip gains "N overlaps".
- Decisions, grants, notes sent and received, and taken-over files are `IndexEntry` rows in the task timeline for the
  human-readable history, and the app also appends each as a line to the task's hook event log (`OverlapDecided`,
  `OverlapNoted`, written by the app, parsed like any hook line), because the Team view is built from that log alone
  (`hooks_list` and `team-event`); so the event stream, the drill-in and a replayed Build show them without the canvas
  learning a second source. **(call)**
- Workspace windows: overlaps across linked projects use the same tab, with the repo badge as elsewhere.

## 8. Settings

Project settings, under a new "Working together" group:

- **Ask before editing another task's files** — on by default; off makes the hook log the overlap and allow, so the
  Overlap tab still fills but no one is stopped.
- **Guard git against other tasks' uncommitted work** — on by default; same off behaviour.
- **Integration**: Merge | Rebase (default Merge), for *Bring <base> in*.

No app-wide setting. Grants are not settings; they are per session.

## 9. Data

- Claims and grants are files under app data (§5.1, §5.2), derived state; the database stays the source of truth for
  attribution (`task_files`, which gains the `Observed` source), footprints (new `task_footprint` table: task, repo,
  path, source Plan | ChangeSet, added_at), the worktree base branch (new nullable column on `tasks`), and overlaps
  (new `task_overlaps` table: task, other_task nullable, repo, path, relation Claimed | GrantedOnTop | TakenOver |
  PlannedBy | Committed, pending decision as JSON (tool name and input snapshot, or the denied command), notes-sent
  flags per direction, decided_at). `task_overlaps` is the source of truth for the Overlap tab, the counts, the
  pin of §5.4 and the taken-over exclusion in `changes::done_paths`; the timeline and the event log carry the
  history. Migrations follow CLAUDE.md's rule; this is plan 11, so `0011_<name>.sql`.
- New `HandbackReason` variants: `FileInUse { pending: u32 }`, `HoldingFor { other: String }` (when every pending
  decision is a Hold) and `MergeNeedsYou { repo: PathBuf, paths: Vec<String> }`; the details live in `task_overlaps`.
  Serde round-trip tests as the rule requires; `chip_text` entries.
- New hook event `ClaimDenied` in `HookEvent`, written by the hook itself (not by Claude Code), parsed by the existing
  log parser from the line's first field.
- Events to the webview: `task-changed` already carries state; a new `overlap-changed { task_id, count, pending }`
  keeps tabs and cards current without a reload.
- Commands (snake_case, `domain_verb`): `overlap_list(task_id)`, `overlap_decide(task_id, decision)`,
  `overlap_integrate(task_id)`, `overlap_resume(task_id, instruction)`, `overlap_abort(task_id)`; all return fast,
  the prompts go through the existing injection path.

## 10. Calls made in this document

Marked so Johnny can check them while reading; everything else above he decided in conversation.

- Awareness is **per task**, not per view: a task owns its session, a view only arranges panes.
- **Claude Code only** for the stop-and-ask; Codex and shell tasks contribute claims and appear in the tab.
- A claim lasts while the file is **uncommitted**; grants last the session and clear with the claims they cover.
- **Hold** keeps the task needs-you and tells it when the file is free.
- **Bring <base> in**, not merge onto main; landing stays the developer's. Merge by default, Rebase by setting. Johnny
  decided the order: he commits first, the card reaches Done, and integration runs from the Done card.
- The "consistency" ask is served by listing each active task's **added files and exported names** in "Others
  working here".
- The claim write **lag** is accepted (§5.7).
- Every decision and note is a **timeline entry**.
- The tab is named **Overlap**; the chips read *File in use*, *Holding for <other>*, *Merge needs you*.
- Replace needs no extra confirmation: the diff is on screen and Continue is the confirmation.
- The hook writes its own `ClaimDenied` line rather than relying on Claude Code's `PreToolUse` log line, so the app
  acts on the denial itself and not on every pre-tool event.
- Soft notes are sent **once per file** per direction.
- From the builder check: hookless sessions get **best-effort attribution** (`Observed`) so their files still claim;
  a **pending decision pins** the task's state and lives in `task_overlaps`, with `FileInUse` carrying only a count;
  **Replace** is an instruction plus a relabel, the app restores nothing from git, and the path moves to the
  replacing task's set; the **base branch** for Claude Code's own worktrees defaults to the repo's default branch and
  is confirmed once; the base is **re-recorded after a Rebase**; decisions are **mirrored into the hook event log**
  so the Team view shows them.

## 11. Left to the builder

- Exact hook-output fields that carry `additionalContext` in the installed Claude Code, and the fallback per event.
- How exported names are extracted for "Others working here" (a regex over `export` lines is acceptable).
- The repo-key hash, the atomic-write helper reuse (`memory::store::write_atomic`), and where the claims rewrite is
  triggered in the shell (`observe.rs` beside the attributor).
- The Overlap tab's component split within budgets; the diff rendering reuses the Changes diff editor.
- Wording of all injected prompts beyond the sentences given, kept in `prompts/templates.rs` with the others.
- Tests: core-level tests for the claim rules, grant lifetime, hook decisions (stdin/stdout like the guard's), the
  `## Files` parser, the git-guard rules, and the hand-back transitions; webview tests for the tab, counts and the
  decision flow with mocked bindings.
