use crate::eval::evaluator::EvalResult;
use crate::fetch::service::ConfigResult;
use crate::{ClientError, PercentageOption, TargetingRule, User};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Details of the flag evaluation's result.
#[derive(Default)]
pub struct EvaluationDetails<T> {
    pub value: T,
    /// Key of the feature flag or setting.
    pub key: String,
    /// Indicates whether the default value passed to the setting evaluation methods is used as the result of the evaluation.
    pub is_default_value: bool,
    /// Variation ID of the feature flag or setting (if available).
    pub variation_id: Option<String>,
    /// The User Object used for the evaluation (if available).
    pub user: Option<User>,
    /// Error in case evaluation failed.
    pub error: Option<ClientError>,
    /// Time of last successful config download.
    pub fetch_time: Option<DateTime<Utc>>,
    /// The targeting rule (if any) that matched during the evaluation and was used to return the evaluated value.
    pub matched_targeting_rule: Option<Arc<TargetingRule>>,
    /// The percentage option (if any) that was used to select the evaluated value.
    pub matched_percentage_option: Option<Arc<PercentageOption>>,
}

impl<T: Default> EvaluationDetails<T> {
    pub(crate) fn from_err(val: T, key: &str, user: Option<User>, err: ClientError) -> Self {
        Self {
            value: val,
            key: key.to_owned(),
            is_default_value: true,
            user,
            error: Some(err),
            ..EvaluationDetails::default()
        }
    }

    pub(crate) fn from_results(eval_result: EvalResult, config_result: &ConfigResult) -> Self {
        Self {
            variation_id: eval_result.variation_id,
            fetch_time: Some(*config_result.fetch_time()),
            matched_targeting_rule: eval_result.rule,
            matched_percentage_option: eval_result.option,
            ..EvaluationDetails::default()
        }
    }
}
