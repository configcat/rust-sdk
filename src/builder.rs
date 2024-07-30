use crate::cache::EmptyConfigCache;
use crate::constants::{SDK_KEY_PREFIX, SDK_KEY_PROXY_PREFIX, SDK_KEY_SECTION_LENGTH};
use crate::errors::{ClientError, ErrorKind};
use crate::model::enums::DataGovernance;
use crate::modes::PollingMode;
use crate::r#override::{FlagOverrides, OptionalOverrides};
use crate::{Client, ConfigCache, OverrideBehavior, OverrideDataSource, User};
use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::time::Duration;

pub struct Options {
    sdk_key: String,
    offline: bool,
    base_url: Option<String>,
    data_governance: DataGovernance,
    http_timeout: Duration,
    cache: Box<dyn ConfigCache>,
    overrides: Option<FlagOverrides>,
    polling_mode: PollingMode,
    default_user: Option<User>,
}

impl Options {
    pub(crate) fn sdk_key(&self) -> &str {
        &self.sdk_key
    }

    pub(crate) fn offline(&self) -> bool {
        self.offline
    }

    pub(crate) fn base_url(&self) -> &Option<String> {
        &self.base_url
    }

    pub(crate) fn data_governance(&self) -> &DataGovernance {
        &self.data_governance
    }

    pub(crate) fn http_timeout(&self) -> &Duration {
        &self.http_timeout
    }

    pub(crate) fn cache(&self) -> &dyn ConfigCache {
        self.cache.borrow()
    }

    pub(crate) fn polling_mode(&self) -> &PollingMode {
        &self.polling_mode
    }

    pub(crate) fn overrides(&self) -> &Option<FlagOverrides> {
        &self.overrides
    }

    pub(crate) fn default_user(&self) -> &Option<User> {
        &self.default_user
    }
}

impl Debug for Options {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("sdk_key", &self.sdk_key)
            .field("offline", &self.offline)
            .field("base_url", &self.base_url)
            .field("data_governance", &self.data_governance)
            .field("http_timeout", &self.http_timeout)
            .field("overrides", &self.overrides)
            .field("polling_mode", &self.polling_mode)
            .field("default_user", &self.default_user)
            .finish_non_exhaustive()
    }
}

/// Builder to create ConfigCat [`Client`].
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
/// use configcat::{DataGovernance, Client, PollingMode};
///
/// let builder = Client::builder("sdk-key")
///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
///     .data_governance(DataGovernance::EU);
///
/// let client = builder.build().unwrap();
/// ```
pub struct ClientBuilder {
    sdk_key: String,
    base_url: Option<String>,
    data_governance: Option<DataGovernance>,
    http_timeout: Option<Duration>,
    cache: Option<Box<dyn ConfigCache>>,
    overrides: Option<FlagOverrides>,
    offline: bool,
    polling_mode: Option<PollingMode>,
    default_user: Option<User>,
}

impl ClientBuilder {
    pub(crate) fn new(sdk_key: &str) -> Self {
        Self {
            sdk_key: sdk_key.to_owned(),
            offline: false,
            http_timeout: None,
            base_url: None,
            cache: None,
            polling_mode: None,
            data_governance: None,
            overrides: None,
            default_user: None,
        }
    }

    /// Indicates whether the SDK should be initialized in offline mode or not.
    /// Default value is `false`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::Client;
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .offline(true);
    /// ```
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// Sets the HTTP request timeout.
    /// Default value is `30` seconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use configcat::Client;
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .http_timeout(Duration::from_secs(60));
    /// ```
    pub fn http_timeout(mut self, timeout: Duration) -> Self {
        self.http_timeout = Some(timeout);
        self
    }

    /// Sets a custom base URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::Client;
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .base_url("https://custom-cdn-url.com");
    /// ```
    pub fn base_url(mut self, base_url: &str) -> Self {
        self.base_url = Some(base_url.to_owned());
        self
    }

    /// Sets the [`DataGovernance`] option.
    /// Default value is [`DataGovernance::Global`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::{DataGovernance, Client};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .data_governance(DataGovernance::EU);
    /// ```
    pub fn data_governance(mut self, data_governance: DataGovernance) -> Self {
        self.data_governance = Some(data_governance);
        self
    }

