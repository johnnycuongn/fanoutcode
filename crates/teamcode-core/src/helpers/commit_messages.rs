//! Commit message drafts (§6.6): one headless call per repo the task touched, written to `commits.md`; and the
//! guarded commit for when "Team Code may commit" is on. The app never commits on its own.

use std::path::{Path, PathBuf};

use livespec_core::llm::{ChatRequest, Provider};
use serde::{Deserialize, Serialize};

use crate::git;
use crate::{Error, Result};

pub const COMMIT_SYSTEM: &str = "You draft commit messages for one repository from its diff. Follow Conventional \
Commits: `type(scope): subject`, imperative, under 72 characters, types feat, fix, refactor, test, docs, chore, perf \
or build, a scope when the repository has an obvious one; then a blank line and two to six lines on what changed and \
why, wrapped at 72 columns. Write one message per clearly separable concern a reviewer would want to read apart; \
otherwise one message. Every listed file belongs to exactly one message. Answer only in this format, one block per \
message, and nothing else:\n\n### <subject>\n\n```\n<the full commit message>\n```\n\nFiles: `path/one`, `path/two`";

/// One drafted commit and the files it covers (paths relative to its repo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DraftCommit {
    pub message: String,
    pub files: Vec<String>,
}

/// One repo's section of `commits.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RepoCommits {
    pub repo_root: PathBuf,
    pub commits: Vec<DraftCommit>,
}

/// The request for one repo: its diff and file list, no tools.
pub fn request(repo_root: &Path, diff: &str, files: &[String], provider: &Provider) -> ChatRequest {
    let budget = provider.context_budget_chars() * 3 / 4;
    let diff: String = diff.chars().take(budget).collect();
    ChatRequest {
        system: COMMIT_SYSTEM.to_owned(),
        user: format!(
            "<repository path=\"{}\">\n<files>\n{}\n</files>\n<diff>\n{diff}\n</diff>\n</repository>",
            repo_root.display(),
            files.join("\n")
        ),
        max_tokens: 2_000,
        cwd: None,
        allow_repo_tools: false,
        max_turns: None,
    }
}

/// The backticked paths of a `Files:` line (comma-separated plain paths when nothing is backticked).
fn files_of(line: &str) -> Vec<String> {
    let rest = line.trim_start_matches("Files:").trim();
    let ticked: Vec<String> = rest
        .split('`')
        .enumerate()
        .filter(|(i, p)| i % 2 == 1 && !p.trim().is_empty())
        .map(|(_, p)| p.trim().to_owned())
        .collect();
    if !ticked.is_empty() {
        return ticked;
    }
    rest.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The `### …` blocks of one repo: each message is its fenced block (the heading when there is none).
fn blocks(text: &str) -> Vec<DraftCommit> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(subject) = line.strip_prefix("### ") else {
            continue;
        };
        let (mut message, mut files, mut fence) = (None::<Vec<&str>>, Vec::new(), false);
        while let Some(next) = lines.peek() {
            if next.starts_with("### ") || next.starts_with("## ") {
                break;
            }
            let l = lines.next().expect("peeked");
            if l.trim_start().starts_with("```") {
                fence = !fence;
                if fence && message.is_none() {
                    message = Some(Vec::new());
                }
                continue;
            }
            if fence {
                if let Some(m) = message.as_mut() {
                    m.push(l);
                }
            } else if l.trim_start().starts_with("Files:") {
                files = files_of(l.trim());
            }
        }
        out.push(DraftCommit {
            message: message.map_or_else(|| subject.trim().to_owned(), |m| m.join("\n").trim_end().to_owned()),
            files,
        });
    }
    out
}

/// The model's answer for the repo at `_repo_root` (its paths are already repo-relative).
pub fn parse(answer: &str, _repo_root: &Path) -> Vec<DraftCommit> {
    blocks(answer)
}

/// The whole `commits.md`: `# Commits`, then `## <repo_root>` and its blocks per repo.
pub fn render(all: &[RepoCommits]) -> String {
    let mut md = String::from("# Commits\n");
    for repo in all {
        md.push_str(&format!("\n## {}\n", repo.repo_root.display()));
        for c in &repo.commits {
            let subject = c.message.lines().next().unwrap_or("");
            let files: Vec<String> = c.files.iter().map(|f| format!("`{f}`")).collect();
            md.push_str(&format!(
                "\n### {subject}\n\n```\n{}\n```\n\nFiles: {}\n",
                c.message,
                files.join(", ")
            ));
        }
    }
    md
}

