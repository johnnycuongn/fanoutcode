//! A task's `intent.md` from a Jira issue.

use super::client::Issue;

/// `# KEY: summary`, a status line, the link, then the description.
pub fn intent_from_issue(issue: &Issue) -> String {
    let description = issue.description.trim();
    format!(
        "# {}: {}\n\nStatus: {} · Type: {} · Assignee: {}\nLink: {}\n\n{}\n",
        issue.key,
        issue.summary,
        issue.status,
        issue.issue_type,
        issue.assignee.as_deref().unwrap_or("Unassigned"),
        issue.url,
        if description.is_empty() {
            "(no description)"
        } else {
            description
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::test_support::sample_issue;

    #[test]
    fn intent_has_header_meta_and_description() {
        let i = sample_issue("PROJ-7", Some("Jane"), "Users sign in.");
        assert_eq!(
            intent_from_issue(&i),
            "# PROJ-7: Add login\n\nStatus: In Progress · Type: Story · Assignee: Jane\nLink: https://acme.atlassian.net/browse/PROJ-7\n\nUsers sign in.\n"
        );
        let none = intent_from_issue(&sample_issue("PROJ-8", None, ""));
        assert!(none.contains("Assignee: Unassigned\n") && none.ends_with("(no description)\n"));
    }
}
