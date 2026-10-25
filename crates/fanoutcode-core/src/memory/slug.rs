//! Slugs, task folder names (`YYYY-MM-DD-<slug>`) and titles derived from an intent.

use std::path::Path;

use chrono::NaiveDate;

/// The longest slug a folder name gets.
pub const MAX_SLUG_LEN: usize = 48;

/// How far back from the cut a `-` may be and still be used as the cut point.
const WORD_CUT_WINDOW: usize = 16;

/// Lowercase ASCII letters and digits; every other run of characters becomes one `-`; no leading or trailing `-`;
/// at most `MAX_SLUG_LEN`, cut at a `-` in the last 16 characters when there is one. Empty becomes `"task"`.
pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    if slug.len() > MAX_SLUG_LEN {
        // the slug is ASCII, so byte offsets are character offsets
        let at_boundary = slug.as_bytes()[MAX_SLUG_LEN] == b'-';
        let head = &slug[..MAX_SLUG_LEN];
        let cut = match head.rfind('-') {
            _ if at_boundary => MAX_SLUG_LEN,
            Some(i) if i >= MAX_SLUG_LEN - WORD_CUT_WINDOW => i,
            _ => MAX_SLUG_LEN,
        };
        slug.truncate(cut);
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "task".to_owned()
    } else {
        slug.to_owned()
    }
}

/// `YYYY-MM-DD-<slug>`.
pub fn folder_name(date: NaiveDate, title: &str) -> String {
    format!("{}-{}", date.format("%Y-%m-%d"), slugify(title))
}

/// `folder_name`, then `-2`, `-3`, … until `root.join(name)` does not exist.
pub fn unique_folder_name(root: &Path, date: NaiveDate, title: &str) -> String {
    let base = folder_name(date, title);
    if !root.join(&base).exists() {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|name| !root.join(name).exists())
        .expect("an unbounded counter finds a free name")
}

/// The longest title derived from an intent, in characters.
const MAX_TITLE_CHARS: usize = 60;

/// The first line with words in it, leading `#`, `-`, `*` markers and whitespace stripped, clipped to 60 characters
/// at a word boundary; `"Untitled task"` when there is none.
pub fn title_from_intent(intent: &str) -> String {
    let Some(line) = intent
        .lines()
        .map(|l| {
            l.trim_start_matches(|c: char| c == '#' || c == '-' || c == '*' || c.is_whitespace())
                .trim_end()
        })
        .find(|l| !l.is_empty())
    else {
        return "Untitled task".to_owned();
    };
    if line.chars().count() <= MAX_TITLE_CHARS {
        return line.to_owned();
    }
    let clipped: String = line.chars().take(MAX_TITLE_CHARS).collect();
    let next_is_space = line.chars().nth(MAX_TITLE_CHARS).is_some_and(char::is_whitespace);
    let cut = match clipped.rfind(char::is_whitespace) {
        _ if next_is_space => clipped.len(),
        Some(i) if i > 0 => i,
        _ => clipped.len(),
    };
    clipped[..cut].trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_folds_punctuation_unicode_and_case() {
        assert_eq!(
            slugify("Add OAuth login / refresh tokens!"),
            "add-oauth-login-refresh-tokens"
        );
        assert_eq!(slugify("  Émoji 🚀 title  "), "moji-title");
        assert_eq!(slugify("///"), "task");
        assert_eq!(slugify(""), "task");
    }

    #[test]
    fn slugify_is_bounded_and_cuts_at_a_word() {
        let long = "word ".repeat(80);
        let s = slugify(&long);
        assert!(s.len() <= MAX_SLUG_LEN, "{s}");
        assert!(!s.ends_with('-'));
        assert_eq!(slugify(&"a".repeat(300)).len(), MAX_SLUG_LEN);
    }

    #[test]
    fn folder_name_is_date_then_slug() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        assert_eq!(folder_name(d, "Fix the board"), "2026-10-24-fix-the-board");
    }

    #[test]
    fn unique_folder_name_appends_a_counter_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let d = chrono::NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        std::fs::create_dir(dir.path().join("2026-10-24-fix")).unwrap();
        std::fs::create_dir(dir.path().join("2026-10-24-fix-2")).unwrap();
        assert_eq!(unique_folder_name(dir.path(), d, "fix"), "2026-10-24-fix-3");
        assert_eq!(unique_folder_name(dir.path(), d, "other"), "2026-10-24-other");
    }

    #[test]
    fn title_from_intent_takes_the_first_meaningful_line() {
        assert_eq!(
            title_from_intent("\n\n# Add dark mode\n\nDetails here"),
            "Add dark mode"
        );
        assert_eq!(title_from_intent("   "), "Untitled task");
        let t = title_from_intent(&"verylongword ".repeat(20));
        assert!(t.len() <= 60);
    }
}
