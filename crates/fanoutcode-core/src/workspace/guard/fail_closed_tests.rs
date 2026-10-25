//! The Bash guard's fail-closed rules: shell keywords, `$HOME`, paths it cannot read, the app-data text rule and
//! the copiers that check every argument.

use super::tests::app_data_guard;
use super::*;

/// `/w/api` protected, home `/Users/j`.
fn deny(cmd: &str, cwd: &str) -> bool {
    deny_with(cmd, cwd, &["/w/api"])
}

fn deny_with(cmd: &str, cwd: &str, protected: &[&str]) -> bool {
    let roots: Vec<PathBuf> = protected.iter().map(PathBuf::from).collect();
    let input = GuardInput {
        command: cmd.into(),
        cwd: cwd.into(),
    };
    matches!(
        evaluate(&input, &roots, Path::new("/Users/j")),
        GuardDecision::Deny { .. }
    )
}

fn guard_says_deny(roots: &Path, cmd: &str, cwd: &str) -> bool {
    let input = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": cmd}, "cwd": cwd});
    let mut out = Vec::new();
    run_guard(&mut input.to_string().as_bytes(), &mut out, roots);
    String::from_utf8(out).unwrap().contains("\"deny\"")
}

#[test]
fn shell_keywords_before_the_program_are_skipped() {
    for cmd in [
        "if true; then rm -rf /w/api; fi",
        "if test -d x; then :; else rm -rf /w/api; fi",
        "if false; then :; elif rm -rf /w/api; then :; fi",
        "while true; do touch /w/api/x; done",
        "{ rm /w/api/x; }",
        "! rm /w/api/x",
        "exec rm /w/api/x",
        "if git -C /w/api commit -m x; then :; fi",
    ] {
        assert!(deny(cmd, "/w/web"), "{cmd}");
    }
    assert!(!deny("if true; then rm -rf dist; fi", "/w/web"));
}

#[test]
fn home_is_expanded_in_paths() {
    let home = ["/Users/j/api"];
    for cmd in [
        "rm -rf $HOME/api",
        "rm -rf ${HOME}/api/x",
        "touch ~/api/x",
        "cp notes.txt \"$HOME/api/\"",
        "echo x > $HOME/api/out.txt",
    ] {
        assert!(deny_with(cmd, "/w/web", &home), "{cmd}");
    }
    for cmd in ["rm -rf $HOME/other", "rm -rf ${HOME}/apiary", "cat $HOME/api/x"] {
        assert!(!deny_with(cmd, "/w/web", &home), "{cmd}");
    }
}

#[test]
fn a_path_the_guard_cannot_read_denies_a_writer() {
    for cmd in [
        "rm -rf $DIR",
        "rm -rf \"${OUT}/x\"",
        "mv a ${X}",
        "rm $(cat list)",
        "rm `cat list`",
        "cp a $DEST",
        "cp -t $DEST a",
        "mkdir -p \"$(dirname x)\"",
        "touch $F",
        "ln -s $T link",
        "find . | xargs rm",
        "tar -xf $PKG",
        "unzip $ZIP",
        "sed -i s/a/b/ $F",
        "echo x > $LOG",
        "git -C $REPO commit -m x",
        "git --work-tree=$W checkout .",
    ] {
        assert!(deny(cmd, "/w/web"), "{cmd}");
    }
    for cmd in [
        "cp $SRC /tmp/x",
        "echo $HOME $PATH",
        "rm -rf $HOME/tmp/x",
        "sed -i 's/a$/b/' file.txt",
        "ls $DIR",
        "npm test -- $ARGS",
        "git -C $REPO log",
    ] {
        assert!(!deny(cmd, "/w/web"), "{cmd}");
    }
}

#[test]
fn a_command_naming_app_data_is_denied_unless_it_only_reads() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    for cmd in [
        format!("sqlite3 '{a}/fanoutcode.db' 'delete from tasks'"),
        format!("TAR_OPTIONS=--directory='{a}' tar xf /tmp/x.tar"),
        format!("UNZIP='-d {a}' unzip /tmp/x.zip"),
        format!("python3 -c 'open(\"{a}/grants/t.json\", \"w\")'"),
        format!("node -e 'require(\"fs\").rmSync(\"{a}\", {{recursive: true}})'"),
        format!("tar -tf '{a}/x.tar' --to-command='sh -c x'"),
        format!("tar -tf '{a}/x.tar' --checkpoint-action=exec=sh"),
        format!("tar -t -I gzip -f '{a}/x.tar'"),
        format!("tar -tf host:'{a}/x.tar'"),
        format!("rg --pre sh foo '{a}'"),
        format!("less -o /tmp/log '{a}/events/t.jsonl'"),
        format!("file -C -m '{a}/magic'"),
        format!("GREP_OPTIONS='{a}' grep x /tmp/y"),
    ] {
        assert!(guard_says_deny(&roots, &cmd, "/tmp"), "{cmd}");
    }
    for cmd in [
        format!("cat '{a}/grants/t.json'"),
        format!("grep -r x '{a}'"),
        format!("rg foo '{a}/claims'"),
        format!("head -n 5 '{a}/events/t.jsonl' | wc -l"),
        format!("ls -la '{a}'"),
        format!("stat '{a}/grants'"),
        format!("diff '{a}/grants/a.json' /tmp/b.json"),
        format!("tar -tf '{a}/x.tar'"),
        format!("unzip -l '{a}/x.zip'"),
        format!("cd '{a}' && ls"),
    ] {
        assert!(!guard_says_deny(&roots, &cmd, "/tmp"), "{cmd}");
    }
}

