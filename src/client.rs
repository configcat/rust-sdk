use crate::builder::{ClientBuilder, Options};
use crate::errors::ErrorKind;
use crate::eval::details::EvaluationDetails;
use crate::eval::evaluator::{eval, EvalResult};
use crate::fetch::service::ConfigService;
use crate::r#override::OptionalOverrides;
use crate::value::{OptionalValueDisplay, Value, ValuePrimitive};
use crate::{ClientCacheState, ClientError, Setting, User};
use log::{error, warn};
use std::any::type_name;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

/// The main component for evaluating feature flags and settings.
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
/// use configcat::{Client, PollingMode, User};
///
/// #[tokio::main]
/// async fn main() {
///     let client = Client::builder("sdk-key")
///         .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
///         .build()
///         .unwrap();
///
///     let user = User::new("user-id");
///     let is_flag_enabled = client.get_value("flag-key", false, Some(user)).await;
/// }
/// ```
pub struct Client {
    options: Arc<Options>,
    service: ConfigService,
    default_user: Arc<Mutex<Option<User>>>,
}

impl Client {
    pub(crate) fn with_options(options: Options) -> Result<Self, ClientError> {
        let opts = Arc::new(options);
        match ConfigService::new(Arc::clone(&opts)) {
            Ok(service) => Ok(Self {
                options: Arc::clone(&opts),
                service,
                default_user: Arc::new(Mutex::new(opts.default_user().cloned())),
            }),
            Err(err) => Err(err),
        }
    }

