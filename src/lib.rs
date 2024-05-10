#[macro_use]
mod macros;
mod cache;
mod client;
mod constants;
mod errors;
mod eval;
mod fetch;
mod model;
mod modes;
mod options;
mod r#override;
mod user;
mod utils;
mod value;

pub use cache::ConfigCache;
pub use client::Client;
pub use constants::PKG_VERSION;
pub use errors::{ClientError, ErrorKind};
pub use eval::details::EvaluationDetails;
pub use model::config::{
    Condition, PercentageOption, PrerequisiteFlagCondition, Segment, SegmentCondition, ServedValue,
    Setting, SettingValue, TargetingRule, UserCondition,
};
pub use model::enums::{
    DataGovernance, PrerequisiteFlagComparator, SegmentComparator, SettingType, UserComparator,
};
pub use modes::PollingMode;
pub use options::{Options, OptionsBuilder};
pub use r#override::behavior::OverrideBehavior;
pub use user::{User, UserValue};
pub use value::Value;
