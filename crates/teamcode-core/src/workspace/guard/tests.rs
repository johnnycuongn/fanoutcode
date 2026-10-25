//! Guard tests.

use super::*;

const P: &str = "/w/api"; // protected; home_dir = /Users/j; default cwd = /w/web
fn deny(cmd: &str, cwd: &str) -> bool {
    matches!(
        evaluate(
            &GuardInput {
                command: cmd.into(),
                cwd: cwd.into()
            },
            &[PathBuf::from(P)],
            Path::new("/Users/j")
        ),
        GuardDecision::Deny { .. }
    )
}

#[test]
fn guard_cases() {
    let cases: &[(&str, &str, bool)] = &[
        ("rm -rf /w/api/dist", "/w/web", true),
        ("cat /w/api/README.md", "/w/web", false),
        ("echo hi > /w/api/notes.txt", "/w/web", true),
        ("npm test 2>&1 | tee /w/web/out.log", "/w/web", false),
        ("npm test 2>/dev/null", "/w/api", false),
        ("cd /w/api && npm install", "/w/web", true),
        ("cd /w/api && git status", "/w/web", false),
        ("git -C /w/api commit -m x", "/w/web", true),
        ("git -C /w/api log --oneline", "/w/web", false),
        ("sed -i '' 's/a/b/' /w/api/src/x.ts", "/w/web", true),
        ("sed -n 1p /w/api/src/x.ts", "/w/web", false),
        ("cp /w/api/schema.json /w/web/schema.json", "/w/web", false),
        ("cp /w/web/schema.json /w/api/schema.json", "/w/web", true),
        ("grep -r foo /w/api", "/w/web", false),
        ("ls /w/api; rm /w/web/tmp", "/w/web", false),
        ("(cd /w/api; touch x)", "/w/web", true),
        ("echo \"rm -rf /w/api\"", "/w/web", false),
        ("cd /w/api/sub && echo x > out.txt", "/w/web", true),
        ("cd /w/api && git branch --list", "/w/web", false),
        ("cd /w/api && git branch -D old", "/w/web", true),
        ("find /w/api -name '*.log' -delete", "/w/web", true),
        ("pnpm --dir /w/api add zod", "/w/web", true),
        ("git stash", "/w/api", true),
        ("python3 /w/api/scripts/gen.py", "/w/web", false), // documented best-effort gap
    ];
    for (cmd, cwd, expect) in cases {
        assert_eq!(deny(cmd, cwd), *expect, "command: {cmd} (cwd {cwd})");
    }
}

#[test]
fn more_shapes() {
    let cases: &[(&str, &str, bool)] = &[
        ("FOO=1 sudo -u me rm /w/api/x", "/w/web", true),
        ("cd ../api && touch x", "/w/web", true),
        ("cd /w/api && cd - && touch x", "/w/web", true), // `cd -` leaves the running cwd (conservative)
        ("rm /w/apiary/x", "/w/web", false),
        ("echo x >> \"/w/api/log\"", "/w/web", true),
        ("perl -pi -e 's/a/b/' /w/api/x", "/w/web", true),
        ("git -C /w/api fetch", "/w/web", false),
        ("mv /w/api/a /w/web/a", "/w/web", false),
        ("npm install --prefix=/w/api", "/w/web", true),
        ("find /w/web -exec rm {} +", "/w/web", false),
        ("xargs rm < list.txt", "/w/api", true),
    ];
    for (cmd, cwd, expect) in cases {
        assert_eq!(deny(cmd, cwd), *expect, "command: {cmd} (cwd {cwd})");
    }
}

#[test]
fn nested_sh_c_is_unwrapped() {
    assert!(deny("sh -c 'rm -rf /w/api/dist'", "/w/web"));
    assert!(deny("bash -lc \"cd /w/api && npm i\"", "/w/web"));
    assert!(!deny("sh -c 'ls /w/api'", "/w/web"));
}

#[test]
fn tilde_spelling_of_protected_root_is_matched() {
    let d = evaluate(
        &GuardInput {
            command: "rm -rf ~/w/api/dist".into(),
            cwd: "/Users/j".into(),
        },
        &[PathBuf::from("/Users/j/w/api")],
        Path::new("/Users/j"),
    );
    assert!(matches!(
        d,
        GuardDecision::Deny {
            rule: GuardRule::Program("rm"),
            ..
        }
    ));
}

#[test]
fn deny_reason_names_root_and_rule() {
    let r = deny_reason(Path::new("/w/api"), &GuardRule::Git("commit"));
    assert!(r.starts_with("Team Code: /w/api is read-only in this workspace") && r.contains("git commit"));
}