    /// Creates a new [`ClientBuilder`] used to build a [`Client`].
    ///
    /// # Errors
    ///
    /// This method fails if the given SDK key is empty or has an invalid format.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use configcat::{DataGovernance, Client, PollingMode};
    ///
    /// let client = Client::builder("sdk-key")
    ///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
    ///     .data_governance(DataGovernance::EU)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder(sdk_key: &str) -> ClientBuilder {
        ClientBuilder::new(sdk_key)
    }

    /// Creates a new [`Client`] with default options.
    ///
    /// # Errors
    ///
    /// This method fails if the given SDK key is empty or has an invalid format.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::Client;
    ///
    /// let client = Client::new("sdk-key").unwrap();
    /// ```
    pub fn new(sdk_key: &str) -> Result<Self, ClientError> {
        ClientBuilder::new(sdk_key).build()
    }

    /// Initiates a force refresh on the cached config JSON data.
    ///
    /// # Errors
    ///
    /// This method fails in the following cases:
    /// - The SDK is in offline mode.
    /// - The SDK has a [`crate::OverrideBehavior::LocalOnly`] override set.
    /// - The HTTP request that supposed to download the new config JSON fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let result = client.refresh().await.unwrap();
    /// }
    /// ```
    pub async fn refresh(&self) -> Result<(), ClientError> {
        if self.options.offline() {
            let err = ClientError::new(
                ErrorKind::OfflineClient,
                "Client is in offline mode, it cannot initiate HTTP calls.".to_owned(),
            );
            warn!(event_id = err.kind.as_u8(); "{}", err);
            return Err(err);
        }
        if self.options.overrides().is_local() {
            let err = ClientError::new(
                ErrorKind::LocalOnlyClient,
                "Client has local-only overrides, it cannot initiate HTTP calls.".to_owned(),
            );
            warn!(event_id = err.kind.as_u8(); "{}", err);
            return Err(err);
        }
        self.service.refresh().await
    }

    /// Evaluates a feature flag or setting identified by the given `key`.
    ///
    /// Returns `default` if the flag doesn't exist, or there was an error during the evaluation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let value = client.get_value("flag-key", false, Some(user)).await;
    /// }
    /// ```
    pub async fn get_value<T: ValuePrimitive + Clone + Default>(
        &self,
        key: &str,
        default: T,
        user: Option<User>,
    ) -> T {
        self.get_value_details(key, default, user).await.value
    }

    /// The same as [`Client::get_value`] but returns an [`EvaluationDetails`] that
    /// contains additional information about the result of the evaluation process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let details = client.get_value_details("flag-key", String::default(), Some(user)).await;
    /// }
    /// ```
    pub async fn get_value_details<T: ValuePrimitive + Clone + Default>(
        &self,
        key: &str,
        default: T,
        user: Option<User>,
    ) -> EvaluationDetails<T> {
        let result = self.service.config().await;
        let mut eval_user = user;
        if eval_user.is_none() {
            eval_user = self.read_def_user();
        }
        match eval_flag(
            &result.config().settings,
            key,
            eval_user.as_ref(),
            Some(&default.clone().into()),
        ) {
            Ok(eval_result) => {
                if let Some(val) = T::from_value(&eval_result.value) {
                    EvaluationDetails {
                        value: val,
                        key: key.to_owned(),
                        user: eval_user,
                        fetch_time: Some(*result.fetch_time()),
                        ..eval_result.into()
                    }
                } else {
                    let err = ClientError::new(ErrorKind::SettingValueTypeMismatch, format!("The type of a setting must match the requested type. Setting's type was '{}' but the requested type was '{}'. Learn more: https://configcat.com/docs/sdk-reference/rust/#setting-type-mapping", eval_result.setting_type, type_name::<T>()));
                    error!(event_id = err.kind.as_u8(); "{}", err);
                    EvaluationDetails::from_err(default, key, eval_user, err)
                }
            }
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(default, key, eval_user, err)
            }
        }
    }

    /// Evaluates a feature flag identified by the given `key`.
    ///
    /// Returns an [`EvaluationDetails`] that contains the evaluated feature flag's value in a [`Value`] variant.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let details = client.get_flag_details("flag-key", Some(user)).await;
    /// }
    /// ```
    pub async fn get_flag_details(
        &self,
        key: &str,
        user: Option<User>,
    ) -> EvaluationDetails<Option<Value>> {
        let result = self.service.config().await;
        let mut eval_user = user;
        if eval_user.is_none() {
            eval_user = self.read_def_user();
        }
        match eval_flag(&result.config().settings, key, eval_user.as_ref(), None) {
            Ok(eval_result) => EvaluationDetails {
                value: Some(eval_result.value),
                key: key.to_owned(),
                user: eval_user,
                fetch_time: Some(*result.fetch_time()),
                is_default_value: false,
                variation_id: eval_result.variation_id,
                matched_targeting_rule: eval_result.rule,
                matched_percentage_option: eval_result.option,
                error: None,
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(None, key, eval_user, err)
            }
        }
    }

    /// Evaluates all feature flags and settings.
    ///
    /// Returns a [`HashMap`] of [`String`] keys and evaluated [`Value`]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let values = client.get_all_values(Some(user)).await;
    /// }
    /// ```
    pub async fn get_all_values(&self, user: Option<User>) -> HashMap<String, Value> {
        let details = self.get_all_value_details(user).await;
        let mut result = HashMap::<String, Value>::with_capacity(details.len());
        for detail in details {
            if let Some(val) = detail.value {
                result.insert(detail.key, val);
            }
        }
        result
    }

    /// The same as [`Client::get_all_values`] but returns a [`Vec`] of [`EvaluationDetails`] that
    /// contains additional information about each evaluation process and the evaluated
    /// feature flag values in [`Value`] variants.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let all_details = client.get_all_value_details(Some(user)).await;
    /// }
    /// ```
    pub async fn get_all_value_details(
        &self,
        user: Option<User>,
    ) -> Vec<EvaluationDetails<Option<Value>>> {
        let config_result = self.service.config().await;
        let mut eval_user = user;
        if eval_user.is_none() {
            eval_user = self.read_def_user();
        }
        let settings = &config_result.config().settings;
        let mut result = Vec::<EvaluationDetails<Option<Value>>>::with_capacity(settings.len());
        for k in settings.keys() {
            let usr_clone = eval_user.clone();
            let details = match eval_flag(settings, k, usr_clone.as_ref(), None) {
                Ok(eval_result) => EvaluationDetails {
                    value: Some(eval_result.value),
                    key: k.to_owned(),
                    user: usr_clone,
                    fetch_time: Some(*config_result.fetch_time()),
                    variation_id: eval_result.variation_id,
                    matched_targeting_rule: eval_result.rule,
                    matched_percentage_option: eval_result.option,
                    ..EvaluationDetails::default()
                },
                Err(err) => {
                    error!(event_id = err.kind.as_u8(); "{}", err);
                    EvaluationDetails::from_err(None, k, usr_clone, err)
                }
            };
            result.push(details);
        }
        result
    }

    /// Returns the keys of all feature flags and settings.
    ///
    /// If there's no config JSON to work on, this method returns an empty [`Vec`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let keys = client.get_all_keys().await;
    /// }
    /// ```
    pub async fn get_all_keys(&self) -> Vec<String> {
        let config_result = self.service.config().await;
        let settings = &config_result.config().settings;
        if !settings.is_empty() {
            return settings.keys().cloned().collect();
        }
        error!(event_id = 1000; "Config JSON is not present. Returning empty vector.");
        vec![]
    }

    /// Puts the [`Client`] into offline mode.
    ///
    /// In this mode the SDK is not allowed to initiate HTTP request and works only from the configured cache.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     client.offline();
    /// }
    /// ```
    pub fn offline(&self) {
        self.service.set_mode(true);
    }

    /// Puts the [`Client`] into online mode.
    ///
    /// In this mode the SDK initiates HTTP requests to fetch the latest config JSON data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("sdk-key").unwrap();
    ///
    ///     client.online();
    /// }
    /// ```
    pub fn online(&self) {
        self.service.set_mode(false);
    }

    /// Returns `true` when the SDK is configured not to initiate HTTP requests, otherwise `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::builder("sdk-key")
    ///         .offline(true)
    ///         .build()
    ///         .unwrap();
    ///
    ///     let offline = client.is_offline();
    /// }
    /// ```
    pub fn is_offline(&self) -> bool {
        self.service.is_offline()
    }

    /// Sets the default user.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("sdk-key").unwrap();
    ///
    ///     client.set_default_user(User::new("user-id"));
    /// }
    /// ```
    pub fn set_default_user(&mut self, user: User) {
        self.set_def_user(Some(user));
    }

    /// Clears the default user.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::builder("sdk-key")
    ///         .default_user(User::new("user-id"))
    ///         .build()
    ///         .unwrap();
    ///
    ///     client.clear_default_user();
    /// }
    /// ```
    pub fn clear_default_user(&mut self) {
        self.set_def_user(None);
    }

    /// Asynchronously waits for the initialization of the [`Client`] for a maximum duration specified in `wait_timeout`.
    ///
    /// # Errors
    ///
    /// This method fails if the initialization takes more time than the specified `wait_timeout`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, ClientCacheState};
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///
    ///     let client = Client::new("sdk-key").unwrap();
    ///     let state = client.wait_for_ready(Duration::from_secs(5)).await.unwrap();
    ///
    ///     assert!(matches!(state, ClientCacheState::HasUpToDateFlagData));
    /// }
    /// ```
    pub async fn wait_for_ready(
        &self,
        wait_timeout: Duration,
    ) -> Result<ClientCacheState, ClientError> {
        let init = timeout(wait_timeout, self.service.wait_for_init()).await;
        if let Ok(state) = init {
            Ok(state)
        } else {
            let err = ClientError::new(
                ErrorKind::ClientInitTimedOut,
                format!(
                    "Client initialization timed out after {}s.",
                    wait_timeout.as_secs()
                ),
            );
            warn!(event_id = err.kind.as_u8(); "{}", err);
            Err(err)
        }
    }

    fn read_def_user(&self) -> Option<User> {
        let user = self.default_user.lock().unwrap();
        user.clone()
    }

    fn set_def_user(&self, user: Option<User>) {
        let mut def_user = self.default_user.lock().unwrap();
        *def_user = user;
    }
}

