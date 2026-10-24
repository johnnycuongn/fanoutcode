//! The pipeline (§6): stage helpers, hand-back detection, the transition state machine and applying its effects.

pub mod handback;
pub mod stage;

pub use handback::{detect_handback, SENTINEL_PREFIX};
pub use stage::StageArtifact;
