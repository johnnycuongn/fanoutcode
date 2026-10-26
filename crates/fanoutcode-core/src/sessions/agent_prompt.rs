//! Claude Code's own questions before its first hook (§7.2): trusting the folder, signing in, first-run setup. No
//! hook fires until they are answered, so a Claude Code session's output is read for them until `SessionStart`.

use crate::domain::AgentQuestion;

/// How much raw output is kept to match against, in characters.
const TAIL_CHARS: usize = 4096;

/// Phrases that mean each question, lowercase, checked in this order against de-styled output.
const PHRASES: &[(AgentQuestion, &[&str])] = &[
    (
        AgentQuestion::TrustFolder,
        &[
            "do you trust the files in this folder",
            "is this a project you created or one you trust",
            "yes, i trust this folder",
        ],
    ),
    (
        AgentQuestion::SignIn,
        &[
            "select login method",
            "please run /login",
            "invalid api key",
            "claude account with subscription",
        ],
    ),
    (AgentQuestion::Setup, &["choose the text style", "let's get started"]),
];

/// Output with escape sequences replaced by a space (Ink moves the cursor between words), lowercased, with runs of
/// whitespace collapsed to one space.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.next() {
                // CSI: parameters and intermediates up to a final byte in @..~
                Some('[') => {
                    for d in chars.by_ref() {
                        if ('@'..='~').contains(&d) {
                            break;
                        }
                    }
                }
                // OSC: up to BEL or ST (ESC \)
                Some(']') => {
                    while let Some(d) = chars.next() {
                        if d == '\u{7}' || (d == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            }
            out.push(' ');
        } else if c.is_control() || c.is_whitespace() {
            out.push(' ');
        } else {
            out.extend(c.to_lowercase());
        }
    }
    let mut collapsed = String::with_capacity(out.len());
    for word in out.split(' ').filter(|w| !w.is_empty()) {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }
    collapsed
}

/// The question `text` (already `plain`) shows, if any.
pub fn detect(text: &str) -> Option<AgentQuestion> {
    PHRASES
        .iter()
        .find(|(_, phrases)| phrases.iter().any(|p| text.contains(p)))
        .map(|(q, _)| *q)
}

/// Reads a session's output until its first hook.
#[derive(Debug, Default)]
pub struct PromptWatch {
    /// The latest raw output, de-styled whole on every read so a word split across two reads still matches.
    raw: String,
    asked: Option<AgentQuestion>,
}

impl PromptWatch {
    /// Takes more output; the question when it newly appears. A shown question clears the output read so far, so the
    /// next screen (signing in after trusting the folder) is judged on its own.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<AgentQuestion> {
        self.raw.push_str(&String::from_utf8_lossy(bytes));
        let excess = self.raw.chars().count().saturating_sub(TAIL_CHARS);
        if excess > 0 {
            self.raw = self.raw.chars().skip(excess).collect();
        }
        let q = detect(&plain(&self.raw))?;
        self.raw.clear();
        if self.asked == Some(q) {
            return None;
        }
        self.asked = Some(q);
        Some(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The trust screen as Claude Code 2.1 drew it in the UX run, with Ink's styling and cursor moves.
    const TRUST: &str = "\u{1b}[1mQuick safety check:\u{1b}[22m Is this a project you created or one you\u{1b}[1C\
        trust?\r\n\u{1b}[36m❯ No, exit\u{1b}[39m\r\n  Yes, I trust this folder\r\n";

    #[test]
    fn plain_drops_styling_and_cursor_moves() {
        assert_eq!(
            plain("\u{1b}[1mHi\u{1b}[22m\u{1b}[2Cthere\r\n\u{1b}]0;title\u{7}ok"),
            "hi there ok"
        );
    }

    #[test]
    fn the_trust_screen_is_a_trust_question() {
        assert_eq!(detect(&plain(TRUST)), Some(AgentQuestion::TrustFolder));
    }

    #[test]
    fn sign_in_and_setup_screens_are_recognised() {
        assert_eq!(
            detect(&plain("Select login method:\r\n ❯ 1. Claude account with subscription")),
            Some(AgentQuestion::SignIn)
        );
        assert_eq!(
            detect(&plain("Invalid API key · Please run /login")),
            Some(AgentQuestion::SignIn)
        );
        assert_eq!(
            detect(&plain("Let's get started.\r\nChoose the text style that looks best")),
            Some(AgentQuestion::Setup)
        );
        assert_eq!(
            detect(&plain("✻ Welcome to Claude Code! ❯ Try \"fix lint errors\"")),
            None
        );
    }

    #[test]
    fn a_question_split_across_reads_is_seen_once_and_the_next_screen_after_it() {
        let mut w = PromptWatch::default();
        // split inside "created", so neither read holds a whole phrase
        let at = TRUST.find("created").unwrap() + 3;
        let (a, b) = TRUST.split_at(at);
        assert!(detect(&plain(b)).is_some_and(|q| q == AgentQuestion::TrustFolder)); // "yes, i trust this folder"
        let b = b.replace("Yes, I trust this folder", "Yes");
        assert_eq!(w.feed(a.as_bytes()), None);
        assert_eq!(w.feed(b.as_bytes()), Some(AgentQuestion::TrustFolder));
        // a redraw of the same screen asks nothing new
        assert_eq!(w.feed(TRUST.as_bytes()), None);
        assert_eq!(w.feed(b"Select login method:"), Some(AgentQuestion::SignIn));
    }
}
