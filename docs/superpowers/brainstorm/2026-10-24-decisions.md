# Team Code — brainstorm decisions log

Running record of what Johnny chose during the 2026-10-24 brainstorm, so the design document can be written from it. "Johnny" lines are their picks; "Claude" lines are calls Claude made that stand unless overridden.

## Round 1: the shell

- **Shell layout: B, Warp-style tabs.** (Johnny) Projects on a left rail, ⌘1–9. Each project has tabs: the Board is tab zero (⌘B), every other tab is a view. A view is a split-pane canvas of task pages. ⌘⇧[ and ⌘⇧] switch views. An attention strip along the bottom shows "needs you" items while working in a view.
- **Columns: seven, hand-backs as chips.** (Johnny) Spec · Plan · Build · Review · Test (optional) · Commit · Done. The hand-back sub-stages (spec ready, plan written, build done, message drafted) are an amber status chip on the card, not separate columns. Green pulsing chip = agent working, grey = idle. A "Needs you" filter and the attention strip collect the amber ones.
- **Card density: compact by default, configurable.** (Johnny) Default card is two lines: title, priority flag, Jira key, one status chip, which agent, elapsed time, task colour on the left edge. A density setting switches to the rich card (stage stepper, current file and diff size, subagent team).

## Standing calls from Claude (not yet objected to)

1. Real PTY terminals per task (xterm.js in the webview, portable-pty in Rust) running interactive Claude Code or Codex. Stage transitions send prompts into the session; status comes back through Claude Code hooks.
2. Build-stage fan-out is done by the task's own Claude Code session using its subagents and the team plan. The app observes through hooks. The app does not spawn its own worker processes in v1.
3. LiveSpec is copied into this repo's Cargo workspace (core crate and the draft/notes React panes), not a git dependency.
4. Claude Code is first-class in v1; Codex runs as a plain terminal with manual stage moves.
5. Stack: Tauri 2, React 19, TypeScript, Vite, Tailwind v4, shadcn/ui + Radix, Motion, Sonner, Vaul, dnd-kit for the board, Lottie and Rive where they earn it. App state in SQLite, task memory as markdown files.
6. Non-goals: not an editor/IDE, no cloud sync or multi-user in v1, never commits on its own.

## Round 2: the task page

- **Frame: B, terminal plus bottom dock, by default.** (Johnny) Full-width terminal with a dock underneath holding Memory, Team, Changes and Ticket tabs. A setting moves the dock to a right inspector (option A). Header is shared: back, title, stage stepper, one primary action per stage.
- **Spec stage: A, you write and LiveSpec reviews, with LiveSpec optional.** (Johnny) The draft is `spec.md` in the task folder. LiveSpec is off by default. A switch on the spec page offers Off / Simple / Heavy. Simple uses fewer agents to check the work; Heavy is the full review. The terminal is a strip until Plan.
- **Review stage: A, in-app diff, and it edits.** (Johnny) The diff view is also a lightweight code editor, VS Code-like but small: edit the current file in place, save, and see the diff update. Reviewer and security notes are pinned to lines.

Claude's calls under these picks (not yet objected to):

7. Simple = one lane, fast tier, Conflicts, Gaps and Questions only, no fact-check. Heavy = all five sections, parallel lanes, deep tier, fact-check when a TypeSafe key exists. The last choice is remembered per project.
8. The editor is CodeMirror 6 with the merge extension: edit the working-tree side, ⌘S saves, diff recomputes against the task's base (worktree base or HEAD). No LSP, no multi-file tabs beyond the changed-files list in v1.

## Round 3: look and feel

- **Character: Graphite.** (Johnny) Neutral greys, 1px lines, small radii, system sans, one blue accent; colour only for stage, status and accent. Johnny asked for a **soft white** light variant rather than the default dark, so the light theme is the lead and Graphite dark is its twin. Tint and terminal treatment pending in round 3b.
- **Motion: expressive.** (Johnny) Springs on drag and drop, a glow where a card lands, Rive indicators for live agents, Lottie illustrations for empty states and first run, a small moment when a task reaches Done. Interactions stay under 250 ms and respect reduced motion.
- **Pane splits: free-form.** (Johnny) iTerm/Warp style nested splits in any direction, draggable dividers, ⌘D split right, ⌘⇧D split down, keyboard pane focus. The layout tree persists per view.

