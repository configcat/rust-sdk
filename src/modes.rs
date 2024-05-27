use std::time::Duration;

/// Describes the available polling modes.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use configcat::PollingMode;
///
/// let auto_poll = PollingMode::AutoPoll(Duration::from_secs(60));
/// let lazy_load = PollingMode::LazyLoad(Duration::from_secs(60));
/// let manual = PollingMode::Manual;
/// ```
pub enum PollingMode {
    /// Specifies how frequently the locally cached config will be refreshed by fetching the latest version from the remote server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use configcat::{Client, PollingMode};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)));
    /// ```
    AutoPoll(Duration),
    /// Specifies how long the locally cached config can be used before refreshing it again by fetching the latest version from the remote server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use configcat::{Client, PollingMode};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .polling_mode(PollingMode::LazyLoad(Duration::from_secs(60)));
    /// ```
    LazyLoad(Duration),
    /// In this polling mode the SDK will refresh only when [`crate::Client::refresh`] is called.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use configcat::{Client, PollingMode};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .polling_mode(PollingMode::Manual);
    /// ```
    Manual,
}

impl PollingMode {
    pub(crate) fn mode_identifier(&self) -> &str {
        match self {
            PollingMode::AutoPoll(_) => "a",
            PollingMode::LazyLoad(_) => "l",
            PollingMode::Manual => "m",
        }
    }
}
