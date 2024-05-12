use crate::Setting;
use std::collections::HashMap;

/// Data source that provides feature flag and setting value overrides.
pub trait OverrideDataSource: Sync + Send {
    /// Gets the overridden feature flag or setting values.
    fn settings(&self) -> &HashMap<String, Setting>;
}
