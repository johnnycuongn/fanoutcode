//! A small shell tokeniser for the Bash guard: words with quotes removed, split into segments on unquoted
//! `;`, `&&`, `||`, `|`, `&`, newlines, parentheses, `$(` and backticks, with output redirections recorded apart from
//! the words. It is best effort by design: anything inside quotes is data, never a command, except where the guard
//! itself unwraps `sh -c '…'`.

/// One simple command.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Segment {
    pub words: Vec<String>,
    /// Targets of writing redirections (`>`, `>>`, `>|`, `&>`, `&>>`, `N>`), in order.
    pub write_targets: Vec<String>,
    /// An unquoted `$(` or backtick opened inside this command: some of its words come from a substitution the
    /// guard cannot read.
    pub substituted: bool,
}

#[derive(PartialEq, Eq)]
enum Pending {
    None,
    /// The next word is a redirection target; `true` when it is written to.
    Target(bool),
}

struct Lexer {
    segments: Vec<Segment>,
    current: Segment,
    word: String,
    /// The word has content (even an empty quoted string) or quotes.
    in_word: bool,
    /// The word so far is only unquoted digits (a file descriptor before `>`).
    digits_only: bool,
    pending: Pending,
    /// Inside an unquoted backtick substitution (the next backtick closes it).
    in_backtick: bool,
}

impl Lexer {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            current: Segment::default(),
            word: String::new(),
            in_word: false,
            digits_only: true,
            pending: Pending::None,
            in_backtick: false,
        }
    }

    fn push_char(&mut self, c: char, quoted: bool) {
        self.in_word = true;
        if quoted || !c.is_ascii_digit() {
            self.digits_only = false;
        }
        self.word.push(c);
    }

    fn end_word(&mut self) {
        if !self.in_word {
            return;
        }
        let w = std::mem::take(&mut self.word);
        match std::mem::replace(&mut self.pending, Pending::None) {
            Pending::Target(true) => self.current.write_targets.push(w),
            Pending::Target(false) => {}
            Pending::None => self.current.words.push(w),
        }
        self.in_word = false;
        self.digits_only = true;
    }

    /// An unquoted substitution opens here: the command it sits in is marked before its segment ends.
    fn open_substitution(&mut self) {
        self.current.substituted = true;
        self.end_segment();
    }

    fn end_segment(&mut self) {
        self.end_word();
        self.pending = Pending::None;
        let seg = std::mem::take(&mut self.current);
        if !seg.words.is_empty() || !seg.write_targets.is_empty() {
            self.segments.push(seg);
        }
    }
}

/// Splits `command` into simple commands.
pub(crate) fn segments(command: &str) -> Vec<Segment> {
    let chars: Vec<char> = command.chars().collect();
    let mut lx = Lexer::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\\' => {
                match next {
                    Some('\n') => {}
                    Some(n) => lx.push_char(n, true),
                    None => {}
                }
                i += 2;
                continue;
            }
            '\'' => {
                lx.in_word = true;
                lx.digits_only = false;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    lx.word.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                lx.in_word = true;
                lx.digits_only = false;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && matches!(chars.get(i + 1), Some('"' | '\\' | '$' | '`')) {
                        i += 1;
                    }
                    lx.word.push(chars[i]);
                    i += 1;
                }
            }
            ' ' | '\t' => lx.end_word(),
            '`' if !lx.in_backtick => {
                lx.in_backtick = true;
                lx.open_substitution();
            }
            '`' => {
                lx.in_backtick = false;
                lx.end_segment();
            }
            ';' | '\n' | '(' | ')' => lx.end_segment(),
            '$' if next == Some('(') => {
                lx.open_substitution();
                i += 1;
            }
            '|' => {
                lx.end_segment();
                if matches!(next, Some('|' | '&')) {
                    i += 1;
                }
            }
            '&' if next == Some('>') => {
                // &> and &>> write stdout and stderr to the target
                lx.end_word();
                i += 1;
                if chars.get(i + 1) == Some(&'>') {
                    i += 1;
                }
                lx.pending = Pending::Target(true);
            }
            '&' => {
                lx.end_segment();
                if next == Some('&') {
                    i += 1;
                }
            }
            '>' => {
                if lx.in_word && lx.digits_only {
                    // `2>`: the digits name a descriptor, not a word
                    lx.word.clear();
                    lx.in_word = false;
                }
                lx.end_word();
                if matches!(next, Some('>' | '|')) {
                    i += 1;
                }
                if chars.get(i + 1) == Some(&'&') {
                    // `>&2`, `2>&1`, `>&-`: duplicating a descriptor writes no file
                    i += 1;
                    let mut j = i + 1;
                    while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '-') {
                        j += 1;
                    }
                    if j > i + 1 {
                        i = j;
                        continue;
                    }
                }
                lx.pending = Pending::Target(true);
            }
            '<' => {
                lx.end_word();
                while chars.get(i + 1) == Some(&'<') {
                    i += 1;
                }
                lx.pending = Pending::Target(false);
            }
            _ => lx.push_char(c, false),
        }
        i += 1;
    }
    lx.end_segment();
    lx.segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(cmd: &str) -> Vec<Vec<String>> {
        segments(cmd).into_iter().map(|s| s.words).collect()
    }

    #[test]
    fn splits_on_operators_and_keeps_quoted_text_whole() {
        assert_eq!(
            words("ls /a; rm 'x y' && echo \"a;b\""),
            [vec!["ls", "/a"], vec!["rm", "x y"], vec!["echo", "a;b"]]
        );
        assert_eq!(words("(cd /a; touch x)"), [vec!["cd", "/a"], vec!["touch", "x"]]);
        assert_eq!(words("echo $(rm x)"), [vec!["echo"], vec!["rm", "x"]]);
    }

    #[test]
    fn marks_the_command_an_unquoted_substitution_opens_in() {
        let marked = |cmd: &str| segments(cmd).iter().map(|s| s.substituted).collect::<Vec<_>>();
        assert_eq!(marked("rm $(cat list)"), [true, false]);
        assert_eq!(marked("rm `cat list` && ls"), [true, false, false]);
        assert_eq!(marked("echo \"$(date)\" > x"), [false], "quoted: kept in the word");
    }

    #[test]
    fn redirections_are_targets_not_words() {
        let s = &segments("npm test 2>&1 >out.log 2>/dev/null < in.txt")[0];
        assert_eq!(s.words, ["npm", "test"]);
        assert_eq!(s.write_targets, ["out.log", "/dev/null"]);
        assert_eq!(segments("echo hi &>> all.log")[0].write_targets, ["all.log"]);
        assert_eq!(segments("echo 2>x")[0].words, ["echo"], "N> is a descriptor");
        assert_eq!(segments("echo a2>x")[0].words, ["echo", "a2"]);
    }
}
