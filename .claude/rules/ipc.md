---
paths: ["src-tauri/**", "src/lib/bindings.ts"]
---

# Shell and IPC rules

- Commands return fast. Long work (sessions, tests, model calls) streams over a `Channel` passed by the caller.
- App-wide facts (task changed, needs-you count, workspace membership) are events emitted to every window.
- Every id from the webview is re-parsed with its newtype's `parse` (which checks
  `teamcode_core::ids::valid_id`) before use; serde's transparent `Deserialize` does not validate. Every path is
  canonicalised before use.
- No secret crosses IPC. `secrets::Secret` has no `Serialize` or `specta::Type`; keep it that way.
- Command names are `domain_verb` in snake_case in `src-tauri/src/commands/<domain>.rs`; events are `kebab-case`
  structs in `src-tauri/src/events.rs`.
- Every command carries `#[tauri::command] #[specta::specta]` and is registered in `specta::builder()`. After any
  change run `npm run bindings` and commit `src/lib/bindings.ts`; never edit it by hand.
- The shell holds no logic worth a unit test. Anything testable moves into `teamcode-core`.
- `std` locks are never held across `.await`. Build windows only from `setup` or an `async` command.
- Windows are created in Rust only; the capability grants no window-creation permission to the webview.
