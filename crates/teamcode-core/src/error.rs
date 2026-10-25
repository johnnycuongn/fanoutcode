//! The one error type. It serialises as its message so the webview shows it verbatim.

use serde::{Serialize, Serializer};

/// `Precondition` messages are instructions shown verbatim; `Provider` is a model, CLI or network failure;
/// `Invalid` is a bad id or malformed input; the rest wrap their sources.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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

/// Exported to TypeScript as `string`, matching the serialised form.
impl specta::Type for Error {
    fn inline(type_map: &mut specta::TypeCollection, generics: specta::Generics) -> specta::DataType {
        <String as specta::Type>::inline(type_map, generics)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Db(e.to_string())
    }
}

/// livespec-core's errors keep their kind where Team Code has one; network and archive failures are provider and
/// I/O failures.
impl From<livespec_core::Error> for Error {
    fn from(e: livespec_core::Error) -> Self {
        match e {
            livespec_core::Error::Precondition(m) => Error::Precondition(m),
            livespec_core::Error::Provider(m) => Error::Provider(m),
            livespec_core::Error::Io(e) => Error::Io(e.to_string()),
            livespec_core::Error::Http(e) => Error::Provider(e.to_string()),
            livespec_core::Error::Json(e) => Error::Invalid(e.to_string()),
            livespec_core::Error::Archive(m) => Error::Io(m),
        }
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
