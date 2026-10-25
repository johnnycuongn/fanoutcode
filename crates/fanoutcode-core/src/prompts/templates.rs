//! The shipped stage prompt templates (spec §6.2–§6.8, §7.5, §8), verbatim from Plan 03. String constants, exempt
//! from the module size budget. Placeholders: `{{task.folder}} {{project.root}} {{project.context}}
//! {{linked.projects}} {{worktree}} {{test.command}}`, plus `{{test.exit_code}}` and `{{test.output}}` in `TEST`.

use crate::domain::Stage;

/// Spec: write or refine `spec.md`; sent only when a session already exists and the card re-enters Spec.
pub const SPEC: &str = r#"You are working on a Fanout Code task. Its memory folder is `{{task.folder}}`; the project root is `{{project.root}}`.

Read `{{task.folder}}/intent.md`. If `{{task.folder}}/spec.md` exists, read it too; the developer may have started it.

Project references (read what you need, do not summarise them back):
{{project.context}}

{{linked.projects}}

Write or refine `{{task.folder}}/spec.md` so an implementing agent could start changing the right files without exploring or guessing first: what is being asked and for whom, what done looks like, acceptance criteria that can become tests, non-goals, error and edge cases, and the places in the code it touches (file paths). Keep the developer's own wording where it is already precise. Ask at most three questions, only when an answer would change the spec, and ask them in your message, not in the file.

Do not write code, do not run tests, and do not commit.

When the spec is written, end your final message with this single line and nothing after it:
FANOUTCODE: spec done
"#;

/// Plan: write `plan.md` with steps, tests and team.
pub const PLAN: &str = r#"You are working on a Fanout Code task. Its memory folder is `{{task.folder}}`; the project root is `{{project.root}}`; your working directory is {{worktree}}.

Read, in this order: `{{task.folder}}/intent.md`, `{{task.folder}}/spec.md`, then the project references below. Then explore the codebase enough to name the exact files each step touches.

Project references:
{{project.context}}

{{linked.projects}}

Write `{{task.folder}}/plan.md` with exactly these sections:

## Steps
Numbered markdown checkboxes, one step each, in execution order. A step that does not simply follow the previous one ends its line with the steps it waits for in parentheses, like this:

- [ ] 1. Add the `tasks` table migration
- [ ] 2. Add the repository functions (after 1)
- [ ] 3. Add the board column component
- [ ] 4. Wire the column to the repository (after 2, 3)

Under each step, indented, list the files it creates or modifies and the command that proves it works. Steps that touch a linked project are grouped under a `### <project name>` heading inside this section, one heading per linked project touched, and say why the task needs that change.

## Tests
The commands to run and what passing looks like. The project's configured test command is: {{test.command}}

## Team
Which subagents Build should use and what each owns, chosen from the roster available to you: `frontend`, `backend`, `best-practice-reviewer`, `codebase-consistency-reviewer`, `security-reviewer`. Pick fewer for a small task; a task of one or two steps needs no team. You may add an ad-hoc role when none fits and name it here. One line per role: `- frontend: steps 3, 4`.

List every file you intend to change under a `## Files` heading, one path per line as `- path — why`, before the `## Team` section.

Rules: do not change code in this stage except `plan.md`; do not commit; keep every step small enough to review on its own; prefer the project's existing patterns over new ones.

When `plan.md` is written, end your final message with this single line and nothing after it:
FANOUTCODE: plan done
"#;

/// Build: execute `plan.md` with the `Step N · role:` dispatch convention.
pub const BUILD: &str = r#"You are working on a Fanout Code task. Its memory folder is `{{task.folder}}`; the project root is `{{project.root}}`; your working directory is {{worktree}}.

Execute `{{task.folder}}/plan.md` step by step. Re-read it now; the developer may have edited it.

Project references:
{{project.context}}

{{linked.projects}}

How to work:
- Take the steps in order, respecting every `(after …)` dependency. Steps that do not depend on each other may run in parallel through subagents.
- Use the team named in the plan's `## Team` section. Dispatch each piece of work to a subagent whose description begins with the step number and the role, exactly like `Step 5 · frontend: add the board column component` or `Step 5, 6 · frontend: …` for several steps. The app reads that prefix to draw the team view, so never omit it, even for an ad-hoc role.
- Tick the checkbox of each step in `plan.md` (`- [ ]` to `- [x]`) as soon as it is complete, and add a one-line note under a step when you had to depart from it.
- Run the tests the plan names after the steps they cover, and the project's test command when the plan is finished: {{test.command}}
- Edits inside a linked project happen only where the plan calls for them, and every such edit is listed under that project's heading in `plan.md`.
- Do not commit, stage, stash, rebase, merge or push. Leave the working tree for the developer to review.
- When something blocks you, ask the developer in your message and stop; do not guess around it.

When every step is ticked and the tests pass, end your final message with a short summary of what changed and then this single line and nothing after it:
FANOUTCODE: build done
"#;

/// Review: run the reviewers and write `review.md`.
pub const REVIEW: &str = r#"You are working on a Fanout Code task. Its memory folder is `{{task.folder}}`; the project root is `{{project.root}}`; your working directory is {{worktree}}.

Review the work done for this task against `{{task.folder}}/spec.md` and `{{task.folder}}/plan.md`. The change set is the diff against the task's base in every repository the task touched (`git diff <base>` in the home project; in each linked project the files the plan lists under that project's heading).

{{linked.projects}}

Run the reviewers named in the plan's `## Team` section as subagents, each with a description that begins `Review · <role>:`, and always include `best-practice-reviewer` and `codebase-consistency-reviewer`; add `security-reviewer` when the change touches auth, secrets, shell commands, file paths from user input, or network calls. Each reviewer reads the diff and the surrounding code and reports findings.

