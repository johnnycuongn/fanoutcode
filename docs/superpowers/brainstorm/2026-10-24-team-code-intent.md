# Team Code (formerly ContextSwitch) — initial intent

Johnny's original brief, pasted into Claude Code on 2026-10-24. Kept verbatim as the "user initial intent spec" for this project. The definition that grows out of it lives in `docs/superpowers/specs/`.

---

Manage between stages like Kanban board or brainstorm the UI with me for best multi tasks:

1. Brainstorming spec with LiveSpec within the page, pull the code and copy the code over, or improve or modify upon it (https://github.com/johnnycuongn/live-spec)
   1.1. Spec Completed -> Prepared for implementation plan
2. Writing implementation plan for spec
   2.1. Implementation Plan Written, with a small team execution plan to plan a team of subagents, could be frontend and backend and reviewer (code reviewer (best practice reviewer and codebase consistency reviewer) and security reviewer) to execute implementation plan later
3. Executing implementation plan = writing code. This stage will fan out many subagents to write using the team execution plan written above. Use worktrees or not depend on the tasks, also configurable locally by users. Don't commit unless user configure, false by default
   3.1. Execution done, waiting for developer feedback and reviewer, 1 agent to listen to code changes in the git changes to update the spec
4. Run unit tests, or Playwright, or project tests (depend on each project). This is optional since test can be tested manually by users
5. Writing commit message or messages if multiple commit needed for ease to read
6. User committed, task is staged as done. Saved as session and can be 'archived'

## User Interface

- Each project have 1 kanban board, and the kanban board should be interactive, drag and drop, click a task card will open up the terminal running, contain enough information, goal is help developers easy to switch context between multiple projects and multiple tasks within 1 project.
- Each project can have multiple views, each views can contain 1 or more terminal session (or task pages)
- Each project will link to Atlassian jira and confluence (optional), or have multiple markdown contexts or doc or pdf or images
- Each tasks will have their own session in the folder, or storage of this app, or users can specify the location of this project's tasks memory, can be within the project's folder as well, or in current projects /docs for example. Design a folder selection so user can select
- Each task own a terminal session, and a folder memory, clicking into a task, will open a page of terminal. Users can have multiple terminals, or task pages, in 1 window or view, similar to Warp.
- Each task has their own memory, which store in a folder, the folder will contain markdown files of spec, implementation plan, session index.md and user initial intent spec.
- User can rename the session/task, or use Claude session name by default, 1 run a session, claude or codex should name the session by default, make the name easy to read and brief according to the project.
- User can archive the task at any stage
- User can hide or flag or change the color of the tasks, or change the priority of the task
- Each task can own a Jira ticket, connect to Atlassian, and use project references if needed

## User experience

- Visually appealing. Lightweight, fast and animatedly good. Interactive.
- Allow for different themes, even dark modes.
- Drag and drop if needed. Fast switching, using keyboards, like switching projects like Command 1, 2, 3. Switching views is Command Shift [, ].

Consider these (light, reliable on all webviews):

- Motion. It's the best fit for desktop app UI: panel transitions, layout animations, list reordering, and modals.
- shadcn/ui + Radix. These are ideal, since desktop apps need solid keyboard navigation and focus handling.
- Sonner and Vaul. Toasts and drawers that feel native.
- Lottie for icons and empty-state illustrations.
- Rive (WASM works fine in Tauri) for interactive states.

Brainstorm the definition first, then popup some visual brainstorm to design the UI. This is a desktop app built with Tauri.
