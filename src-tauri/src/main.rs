// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `fanoutcode guard <roots-file>`: the PreToolUse hook of a protected workspace, answered before any Tauri setup
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("guard") {
        let roots = args.get(2).map(String::as_str).unwrap_or("");
        std::process::exit(fanoutcode_core::workspace::guard::run_guard(
            &mut std::io::stdin(),
            &mut std::io::stdout(),
            std::path::Path::new(roots),
        ));
    }
    // `fanoutcode claim-check <task-id> <grants> <notes> <log> <0|1> <claims…>`: the PreToolUse hook over other
    // tasks' uncommitted files (Plan 11); arguments that do not parse allow (fail open)
    if args.get(1).map(String::as_str) == Some("claim-check") {
        use fanoutcode_core::overlap::check::{run_check, CheckArgs};
        let Some(check) = CheckArgs::parse(&args[2..]) else {
            eprintln!("fanoutcode claim-check: bad arguments; allowing");
            std::process::exit(0);
        };
        std::process::exit(run_check(&mut std::io::stdin(), &mut std::io::stdout(), &check));
    }
    fanoutcode_lib::run()
}
