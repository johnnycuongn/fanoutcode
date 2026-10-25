//! Prompt fragments a workspace adds: the Linked projects block, the Project and Workspace references, and the
//! stage instructions from the workspace file.

use std::path::{Path, PathBuf};

use super::context_docs::{is_visible, text_cache_path, ContextDoc, ContextDocKind};
use crate::domain::{Container, LinkPolicy};

pub const LINK_POLICY_TEXT: &str = "Read linked projects freely for references. Modify a linked project only when the task's intent or plan calls for it, and list any such edit in plan.md under that project's heading.";
pub const READ_ONLY_POLICY_TEXT: &str = "This workspace's linking policy is Read only: read linked projects freely for references, but do not modify them. File tools are blocked there and shell commands are checked.";

/// One other project in the task's bound set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedProject {
    pub name: String,
    pub root: PathBuf,
    pub brief: Option<String>,
    pub protected: bool,
}

/// Everything the workspace contributes to a task's prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkContext {
    /// The workspace, as the container its references belong to.
    pub workspace: Container,
    /// Its members' roots: where its file and folder references may point.
    pub workspace_roots: Vec<PathBuf>,
    pub policy: LinkPolicy,
    pub linked: Vec<LinkedProject>,
    pub workspace_docs: Vec<ContextDoc>,
    /// The stage's `prompt_fragments` text from the workspace file.
    pub workspace_instructions: Option<String>,
}

/// `project_context` fills `{{project.context}}`, `linked_projects` fills `{{linked.projects}}`, and
/// `workspace_fragment` is appended after the base prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptFragments {
    pub project_context: String,
    pub linked_projects: String,
    pub workspace_fragment: Option<String>,
}

pub fn linked_projects_fragment(ctx: &LinkContext) -> String {
    if ctx.linked.is_empty() {
        return String::new();
    }
    let mut out = String::from("## Linked projects\n\n");
    for p in &ctx.linked {
        out.push_str(&format!("- **{}** — `{}`", p.name, p.root.display()));
        if p.protected {
            out.push_str(" — protected target: read only, do not modify.");
        }
        out.push('\n');
        match p.brief.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
            Some(brief) => {
                for line in brief.lines() {
                    out.push_str(&format!("  {line}\n"));
                }
            }
            None => out.push_str("  (brief pending)\n"),
        }
    }
    out.push('\n');
    out.push_str(match ctx.policy {
        LinkPolicy::ReadOnly => READ_ONLY_POLICY_TEXT,
        LinkPolicy::ReadAndModifyOnRequest => LINK_POLICY_TEXT,
    });
    out
}

/// One reference of `container` as a prompt line, without its list marker. A cached text is named only when its
/// file is there (a cache from before per-container folders reads as not fetched until fetched again).
pub fn reference_line(doc: &ContextDoc, container: &Container, app_data: &Path) -> String {
    let cached = || match (doc.fetched_at, text_cache_path(app_data, container, &doc.id)) {
        (Some(_), Ok(p)) if p.is_file() => format!(" — text: `{}`", p.display()),
        _ => " — not fetched yet".to_owned(),
    };
    match doc.kind {
        ContextDocKind::File => format!("file: `{}`", doc.target),
        ContextDocKind::Folder => format!("folder: `{}/`", doc.target.trim_end_matches('/')),
        ContextDocKind::Url => format!("url: {}{}", doc.target, cached()),
        ContextDocKind::Confluence => format!("confluence: \"{}\" {}{}", doc.label, doc.target, cached()),
    }
}

/// The lines for `docs`: only the visible ones (`is_visible`), named; the rest counted in one line with no paths.
pub fn reference_lines(container: &Container, docs: &[ContextDoc], roots: &[PathBuf], app_data: &Path) -> Vec<String> {
    let mut lines: Vec<String> = docs
        .iter()
        .filter(|d| is_visible(d, roots))
        .map(|d| reference_line(d, container, app_data))
        .collect();
    let waiting = docs.iter().filter(|d| !d.trusted).count();
    if waiting > 0 {
        lines.push(waiting_line(waiting));
    }
    lines
}