## Round 3b: soft white

- **Tint: Neutral.** (Johnny) True grey page (#f6f6f7), white cards, lines #e2e2e6, ink #18181b, accent #3b6cff. Matches macOS system greys. This is the default light theme; Graphite dark is its twin; System follows macOS.
- **Terminal: goes light with the app, and is configurable.** (Johnny) Default is a light ANSI palette tuned for soft white. A terminal theme setting lets the terminal follow the app, stay dark, stay light, or use a named terminal palette.

Visual rounds complete. Next: design document at `docs/superpowers/specs/2026-10-24-contextswitch-design.md`, builder check, then one round of questions.

## Builder check round

- **Worktrees: ask on first run.** (Johnny) The first-run onboarding asks which worktree convention to use: Claude Code's built-in worktrees (`claude --worktree`, under `.claude/worktrees/`, honouring the user's `worktree` settings) or ContextSwitch-managed worktrees (`<root>/.worktrees/<slug>`, branch `cs/<slug>`). The answer becomes the app default, overridable per project. Either way the worktree is created when the task's session first starts (Plan entry), not at Build.
- **Owned processes first.** (Johnny, following Claude's recommendation) Session layer behind a driver trait with Owned and Background implementations; Owned is the v1 default for every agent kind; hooks append to a durable per-task event log instead of a localhost server; Background (`claude --bg` + `attach`) is spiked in week one and becomes the Claude Code default before release if it holds up.
- **Cost overview setting.** (Johnny) Off by default; when on, shows cost and tokens per task and the total per view, plus per project on the board.
- Settled by Claude after the builder check (in the design doc as calls): stage-done sentinel plus artifact check; hook-attributed per-task change set; memory folder on the main checkout with `--add-dir`; shipped roster of five subagents via `--agents`; Test runs as a separate process in a Test tab.

## Round 4: the team view during Build

- **Shape: a free canvas.** (Johnny) Not one of the three offered. Each plan step is a card; agents are nodes; an agent points at every card it is working on. Arrangement, card content and live-layout behaviour are being settled in round 5.
- **Where it lives: the dock grows, by default.** (Johnny) All three placements (swap with terminal, own pane, dock grows) exist and are selectable in settings.
- **Drill-in lens: by file, by default.** (Johnny) The event stream is the other lens, selectable in settings.

## Round 5: the team canvas

- **Arrangement: Hub graph by default, Flow canvas in settings.** (Johnny)
- **Step card content: Simple by default, as one app-wide Detail level with three options, Simple / Medium / Detailed, that also drives board cards and team rows.** (Johnny) The rich card is the Detailed level.
- **Live layout: Auto re-flow by default, Pinned in settings.** (Johnny)
- Claude's calls: React Flow (xyflow) with elkjs; positions persisted per task; Outline kept as second mode; Build prompt conventions (checkbox steps in plan.md, `Step N:` prefix on subagent dispatches, stage sentinel) so the canvas can be drawn from the hook event log.

Visual rounds complete.

## Second builder check

Five questions, all settled by Claude as calls in the design doc: `worktree.bgIsolation: none` in the generated settings for Background tasks with the worktree flag off; worktree lifecycle follows the chosen convention (Claude Code's own rules under its convention, offered-never-automatic under ContextSwitch-managed) with graceful session exit; `(after …)` dependency notation and `Step N · role:` dispatch prefix added to the prompt conventions; the per-task worktree flag is editable until Plan entry, afterwards via Start fresh session; a concrete pass bar for the Background spike. Document handed to Johnny for approval.

## Round 6: windows, workspaces, sidebar

- **Model change.** (Johnny) Like VS Code: a window holds one project; a workspace links several projects into one window, and only then does a multi-project left sidebar appear. In a workspace all projects are linked: agents can read each other's code for references and modify it when the task asks. Design document rewritten around this (sections 4, 5, 7.2–7.3, 9, 10.1–10.2, 12–14).
- **Single-project sidebar: collapsible sidebar with an icon panel across its top.** (Johnny) The icons switch the sidebar's section; Claude's call on the set: Tasks, Docs, Changes, Ticket, Settings.
- **Workspace project layer: multi-root sidebar by default, project rail in settings.** (Johnny)
- **Workspace board: swimlanes per project by default; One project at a time and Mixed with badges in settings.** (Johnny)
- Claude's calls pending objection: workspace file `<name>.contextswitch-workspace.json`; linking via `--add-dir` for every linked root with a Read-and-modify-on-request default and a Read-only policy; only the home project gets a worktree; a task has one home project; settings resolve app → workspace → project → task; ⌘1–9 focus projects in a workspace and tabs in a single-project window; ⌘` cycles windows; Dock badge shows the needs-you total.

## Third builder check (workspace model)

Seven questions, all settled by Claude as calls in the design doc: a task's links bind at first session start to the issuing window's workspace; views belong to the window's container and carry over on conversion; Read-only overrides protect a project as a target, enforced by a file-tool denylist plus a best-effort Bash guard hook; one session attaches to one pane at a time ("Bring here"); prompts assemble in layers (base, workspace fragment, task additions) and rosters union with name shadowing; the database is the workspace's source of truth with the JSON file as a mirror (write-through in app data, explicit save elsewhere); linking is Claude Code only in v1. Document handed to Johnny for approval.

## Rename and engineering standards

- **The application is named "Team Code"** (Johnny, 2026-10-24), formerly ContextSwitch. Identifiers follow: `app.teamcode.desktop`, `teamcode.db`, `<name>.teamcode-workspace.json`, crate `teamcode-core`, branch prefix `tc/`, sentinel `TEAMCODE: <stage> done`. The repo folder is still `ContextSwitch` until Johnny renames it.
- **Engineering standards.** (Johnny) The codebase must be well structured and maintainable, with a good `CLAUDE.md` and rules for consistency, and lightweight reusable components instead of raw HTML controls. Written into the design as section 15: feature folders, generated IPC bindings via tauri-specta, size budgets, `CLAUDE.md` plus path-scoped `.claude/rules/`, ADRs, a primitives library in `src/components/ui/` enforced by ESLint, example pages for primitives, and quality gates with lefthook and CI. The specifics are Claude's calls.

## Approval

- **Design approved by Johnny on 2026-10-24.** Next: implementation plans, one per subsystem, written by parallel agents from a shared contracts document in `docs/superpowers/plans/`. Folder rename to `TeamCode` requested; done after the plans are written.

## Implementation plans

- **Ten plans written by parallel agents** (2026-10-24/25) in `docs/superpowers/plans/`, from a shared contracts document (`…-00-overview-and-contracts.md`). Totals: 177 tasks across foundation (15), primitives (17), core pipeline (17), sessions and terminals (16), observation (19), shell (19), board (18), task page (19), canvas (17), workspaces and integrations (20).
- **Reconciliation.** The plans' proposed contract additions were reconciled into binding amendments A1–A20 in the overview, and every plan was updated to match. Key calls: internally tagged enums with only unit or struct variants; snake_case fields and Rust variant names, no `rename_all`; canonical hand-back chip texts with a Rust-written fixture the TypeScript mirror tests against; a buffered one-write hook command; migrations numbered by owning plan; build order 01, 02, 03, 05, 04, then 06 ∥ 07, 08 ∥ 09, 10.

## Round 7: tab strip and Settings layout

- **Tab strip placement: above the main content, beside the sidebar.** (Johnny, 2026-10-25) Like VS Code, the strip moves off the top of the left sidebar and sits above the main column (Board and the views), not above the whole window. It stays visible when a task page or the Team dock is open with no terminal showing; Board is the default tab. Supersedes the Round 1 "Warp-style tabs" placement; the Board-is-tab-zero and ⌘B calls stand.
- **Board tab is fixed.** (Johnny) It is always tab zero and cannot be closed, reordered or renamed.
- **Board icon: tab strip only.** (Johnny) A board icon (lucide `SquareKanban`) at the left of the Board tab, the size and style of the sidebar icon strip; none in the sidebar strip or the board header.
- **Settings in the sidebar must not overflow.** (Johnny) Claude's call: setting rows lay out by container width, stacking label over control when narrow and going two-column when the container is wide (`Field` row layout with `@container`), segmented controls wrap, and the sidebar Settings section gets padding and `min-w-0`. The Settings dialog marks its panels as containers so it keeps two columns.
