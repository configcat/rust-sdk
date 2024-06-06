//! ConfigCat SDK for Rust.

#![warn(missing_docs)]

#[macro_use]
mod macros;
mod builder;
mod cache;
mod client;
mod constants;
mod errors;
mod eval;
mod fetch;
mod model;
mod modes;
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
    Condition, Config, PercentageOption, PrerequisiteFlagCondition, Segment, SegmentCondition,
    ServedValue, Setting, SettingValue, TargetingRule, UserCondition,
};

pub use model::enums::{
    ClientCacheState, DataGovernance, PrerequisiteFlagComparator, SegmentComparator, SettingType,
    UserComparator,
};

pub use r#override::{
    behavior::OverrideBehavior, file::FileDataSource, file::SimplifiedConfig, map::MapDataSource,
    source::OverrideDataSource,
};

pub use builder::ClientBuilder;
pub use modes::PollingMode;

pub use user::{User, UserValue};
pub use value::{Value, ValuePrimitive};
