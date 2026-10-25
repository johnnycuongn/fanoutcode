//! The write-capable programs against app data: every argument, whatever its option spelling.

use std::path::Path;

use crate::workspace::guard::run_guard;
use crate::workspace::guard::tests::app_data_guard;

/// The guard's answer for `cmd` run from `cwd`, with app data's overlap files guarded.
fn denied_in(roots: &Path, cmd: &str, cwd: &str) -> bool {
    let input = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": cmd}, "cwd": cwd});
    let mut out = Vec::new();
    run_guard(&mut input.to_string().as_bytes(), &mut out, roots);
    String::from_utf8(out).unwrap().contains("\"deny\"")
}

#[test]
fn tar_with_any_spelling_of_directory_into_app_data_is_denied() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    for cmd in [
        format!("tar -xf /tmp/x.tar --direc={a}"),
        format!("tar -xf /tmp/x.tar --direc {a}"),
        format!("tar -xf /tmp/x.tar -C{a}"),
        format!("tar -xf /tmp/x.tar -C{a}/grants"),
        format!("tar -xfC /tmp/x.tar {a}"),
        format!("tar xfC /tmp/x.tar {a}"),
    ] {
        assert!(denied_in(&roots, &cmd, "/tmp"), "{cmd}");
    }
}

#[test]
fn tar_transform_or_absolute_names_into_app_data_is_denied() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    for cmd in [
        format!("tar -xf /tmp/x.tar -C /tmp/out --transform='s,^,{a}/grants/,'"),
        format!("tar -xf /tmp/x.tar -C /tmp/out --xform 's|^|{a}/|'"),
        format!("tar -xf /tmp/x.tar -C /tmp/out -s ',^,{a}/claims/,'"),
        // absolute member names land wherever the archive says: the guard cannot tell where
        "tar -xPf /tmp/x.tar -C /tmp/out".to_owned(),
        "tar -x --absolute-names -f /tmp/x.tar".to_owned(),
        "tar --extract --absol -f /tmp/x.tar".to_owned(),
    ] {
        assert!(denied_in(&roots, &cmd, "/tmp"), "{cmd}");
    }
}

#[test]
fn unzip_into_app_data_is_denied() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display().to_string();
    for cmd in [
        format!("unzip /tmp/x.zip -d{a}"),
        format!("unzip -o /tmp/x.zip -d{a}/claims"),
        format!("unzip -od {a} /tmp/x.zip"),
    ] {
        assert!(denied_in(&roots, &cmd, "/tmp"), "{cmd}");
    }
    // no `-d`: it unpacks into the cwd
    assert!(denied_in(&roots, "unzip /tmp/x.zip", &a));
    assert!(denied_in(&roots, "unzip -o /tmp/x.zip", &format!("{a}/grants")));
}

#[test]
fn cp_with_attached_or_clustered_target_into_app_data_is_denied() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    for cmd in [
        format!("cp -t{a} x"),
        format!("cp -rt {a} x"),
        format!("cp -rt{a}/grants x"),
        format!("cp --target={a} x"),
        format!("cp --target-directory {a}/grants x"),
        format!("install -t{a}/grants x"),
        format!("ln -st {a} /tmp/evil"),
        format!("mv -t{a} x"),
        format!("rsync -a --temp-dir={a} x/ /tmp/out/"),
    ] {
        assert!(denied_in(&roots, &cmd, "/tmp"), "{cmd}");
    }
}

#[test]
fn the_same_tools_aimed_elsewhere_are_allowed() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    for cmd in [
        "tar -xf /tmp/x.tar --direc=/tmp/out".to_owned(),
        "tar -xf /tmp/x.tar -C/tmp/out".to_owned(),
        "tar -xzf /tmp/x.tgz -C /tmp/out --transform='s,^,pkg/,'".to_owned(),
        "tar -cf /tmp/out.tar src".to_owned(),
        format!("tar -tf {a}/x.tar"),
        "unzip /tmp/x.zip -d/tmp/out".to_owned(),
        "unzip -o /tmp/x.zip".to_owned(),
        format!("unzip -l {a}/x.zip"),
        "cp -t/tmp/out x".to_owned(),
        "cp -rt /tmp/out x".to_owned(),
        "cp --target=/tmp/out x".to_owned(),
        format!("cat {a}/grants/t-1.json > /tmp/copy.json"),
        "install -t/tmp/out x".to_owned(),
        "ln -st /tmp/out /tmp/target".to_owned(),
        "rsync -a --temp-dir=/tmp/t x/ /tmp/out/".to_owned(),
    ] {
        assert!(!denied_in(&roots, &cmd, "/tmp"), "{cmd}");
    }
}
