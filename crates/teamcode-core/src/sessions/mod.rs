//! Sessions (§7.1–7.3): everything about a task's or a plain terminal's PTY session that can be tested without a
//! terminal. The shell's `pty.rs` implements `PtySpawner` with portable-pty; `session_service.rs` wires the rest.

pub mod batch;
pub mod flow;
pub mod scrollback;

pub use batch::Batcher;
pub use flow::FlowControl;
pub use scrollback::{Scrollback, ScrollbackBuffer};
