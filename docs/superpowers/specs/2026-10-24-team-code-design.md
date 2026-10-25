# Fanout Code — design

Renamed from Team Code to Fanout Code on 2026-10-25; identifiers follow the new name.

Status: **approved by Johnny on 2026-10-24.** Rewritten the same day around windows and workspaces before approval. Written from the brainstorm in `docs/superpowers/brainstorm/` (original brief in `2026-10-24-team-code-intent.md`, choices in `2026-10-24-decisions.md`). Lines marked **(call)** are Claude's decisions, made so a builder can plan; Johnny can overturn any of them. Everything else is what Johnny chose or said.

## 1. What this is and why

Team Code is a macOS desktop app, built with Tauri 2, for one developer who runs several AI coding agents at once across several projects. It works like VS Code at the top level: a window holds one project, and a **workspace** links several projects into one window. Each project has one Kanban board. Each card is a task that owns a real terminal session running Claude Code (or Codex, or a plain shell) and a folder of markdown that remembers the task: what was asked, the spec, the plan, what happened. A task moves through a fixed pipeline, Spec → Plan → Build → Review → Test → Commit → Done, and at the end of every stage the agent stops and hands back to the developer.

The job of the app is context switching. Switching project, window, task or pane has to be instant and complete: the terminal is still there, still scrolled where it was, the agent never stopped, and the board tells you in one glance which tasks are waiting on you. In a workspace the projects are linked, so an agent working in one repo can read the others for reference and, when the task asks for it, change them too.

LiveSpec (Johnny's existing Tauri app, github.com/johnnycuongn/live-spec) is the seed. Its Rust core and its draft-and-notes editor are copied into this repo and reused for the Spec stage and for every headless model call.

## 2. Goals, non-goals, anti-goals

**Goals**

- Run many agent tasks in parallel, across projects and windows, and never lose track of one.
- Make "what needs me right now" visible from anywhere in the app.
- Make switching window, project, view, task or pane a keyboard reflex.
- Let linked projects in a workspace see each other, so cross-repo features are one task, not three.
- Keep every task's memory on disk as plain markdown the developer can read, edit, and keep in the repo if they want.
- Feel light and fast: native-feeling motion, instant switching, idle CPU near zero.
- Ship light and dark themes with a configurable terminal palette.

**Non-goals for v1**

- Not an editor or IDE. The only code editing is the lightweight diff editor in Review.
- No cloud sync, accounts, or multi-user. Everything is local.
- No Windows or Linux polish. macOS first, Apple Silicon first, code kept portable as LiveSpec's is. **(call)**
- No Jira OAuth. API token only. **(call)**
- No plugin system. Stage prompts are editable text, not code.
- The app does not merge branches or push. It can commit only when the developer turns that on.
- Workspaces do not share tasks: a task has one home project. **(call)**

**Anti-goals** (failure even if it technically works)

- It commits, pushes or merges anything the developer did not explicitly ask for.
- A running agent is killed or its output lost because the developer switched window, project, view or pane, or closed a tab.
- An agent edits a linked project the task never asked it to touch, without that edit being visible in the task's Changes.
- The UI blocks on a model call or on terminal output.
- A task's state exists only inside the app. Memory folders must stay complete and readable without Team Code.
- Hooks or prompts the app installs leak into the developer's normal Claude Code usage outside the app.

## 3. Constraints

- Tauri 2 shell, Rust core, React 19 + TypeScript + Vite + Tailwind v4 in the webview, as in LiveSpec. Same three-layer rule: the core never depends on Tauri; the shell stays thin; the webview has no state library and uses module stores with `useSyncExternalStore`. **(call, carried over from LiveSpec)**
- UI kit: shadcn/ui on Radix for menus, dialogs, popovers, tabs, command palette; Motion for animation; Sonner for toasts; Vaul for drawers; Lottie (dotLottie) for empty states; Rive for live indicators. All chosen by Johnny for reliability in webviews.
- Terminal: xterm.js with the WebGL addon and canvas fallback in the webview, `portable-pty` in Rust. **(call)**
- Diff editor: CodeMirror 6 with `@codemirror/merge`. **(call)**
- Board drag and drop: dnd-kit. **(call)**
- Team canvas: React Flow (xyflow) with elkjs for auto-layout. **(call)**
- Storage: SQLite via `rusqlite` (bundled) for app state; markdown files for task memory; JSON files for workspaces; OS keychain for secrets. **(call)**
- Fonts bundled, never fetched (offline, strict CSP): Inter for UI, JetBrains Mono for terminal and code, Literata for the spec editor. **(call, same as LiveSpec)**
- The app never reads Claude Code's token files. It runs the developer's own `claude` binary, as LiveSpec does.

## 4. Windows, workspaces and projects

### 4.1 The model (Johnny's decision)

- **A window holds one project.** Opening a project folder opens a window for it. Everything in that window, the board, the views, the attention strip, belongs to that project. There is no project switcher in a single-project window.
- **A workspace links several projects into one window.** The developer creates one by adding a second project to a window ("Add project to workspace…") or by opening a saved workspace file. Only then does the left side show multiple projects. A workspace window has one Board tab and one set of views for all its projects.
- **Linked means visible.** Every agent session started from a workspace window can read every project in it, for references, and may modify a linked project when the task asks for it (section 4.3).
- Several windows may be open at once, each a project or a workspace. The same project may be open in a single-project window and inside a workspace at the same time; its board and tasks are the same data either way. **(call)**
- **A task's links are fixed when its session first starts. (call)** The window that issues the first stage action decides: from a workspace window the task binds to that workspace and its project set at that moment, recorded in `index.md` and the database; from a single-project window it has no links, even if the project also sits in a workspace. Later stage actions, resumes after relaunch and "Start fresh session" reuse the bound set. Adding or removing a project in the workspace does not touch running sessions; affected tasks show "Links changed, start a fresh session to apply".
- **Views on conversion. (call)** Views belong to the window's container. When a project window becomes a workspace ("Add project to workspace…") its views carry over; "New workspace from open windows" merges every window's views into the new one. When a project is removed from a workspace, panes showing its tasks close (sessions keep running) and the views stay. Views are never migrated back to a single-project window.

### 4.2 Workspace files

A workspace lives in the database with a stable id and is mirrored to a JSON file, `<name>.teamcode-workspace.json`: the projects (path, display colour, order), workspace-level setting overrides (section 13), the linking policy (section 4.3), and workspace prompt fragments. While the file is in app data (the default) it is written through on every change. After "Save workspace as…" puts it somewhere else, for example in a parent folder of the repos so it can be shared or committed, it is written only on an explicit save (⌘S with the sidebar focused, or on closing the window), and the window title shows a dirty dot meanwhile; the app watches the file and offers to reload when it changes on disk. Opening the file opens the workspace window; opening a file the database already knows by id reuses that record. The recent list, `index.md` and per-window state refer to the id plus the last known path, so a moved or deleted file leaves the workspace intact and marks the file missing on the start screen with relocate and re-export. A project path that no longer exists shows as missing in the sidebar and can be relocated or removed. **(call)**

### 4.3 Linking semantics (call)

- Each Claude Code task session bound to a workspace (section 4.1) gets `--add-dir <root>` for every other project in its bound set, so Claude Code's tools may read and write there, subject to the developer's normal permission prompts. Linking is Claude Code only in v1; a Codex or shell task sees only its home project, and the task creation sheet says so when that agent kind is chosen inside a workspace. **(call)**
- Every stage prompt gains a **Linked projects** section: each project's name, root path, and one-paragraph brief (LiveSpec's project brief, built lazily the first time a project is linked). The prompt states the policy: read linked projects freely for references; modify one only when the task's intent or plan calls for it; list any such edit in `plan.md` under that project's heading.
- The workspace linking policy has two values: **Read and modify on request** (default) and **Read only**. A per-project override in the workspace file marks that project as a **protected target**: no task from another project may modify it, whatever the workspace default. **(call)** Read only is enforced two ways, both through the generated settings file: a permission denylist for `Edit`, `Write`, `MultiEdit` and `NotebookEdit` under protected roots, which is hard, and a `PreToolUse` guard hook that denies `Bash` commands referencing a protected root together with a write-shaped pattern (redirects, `rm`, `mv`, `cp`, `sed -i`, mutating `git`, package installs), which is best effort. The setting's label says so: "Read only: file tools blocked, shell commands checked".
- LiveSpec retrieval in the Spec stage searches the home project first and linked projects second, with home snippets weighted higher.
- Changes made in a linked project belong to the task's change set (section 9) and are shown grouped by repo. Only the home project gets a worktree; linked projects are edited in place.
- Jira sites, Confluence spaces and context docs stay per project; a workspace may add its own context docs and a default Jira site that projects without one inherit.

