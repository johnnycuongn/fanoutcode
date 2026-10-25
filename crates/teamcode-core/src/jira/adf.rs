//! Atlassian Document Format to plain text. STUB until Task 13: concatenates text nodes.

pub fn adf_to_text(node: &serde_json::Value) -> String {
    let mut out = String::new();
    fn walk(n: &serde_json::Value, out: &mut String) {
        if let Some(t) = n.get("text").and_then(|t| t.as_str()) {
            out.push_str(t);
        }
        for c in n.get("content").and_then(|c| c.as_array()).into_iter().flatten() {
            walk(c, out);
        }
    }
    walk(node, &mut out);
    out
}
