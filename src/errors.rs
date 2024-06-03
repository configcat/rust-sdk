use std::error::Error;
use std::fmt::{Display, Formatter};

/// Error kind that represents failures reported by the [`crate::Client`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ErrorKind {
    /// No error occurred.
    NoError,
    /// Initialization of the internal [`reqwest::Client`] failed.
    HttpClientInitFailure,
    /// The evaluation failed because the config JSON was not available locally.
    ConfigJsonNotAvailable = 1000,
    /// The evaluation failed because the key of the evaluated setting was not found in the config JSON.
    SettingKeyMissing = 1001,
    /// The evaluation failed because the key of the evaluated setting was not found in the config JSON.
    EvaluationFailure = 1002,
    /// An HTTP response indicating an invalid SDK Key was received (403 Forbidden or 404 Not Found).
    InvalidSdkKey = 1100,
    /// Invalid HTTP response was received (unexpected HTTP status code).
    UnexpectedHttpResponse = 1101,
    /// The HTTP request timed out.
    HttpRequestTimeout = 1102,
    /// The HTTP request failed (most likely, due to a local network issue).
    HttpRequestFailure = 1103,
    /// Redirection loop encountered while trying to fetch config JSON.
    RedirectLoop = 1104,
    /// An invalid HTTP response was received (200 OK with an invalid content).
    InvalidHttpResponseContent = 1105,
    /// An invalid HTTP response was received (304 Not Modified when no config JSON was cached locally).
    InvalidHttpResponseWhenLocalCacheIsEmpty = 1106,
    /// The evaluation failed because of a type mismatch between the evaluated setting value and the specified default value.
    SettingValueTypeMismatch = 2002,
    /// The client is in offline mode, it cannot initiate HTTP requests.
    OfflineClient = 3200,
    /// The refresh operation failed because the client is configured to use the [`crate::OverrideBehavior::LocalOnly`] override behavior,
    LocalOnlyClient = 3202,
}

impl ErrorKind {
    pub(crate) fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Error struct that holds the [`ErrorKind`] and message of the reported failure.
#[derive(Debug, PartialEq)]
pub struct ClientError {
    /// Error kind that represents failures reported by the [`crate::Client`].
    pub kind: ErrorKind,
    /// The text representation of the failure.
    pub message: String,
}

impl ClientError {
    pub(crate) fn new(kind: ErrorKind, message: String) -> Self {
        Self { message, kind }
    }
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for ClientError {}