/// Reads `commits.md` back into repos and drafts (Plan 08's Commit tab, `git_commit`).
pub fn parse_commits_md(md: &str) -> Vec<RepoCommits> {
    let mut out = Vec::new();
    let mut sections = md.split("\n## ").skip(1);
    for section in sections.by_ref() {
        let (root, body) = section.split_once('\n').unwrap_or((section, ""));
        out.push(RepoCommits {
            repo_root: PathBuf::from(root.trim()),
            commits: blocks(body),
        });
    }
    out
}

/// Committing from Team Code needs "Team Code may commit" (§6.6, default off).
pub fn commit_guard(may_commit: bool) -> Result<()> {
    if may_commit {
        Ok(())
    } else {
        Err(Error::Precondition(
            "Turn on \"Team Code may commit\" in project settings to commit from here.".into(),
        ))
    }
}

/// `git add -- <files>` then `git commit -F - -- <files>` (only those paths, whatever else is staged); the new sha.
pub async fn commit_files(repo: &Path, c: &DraftCommit) -> Result<String> {
    if c.files.is_empty() {
        return Err(Error::Precondition("That draft lists no files to commit.".into()));
    }
    let mut add = vec!["add", "--"];
    add.extend(c.files.iter().map(String::as_str));
    git::run(repo, &add).await?;
    let mut commit = vec!["commit", "-q", "-F", "-", "--"];
    commit.extend(c.files.iter().map(String::as_str));
    git::run_with_input(repo, &commit, Some(&c.message)).await?;
    git::head(repo).await
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use livespec_core::llm::{OpenAiCompat, Provider};

    use super::*;
    use crate::git::test_support::{temp_repo, write};
    use crate::git::{run, status};

    const BLOCK: &str = "### feat(board): add swimlane rows\n\n```\nfeat(board): add swimlane rows\n\nOne row per project, collapsible.\n```\n\nFiles: `src/features/board/Swimlane.tsx`, `src/features/board/store.ts`\n";

    fn answer() -> String {
        format!(
            "{BLOCK}\n{}",
            BLOCK
                .replace("add swimlane rows", "store row order")
                .replace("One row per project, collapsible.", "Rows keep their order.")
        )
    }

    fn local_provider() -> Provider {
        Provider::OpenAiCompat(OpenAiCompat {
            base_url: "http://127.0.0.1:9".into(),
            model: "qwen3-4b-instruct".into(),
            context_tokens: 16_384,
            http: reqwest::Client::new(),
        })
    }

    #[test]
    fn request_covers_one_repo_with_its_diff() {
        let r = request(
            Path::new("/ws/app"),
            "diff --git a/x b/x",
            &["x".into()],
            &local_provider(),
        );
        assert!(r.system.contains("Conventional Commits") && r.user.contains("diff --git") && !r.allow_repo_tools);
    }

    #[test]
    fn answer_parses_into_messages_and_files() {
        let c = parse(&answer(), Path::new("/ws/app")); // the block above, twice with different subjects
        assert_eq!(c.len(), 2);
        assert!(c[0].message.starts_with("feat(board): add swimlane rows\n\nOne row"));
        assert_eq!(
            c[0].files,
            vec!["src/features/board/Swimlane.tsx", "src/features/board/store.ts"]
        );
    }

    #[test]
    fn render_then_parse_roundtrips() {
        let all = vec![RepoCommits {
            repo_root: "/ws/app".into(),
            commits: parse(&answer(), Path::new("/ws/app")),
        }];
        let md = render(&all);
        assert!(md.starts_with("# Commits\n\n## /ws/app\n"));
        assert_eq!(parse_commits_md(&md), all);
    }

    #[test]
    fn git_commit_requires_may_commit() {
        assert!(commit_guard(true).is_ok());
        assert!(matches!(commit_guard(false), Err(Error::Precondition(m)) if m.starts_with("Turn on")));
    }

    #[tokio::test]
    async fn commit_files_commits_only_the_listed_paths() {
        let r = temp_repo();
        write(&r, "a.rs", "1");
        write(&r, "b.rs", "2");
        let sha = commit_files(
            r.path(),
            &DraftCommit {
                message: "feat: a".into(),
                files: vec!["a.rs".into()],
            },
        )
        .await
        .unwrap();
        assert_eq!(sha.len(), 40);
        assert_eq!(
            run(r.path(), &["show", "--name-only", "--format=%s", "HEAD"])
                .await
                .unwrap()
                .trim(),
            "feat: a\n\na.rs"
        );
        assert_eq!(status(r.path()).await.unwrap()[0].path, "b.rs");
    }
}