### 4.4 Start screen and first run

The first launch asks two things: the worktree convention (section 9) and the theme. The start screen, shown when no window is open and from the Window menu, lists recent projects and workspaces, with Open project…, Open workspace…, and New workspace from open windows (which closes those windows into one). **(call)**

## 5. Concepts

| Thing | What it is | Owns |
|---|---|---|
| **Window** | A macOS window holding one project or one workspace | tab strip (Board + views), attention strip, sidebar state |
| **Workspace** | A saved set of linked projects opened together in one window | project list and order, linking policy, workspace-level settings, context docs |
| **Project** | A folder on disk, usually a git repo | name, colour, root path, memory location, stage prompt overrides, test command, worktree default, Jira/Confluence link, context docs, one board |
| **Board** | The project's Kanban | seven columns, filters |
| **View** | A named tab holding a free-form split layout of panes; belongs to the window's project or workspace | layout tree, pane → task or plain terminal |
| **Pane** | One slot in a view | a task page, or a plain terminal in a project root |
| **Task** | A card. One unit of work moving through the pipeline. Has exactly one home project | title, stage, hand-back state, priority, colour, flag, hidden, archived, Jira key, agent kind, worktree/branch, sessions, memory folder |
| **Session** | One PTY and the process in it | pty id, cwd, command, Claude session id (when known), scrollback |
| **Memory folder** | The task's markdown on disk | intent.md, spec.md, plan.md, index.md and friends (section 8) |

A task has exactly one primary session, the one its stages drive. The developer can open extra terminals in a view; those are plain panes, not tasks. In a workspace window, a plain terminal pane asks which project root to start in.

## 6. The pipeline

Seven columns: **Spec · Plan · Build · Review · Test · Commit · Done**. Archive is a filter, not a column.

A card's position is its stage. Inside a stage the card has one of three states, shown as a chip:

- **working** (green, pulsing): the agent is running.
- **needs you** (amber): the agent stopped and the developer must act. The hand-back points from the brief (spec ready, plan written, build done, message drafted) are this state, with the chip text saying which.
- **idle** (grey): nothing running, nothing waiting.

**How the app knows a stage is done. (call)** Claude Code's `Stop` hook fires after every assistant turn, including a clarifying question or an error, so `Stop` alone cannot mean "stage finished". Every stage prompt instructs the agent to end its final message with one sentinel line, `TEAMCODE: <stage> done`, and the `Stop` payload's `last_assistant_message` is checked for it. The app also checks the stage artifact (for Plan: `plan.md` changed since entry; for Build: files changed since entry; for Review: `review.md` written). Sentinel plus artifact → the stage hand-back chip (*Plan written, go?*). `Stop` without the sentinel → chip *Replied* with the first line of the message, still amber because the agent is waiting on you, and the primary button reads "Reply" and focuses the terminal. `PermissionRequest` → *Permission needed*. `StopFailure` → *Error*. A `Stop` whose payload still lists running background tasks means "still working", not "needs you". The stage's forward action stays available in every case, because the developer can always decide the stage is done.

Moving a card forward happens three equivalent ways: the primary button in the task page header, dragging the card on the board, or ⌘⏎ on the focused task. Dragging a card backwards re-enters that stage with its stage prompt. Dragging a card more than one column is allowed and runs the skipped stages' exits as no-ops. **(call)**

### 6.1 Spec

