use crate::cache::EmptyConfigCache;
use crate::errors::ClientError;
use crate::model::enums::DataGovernance;
use crate::modes::PollingMode;
use crate::ConfigCache;
use std::borrow::Borrow;
use std::time::Duration;

/// Configuration options for the ConfigCat [`crate::Client`].
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use configcat::{DataGovernance, OptionsBuilder, PollingMode};
///
/// let builder = OptionsBuilder::new("SDK_KEY")
///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
///     .data_governance(DataGovernance::EU);
///
/// let options = builder.build().unwrap();
/// ```
pub struct Options {
    sdk_key: String,
    offline: bool,
    base_url: Option<String>,
    data_governance: DataGovernance,
    http_timeout: Duration,
    cache: Box<dyn ConfigCache>,
    polling_mode: PollingMode,
}

impl Options {
    /// Get the SDK key.
    pub fn sdk_key(&self) -> &str {
        &self.sdk_key
    }

    /// True when the SDK is in offline mode, otherwise false.
    pub fn offline(&self) -> bool {
        self.offline
    }

    /// Get the configured base URL.
    pub fn base_url(&self) -> &Option<String> {
        &self.base_url
    }

    /// Get the configured [`DataGovernance`] option.
    pub fn data_governance(&self) -> &DataGovernance {
        &self.data_governance
    }

    /// Get the configured HTTP request timeout.
    pub fn http_timeout(&self) -> &Duration {
        &self.http_timeout
    }

    /// Get the configured [`ConfigCache`] implementation.
    pub fn cache(&self) -> &dyn ConfigCache {
        self.cache.borrow()
    }

    /// Get the configured [`PollingMode`].
    pub fn polling_mode(&self) -> &PollingMode {
        &self.polling_mode
    }
}

/// Builder to create [`Options`] used by the ConfigCat [`crate::Client`].
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use configcat::{DataGovernance, OptionsBuilder, PollingMode};
///
/// let builder = OptionsBuilder::new("SDK_KEY")
///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
///     .data_governance(DataGovernance::EU);
///
/// let options = builder.build().unwrap();
/// ```
pub struct OptionsBuilder {
    sdk_key: String,
    base_url: Option<String>,
    data_governance: Option<DataGovernance>,
    http_timeout: Option<Duration>,
    cache: Option<Box<dyn ConfigCache>>,
    offline: bool,
    polling_mode: Option<PollingMode>,
}

impl OptionsBuilder {
    /// Create a new [`OptionsBuilder`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::OptionsBuilder;
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY");
    /// ```
    pub fn new(sdk_key: &str) -> Self {
        Self {
            sdk_key: sdk_key.to_owned(),
            offline: false,
            http_timeout: None,
            base_url: None,
            cache: None,
            polling_mode: None,
            data_governance: None,
        }
    }

    /// Indicate whether the SDK should be initialized in offline mode or not.
    /// Default value is `false`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::OptionsBuilder;
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY")
    ///     .offline(true);
    /// ```
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// Set the HTTP request timeout.
    /// Default value is `30` seconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use configcat::OptionsBuilder;
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY")
    ///     .http_timeout(Duration::from_secs(60));
    /// ```
    pub fn http_timeout(mut self, timeout: Duration) -> Self {
        self.http_timeout = Some(timeout);
        self
    }

    /// Set a custom base URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::OptionsBuilder;
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY")
    ///     .base_url("https://custom-cdn-url.com");
    /// ```
    pub fn base_url(mut self, base_url: &str) -> Self {
        self.base_url = Some(base_url.to_owned());
        self
    }

    /// Set the [`DataGovernance`] option.
    /// Default value is [`DataGovernance::Global`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::{DataGovernance, OptionsBuilder};
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY")
    ///     .data_governance(DataGovernance::EU);
    /// ```
    pub fn data_governance(mut self, data_governance: DataGovernance) -> Self {
        self.data_governance = Some(data_governance);
        self
    }

    /// Set a [`ConfigCache`] implementation used for caching.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::{ConfigCache, OptionsBuilder};
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY")
    ///     .cache(Box::new(CustomCache{}));
    ///
    /// struct CustomCache {}
    ///
    /// impl ConfigCache for CustomCache {
    ///     fn read(&self, key: &str) -> Option<String> {
    ///         // read from cache
    ///         Some("from-cache".to_owned())
    ///     }
    ///
    ///     fn write(&self, key: &str, value: &str) {
    ///         // write to cache
    ///     }
    /// }
    /// ```
    pub fn cache(mut self, cache: Box<dyn ConfigCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set the [`PollingMode`] of the SDK.
    /// Default value is [`PollingMode::AutoPoll`] with `60` seconds poll interval.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use configcat::{OptionsBuilder, PollingMode};
    ///
    /// let builder = OptionsBuilder::new("SDK_KEY")
    ///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)));
    /// ```
    pub fn polling_mode(mut self, polling_mode: PollingMode) -> Self {
        self.polling_mode = Some(polling_mode);
        self
    }

    /// Create the [`Options`] from the configuration made on the builder.
    ///
    /// # Errors
    ///
    /// This method fails if the given SDK key has an invalid format.
    pub fn build(self) -> Result<Options, ClientError> {
        if self.sdk_key.is_empty() {
            return Err(ClientError::InvalidSdkKey(
                "SDK Key cannot be empty".to_owned(),
            ));
        }
        Ok(Options {
            sdk_key: self.sdk_key.clone(),
            offline: self.offline,
            cache: self.cache.unwrap_or(Box::new(EmptyConfigCache::new())),
            polling_mode: self
                .polling_mode
                .unwrap_or(PollingMode::AutoPoll(Duration::from_secs(60))),
            base_url: self.base_url,
            data_governance: self.data_governance.unwrap_or(DataGovernance::Global),
            http_timeout: self.http_timeout.unwrap_or(Duration::from_secs(30)),
        })
    }
}