#[test]
fn ln_install_and_rsync_check_every_argument_and_cp_keeps_its_sources() {
    for cmd in [
        "ln -s /w/api/src link",
        "ln /w/api/x y",
        "install /w/api/bin /tmp/bin",
        "rsync -a /w/api/ /tmp/backup/",
        "cp -r /w/web/x /w/api/",
    ] {
        assert!(deny(cmd, "/w/web"), "{cmd}");
    }
    // a lone positional copies into the cwd
    assert!(deny("cp /tmp/x", "/w/api"));
    assert!(deny("ln -s /tmp/x", "/w/api/src"));
    assert!(
        !deny("cp /w/api/schema.json /w/web/", "/w/web"),
        "a copy's source is a read"
    );
    assert!(!deny("cp /w/api/schema.json", "/w/web"));
}

#[test]
fn ordinary_work_still_passes() {
    let (_d, _app, roots) = app_data_guard();
    for cwd in ["/w/web", "/tmp/proj"] {
        for cmd in [
            "rm -rf node_modules",
            "cp -r . /tmp/x",
            "rsync -a src/ ~/backup/",
            "tar xzf pkg.tgz",
            "ln -s ../shared lib",
            "npm install && npm test",
            "git add -A && git commit -m \"$(cat <<'EOF'\nfix: x\nEOF\n)\"",
        ] {
            assert!(!deny(cmd, cwd), "{cmd} in {cwd}");
            assert!(
                !guard_says_deny(&roots, cmd, cwd),
                "{cmd} in {cwd} with app data guarded"
            );
        }
    }
}

#[test]
fn sqlite3_on_the_app_db_is_denied() {
    let (_d, app, roots) = app_data_guard();
    let db = app.join("fanoutcode.db");
    let (a, d) = (app.display(), db.display());
    // `.output` and `ATTACH` write, so not even `-readonly` makes sqlite3 a reader
    for (cmd, cwd) in [
        (format!("sqlite3 '{d}' 'delete from tasks'"), "/tmp".to_owned()),
        (format!("sqlite3 -readonly '{d}' .dump"), "/tmp".to_owned()),
        (
            format!("cd '{a}' && sqlite3 fanoutcode.db 'delete from tasks'"),
            "/tmp".to_owned(),
        ),
        ("sqlite3 fanoutcode.db-wal .tables".to_owned(), a.to_string()),
        (format!("litecli '{d}'"), "/tmp".to_owned()),
        (
            format!("python3 -c 'import sqlite3; sqlite3.connect(\"{d}\").execute(\"delete from tasks\")'"),
            "/tmp".to_owned(),
        ),
    ] {
        assert!(guard_says_deny(&roots, &cmd, &cwd), "{cmd} in {cwd}");
    }
    assert!(!guard_says_deny(&roots, "sqlite3 /tmp/other.db .tables", "/tmp"));
}

#[test]
fn tasks_memory_folders_in_app_data_stay_writable() {
    let (_d, app, roots) = app_data_guard();
    let mem = app.join("projects/p-1/tasks/t-1");
    let m = mem.display();
    for (cmd, cwd) in [
        (format!("echo x > '{m}/notes.md'"), "/tmp".to_owned()),
        (format!("mkdir -p '{m}/scratch'"), "/tmp".to_owned()),
        (format!("cd '{m}' && touch plan.md && rm -f old.md"), "/tmp".to_owned()),
        ("sed -i '' 's/a/b/' plan.md".to_owned(), m.to_string()),
    ] {
        assert!(!guard_says_deny(&roots, &cmd, &cwd), "{cmd} in {cwd}");
    }
    // the rest of app data is still guarded
    assert!(guard_says_deny(
        &roots,
        &format!("touch '{}/other.json'", app.display()),
        "/tmp"
    ));
}