fn eval_flag(
    settings: &HashMap<String, Setting>,
    key: &str,
    user: Option<&User>,
    default: Option<&Value>,
) -> Result<EvalResult, ClientError> {
    if settings.is_empty() {
        return Err(ClientError::new(ErrorKind::ConfigJsonNotAvailable, format!("Config JSON is not present when evaluating setting '{key}'. Returning the `defaultValue` parameter that you specified in your application: '{}'.", default.to_str())));
    }
    match settings.get(key) {
        None => {
            let keys = settings
                .keys()
                .map(|k| format!("'{k}'"))
                .collect::<Vec<String>>()
                .join(", ");
            Err(ClientError::new(ErrorKind::SettingKeyMissing, format!("Failed to evaluate setting '{key}' (the key was not found in config JSON). Returning the `defaultValue` parameter that you specified in your application: '{}'. Available keys: [{keys}].", default.to_str())))
        }
        Some(setting) => {
            let eval_result = eval(setting, key, user, settings, default);
            match eval_result {
                Ok(result) => Ok(result),
                Err(err) => Err(ClientError::new(
                    ErrorKind::EvaluationFailure,
                    format!("Failed to evaluate setting '{key}' ({err})"),
                )),
            }
        }
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("options", &self.options)
            .field("default_user", &self.default_user)
            .finish_non_exhaustive()
    }
}