/// `N context docs are waiting for confirmation in Team Code`.
pub fn waiting_line(n: usize) -> String {
    format!(
        "{n} context doc{} waiting for confirmation in Team Code",
        if n == 1 { " is" } else { "s are" }
    )
}

pub fn references_fragment(
    heading: &str,
    container: &Container,
    docs: &[ContextDoc],
    roots: &[PathBuf],
    app_data: &Path,
) -> String {
    let lines = reference_lines(container, docs, roots, app_data);
    if lines.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = lines.into_iter().map(|l| format!("- {l}")).collect();
    format!("## {heading}\n\n{}", lines.join("\n"))
}

pub fn prompt_fragments(
    project: &Container,
    project_roots: &[PathBuf],
    project_docs: &[ContextDoc],
    link: Option<&LinkContext>,
    app_data: &Path,
) -> PromptFragments {
    let project_context = references_fragment("Project references", project, project_docs, project_roots, app_data);
    let Some(link) = link else {
        return PromptFragments {
            project_context,
            linked_projects: String::new(),
            workspace_fragment: None,
        };
    };
    let linked_projects = linked_projects_fragment(link);
    let parts: Vec<String> = [
        linked_projects.clone(),
        references_fragment(
            "Workspace references",
            &link.workspace,
            &link.workspace_docs,
            &link.workspace_roots,
            app_data,
        ),
        link.workspace_instructions
            .clone()
            .unwrap_or_default()
            .trim()
            .to_owned(),
    ]
    .into_iter()
    .filter(|p| !p.is_empty())
    .collect();
    PromptFragments {
        project_context,
        linked_projects,
        workspace_fragment: (!parts.is_empty()).then(|| parts.join("\n\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::context_docs::{ContextDoc, ContextDocKind};
    use chrono::Utc;

    fn doc(id: &str, kind: ContextDocKind, target: &str, fetched: Option<chrono::DateTime<Utc>>) -> ContextDoc {
        ContextDoc {
            id: id.into(),
            kind,
            target: target.into(),
            label: target.rsplit('/').next().unwrap().into(),
            page_id: None,
            fetched_at: fetched,
            order: 0,
            trusted: false,
        }
    }
    fn one_link() -> LinkContext {
        LinkContext {
            workspace: Container::Workspace {
                id: crate::ids::WorkspaceId::parse("ws-1").unwrap(),
            },
            workspace_roots: vec![],
            policy: LinkPolicy::ReadAndModifyOnRequest,
            workspace_docs: vec![],
            workspace_instructions: None,
            linked: vec![LinkedProject {
                name: "api".into(),
                root: "/w/api".into(),
                brief: None,
                protected: false,
            }],
        }
    }

    #[test]
    fn linked_fragment_lists_each_project_with_root_and_brief() {
        let ctx = LinkContext {
            workspace: Container::Workspace {
                id: crate::ids::WorkspaceId::parse("ws-1").unwrap(),
            },
            workspace_roots: vec![],
            policy: LinkPolicy::ReadAndModifyOnRequest,
            workspace_docs: vec![],
            workspace_instructions: None,
            linked: vec![
                LinkedProject {
                    name: "api".into(),
                    root: "/w/api".into(),
                    brief: Some("Rust service.".into()),
                    protected: false,
                },
                LinkedProject {
                    name: "docs".into(),
                    root: "/w/docs".into(),
                    brief: None,
                    protected: true,
                },
            ],
        };
        assert_eq!(
            linked_projects_fragment(&ctx),
            format!("## Linked projects\n\n- **api** — `/w/api`\n  Rust service.\n- **docs** — `/w/docs` — protected target: read only, do not modify.\n  (brief pending)\n\n{LINK_POLICY_TEXT}")
        );
    }

    #[test]
    fn linked_fragment_uses_read_only_text_and_is_empty_without_links() {
        let mut ctx = one_link();
        ctx.policy = LinkPolicy::ReadOnly;
        assert!(linked_projects_fragment(&ctx).ends_with(READ_ONLY_POLICY_TEXT));
        ctx.linked.clear();
        assert_eq!(linked_projects_fragment(&ctx), "");
    }

    #[test]
    fn references_fragment_lists_kinds_and_cache_paths() {
        let root = tempfile::tempdir().unwrap();
        let web = std::fs::canonicalize(root.path()).unwrap();
        std::fs::create_dir_all(web.join("docs")).unwrap();
        std::fs::create_dir_all(web.join("adr")).unwrap();
        std::fs::write(web.join("docs/arch.md"), "a").unwrap();
        let arch = web.join("docs/arch.md").to_string_lossy().into_owned();
        let adr = web.join("adr").to_string_lossy().into_owned();
        let t = |mut d: ContextDoc| {
            d.trusted = true;
            d
        };
        let docs = vec![
            t(doc("doc-1", ContextDocKind::File, &arch, None)),
            t(doc(
                "doc-2",
                ContextDocKind::Confluence,
                "https://acme.atlassian.net/wiki/spaces/ENG/pages/123/Pay",
                Some(Utc::now()),
            )),
            t(doc("doc-3", ContextDocKind::Url, "https://x.y/spec", None)),
            t(doc("doc-4", ContextDocKind::Folder, &adr, None)),
        ];
        let app = tempfile::tempdir().unwrap();
        let project = Container::Project {
            id: crate::ids::ProjectId::parse("p-1").unwrap(),
        };
        let cache = text_cache_path(app.path(), &project, "doc-2").unwrap();
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(&cache, "Pay text").unwrap();
        let s = references_fragment(
            "Project references",
            &project,
            &docs,
            std::slice::from_ref(&web),
            app.path(),
        );
        assert!(s.starts_with(&format!("## Project references\n\n- file: `{arch}`\n")));
        assert!(s.contains(&format!(
            "- confluence: \"Pay\" https://acme.atlassian.net/wiki/spaces/ENG/pages/123/Pay — text: `{}`",
            cache.display()
        )));
        assert!(s.contains("- url: https://x.y/spec — not fetched yet"));
        assert!(s.contains(&format!("- folder: `{adr}/`")));
        assert_eq!(
            references_fragment("Workspace references", &project, &[], &[], Path::new("/app")),
            ""
        );
    }

    #[test]
    fn untrusted_references_are_counted_never_named() {
        let root = tempfile::tempdir().unwrap();
        let secret = root.path().join("id_rsa");
        std::fs::write(&secret, "k").unwrap();
        let project = Container::Project {
            id: crate::ids::ProjectId::parse("p-1").unwrap(),
        };
        let docs = vec![
            doc("doc-1", ContextDocKind::File, secret.to_str().unwrap(), None),
            doc("doc-2", ContextDocKind::Url, "https://x.y/spec", None),
        ];
        let s = references_fragment(
            "Workspace references",
            &project,
            &docs,
            &[root.path().to_path_buf()],
            Path::new("/app"),
        );
        assert_eq!(
            s,
            "## Workspace references\n\n- 2 context docs are waiting for confirmation in Team Code"
        );
        assert!(!s.contains("id_rsa") && !s.contains("x.y"));
        assert_eq!(
            waiting_line(1),
            "1 context doc is waiting for confirmation in Team Code"
        );
    }

    #[test]
    fn prompt_fragments_compose_in_order() {
        let mut ctx = one_link();
        ctx.workspace_instructions = Some("Run both test suites.".into());
        let f = prompt_fragments(
            &Container::Project {
                id: crate::ids::ProjectId::parse("p-1").unwrap(),
            },
            &[],
            &[],
            Some(&ctx),
            Path::new("/app"),
        );
        let w = f.workspace_fragment.unwrap();
        assert!(
            w.starts_with("## Linked projects") && w.ends_with("Run both test suites.") && f.project_context.is_empty()
        );
        assert!(prompt_fragments(
            &Container::Project {
                id: crate::ids::ProjectId::parse("p-1").unwrap()
            },
            &[],
            &[],
            None,
            Path::new("/app")
        )
        .workspace_fragment
        .is_none());
    }
}
