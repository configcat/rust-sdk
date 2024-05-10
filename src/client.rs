use crate::errors::ErrorKind;
use crate::eval::details::EvaluationDetails;
use crate::eval::evaluator::{eval, EvalResult};
use crate::fetch::service::{ConfigResult, ConfigService};
use crate::options::{Options, OptionsBuilder};
use crate::value::{OptionalValueDisplay, Value};
use crate::{ClientError, User};
use log::{error, warn};
use std::sync::Arc;

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

    /// Create a new [`OptionsBuilder`] used to build a [`Client`].
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
    pub fn builder(sdk_key: &str) -> OptionsBuilder {
        OptionsBuilder::new(sdk_key)
    }

    /// Create a new [`Client`] with the default [`Options`].
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
        OptionsBuilder::new(sdk_key).build()
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
        self.service.refresh().await
    }

    /// Evaluate a feature flag identified by the given `key`.
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

    /// The same as [`Client::get_bool_value`] but returns an [`EvaluationDetails`] which
    /// contains additional information about the evaluation process.
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
            .eval_flag(&result, key, &user, Some(default.into()))
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
        match self.eval_flag(&result, key, &user, None).await {
            Ok(eval_result) => EvaluationDetails {
                value: Some(eval_result.value),
                key: key.to_owned(),
                is_default_value: false,
                variation_id: eval_result.variation_id,
                user,
                error: None,
                fetch_time: Some(*result.fetch_time()),
                matched_targeting_rule: eval_result.rule,
                matched_percentage_option: eval_result.option,
            },
            Err(err) => {
                error!(event_id = err.kind.as_u8(); "{}", err);
                EvaluationDetails::from_err(None, key, user, err)
            }
        }
    }

    async fn eval_flag(
        &self,
        config_result: &ConfigResult,
        key: &str,
        user: &Option<User>,
        default: Option<Value>,
    ) -> Result<EvalResult, ClientError> {
        if config_result.config().settings.is_empty() {
            return Err(ClientError::new(ErrorKind::ConfigJsonNotAvailable, format!("Config JSON is not present when evaluating setting '{key}'. Returning the `defaultValue` parameter that you specified in your application: '{}'.", default.to_str())));
        }

        match config_result.config().settings.get(key) {
            None => {
                let keys = config_result
                    .config()
                    .settings
                    .keys()
                    .map(|k| format!("'{k}'"))
                    .collect::<Vec<String>>()
                    .join(", ");
                Err(ClientError::new(ErrorKind::SettingKeyMissing, format!("Failed to evaluate setting '{key}' (the key was not found in config JSON). Returning the `defaultValue` parameter that you specified in your application: '{}'. Available keys: [{keys}].", default.to_str())))
            }
            Some(setting) => {
                let eval_result = eval(
                    setting,
                    key,
                    user,
                    &config_result.config().settings,
                    &default,
                );
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
