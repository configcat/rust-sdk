use crate::builder::{ClientBuilder, Options};
use crate::errors::ErrorKind;
use crate::eval::details::EvaluationDetails;
use crate::eval::evaluator::{eval, EvalResult};
use crate::fetch::service::ConfigService;
use crate::r#override::OptionalOverrides;
use crate::value::{OptionalValueDisplay, Value};
use crate::{ClientError, Setting, User};
use log::{error, warn};
use std::collections::HashMap;
use std::sync::Arc;

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
///     let client = Client::builder("SDK_KEY")
///         .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
///         .build()
///         .unwrap();
///
///     let user = User::new("user-id");
///     let is_flag_enabled = client.get_bool_value("flag-key", Some(user), false).await;
/// }
/// ```
pub struct Client {
    options: Arc<Options>,
    service: ConfigService,
}

impl Client {
    pub(crate) fn with_options(options: Options) -> Self {
        let opts = Arc::new(options);
        Self {
            options: Arc::clone(&opts),
            service: ConfigService::new(Arc::clone(&opts)),
        }
    }

    /// Create a new [`ClientBuilder`] used to build a [`Client`].
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
    /// let client = Client::builder("SDK_KEY")
    ///     .polling_mode(PollingMode::AutoPoll(Duration::from_secs(60)))
    ///     .data_governance(DataGovernance::EU)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder(sdk_key: &str) -> ClientBuilder {
        ClientBuilder::new(sdk_key)
    }

    /// Create a new [`Client`] with default options.
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
    /// let client = Client::new("SDK_KEY").unwrap();
    /// ```
    pub fn new(sdk_key: &str) -> Result<Self, ClientError> {
        ClientBuilder::new(sdk_key).build()
    }

    /// Initiate a force refresh on the cached config JSON data.
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
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     _ = client.refresh().await
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

    /// Evaluate a bool flag identified by the given `key`.
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
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let value = client.get_bool_value("flag-key", Some(user), false).await;
    /// }
    /// ```
    pub async fn get_bool_value(&self, key: &str, user: Option<User>, default: bool) -> bool {
        self.get_bool_details(key, user, default).await.value
    }

    /// Evaluate a whole number setting identified by the given `key`.
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
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let value = client.get_int_value("flag-key", Some(user), 0).await;
    /// }
    /// ```
    pub async fn get_int_value(&self, key: &str, user: Option<User>, default: i64) -> i64 {
        self.get_int_details(key, user, default).await.value
    }

    /// Evaluate a decimal number setting identified by the given `key`.
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
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let value = client.get_float_value("flag-key", Some(user), 0.0).await;
    /// }
    /// ```
    pub async fn get_float_value(&self, key: &str, user: Option<User>, default: f64) -> f64 {
        self.get_float_details(key, user, default).await.value
    }

    /// Evaluate a string setting identified by the given `key`.
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
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let value = client.get_str_value("flag-key", Some(user), String::default()).await;
    /// }
    /// ```
    pub async fn get_str_value(&self, key: &str, user: Option<User>, default: String) -> String {
        self.get_str_details(key, user, default).await.value
    }

    /// The same as [`Client::get_bool_value`] but returns an [`EvaluationDetails`] which
    /// contains additional information about the evaluation process's result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let details = client.get_bool_details("flag-key", Some(user), false).await;
    /// }
    /// ```
    pub async fn get_bool_details(
        &self,
        key: &str,
        user: Option<User>,
        default: bool,
    ) -> EvaluationDetails<bool> {
        let result = self.service.config().await;
        match self
            .eval_flag(&result.config().settings, key, &user, Some(default.into()))
            .await
        {
            Ok(eval_result) => match eval_result.value.as_bool() {
                Some(val) => EvaluationDetails {
                    value: val,
                    key: key.to_owned(),
                    user,
                    fetch_time: Some(*result.fetch_time()),
                    ..eval_result.into()
                },
                None => {
                    let err = ClientError::new(ErrorKind::SettingValueTypeMismatch, format!("The type of a setting must match the requested type. Setting's type was '{}' but the requested type was 'bool'. Learn more: https://configcat.com/docs/sdk-reference/rust/#setting-type-mapping", eval_result.setting_type));
                    error!(event_id = err.kind.as_u8(); "{}", err);
                    EvaluationDetails::from_err(default, key, user, err)
                }
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(default, key, user, err)
            }
        }
    }

    /// The same as [`Client::get_int_value`] but returns an [`EvaluationDetails`] which
    /// contains additional information about the evaluation process's result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let details = client.get_int_details("flag-key", Some(user), 0).await;
    /// }
    /// ```
    pub async fn get_int_details(
        &self,
        key: &str,
        user: Option<User>,
        default: i64,
    ) -> EvaluationDetails<i64> {
        let result = self.service.config().await;
        match self
            .eval_flag(&result.config().settings, key, &user, Some(default.into()))
            .await
        {
            Ok(eval_result) => match eval_result.value.as_int() {
                Some(val) => EvaluationDetails {
                    value: val,
                    key: key.to_owned(),
                    user,
                    fetch_time: Some(*result.fetch_time()),
                    ..eval_result.into()
                },
                None => {
                    let err = ClientError::new(ErrorKind::SettingValueTypeMismatch, format!("The type of a setting must match the requested type. Setting's type was '{}' but the requested type was 'i64'. Learn more: https://configcat.com/docs/sdk-reference/rust/#setting-type-mapping", eval_result.setting_type));
                    error!(event_id = err.kind.as_u8(); "{}", err);
                    EvaluationDetails::from_err(default, key, user, err)
                }
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(default, key, user, err)
            }
        }
    }

    /// The same as [`Client::get_float_value`] but returns an [`EvaluationDetails`] which
    /// contains additional information about the evaluation process's result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let details = client.get_float_details("flag-key", Some(user), 0.0).await;
    /// }
    /// ```
    pub async fn get_float_details(
        &self,
        key: &str,
        user: Option<User>,
        default: f64,
    ) -> EvaluationDetails<f64> {
        let result = self.service.config().await;
        match self
            .eval_flag(&result.config().settings, key, &user, Some(default.into()))
            .await
        {
            Ok(eval_result) => match eval_result.value.as_float() {
                Some(val) => EvaluationDetails {
                    value: val,
                    key: key.to_owned(),
                    user,
                    fetch_time: Some(*result.fetch_time()),
                    ..eval_result.into()
                },
                None => {
                    let err = ClientError::new(ErrorKind::SettingValueTypeMismatch, format!("The type of a setting must match the requested type. Setting's type was '{}' but the requested type was 'f64'. Learn more: https://configcat.com/docs/sdk-reference/rust/#setting-type-mapping", eval_result.setting_type));
                    error!(event_id = err.kind.as_u8(); "{}", err);
                    EvaluationDetails::from_err(default, key, user, err)
                }
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(default, key, user, err)
            }
        }
    }

    /// The same as [`Client::get_str_value`] but returns an [`EvaluationDetails`] which
    /// contains additional information about the evaluation process's result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{Client, User};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let details = client.get_str_details("flag-key", Some(user), String::default()).await;
    /// }
    /// ```
    pub async fn get_str_details(
        &self,
        key: &str,
        user: Option<User>,
        default: String,
    ) -> EvaluationDetails<String> {
        let result = self.service.config().await;
        match self
            .eval_flag(
                &result.config().settings,
                key,
                &user,
                Some(default.clone().into()),
            )
            .await
        {
            Ok(eval_result) => match eval_result.value.as_str() {
                Some(val) => EvaluationDetails {
                    value: val,
                    key: key.to_owned(),
                    user,
                    fetch_time: Some(*result.fetch_time()),
                    ..eval_result.into()
                },
                None => {
                    let err = ClientError::new(ErrorKind::SettingValueTypeMismatch, format!("The type of a setting must match the requested type. Setting's type was '{}' but the requested type was 'String'. Learn more: https://configcat.com/docs/sdk-reference/rust/#setting-type-mapping", eval_result.setting_type));
                    error!(event_id = err.kind.as_u8(); "{}", err);
                    EvaluationDetails::from_err(default, key, user, err)
                }
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(default, key, user, err)
            }
        }
    }

    /// Evaluate a feature flag identified by the given `key`.
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
    ///     let client = Client::new("SDK_KEY").unwrap();
    ///
    ///     let user = User::new("user-id");
    ///     let value = client.get_flag_details("flag-key", Some(user)).await;
    /// }
    /// ```
    pub async fn get_flag_details(
        &self,
        key: &str,
        user: Option<User>,
    ) -> EvaluationDetails<Option<Value>> {
        let result = self.service.config().await;
        match self
            .eval_flag(&result.config().settings, key, &user, None)
            .await
        {
            Ok(eval_result) => EvaluationDetails {
                value: Some(eval_result.value),
                key: key.to_owned(),
                user,
                fetch_time: Some(*result.fetch_time()),
                is_default_value: false,
                variation_id: eval_result.variation_id,
                matched_targeting_rule: eval_result.rule,
                matched_percentage_option: eval_result.option,
                error: None,
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(None, key, user, err)
            }
        }
    }

    async fn eval_flag(
        &self,
        settings: &HashMap<String, Setting>,
        key: &str,
        user: &Option<User>,
        default: Option<Value>,
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
                let eval_result = eval(setting, key, user, settings, &default);
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
}
