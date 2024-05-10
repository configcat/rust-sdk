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

    pub fn builder(sdk_key: &str) -> OptionsBuilder {
        OptionsBuilder::new(sdk_key)
    }

    pub fn new(sdk_key: &str) -> Result<Self, ClientError> {
        OptionsBuilder::new(sdk_key).build()
    }

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

    pub async fn get_bool_value(&self, key: &str, user: Option<User>, default: bool) -> bool {
        self.get_bool_details(key, user, default).await.value
    }

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
                    ..EvaluationDetails::from_results(eval_result, &result)
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
