---
paths: ["crates/**"]
---

# Core rules

- `teamcode-core` never depends on Tauri. It may depend on `specta` for type export only.
- Errors: `Error::Precondition` messages are instructions shown to the user verbatim ("Open a project first.");
  `Error::Provider` is a model, CLI or network failure; `Error::Invalid` is a bad id or malformed input; `Io`, `Git`
  and `Db` wrap their sources. Errors serialise as their message string.
- Tests live beside the code in `#[cfg(test)] mod tests`, or in a sibling `<module>_tests.rs` through `#[path]`
  when they would push the module over budget. Every data-carrying enum is round-tripped through serde in
  a test, because internally tagged variants fail only at runtime.
- Serde: no `rename_all`; `#[serde(tag = "type")]` on data-carrying enums, unit or struct variants only; id newtypes
  are `#[serde(transparent)]`.
- Files are written atomically (write a sibling temp file, then rename).
- Migrations are `migrations/00NN_<name>.sql` (NN = owning plan) listed in `db::MIGRATIONS`; an applied migration is
  never edited.
- `crates/livespec-core` is copied from LiveSpec and never edited; adaptations live in `teamcode-core`.
- A Rust module stays under 500 lines; split it into a directory module when it grows.