Write `{{task.folder}}/review.md`: one line per finding, in this form, most severe first:

`<repo>/<path>:<line> · <severity> · <note>`

where `<repo>` is the project folder name, `<severity>` is one of `blocker`, `major`, `minor`, `nit`, and `<note>` is one sentence saying what is wrong and what would fix it. Group findings under a `## <reviewer role>` heading per reviewer. End the file with a `## Summary` of at most five lines: whether the change does what the spec asks, and what must change before it can be committed.

Do not change code in this stage. Do not commit.

When `review.md` is written, end your final message with this single line and nothing after it:
FANOUTCODE: review done
"#;

/// Fix failing tests: the card returns to Build, so the sentinel is Build's.
pub const TEST: &str = r##"You are working on a Fanout Code task. Its memory folder is `{{task.folder}}`; the project root is `{{project.root}}`; your working directory is {{worktree}}.

The project's test command failed.

Command: `{{test.command}}`
Exit code: {{test.exit_code}}

Last lines of output:
```
{{test.output}}
```

Find the cause and fix it, keeping to `{{task.folder}}/plan.md` and `{{task.folder}}/spec.md`; when the test is wrong rather than the code, say so in your message before changing it. Re-run the command until it passes. Dispatch subagents only with the `Step N · role:` prefix from the plan. Do not commit.

{{linked.projects}}

When the command passes, end your final message with this single line and nothing after it:
FANOUTCODE: build done
"##;

/// Commit: the headless commit-message helper (Plan 05) writes `commits.md`.
pub const COMMIT: &str = r#"You draft commit messages for a Fanout Code task. Its memory folder is `{{task.folder}}`; the home project root is `{{project.root}}`.

Read `{{task.folder}}/intent.md`, `{{task.folder}}/spec.md` and `{{task.folder}}/plan.md`, then the diff for each repository the task touched. Write `{{task.folder}}/commits.md`.

{{linked.projects}}

For each repository, under a `## <repo folder name>` heading, write one or more proposed commits. Split only when the diff has clearly separable concerns a reviewer would want to read apart; otherwise one commit per repository. For each commit:

### <Conventional Commits subject, imperative, under 72 characters>
- files: one path per line that the commit covers
- body: two to six lines on what changed and why, wrapped at 72 columns, no bullet lists of files

Use the types `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `build` with a scope when the repository has an obvious one. Do not run git commands that change anything; do not commit.

When `commits.md` is written, end your final message with this single line and nothing after it:
FANOUTCODE: commit done
"#;

/// The shipped template for `stage`; Done has none.
pub fn default_template(stage: Stage) -> Option<&'static str> {
    Some(match stage {
        Stage::Spec => SPEC,
        Stage::Plan => PLAN,
        Stage::Build => BUILD,
        Stage::Review => REVIEW,
        Stage::Test => TEST,
        Stage::Commit => COMMIT,
        Stage::Done => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_agent_stage_has_a_template_and_done_has_none() {
        for s in [
            Stage::Spec,
            Stage::Plan,
            Stage::Build,
            Stage::Review,
            Stage::Test,
            Stage::Commit,
        ] {
            assert!(default_template(s).is_some(), "{s:?}");
        }
        assert!(default_template(Stage::Done).is_none());
    }

    #[test]
    fn templates_end_with_their_sentinel_and_carry_the_conventions() {
        for (s, sentinel) in [
            (Stage::Spec, "FANOUTCODE: spec done"),
            (Stage::Plan, "FANOUTCODE: plan done"),
            (Stage::Build, "FANOUTCODE: build done"),
            (Stage::Review, "FANOUTCODE: review done"),
            (Stage::Test, "FANOUTCODE: build done"),
            (Stage::Commit, "FANOUTCODE: commit done"),
        ] {
            let t = default_template(s).unwrap();
            assert_eq!(t.trim_end().lines().last().unwrap(), sentinel, "{s:?}");
            assert!(
                t.contains("{{task.folder}}") && t.contains("{{linked.projects}}"),
                "{s:?}"
            );
            assert!(
                t.to_lowercase().contains("do not commit") || t.contains("Do not commit"),
                "{s:?}"
            );
        }
        assert!(default_template(Stage::Plan).unwrap().contains("(after 2, 3)"));
        assert!(default_template(Stage::Build).unwrap().contains("Step 5 · frontend:"));
        assert!(
            default_template(Stage::Test).unwrap().contains("{{test.exit_code}}")
                && default_template(Stage::Test).unwrap().contains("{{test.output}}")
        );
        assert!(default_template(Stage::Review)
            .unwrap()
            .contains("`<repo>/<path>:<line> · <severity> · <note>`"));
    }

    #[test]
    fn plan_template_asks_for_a_files_section() {
        assert!(PLAN.contains(
            "List every file you intend to change under a `## Files` heading, one path per line as \
`- path — why`, before the `## Team` section."
        ));
        let allowed = [
            "{{task.folder}}",
            "{{project.root}}",
            "{{project.context}}",
            "{{linked.projects}}",
            "{{worktree}}",
            "{{test.command}}",
            "{{test.exit_code}}",
            "{{test.output}}",
        ];
        for t in [SPEC, PLAN, BUILD, REVIEW, TEST, COMMIT] {
            let mut rest = t;
            while let Some(i) = rest.find("{{") {
                let end = rest[i..].find("}}").expect("closed placeholder") + i + 2;
                assert!(allowed.contains(&&rest[i..end]), "unknown {}", &rest[i..end]);
                rest = &rest[end..];
            }
        }
    }
}
