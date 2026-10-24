//! The one error type. It serialises as its message so the webview shows it verbatim.

use serde::{Serialize, Serializer};

/// `Precondition` messages are instructions shown verbatim; `Provider` is a model, CLI or network failure;
/// `Invalid` is a bad id or malformed input; the rest wrap their sources.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Precondition(String),
    #[error("{0}")]
    Provider(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Git(String),
    #[error("{0}")]
    Db(String),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Invalid(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_serialise_as_their_message() {
        assert_eq!(
            serde_json::to_string(&Error::Precondition("Open a project first.".into())).unwrap(),
            "\"Open a project first.\""
        );
        assert_eq!(
            serde_json::to_string(&Error::Db("locked".into())).unwrap(),
            "\"locked\""
        );
    }
}