    /// Sets a [`ConfigCache`] implementation used for caching.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::{ConfigCache, Client};
    ///
    /// let builder = Client::builder("sdk-key")
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

    /// Sets the [`PollingMode`] of the SDK.
    /// Default value is [`PollingMode::AutoPoll`] with `60` seconds poll interval.
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
    pub fn polling_mode(mut self, polling_mode: PollingMode) -> Self {
        self.polling_mode = Some(polling_mode);
        self
    }

    /// Sets the default user, used as fallback when there's no user parameter is passed to the flag evaluation methods.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::{Client, User};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .default_user(User::new("USER_IDENTIFIER"));
    /// ```
    pub fn default_user(mut self, user: User) -> Self {
        self.default_user = Some(user);
        self
    }

    /// Sets feature flag and setting overrides for the SDK.
    ///
    /// With overrides, you can overwrite feature flag and setting values
    /// downloaded from the ConfigCat CDN with local values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use std::time::Duration;
    /// use configcat::{Client, MapDataSource, OverrideBehavior, PollingMode, Value};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .overrides(Box::new(MapDataSource::from([
    ///         ("flag", Value::Bool(true))
    ///     ])), OverrideBehavior::LocalOnly);
    /// ```
    pub fn overrides(
        mut self,
        source: Box<dyn OverrideDataSource>,
        behavior: OverrideBehavior,
    ) -> Self {
        self.overrides = Some(FlagOverrides::new(source, behavior));
        self
    }

    /// Creates a [`Client`] from the configuration made on the builder.
    ///
    /// # Errors
    ///
    /// This method fails in the following cases:
    /// - The given SDK key is empty or has an invalid format.
    /// - The initialization of the internal [`reqwest::Client`] failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use configcat::{DataGovernance, Client, PollingMode};
    ///
    /// let builder = Client::builder("sdk-key")
    ///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
    ///     .data_governance(DataGovernance::EU);
    ///
    /// let client = builder.build().unwrap();
    /// ```
    pub fn build(self) -> Result<Client, ClientError> {
        if self.sdk_key.is_empty() {
            return Err(ClientError::new(
                ErrorKind::InvalidSdkKey,
                "SDK Key cannot be empty".to_owned(),
            ));
        }
        if !self.overrides.is_local()
            && !is_sdk_key_valid(self.sdk_key.as_str(), self.base_url.is_some())
        {
            return Err(ClientError::new(
                ErrorKind::InvalidSdkKey,
                format!("SDK Key '{}' is invalid.", self.sdk_key),
            ));
        }
        Client::with_options(self.build_options())
    }

    pub(crate) fn build_options(self) -> Options {
        Options {
            sdk_key: self.sdk_key,
            offline: self.offline,
            cache: self.cache.unwrap_or(Box::new(EmptyConfigCache::new())),
            polling_mode: self
                .polling_mode
                .unwrap_or(PollingMode::AutoPoll(Duration::from_secs(60))),
            base_url: self.base_url,
            data_governance: self.data_governance.unwrap_or(DataGovernance::Global),
            http_timeout: self.http_timeout.unwrap_or(Duration::from_secs(30)),
            overrides: self.overrides,
            default_user: self.default_user,
        }
    }
}

fn is_sdk_key_valid(sdk_key: &str, is_custom_url: bool) -> bool {
    if is_custom_url
        && sdk_key.len() > SDK_KEY_PROXY_PREFIX.len()
        && sdk_key.starts_with(SDK_KEY_PROXY_PREFIX)
    {
        return true;
    }
    let comps: Vec<&str> = sdk_key.split('/').collect();
    match comps.len() {
        2 => comps[0].len() == SDK_KEY_SECTION_LENGTH && comps[1].len() == SDK_KEY_SECTION_LENGTH,
        3 => {
            comps[0] == SDK_KEY_PREFIX
                && comps[1].len() == SDK_KEY_SECTION_LENGTH
                && comps[2].len() == SDK_KEY_SECTION_LENGTH
        }
        _ => false,
    }
}
