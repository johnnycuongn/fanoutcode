//! Prompt fragments a workspace adds: the Linked projects block, the Project and Workspace references, and the
//! stage instructions from the workspace file.

use std::path::{Path, PathBuf};

use super::context_docs::{text_cache_path, ContextDoc, ContextDocKind};
use crate::domain::LinkPolicy;

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

/// One reference as a prompt line, without its list marker.
pub fn reference_line(doc: &ContextDoc, app_data: &Path) -> String {
    let cached = || match (doc.fetched_at, text_cache_path(app_data, &doc.id)) {
        (Some(_), Ok(p)) => format!(" — text: `{}`", p.display()),
        _ => " — not fetched yet".to_owned(),
    };
    match doc.kind {
        ContextDocKind::File => format!("file: `{}`", doc.target),
        ContextDocKind::Folder => format!("folder: `{}/`", doc.target.trim_end_matches('/')),
        ContextDocKind::Url => format!("url: {}{}", doc.target, cached()),
        ContextDocKind::Confluence => format!("confluence: \"{}\" {}{}", doc.label, doc.target, cached()),
    }
}

pub fn references_fragment(heading: &str, docs: &[ContextDoc], app_data: &Path) -> String {
    if docs.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = docs
        .iter()
        .map(|d| format!("- {}", reference_line(d, app_data)))
        .collect();
    format!("## {heading}\n\n{}", lines.join("\n"))
}

pub fn prompt_fragments(project_docs: &[ContextDoc], link: Option<&LinkContext>, app_data: &Path) -> PromptFragments {
    let project_context = references_fragment("Project references", project_docs, app_data);
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
        references_fragment("Workspace references", &link.workspace_docs, app_data),
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
        let docs = vec![
            doc("doc-1", ContextDocKind::File, "/w/web/docs/arch.md", None),
            doc(
                "doc-2",
                ContextDocKind::Confluence,
                "https://acme.atlassian.net/wiki/spaces/ENG/pages/123/Pay",
                Some(Utc::now()),
            ),
            doc("doc-3", ContextDocKind::Url, "https://x.y/spec", None),
            doc("doc-4", ContextDocKind::Folder, "/w/web/adr", None),
        ];
        let s = references_fragment("Project references", &docs, Path::new("/app"));
        assert!(s.starts_with("## Project references\n\n- file: `/w/web/docs/arch.md`\n"));
        assert!(s.contains("- confluence: \"Pay\" https://acme.atlassian.net/wiki/spaces/ENG/pages/123/Pay — text: `/app/context/doc-2.md`"));
        assert!(s.contains("- url: https://x.y/spec — not fetched yet"));
        assert!(s.contains("- folder: `/w/web/adr/`"));
        assert_eq!(references_fragment("Workspace references", &[], Path::new("/app")), "");
    }

    #[test]
    fn prompt_fragments_compose_in_order() {
        let mut ctx = one_link();
        ctx.workspace_instructions = Some("Run both test suites.".into());
        let f = prompt_fragments(&[], Some(&ctx), Path::new("/app"));
        let w = f.workspace_fragment.unwrap();
        assert!(
            w.starts_with("## Linked projects") && w.ends_with("Run both test suites.") && f.project_context.is_empty()
        );
        assert!(prompt_fragments(&[], None, Path::new("/app"))
            .workspace_fragment
            .is_none());
    }
}
