//! The linking policy as enforcement: which bound roots are protected, and the `Edit` deny rules that block every
//! built-in file tool there (Claude Code's `Edit(path)` rules govern Write, MultiEdit and NotebookEdit too).
//! Protection is decided by project id against the workspace's members, never by matching path text, and fails
//! closed: a root whose project is unknown or not a member is protected.

use std::path::PathBuf;

use super::link::{canonical_root, BoundLink};
use crate::domain::{LinkPolicy, ProjectId, Workspace};
use crate::{Error, Result};

pub const READ_ONLY_LABEL: &str = "Read only: file tools blocked, shell commands checked";
pub const READ_AND_MODIFY_LABEL: &str = "Read and modify on request";
pub const POLICY_APPLIES_NOTE: &str = "Applies to sessions started from now on.";

/// The bound roots that are protected: a link for `home` never is; a link whose project is unrecorded, not a
/// member, or listed more than once always is; otherwise when the workspace is Read only or the member is a
/// protected target. Sorted, deduplicated.
pub fn protected_roots(home: &ProjectId, workspace: &Workspace, bound: &[BoundLink]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = bound
        .iter()
        .filter(|l| {
            let Some(pid) = &l.project_id else { return true };
            if pid == home {
                return false;
            }
            let members: Vec<_> = workspace.projects.iter().filter(|m| &m.project_id == pid).collect();
            match members.as_slice() {
                [m] => workspace.policy == LinkPolicy::ReadOnly || m.protected,
                _ => true,
            }
        })
        .map(|l| l.root.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Characters that mean something inside a permission rule's path pattern.
const SPECIAL: &[char] = &['*', '?', '[', ']', '{', '}', '(', ')', '\\', '\n', '\r'];

/// One `Edit(//<absolute path without its leading slash>/**)` per root, for `permissions.deny`: for the canonical
/// form, and for the root as given when that differs. A root whose name could change the rule's meaning (glob or
/// rule syntax, a newline, not UTF-8) refuses the session instead of producing a rule that might not match.
pub fn deny_rules(protected: &[PathBuf]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for root in protected {
        let canonical = canonical_root(root);
        for form in [canonical_root_text(root)?, canonical_root_text(&canonical)?] {
            let rule = format!("Edit(//{}/**)", form.trim_start_matches('/'));
            if !out.contains(&rule) {
                out.push(rule);
            }
        }
    }
    Ok(out)
}

/// `permissions.deny` rules for single files (`Edit(//<path>)`), in both their written and canonical forms (the
/// canonical form of the parent folder, since the file may not exist yet).
pub fn deny_file_rules(files: &[PathBuf]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for file in files {
        let canonical = match (file.parent(), file.file_name()) {
            (Some(dir), Some(name)) => canonical_root(dir).join(name),
            _ => file.clone(),
        };
        for form in [canonical_root_text(file)?, canonical_root_text(&canonical)?] {
            let rule = format!("Edit(//{})", form.trim_start_matches('/'));
            if !out.contains(&rule) {
                out.push(rule);
            }
        }
    }
    Ok(out)
}

fn canonical_root_text(p: &std::path::Path) -> Result<String> {
    let refuse = || {
        Error::Precondition(format!(
            "The folder {} can't be protected safely because its name contains special characters. Rename it or \
             remove it from the workspace.",
            p.display()
        ))
    };
    let s = p.to_str().ok_or_else(refuse)?;
    if s.contains(SPECIAL) {
        return Err(refuse());
    }
    let t = s.trim_end_matches('/');
    Ok(if t.is_empty() { "/".to_owned() } else { t.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{WorkspaceId, WorkspaceProject};
    use std::path::Path;

    fn pid(name: &str) -> ProjectId {
        ProjectId::parse(match name {
            "api" => "p-a1",
            "web" => "p-b2",
            "docs" => "p-d3",
            _ => "p-e4",
        })
        .unwrap()
    }
    fn root(name: &str) -> PathBuf {
        PathBuf::from(format!("/w/{name}"))
    }
    fn link(name: &str) -> BoundLink {
        BoundLink {
            project_id: Some(pid(name)),
            root: root(name),
        }
    }
    fn ws_with(names: &[&str]) -> Workspace {
        Workspace {
            id: WorkspaceId::parse("ws-1").unwrap(),
            name: "Shop".into(),
            file_path: None,
            dirty: false,
            projects: names
                .iter()
                .enumerate()
                .map(|(i, n)| WorkspaceProject {
                    project_id: pid(n),
                    order: i as u32,
                    protected: false,
                })
                .collect(),
            policy: LinkPolicy::ReadAndModifyOnRequest,
        }
    }
    fn set_protected(ws: &mut Workspace, name: &str) {
        ws.projects
            .iter_mut()
            .find(|m| m.project_id == pid(name))
            .unwrap()
            .protected = true;
    }

    #[test]
    fn read_only_protects_every_bound_root_but_home() {
        let mut ws = ws_with(&["api", "web", "docs"]);
        ws.policy = LinkPolicy::ReadOnly;
        assert_eq!(
            protected_roots(&pid("web"), &ws, &[link("api"), link("docs")]),
            vec![root("api"), root("docs")]
        );
    }

    #[test]
    fn protected_flag_protects_one_root_under_default_policy() {
        let mut ws = ws_with(&["api", "web", "docs"]);
        set_protected(&mut ws, "docs");
        assert_eq!(
            protected_roots(&pid("web"), &ws, &[link("api"), link("docs")]),
            vec![root("docs")]
        );
    }

    #[test]
    fn home_project_is_never_protected_even_when_flagged() {
        let mut ws = ws_with(&["api", "web"]);
        ws.policy = LinkPolicy::ReadOnly;
        set_protected(&mut ws, "web");
        assert!(protected_roots(&pid("web"), &ws, &[link("api"), link("web")])
            .iter()
            .all(|r| r != &root("web")));
    }

    #[test]
    fn only_bound_roots_count() {
        let mut ws = ws_with(&["api", "web", "docs"]);
        ws.policy = LinkPolicy::ReadOnly;
        assert_eq!(protected_roots(&pid("web"), &ws, &[link("api")]), vec![root("api")]);
    }

    #[test]
    fn unmatched_or_departed_roots_fail_closed() {
        let ws = ws_with(&["api", "web"]); // default policy, nothing flagged
        let unrecorded = BoundLink {
            project_id: None,
            root: root("api"),
        };
        let departed = link("infra"); // not a member any more
        assert_eq!(
            protected_roots(&pid("web"), &ws, &[unrecorded, departed]),
            vec![root("api"), root("infra")]
        );
        // trailing slash, symlink and case variants no longer matter: matching is by project id
        let variant = BoundLink {
            project_id: Some(pid("api")),
            root: PathBuf::from("/W/API/"),
        };
        assert!(protected_roots(&pid("web"), &ws, &[variant]).is_empty());
    }

    #[test]
    fn deny_rules_use_double_slash_absolute_edit_rules() {
        assert_eq!(
            deny_rules(&[PathBuf::from("/Users/j/w/api"), PathBuf::from("/Users/j/my api")]).unwrap(),
            vec![
                "Edit(//Users/j/w/api/**)".to_string(),
                "Edit(//Users/j/my api/**)".to_string()
            ]
        );
        assert!(deny_rules(&[]).unwrap().is_empty());
    }

    #[test]
    fn deny_rules_cover_the_canonical_form_of_a_symlinked_root() {
        let base = tempfile::tempdir().unwrap();
        let real = std::fs::canonicalize(base.path()).unwrap().join("api");
        std::fs::create_dir(&real).unwrap();
        let alias = base.path().join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        let rules = deny_rules(std::slice::from_ref(&alias)).unwrap();
        assert!(rules.contains(&format!("Edit(/{}/**)", alias.display())));
        assert!(rules.contains(&format!("Edit(/{}/**)", real.display())));
    }

    #[test]
    fn special_characters_refuse_the_session() {
        for bad in ["/w/api (old)", "/w/a*"] {
            assert!(
                matches!(deny_rules(&[PathBuf::from(bad)]), Err(Error::Precondition(m)) if m.contains("can't be protected safely")),
                "{bad}"
            );
        }
        use std::os::unix::ffi::OsStrExt;
        let not_utf8 = Path::new(std::ffi::OsStr::from_bytes(b"/w/\xff\xfe")).to_path_buf();
        assert!(matches!(deny_rules(&[not_utf8]), Err(Error::Precondition(_))));
    }
}
