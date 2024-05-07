use std::time::Duration;

/// Describes the available polling modes.
pub enum PollingMode {
    /// Specifies how frequently the locally cached config will be refreshed by fetching the latest version from the remote server.
    AutoPoll(Duration),
    /// Specifies how long the locally cached config can be used before refreshing it again by fetching the latest version from the remote server.
    LazyLoad(Duration),
    /// In this polling mode the SDK will refresh only when [`crate::Client::refresh`] is called.
    Manual,
}

impl PollingMode {
    pub fn mode_identifier(&self) -> &str {
        match self {
            PollingMode::AutoPoll(_) => "a",
            PollingMode::LazyLoad(_) => "l",
            PollingMode::Manual => "m",
        }
    }
}
