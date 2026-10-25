//! Atlassian Document Format (Jira descriptions, Confluence-style rich text) to plain markdown-ish text.

use serde_json::Value;

/// `text` (a link mark appends ` (<href>)`); `paragraph` then a blank line; `heading` level n as `#`×n; bullet and
/// ordered lists (nested ones indented two spaces); fenced `codeBlock`; `hardBreak` a newline; `mention` as
/// `@name`; cards as their URL; `rule` as `---`; table rows with cells joined by ` | `. Unknown nodes render their
/// content. Three or more newlines collapse to two; the result is trimmed.
pub fn adf_to_text(node: &Value) -> String {
    collapse(&render(node)).trim().to_owned()
}

fn children(node: &Value) -> String {
    node.get("content")
        .and_then(Value::as_array)
        .map(|c| c.iter().map(render).collect())
        .unwrap_or_default()
}

fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = 0;
    for c in s.chars() {
        if c == '\n' {
            run += 1;
            if run > 2 {
                continue;
            }
        } else {
            run = 0;
        }
        out.push(c);
    }
    out
}

fn list(node: &Value, ordered: bool) -> String {
    let items = node
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}. ", i + 1)
        } else {
            "- ".to_owned()
        };
        let body = collapse(&children(item)).trim().replace("\n\n", "\n");
        for (j, line) in body.lines().enumerate() {
            if j == 0 {
                out.push_str(&marker);
            } else {
                out.push_str("  ");
            }
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
    out
}

fn render(node: &Value) -> String {
    let attr = |k: &str| node["attrs"][k].as_str().unwrap_or_default().to_owned();
    match node.get("type").and_then(Value::as_str) {
        None if node.is_null() => String::new(),
        Some("text") => {
            let mut t = node["text"].as_str().unwrap_or_default().to_owned();
            for m in node["marks"].as_array().into_iter().flatten() {
                if m["type"] == "link" {
                    if let Some(href) = m["attrs"]["href"].as_str() {
                        t.push_str(&format!(" ({href})"));
                    }
                }
            }
            t
        }
        Some("paragraph") => format!("{}\n\n", children(node)),
        Some("heading") => {
            let level = node["attrs"]["level"].as_u64().unwrap_or(1).clamp(1, 6) as usize;
            format!("{} {}\n\n", "#".repeat(level), children(node))
        }
        Some("bulletList") => list(node, false),
        Some("orderedList") => list(node, true),
        Some("codeBlock") => format!("```\n{}\n```\n\n", children(node)),
        Some("hardBreak") => "\n".to_owned(),
        Some("mention") => {
            let t = attr("text");
            if t.starts_with('@') {
                t
            } else {
                format!("@{t}")
            }
        }
        Some("inlineCard") | Some("blockCard") => attr("url"),
        Some("rule") => "---\n\n".to_owned(),
        Some("table") => {
            let rows: Vec<String> = node["content"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|row| {
                    row["content"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|cell| collapse(&children(cell)).trim().replace('\n', " "))
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .collect();
            format!("{}\n\n", rows.join("\n"))
        }
        _ => children(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paragraphs_headings_and_lists() {
        let v = json!({"type":"doc","content":[{"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Goal"}]},{"type":"paragraph","content":[{"type":"text","text":"Do "},{"type":"text","text":"it","marks":[{"type":"strong"}]}]},{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"a"}]}]},{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"b"}]}]}]}]});
        assert_eq!(adf_to_text(&v), "## Goal\n\nDo it\n\n- a\n- b");
    }

    #[test]
    fn code_links_mentions_and_unknown_nodes() {
        let v = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"See","marks":[{"type":"link","attrs":{"href":"https://x.y"}}]},{"type":"mention","attrs":{"text":"@Jane"}}]},{"type":"codeBlock","content":[{"type":"text","text":"ls -la"}]},{"type":"panel","content":[{"type":"paragraph","content":[{"type":"text","text":"note"}]}]}]});
        assert_eq!(adf_to_text(&v), "See (https://x.y)@Jane\n\n```\nls -la\n```\n\nnote");
    }

    #[test]
    fn null_description_is_empty() {
        assert_eq!(adf_to_text(&serde_json::Value::Null), "");
    }

    #[test]
    fn ordered_nested_lists_tables_breaks_and_cards() {
        let li =
            |t: &str| json!({"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":t}]}]});
        let v = json!({"type":"doc","content":[
            {"type":"orderedList","content":[li("one"), {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"two"}]},{"type":"bulletList","content":[li("deep")]}]}]},
            {"type":"paragraph","content":[{"type":"text","text":"a"},{"type":"hardBreak"},{"type":"inlineCard","attrs":{"url":"https://c.d"}}]},
            {"type":"rule"},
            {"type":"table","content":[{"type":"tableRow","content":[{"type":"tableHeader","content":[{"type":"paragraph","content":[{"type":"text","text":"k"}]}]},{"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":"v"}]}]}]}]}
        ]});
        assert_eq!(
            adf_to_text(&v),
            "1. one\n2. two\n  - deep\n\na\nhttps://c.d\n\n---\n\nk | v"
        );
    }
}
