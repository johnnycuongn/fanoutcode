---
paths: ["crates/teamcode-core/src/prompts/**", "crates/teamcode-core/prompts/**"]
---

# Stage prompt rules

The app parses what agents write, so every stage prompt carries these conventions verbatim:

- Plan steps in `plan.md` are checkbox lines: `- [ ] N. Title (after 2, 7)`.
- Each subagent dispatch description starts with `Step N · role:` (or `Step N, M · role:`).
- The last line of a stage's final message is the sentinel `TEAMCODE: <stage> done` (e.g. `TEAMCODE: plan done`).
- The linked-projects policy is stated: which linked roots may be read, and that changes to them happen only when the
  task asks.
- Placeholders are exactly these six; any other `{{…}}` is a bug: `{{task.folder}}`, `{{project.root}}`,
  `{{project.context}}`, `{{linked.projects}}`, `{{worktree}}`, `{{test.command}}`.
- Prompts never tell the agent to commit, merge, rebase, push or delete branches; the Commit stage drafts a message.
