use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum KeystoneError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Protocol(&'static str),
    Internal(String),
}

impl Display for KeystoneError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::Protocol(msg) => write!(f, "protocol error: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for KeystoneError {}

impl From<std::io::Error> for KeystoneError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for KeystoneError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
