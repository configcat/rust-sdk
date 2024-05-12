use crate::eval::evaluator::EvalResult;
use crate::{ClientError, PercentageOption, TargetingRule, User};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Details of the flag evaluation's result.
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
///     
///     let flag_val = details.value;
///     let fetch_time = details.fetch_time.unwrap();
/// }
/// ```
#[derive(Default)]
pub struct EvaluationDetails<T> {
    /// Value of the feature flag or setting.
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
    /// Time of last successful config download on which the evaluation was based.
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
}

impl<T: Default> From<EvalResult> for EvaluationDetails<T> {
    fn from(value: EvalResult) -> Self {
        EvaluationDetails {
            variation_id: value.variation_id,
            matched_targeting_rule: value.rule,
            matched_percentage_option: value.option,
            ..EvaluationDetails::default()
        }
    }
}