- **Entry.** A task is created from a typed intent, from a Jira issue (title and description become the intent), or empty. `intent.md` is written. The agent names the task in the background from the intent (section 7.4); the developer can rename at any time. In a workspace window, creating a task asks which project is its home.
- **Page.** The spec editor, bound to `spec.md`. The terminal is collapsed to a strip (expand with ⌃`). No agent runs unless the developer asks for one in the terminal.
- **LiveSpec switch** on the page: **Off** (default), **Simple**, **Heavy**. Simple runs one lane on the fast tier with the Conflicts, Gaps and Questions sections and no fact-check. Heavy runs all five sections in parallel lanes on the deep tier, with TypeSafe fact-check when a key exists. The last choice is remembered per project. **(call for the Simple/Heavy mapping)**
- **Exit.** "Approve spec" marks the chip *Spec ready* and moves the card to Plan in one click. **(call: the amber state alone is for when an agent produced the spec and the developer hasn't looked yet)**

### 6.2 Plan

- **Entry.** The task's primary session starts, inside the task's worktree when that flag is on (section 9), with the Plan prompt: read `intent.md`, `spec.md`, the project context docs and the linked projects; explore the codebase; write `plan.md` with ordered steps, files to touch, test commands, and a **Team** section naming the subagents to use in Build (frontend, backend, best-practice reviewer, codebase-consistency reviewer, security reviewer, or fewer when the task is small) and what each owns. Steps that touch a linked project are grouped under that project's heading.
- **Page.** Terminal plus dock. The dock's Memory tab shows `plan.md` rendering live as the agent writes it.
- **Exit.** Sentinel plus a changed `plan.md` flips the chip to *Plan written, go?*. The developer reads, edits `plan.md` if they like, and presses "Start build".

### 6.3 Build

- **Entry.** The Build prompt tells the agent to execute `plan.md` step by step, fan out the team from the Team section as subagents, run the plan's tests, and **not commit**.
- **Page.** Terminal plus dock; the dock opens on the Team tab and grows to two thirds of the pane, showing the team canvas (section 10.7). The Changes tab counts files as they change, grouped by repo.
- **Conventions the Build prompt enforces (call)** so the canvas can be drawn from the event log: `plan.md` steps are numbered markdown checkboxes, and a step that does not simply follow the previous one ends its line with `(after 2, 7)`, which is what the Flow canvas draws as dependency edges; the orchestrating agent ticks each box as it completes; every subagent dispatch description begins with `Step N · role:` (several steps as `Step 5, 6 · frontend:`), which the app reads from the `Task` tool input in the hook event, so an ad-hoc role is named there and appears on the canvas under that name with a general-purpose tag; the final message ends with the stage sentinel.
- **Exit.** Sentinel plus changed files → *Build done*. The developer presses "Request review" or drags to Review.

### 6.4 Review

- **Entry.** The Review prompt asks the reviewers named in the Team section to review the diff against the task's base, in every repo the task touched, and write their notes to `review.md` as `repo/path:line` entries with a severity. Concurrently the **spec watcher** runs (section 7.4): a headless agent that reads the diff and updates `spec.md` so it describes what was actually built, touching only `spec.md`.
- **Page.** The dock opens on Changes: file list with change bars, grouped by repo; selecting a file opens the diff editor (unified or side by side) with reviewer notes pinned to lines. The developer can edit the working-tree side and save; the diff recomputes. Replying to a note sends it to the agent in the terminal. **(call: a reply is a prompt injected into the primary session, prefixed with the note's file and line)**
- **Exit.** "Approve changes" → Test if the home project has a test command and the task has Test enabled, else Commit.

### 6.5 Test (optional)

- **Entry.** The home project's configured command runs as a separate child process owned by the app, in the task's cwd, with its output streamed into a **Test** tab in the dock and its exit code captured. It does not run inside the agent's TUI, because that cannot return an exit code. Green chip on 0, amber on anything else. Linked projects' test commands can be added to the task from the Test tab. **(call)**
- **Exit.** Pass → Commit automatically if the project setting "advance on green" is on, else needs you. Fail → needs you; "Fix failing tests" sends the command, the exit code and the last 200 lines of output to the agent as a prompt and returns the card to Build. **(call)**

### 6.6 Commit

- **Entry.** A headless call drafts the commit message, or several messages when the diff has clearly separable concerns, one set per repo the task touched, each with the files it covers, written to `commits.md`. Chip: *Message drafted*.
- **Page.** The dock opens on a Commit tab: the drafted messages grouped by repo, each with Copy. A **Commit** button exists only when the project setting "Team Code may commit" is on (default off). With it off, the developer commits however they like.
- **Exit.** The app watches each repo: when the task's changed files are committed in every repo it touched, the card moves to Done with a toast. The developer can also drag it. **(call)**

### 6.7 Done and Archive

Done cards stay on the board until archived. Archive is available at any stage from the card menu or ⌘⇧A. Archiving ends the session (after confirmation if the agent is working), keeps the memory folder, and, under the Team Code-managed worktree convention, offers to remove the worktree if one exists. Archived tasks are listed behind a filter and can be restored.

### 6.8 Stage prompts

Each stage has a prompt template with placeholders (`{{task.folder}}`, `{{project.root}}`, `{{project.context}}`, `{{linked.projects}}`, `{{worktree}}`, `{{test.command}}`). Defaults ship in the core crate. Prompts are assembled in layers rather than replaced whole **(call)**: the **base** is the project's override if it has one, else the app's; the **workspace fragment** (the Linked projects block and any instructions the workspace adds) is appended to whatever base resolved, so a project with its own Build prompt still receives the cross-repo rules; **task additions** (the test command, the worktree path, the reviewer's reply) come last. Bases are plain text in `<app_config>/prompts/` and `<project memory>/prompts/`; workspace fragments live in the workspace file. The Plan, Build and Review prompts are written to work with a stock Claude Code; when the superpowers skills are installed they may be referenced by name but nothing depends on them. Every prompt carries the conventions the app relies on: numbered checkbox steps with optional `(after …)` dependencies in `plan.md`, `Step N · role:` at the start of each subagent dispatch, the stage sentinel as the last line of the final message, and the linked-projects policy.

## 7. Agents and terminals

### 7.1 Terminals

Every pane is a PTY owned by the Rust shell and rendered by xterm.js. Output streams to the webview over a Tauri channel in binary batches coalesced at roughly 60 Hz; input and resize go back as commands. Sessions live in `AppState`, not in React and not in any window: closing a pane or a window detaches the view, it does not kill the process. A session is attached to at most one pane at a time **(call)**: opening a task that is already open elsewhere offers "Show in <window>" or "Bring here", and bringing it moves the attachment, leaving a placeholder with "Bring back" in the old pane. A task's pane can be opened in any window whose project or workspace contains the task's home project. A task's primary session ends only by archiving the task, by an explicit "Stop agent" or "Start fresh session", or on app quit (with a warning listing tasks whose agents are still working). Ending is graceful first: the app asks the agent to exit the way a user would (`/exit` for Claude Code), waits up to 5 s so Claude Code's own exit logic can run, then kills the process. Scrollback is kept in Rust (configurable lines, default 10,000) so a re-attached pane repaints in full. **(call)**

**Session driver.** Sessions sit behind a `SessionDriver` trait in `teamcode-core` with two implementations: **Owned** (the app spawns the agent in its PTY; default for every agent kind in v1) and **Background** (for Claude Code: `claude --bg` starts the agent detached, the pane runs `claude attach <id>`, and on launch the app reconciles with `claude agents --json --cwd <root>`). Johnny chose Owned first. The Background driver is spiked in the first week against a long Build. **Pass bar for the spike (call):** attach shows the session's full current screen within 500 ms and echoes keystrokes within 50 ms under heavy output; after the app quits and relaunches, a session that was mid-turn is still running and re-attaches within 2 s with no lost turn, without any service the developer has to install or keep alive themselves (if macOS stops background sessions when Claude's background service stops, the driver must own keeping that service alive, or it fails this bar); hooks keep landing in the event log while the app is closed; memory overhead per attached task stays under 150 MB above Owned. Any failure rejects Background for this release. The switch also changes behaviour the plan must cover: Stop agent becomes `claude stop`, archive becomes `claude rm` (which may remove the session's worktree), quit no longer ends sessions, and launch reconciles against `claude agents --json`. The hook transport below is durable from day one so that switch costs nothing on the event side.

### 7.2 Agent kinds

- **Claude Code** (default): full stage automation. The primary session is `claude` launched in the home project root or worktree, with `--add-dir` for the memory folder when it is outside the cwd and for every linked project.
- **Codex**: the stages' prompts are injected the same way, but no hooks exist, so the chip only knows idle versus "process has had no output for N seconds". Stage moves are manual. **(call)**
- **Shell**: no automation; for plain terminals.

### 7.3 Driving and observing Claude Code

The app launches `claude` with `--settings <generated.json>` pointing at a per-task settings file it writes under app data. That file carries only what the app needs, so the developer's own settings, hooks, MCP servers and permissions apply as they normally would: the hooks (including the read-only guard hook when a linked root is protected), the status-line command from section 10.6, the permission denylist for protected roots, and, for Background-driver tasks whose worktree flag is off, `worktree.bgIsolation: "none"` so a background session may edit the shared checkout (Claude Code 2.1.169 and later otherwise refuse edits until the agent enters a worktree; the spike verifies this). The subagent roster goes through `--agents`, not this file. The app never writes into the project's `.claude/` folder. **(call)**

Each hook's command appends its stdin JSON as one line to a per-task event log, `<app_data>/events/<task-id>.jsonl`, with the hook event name added. The app tails these files with filesystem notifications. There is no server, nothing is lost while the app is closed, and the files double as an audit trail the Team view can replay. **(call)** Events used, all verified present in Claude Code 2.1.310 on Johnny's machine: `SessionStart` (capture the session id), `UserPromptSubmit`, `PostToolUse` (file edits feed the Changes tab and the change set), `SubagentStart` / `SubagentStop` (open and close a Team row, keyed by agent name), `PermissionRequest` and `Notification` (chip *Permission needed*), `Stop` (turn finished; `last_assistant_message` is checked for the stage sentinel, section 6), `StopFailure` (chip *Error*). If a later Claude Code version renames or drops an event, the app degrades to the Codex behaviour for that signal and says so in the Team tab.

Stage prompts are injected into the PTY as typed input followed by Enter, only when the session is at its prompt (the last event was `Stop` or `SessionStart`). If the agent is working, the stage action is queued and shown as "will send when idle". Bracketed-paste is used so multi-line prompts arrive intact. **(call)**

One Claude session per task by default, resumed across stages with `--resume <id>` after a restart. Each stage's primary action has a secondary "Start fresh session" for when the context is stale. **(call)**

### 7.4 Headless helpers

Short, single-turn calls through the reused LiveSpec provider layer (`claude -p` with `--restricted`, or the API key, or the local model, whichever Settings selects). They never take over the terminal.

- **Naming**: 3 to 6 word title from `intent.md`, fast tier, at task creation and again if the developer asks.
- **Spec watcher**: during Review, debounced on git changes in any repo the task touched, reads the diff and `spec.md`, rewrites only `spec.md`, logs each change in the dock's Memory tab with a one-line reason. It is allowed `Read`, `Grep`, `Glob` and `Edit` on `spec.md` only. **(call)**
- **Commit messages**: in Commit, from the diff, per repo, writes `commits.md`.
- **LiveSpec notes**: the Spec stage's live review, exactly as in LiveSpec.
- **Project briefs**: LiveSpec's one-time brief per project, built lazily, reused for the Linked projects section of every prompt.

### 7.5 Subagent roster

The app ships five subagent definitions as editable markdown files in `<app_config>/agents/`. The roster a task receives is the union of app, workspace and project definitions, with a name defined at a more specific level shadowing the same name above it, so a workspace can add a role and a project can retune one. **(call)** The five: **frontend**, **backend**, **best-practice reviewer**, **codebase-consistency reviewer**, **security reviewer**. They are injected into every Claude Code task session with `--agents` (converted to inline JSON at launch, which is what interactive mode accepts), so nothing is written into the project's `.claude/` folder and the developer's own agents remain available too. The Plan prompt asks the agent to choose from this roster for the `## Team` section (fewer for a small task) and may add ad-hoc roles. The Build and Review prompts refer to the roster by name, and the Team tab keys its rows on the agent name carried by `SubagentStart`. **(call, implementing the team Johnny described)**

## 8. Task memory on disk

**Location** is a project setting with a folder picker:

- **App storage** (default): `<app_data>/projects/<project-id>/tasks/`.
- **Inside the project**, any folder the developer chooses, for example `<root>/docs/tasks/`. The app offers to add it to `.gitignore` or leave it tracked; it does not decide.

A task folder is named `YYYY-MM-DD-<slug>/` and contains:

| File | Written by | Content |
|---|---|---|
| `intent.md` | developer (or Jira import) | the original ask, verbatim, never rewritten |
| `spec.md` | developer, LiveSpec-reviewed; later the spec watcher | the spec |
| `plan.md` | agent in Plan | numbered checkbox steps with `(after …)`, files, tests, a `## Team` section, and a heading per linked project touched |
| `review.md` | reviewers in Review | `repo/path:line · severity · note` entries |
| `commits.md` | commit helper | drafted message(s) per repo and the files each covers |
| `index.md` | the app | front matter (ids, home project, workspace if any, Jira key, branch, worktree, Claude session ids, stage) and a timeline: each stage entry and exit with timestamps, each hand-back, commits detected per repo |

`index.md` is the one file the app rewrites; everything else is append-or-edit by whoever owns it. Deleting the folder makes the task "memory missing" on the card; the task is not deleted. **(call)**

The task folder path always resolves against the home project root (the main checkout), never against a worktree, so memory does not live on a task branch and does not disappear when a worktree is removed. When the agent's cwd is a worktree, its session is launched with `--add-dir <task folder>` so it can read and write the memory files. Archiving never deletes memory. **(call)**

## 9. Git, worktrees and base

- **Worktree** is a project default (off) with a per-task override set at creation or before Plan. First-run onboarding asks which convention to use (Johnny's decision), stored as the app default and overridable per project: **Claude Code's own worktrees** (the session is launched with `claude --worktree <slug>`, honouring the user's `worktree` settings such as `baseRef`, under `.claude/worktrees/`), or **Team Code-managed** (`git worktree add <worktrees-dir>/<task-slug> -b tc/<task-slug>` from HEAD, `<worktrees-dir>` defaulting to `<root>/.worktrees/`). Codex and shell tasks always use the Team Code-managed kind. The worktree is created when the task's session first starts, at Plan entry, because a running session cannot change directory. Only the home project gets a worktree; linked projects are edited in place. **(call on the mechanics)**
- **When the flag can change (call).** The per-task worktree flag is editable until the session first starts at Plan entry. After that the card-menu item is greyed out with a hint; changing it means "Start fresh session", which ends the session and launches a new one in the new location, with `plan.md` and the memory folder carrying the state.
- **Base** for diffs and the Changes tab: per repo, the commit the task started Build from (recorded in `index.md`), or HEAD when no worktree is used and the developer has committed in between.
- **The task's change set. (call)** With a worktree, the home repo's part is the whole diff against base. Otherwise, and always for linked repos, several tasks may share a working tree, so the change set is the files this task's session and its subagents touched, attributed from `PostToolUse` events for `Edit`, `Write`, `MultiEdit` and `NotebookEdit`, plus files that appear in `git status` while a `Bash` tool call from this session is running. Reviewers, the spec watcher, the Changes tab and the Commit-stage done detection all use this set. Files changed in a working tree but not attributed to the task are listed under "Other changes" in the Changes tab, excluded by default, and can be pulled into the task's set with one click.
- The app runs git through the `git` binary, never a bundled library, so hooks, LFS and credential helpers behave as the developer expects. **(call)**
- The app never merges, rebases, pushes or deletes branches. Worktree lifecycle follows the convention chosen at first run **(call)**. Under **Team Code-managed**, removing a worktree on archive is offered, never automatic. Under **Claude Code's own**, Claude Code's rules apply and the first-run screen says so: a clean worktree is removed when its session exits, a dirty one gets Claude's keep-or-remove prompt in the terminal, stale worktrees under `.claude/worktrees/` are swept by Claude Code, and `claude rm` removes a session's worktree when safe. The app ends sessions gracefully (section 7.1) so those rules can run, resumes with `-w <name> -r <id>`, and treats a clean worktree that Claude removed between Plan and Build as nothing lost: the next launch recreates it from the same name.

## 10. The interface

### 10.1 Shell

**Single-project window.** Title bar shows the project name. Below it the **tab strip**: **Board** is tab zero (⌘B), every other tab is a view; ⌘⇧[ and ⌘⇧] cycle, ⌘N makes a view, tabs drag to reorder, and a tab shows a small amber count when a pane inside it needs you. Along the bottom the **attention strip**: the project's needs-you tasks as chips, newest first, click to open that task in the current view; it collapses on the board, which shows the same thing. On the left, a collapsible **sidebar** (⌘\), Johnny's pick, with an **icon strip across its top** that switches what the sidebar shows. The strip is a horizontal row of icons at the top of the sidebar, not a vertical activity bar. Its sections **(call on the set)**: **Tasks** (Views, then Needs you, Running and Idle, each task one row with its colour and chip; click opens it in the current view), **Docs** (the project's context docs and Confluence pages, the memory folder location, drag a file in to add it), **Changes** (files changed across the project's active tasks, grouped by task, click opens that task's Changes tab), **Ticket** (linked Jira issues and a filter box), and **Settings** for the project. The sidebar is hidden by default below 1,100 px, and the selected section is remembered per window.

**Workspace window.** Title bar shows the workspace name. The sidebar becomes **multi-root**, Johnny's pick, VS Code style: under the same icon strip, each project is a collapsible root with its colour, its needs-you count and its tasks, and in every section the content is grouped by project root. Views lists the workspace's views, each tab carrying the colours of the projects whose tasks it holds. A setting switches the project layer to the **project rail** instead: a narrow rail of project icons with needs-you badges on the far left, ⌘1 to ⌘9 to switch, the sidebar showing only the focused project and the Board tab showing that project's board, with the swimlane board still reachable as All. The tab strip and the views stay workspace-wide in both modes, so toggling the setting moves nothing. **(call)** The Board tab shows all projects as swimlanes (section 10.2). ⌘1 to ⌘9 focus the Nth project: with the multi-root sidebar it scrolls to that root and highlights its swimlane; with the rail it switches the shown project; pressing the same key again clears the focus. The attention strip shows every project's needs-you items with the project colour.

**Everywhere.** The **command palette** ⌘K: go to task, project, view or window; new task; stage actions for the focused task; add project to workspace; theme and settings. ⌘` cycles windows, as on any Mac app. The macOS Dock badge shows the needs-you total across all windows. **(call)**

### 10.2 Board

- Seven columns, horizontally scrollable past about 1,200 px, with column counts in the header.
- Cards follow the app-wide **Detail level** setting (Johnny's decision: three levels, Simple by default, applied to board cards, canvas step cards and team rows alike; app level with a per-project override). **Simple**: two lines, title with priority flag and Jira key, then status chip, agent avatar and elapsed time; task colour is a 2 px left edge; hover shows the last agent line. **Medium**: adds a third line with the stage stepper and the file the agent is touching with `+/-`. **Detailed**: adds the team row with each subagent's state and, when the cost overview is on, the task's cost. **(call on what Medium and Detailed contain)**
- **In a workspace window** the board is **swimlanes** by default, Johnny's pick: columns are stages, one row per project with its colour and name at the left edge, rows collapsible and reorderable by drag, and dragging a card within its row works as on a single-project board. Cards cannot be dragged between rows, because a task has one home project. Settings offer **One project at a time** (a picker in the board header chooses the project; an All option shows the swimlanes read-only) and **Mixed** (one set of columns, every card wearing a badge in its home project's colour, a chip row above the columns filtering by project with counts and a Needs you chip). **(call on the row controls)**
- Card menu and shortcuts: open in current view, open in new view (⌘-click), open in another window, rename (⌘E), priority, colour, flag, hide, archive (⌘⇧A), link Jira issue, toggle worktree (until the session first starts; afterwards through Start fresh session), stop agent.
- Filters in the board header: needs you, by priority, hidden, archived, by colour, by Jira.
- Drag and drop between and within columns with dnd-kit; the drop animates with Motion's layout animation and a spring; the destination column glows briefly.
- Empty column: a small Lottie illustration and a hint line.

### 10.3 Task page

Opens in a pane. Header: back to board, home project badge (in workspace windows), title (click to rename), stage stepper (six chips, done ones green, current one in its stage colour), the stage's primary action button, and an overflow menu. The body is the terminal; below it the **dock** (⌘J), which a setting can move to a **right inspector**. Dock tabs: **Memory** (intent, spec, plan, review, commits, index as rendered markdown with an edit toggle), **Team** (the team view, section 10.7), **Changes** (file list grouped by repo and the diff editor), **Ticket** (Jira summary, status, assignee, link; comment composer), **Test** (the test run's output and exit code) when the project has a test command, and in Commit stage **Commit**. The dock remembers its tab per stage.

In Spec the body is the editor (section 6.1) and the terminal is a strip; expanding it splits the pane.

### 10.4 Views and splits (Johnny's pick: free-form)

A view's layout is a binary tree of horizontal and vertical splits with ratios. ⌘D splits right, ⌘⇧D splits down, ⌘W closes the pane, ⌘⌥ arrows move focus, dragging a divider resizes, dragging a pane header moves it to another slot. A new split opens the task picker (⌘P in a pane does the same); in a workspace the picker lists tasks from every project with their badges. Layouts persist per view, and a pane remembers which task it showed so relaunch restores everything, including re-attaching to sessions still running.

### 10.5 Keyboard map (proposed; the builder resolves conflicts with the terminal)

| Keys | Action |
|---|---|
| ⌘` | next window |
| ⌘1…⌘9 | workspace window: focus the Nth project; single-project window: the Nth tab |
| ⌘B | board |
| ⌘⇧[ / ⌘⇧] | previous / next view |
| ⌘N / ⌘T | new view / new task |
| ⌘K | command palette |
| ⌘P | open task in this pane |
| ⌘\ | toggle sidebar |
| ⌘D / ⌘⇧D | split right / down |
| ⌘W | close pane |
| ⌘⌥ ←↑→↓ | focus pane |
| ⌘J | toggle dock |
| ⌘⇧J | grow or shrink the Team tab (or swap / split, per the placement setting) |
| ⌘⏎ | primary stage action |
| ⌘E | rename task |
| ⌘⇧A | archive task |
| ⌃` | expand/collapse terminal strip in Spec |
| ⌘, | settings |

The terminal receives every key the app does not claim; the app claims only ⌘ chords listed here.

### 10.6 Cost overview (Johnny's addition)

A setting, off by default, that turns on cost and token display everywhere a task appears:

- **Per task**: in the task page header next to the stepper (USD and tokens for the whole task, all sessions and subagents) and in the card tooltip; a small context-window meter for the primary session so you can see when a fresh session is due.
- **Per view**: the total of the tasks open in that view, in the view's tab tooltip and at the right end of the attention strip.
- **Per project and per workspace**: the board header shows the total for cards on the board, with archived cards excluded, and a Costs panel in project or workspace settings breaks it down by project, task, stage and day.

Sources **(call)**: the session transcript JSONL that Claude Code writes (the `transcript_path` in every hook payload), which carries per-message token usage and model for the main session and its subagents, priced with the same table LiveSpec's usage ledger uses; and, for live numbers between turns, a status-line command injected through the same generated settings file, which receives Claude Code's running `cost.total_cost_usd` and context-window usage and appends them to the task's event log. The injected status line forwards its input to the user's own status-line command so their display is unchanged. Headless helper calls and LiveSpec notes are metered by the reused ledger. Estimates are shown with "~", as in LiveSpec.

### 10.7 Team view (Johnny's rounds 4 and 5)

The Team tab is a **canvas**: every plan step is a card, every agent is a node, and an agent draws an edge to each card it is touching. A toggle switches to **Outline**, `plan.md` rendered as a live checklist whose active steps expand to show that agent's tool and hook stream. Swimlane and mission-control layouts are out of v1.

**Arrangement.** Default **Hub graph**: agents are hubs in the middle, their step cards around them, files as small cards of their own, so a file two agents both edit sits between them with a warning border. No dependency edges. The **Flow canvas** is available in settings: step cards left to right by dependency with grey "depends on" edges, agents docked along the bottom pointing up at their current cards, files inside the cards. In a workspace task, cards and files carry the badge of the repo they belong to.

**Nodes.** Step cards follow the Detail level: Simple shows number, title, owner, status and a file count; Medium adds the files with `+/-`; Detailed adds the live tool line, a progress bar and cost. Done and upcoming steps always collapse to the title line. Agent hubs show the role, a Rive ring while working, and cost when the overview is on. File cards show the path, net `+/-`, and the owners' avatars.

**Live layout.** Default **Auto re-flow**: done steps slide into a stack, the active ones stay centred, new file cards slot in beside their agent, all with a 300 ms spring. **Pinned** is available in settings: auto-layout runs at Build start and on Tidy, otherwise cards stay where they are or where the developer dragged them. Positions are saved per task so a hand-arranged canvas comes back the same. Pan, zoom, minimap, Tidy.

**Placement.** Default: the dock grows to two thirds of the pane on ⌘⇧J or by dragging its handle, remembering the height per stage. Settings offer **Swap** (⌘⇧J flips the pane between terminal and team, the hidden one shown as a one-line strip) and **Own pane** (the team view opens as a linked pane beside the terminal and behaves like any pane).

**Drill-in.** Clicking an agent or a file opens the drill-in panel. Default lens **By file**: every file the agent touched, who else touched it, net change, and under each file its edits in order with a mini diff and the plan step each served; commands and hook firings roll up below. The **Event stream** lens is available in settings: newest first, one line per tool call with time, tool, target, result and duration, hook rows muted, filter chips by kind and by agent. Both lenses show hook firings.

**Data. (call)** Everything is derived from the task's hook event log and the memory folder: `SubagentStart` / `SubagentStop` create and close agent nodes; the `Task` tool's input in `PreToolUse` gives each subagent its description, from which `Step N · role:` is parsed; dependencies come from `(after …)` in `plan.md`, and a step without one follows the previous step; `PostToolUse` for `Edit`, `Write`, `MultiEdit`, `NotebookEdit` and `Bash` gives files, `+/-` and commands; the checkboxes in `plan.md` give step status; a file touched by two agents is flagged the moment the second edit arrives. Because the log is durable, a finished Build can be replayed on the canvas later. Where hook payloads for tool calls made inside a subagent do not carry the agent id, the builder falls back to the subagent transcripts (section 17).

## 11. Look and feel

### 11.1 Themes (Johnny's picks)

- **Light, default: Soft White Neutral.** Page `#f6f6f7`, bars `#f0f0f2`, column wells `#ededef`, cards `#ffffff`, lines `#e2e2e6`, ink `#18181b`, muted `#71717a`, accent `#3b6cff`, ok `#1f9d55`, warn `#d97706`.
- **Dark: Graphite.** Page `#111318`, bars `#0d0f13`, wells `#161920`, cards `#1a1e26`, lines `#262b35`, ink `#e7e9ee`, muted `#8a91a0`, accent `#5b8cff`, ok `#3ddc84`, warn `#f5a524`.
- **System** follows macOS.
- Stage colours are shared by both themes: Spec violet `#7c5cff`, Plan blue `#2f6fed`, Build amber `#e8961e`, Review pink `#e0457b`, Test teal `#14b8a6`, Commit green `#1f9d55`, Done grey `#8e8e93`. Project colours in a workspace are chosen from a separate palette so they never collide with stage colours. **(call)**
- Character: 1 px lines, 5 px radii on cards and 6 px on wells, no shadows in dark and a 1 px shadow in light, Inter, colour only for stage, status, project and accent.
- Accent presets (blue default, plus violet, teal, orange) change the accent token only. **(call)**
- Tokens are CSS variables on `:root`, switched by a `data-theme` attribute, with Tailwind v4 semantic names as in LiveSpec (`bg-page`, `text-ink`, …).

### 11.2 Terminal theme

Default **follows the app**: a light ANSI palette tuned for Soft White, and Graphite's palette in dark. A separate setting offers Follow app, Always dark, Always light, or a named palette from a small bundled set (Graphite, Soft White, One Dark, Solarized Light/Dark). Font and size are settings; ligatures off by default. **(call on the bundled set)**

### 11.3 Motion (Johnny's pick: expressive)

- Motion's layout animations for board moves, list reordering and pane changes; springs on drag end; the destination column glows for 400 ms when a card lands.
- Rive: the "agent working" indicator on cards and on the canvas's agent hubs, one small looping file; the stage stepper's current chip breathes.
- Canvas: edges from a working agent to its cards carry a flowing dash; cards re-flow with a 300 ms spring; a newly shared file pulses its warning border once.
- Lottie: empty columns, empty board, first run, and a short moment when a card reaches Done.
- Budget: nothing a user triggers takes longer than 250 ms to respond; decorative motion never blocks input; `prefers-reduced-motion` turns springs into 150 ms eases and stops loops.

## 12. Integrations and context

- **Jira** (optional, per project; a workspace may set a default site): site URL, email and API token in the keychain. Link a task to an issue key; the Ticket tab shows summary, status, assignee and a link; a task can be created from an issue; the developer can post a comment from the Ticket tab. Automatic transitions on stage change are a per-project option, off by default. **(call on scope)**
- **Confluence** (optional): page links attached as project or workspace context; the page text is fetched on demand and passed to stage prompts as context. **(call)**
- **Context docs**: a list of files or folders (markdown, PDF, images) and URLs per project, and per workspace. Their paths are included in every stage prompt under "Project references" and "Workspace references". The app does not parse them; the agent reads what it needs.

## 13. Settings and defaults

Settings resolve app → workspace → project → task, the most specific winning, except stage prompts and the subagent roster, which layer as described in sections 6.8 and 7.5. **(call)**

| Setting | Level | Default |
|---|---|---|
| Theme / accent / terminal theme | app | System / blue / follow app |
| Detail level (Simple / Medium / Detailed) | app, project override | Simple |
| Sidebar shown / section | per window | shown, hidden below 1,100 px / Tasks |
| Workspace project layer | app, workspace override | Multi-root sidebar (Project rail available) |
| Workspace board | app, workspace override | Swimlanes (One project at a time, Mixed available) |
| Dock position | app | bottom |
| Team view arrangement | app | Hub graph (Flow canvas available) |
| Team view placement | app | Dock grows (Swap with terminal, Own pane available) |
| Team canvas layout | app | Auto re-flow (Pinned available) |
| Team drill-in lens | app | By file (Event stream available) |
| Memory location | project | app storage |
| Worktree | project, task override | off |
| Worktree convention | app (asked on first run), project override | Claude Code's own or Team Code-managed |
| Worktrees dir | project, Team Code-managed kind only | `<root>/.worktrees/` |
| Linking policy | workspace, project override | Read and modify on request |
| Team Code may commit | project | off |
| Test command / advance on green | project | none / off |
| LiveSpec level | remembered per project | off |
| Stage prompts | app, workspace and project overrides | shipped defaults |
| Agent kind for new tasks | project | Claude Code |
| Subagent roster | app, workspace and project overrides | the five shipped roles |
| Model provider and tiers for headless calls | app | as LiveSpec |
| Cost overview | app | off |
| Jira / Confluence | project, workspace default | unlinked |
| Scrollback lines | app | 10,000 |

## 14. Architecture

```
src/                  React UI
  components/ui/      the primitives library (section 15.3), the only place raw form elements appear
  features/<name>/    board, sidebar, task-page, dock, canvas, views, spec-editor, start-screen, palette, settings
  lib/                module stores, generated IPC bindings, pure logic
  livespec/           the panes copied from LiveSpec
src-tauri/            thin shell: commands, AppState (ptys, sessions, hook tailer, git watchers), windows, keychain, menu
crates/teamcode-core/       no Tauri: domain model (windows, workspaces, projects, tasks), pipeline state machine, prompts, memory folder I/O, git, SQLite, jira, hooks protocol
crates/livespec-core/ copied from LiveSpec: providers (claude -p, API, local), project brief and retrieval, LiveSpec sections and prompts, usage ledger
```

- `teamcode-core` depends on `livespec-core` for every model call and for the Spec stage. The LiveSpec React panes (`DraftPane`, `NotesPane`, `MiniMarkdown`, `TabSession` logic) are copied into `src/livespec/` and adapted to bind to a file instead of a tab draft. **(call)**
- **Windows** are created in Rust only, as in LiveSpec (`w-<n>`), one per open project or workspace. Each window's webview holds only that window's UI state; sessions, tasks and the pipeline live in `AppState` and `teamcode-core`, shared by every window. App-wide facts (stage changed, needs-you count, workspace membership changed) go out as events to every window. **(call)**
- The pipeline is a state machine in `teamcode-core` with an event log; every transition writes to SQLite and appends to `index.md`. UI never mutates stage directly.
- The hook tailer watches `<app_data>/events/*.jsonl`, parses each new line into a typed event in `teamcode-core`, and drives the pipeline state machine, the Team view, the change set and the cost ledger. On launch it replays what arrived while the app was closed.
- Git watching uses filesystem events on each repo a task touches (`notify` crate) debounced to 500 ms, then `git status --porcelain` and `git diff --numstat` against the base. **(call)**
- IPC mirrors LiveSpec: commands return quickly, long work streams over channels, app-wide facts go out as events to every window. Types are mirrored by hand between Rust and TypeScript with a list in `AGENTS.md`, as LiveSpec does.
- Persistence: SQLite at `<app_data>/teamcode.db` (projects, tasks, sessions, views and layouts as JSON, events); workspace files as JSON (section 4.2); settings JSON in `<app_config>`; prompts and agent definitions as text files; secrets in the keychain under `app.teamcode.desktop`.
- Security: no secret crosses IPC; model and agent output render through `MiniMarkdown` or xterm, never `innerHTML`; CSP `default-src 'self'`; task, session and window ids validated before they become paths; the generated Claude settings file contains only the hooks (including the read-only guard), the status line, the protected-root permission rules and, when needed, the background-isolation key; `--add-dir` is granted only for projects the developer put in the workspace.

## 15. Engineering standards (Johnny's requirement)

Johnny asked for three things of the build itself: a codebase that is well structured and stays maintainable, a good `CLAUDE.md` with rules that keep it consistent, and lightweight reusable components instead of raw HTML controls. They bind the implementation plan; the calls below are how Claude proposes to meet them.

### 15.1 Structure

- The three layers of section 14, and inside the webview a **feature folder per area**: `src/features/<feature>/` with `components/`, `store.ts`, `types.ts` and an `index.ts` that is the feature's only public surface. Shared code lives in `src/components/ui/` (primitives), `src/lib/` (stores, bindings, pure logic) and `src/livespec/`. A feature imports another feature only through its `index.ts`, never its store or internals. **(call)**
- Rust: `crates/teamcode-core/src/<domain>/` modules (pipeline, sessions, memory, git, workspace, hooks, cost, jira, prompts), each with its tests beside it; the shell groups commands in one file per domain.
- **One source of truth for IPC types**: `tauri-specta` generates `src/lib/bindings.ts` from the Rust command and event types at build time; hand-written mirrors are forbidden. **(call: departs from LiveSpec's manual mirroring because this surface is several times larger)**
- Size budgets, enforced in review: a component file under about 200 lines, a store under 300, a Rust module under 500. A file that outgrows its budget is split, not excused. **(call)**
- Naming follows LiveSpec: components `PascalCase.tsx`, lib and stores `camelCase.ts`, Rust `snake_case`, semantic Tailwind tokens only (`bg-page`, `text-ink`, …), no hex colours outside the theme file.

### 15.2 CLAUDE.md and rules

- `CLAUDE.md` at the repo root, kept under about 150 lines: what the app is in three lines; the layer rules; the commands to run (dev, typecheck, lint, unit tests, clippy); the primitives rule; the generated-bindings rule; naming; how to add a command, an event, a feature and a primitive; where the design document, the decisions log and the ADRs live; and the current stage of the project.
- Path-scoped rules in `.claude/rules/`, each short and applied only to the paths it names **(call)**: `ui.md` for `src/**` (primitives only, tokens only, Motion for animation, keyboard and focus behaviour, reduced motion), `ipc.md` for `src-tauri/**` and the bindings (commands return fast, long work streams, ids validated before they become paths, no secrets across IPC), `core.md` for `crates/**` (no Tauri dependency, `Error::{Precondition, Provider}` semantics, tests beside code), `prompts.md` for the stage prompts and the conventions the app depends on (checkbox steps, `Step N · role:`, the sentinel).
- `ARCHITECTURE.md` kept current the way LiveSpec's is, and a one-page ADR in `docs/adr/` for every decision that changes a layer rule or adds a dependency. **(call)**
- These files are the first implementation task, written before any feature code, and every review step in the plan checks the change against them.

### 15.3 UI primitives

- `src/components/ui/` is the only place raw HTML form and control elements appear. Everything else composes the primitives: `Button`, `IconButton`, `Input`, `Textarea`, `Select`, `Combobox`, `Checkbox`, `Switch`, `Segmented`, `Tabs`, `Menu`, `ContextMenu`, `Popover`, `Tooltip`, `Dialog`, `Drawer` (Vaul), `Toast` (Sonner), `Field` (label, control, hint, error), `Chip`, `Badge`, `Avatar`, `KeyHint`, `StatusDot`, `EmptyState` (Lottie), `Panel`, `Resizable`. They are thin wrappers over shadcn/ui and Radix, styled with the semantic tokens, with keyboard and focus behaviour built in and no business logic. **(call on the list)**
- Enforced by lint, not by memory: an ESLint rule fails the build on `<input>`, `<select>`, `<textarea>`, `<button>` and `<dialog>` outside `src/components/ui/`, and on hex colours or inline colour styles outside the theme file. **(call)**
- Each primitive has an example page under `src/dev/primitives/`, reachable only in dev builds, so a new one is designed and reviewed in isolation before it is used. **(call)**

### 15.4 Quality gates

- On every commit: `npm run typecheck`, `npm run lint`, `npx vitest run`, `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`. A lefthook pre-commit runs the fast subset; CI runs everything. **(call)**
- Tests where the logic lives: Vitest for `src/lib` and feature stores, cargo tests for the core, and a small Playwright smoke suite against the dev build covering the board, the task page and the canvas. Components are tested through their stores and a few render tests, not by snapshotting everything.
- Conventional commit messages; the Commit stage's helper drafts in the same convention.

## 16. Performance targets

| Target | Number |
|---|---|
| Cold start to interactive | under 1 s |
| Window, project or view switch | under 100 ms, no terminal repaint flicker |
| Terminal output at full speed | no dropped frames at 60 Hz, batched |
| Idle CPU with 10 attached terminals | under 1 % |
| Board with 200 cards | drag at 60 fps |

## 17. Left to the builder

- Exact xterm.js and portable-pty wiring, flow control, and the binary channel format.
- The layout-tree data structure and persistence format for views.
- CodeMirror language packs to bundle and how reviewer notes map to editor decorations.
- The Lottie and Rive assets themselves, and the exact spring constants.
- ELK layout options for the hub graph and the flow canvas, and the collapse animation for done-step stacks.
- `--agents` takes inline JSON in interactive mode, so the markdown roster is converted at launch.
- Per-subagent attribution: verify which hook payloads carry `agent_id` / `agent_type` for tool calls made inside a subagent in the installed Claude Code, and the on-disk layout of subagent transcripts; fall back to parsing those transcripts when hook payloads are not attributed.
- Schema details, migrations, and how tasks are listed when their memory folder lives in a repo that is cloned fresh on another machine (reading `index.md` front matter to rebuild rows is the intended path).
- Moving a pane or tab between windows by drag; v1 offers "Open in window…" from the card menu.

## 18. Decisions reference

Every pick and call above traces to `docs/superpowers/brainstorm/2026-10-24-decisions.md`. Rounds: 1 shell and board, 2 task page and stages, 3 look and feel, 3b soft white, 4 team view, 5 team canvas, 6 windows, workspaces and sidebar.
