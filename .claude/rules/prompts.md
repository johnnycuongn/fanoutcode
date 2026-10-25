---
paths: ["crates/fanoutcode-core/src/prompts/**", "crates/fanoutcode-core/prompts/**"]
---

# Stage prompt rules

The app parses what agents write, so every stage prompt carries these conventions verbatim:

- Plan steps in `plan.md` are checkbox lines: `- [ ] N. Title (after 2, 7)`.
- Each subagent dispatch description starts with `Step N · role:` (or `Step N, M · role:`).
- The last line of a stage's final message is the sentinel `FANOUTCODE: <stage> done` (e.g. `FANOUTCODE: plan done`).
- The linked-projects policy is stated: which linked roots may be read, and that changes to them happen only when the
  task asks.
- Placeholders are exactly these eight; any other `{{…}}` is a bug: `{{task.folder}}`, `{{project.root}}`,
  `{{project.context}}`, `{{linked.projects}}`, `{{worktree}}`, `{{test.command}}`, and in the fix-tests prompt
  `{{test.exit_code}}` and `{{test.output}}` (`prompts::render`).
- Prompts never tell the agent to commit, push or delete branches; the Commit stage drafts a message. The one prompt
  that says merge or rebase is the integrate prompt of Plan 11 (`overlap::prompts::integrate`), sent only from a Done
  card by the developer.
