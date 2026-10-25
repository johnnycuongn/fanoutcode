//! The pure pieces of "Bring <base> in" (§6.3): reading the session's `Stop` after the integrate prompt, and the
//! other task's intent excerpted from its memory folder for each overlapping path.

// TODO(merge): use prompts::INTEGRATE_SENTINEL
const INTEGRATE_SENTINEL: &str = "FANOUTCODE: integrate done";

/// What an integrating session's `Stop` means (§6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationResult {
    /// The sentinel on a clean tree: back to Idle, recorded in the timeline.
    Done,
    /// Conflict markers or a merge or rebase in progress: `MergeNeedsYou`, whatever the session said.
    Conflicted,
    /// Neither: the session is still at it.
    StillWorking,
}

/// `Conflicted` when the tree has conflicts, regardless of the sentinel; `Done` when the last non-empty line of
/// `message` is the sentinel; otherwise `StillWorking`.
pub fn integration_result(message: &str, has_conflicts: bool) -> IntegrationResult {
    if has_conflicts {
        return IntegrationResult::Conflicted;
    }
    let last = message.lines().map(str::trim).rfind(|l| !l.is_empty());
    if last == Some(INTEGRATE_SENTINEL) {
        IntegrationResult::Done
    } else {
        IntegrationResult::StillWorking
    }
}

/// The other task's intent for `path`: the first `## ` section of its `spec.md`, and from its `plan.md` the
/// `## Files` lines naming `path` followed by the checkbox step lines mentioning it.
pub fn excerpts(path: &str, spec_md: &str, plan_md: &str) -> (String, String) {
    (first_section(spec_md), plan_lines(path, plan_md))
}

/// A `# ` or `## ` heading, which ends a `## ` section (`### ` stays inside it).
fn is_section_heading(line: &str) -> bool {
    line.starts_with("# ") || line.starts_with("## ")
}

fn first_section(md: &str) -> String {
    let mut lines = md.lines().skip_while(|l| !l.starts_with("## "));
    let Some(heading) = lines.next() else {
        return String::new();
    };
    let body: Vec<&str> = lines.take_while(|l| !is_section_heading(l)).collect();
    format!("{heading}\n{}", body.join("\n")).trim().to_owned()
}

fn is_step(line: &str) -> bool {
    let t = line.trim_start();
    ["- [ ]", "- [x]", "- [X]", "* [ ]", "* [x]", "* [X]"]
        .iter()
        .any(|m| t.starts_with(m))
}

fn path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

/// `line` names `path` as a whole path: `src/a.ts` is not in `src/a.tsx` or `lib/src/a.ts`, but is in
/// `` `src/a.ts`. `` (a trailing full stop ends the sentence, not the path).
fn mentions(line: &str, path: &str) -> bool {
    !path.is_empty()
        && line.match_indices(path).any(|(i, _)| {
            let before = line[..i].chars().next_back();
            let mut rest = line[i + path.len()..].chars();
            let continues = match rest.next() {
                None => false,
                Some('.') => rest.next().is_some_and(char::is_alphanumeric),
                Some(c) => path_char(c),
            };
            !before.is_some_and(|c| path_char(c) || c == '.') && !continues
        })
}

fn plan_lines(path: &str, md: &str) -> String {
    let (mut files, mut steps) = (Vec::new(), Vec::new());
    let mut in_files = false;
    for line in md.lines() {
        if is_section_heading(line) {
            in_files = line.trim() == "## Files";
            continue;
        }
        if !mentions(line, path) {
            continue;
        }
        if in_files {
            files.push(line.trim_end());
        } else if is_step(line) {
            steps.push(line.trim_end());
        }
    }
    files.extend(steps);
    files.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_result_rules() {
        let done = "Merged main into fo/login.\nTests pass.\n\nFANOUTCODE: integrate done\n\n";
        assert_eq!(integration_result(done, false), IntegrationResult::Done);
        assert_eq!(integration_result(done, true), IntegrationResult::Conflicted);
        assert_eq!(
            integration_result("I could not keep both in a.ts.", true),
            IntegrationResult::Conflicted
        );
        assert_eq!(
            integration_result("Still merging, running the tests now.", false),
            IntegrationResult::StillWorking
        );
        // the sentinel must be the last line, not merely quoted earlier
        assert_eq!(
            integration_result(
                "I will say FANOUTCODE: integrate done when finished.\nRunning tests.",
                false
            ),
            IntegrationResult::StillWorking
        );
        assert_eq!(integration_result("", false), IntegrationResult::StillWorking);
    }

    #[test]
    fn excerpts_take_spec_section_and_plan_lines() {
        let spec = "# Login\n\nIntro line.\n\n## Summary\n\nUsers sign in with a magic link.\nSessions last a day.\n\n## Details\n\nMore.\n";
        let plan = "# Plan\n\n## Files\n\n- src/auth/session.ts — session expiry\n- src/auth/link.ts\n\n## Steps\n\n- [ ] Add expiry to src/auth/session.ts\n- [x] Send the link from src/auth/link.ts\n- [ ] Test src/auth/session.ts expiry\nNot a step: src/auth/session.ts\n";
        let (s, p) = excerpts("src/auth/session.ts", spec, plan);
        assert_eq!(
            s,
            "## Summary\n\nUsers sign in with a magic link.\nSessions last a day."
        );
        assert_eq!(
            p,
            "- src/auth/session.ts — session expiry\n- [ ] Add expiry to src/auth/session.ts\n- [ ] Test src/auth/session.ts expiry"
        );
        assert_eq!(excerpts("src/x.ts", "no sections", ""), (String::new(), String::new()));
        let near = "## Files\n\n- src/auth/session.tsx\n- lib/src/auth/session.ts\n- `src/auth/session.ts`.\n";
        assert_eq!(excerpts("src/auth/session.ts", "", near).1, "- `src/auth/session.ts`.");
    }
}
