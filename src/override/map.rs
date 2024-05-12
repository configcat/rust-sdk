use crate::r#override::source::OverrideDataSource;
use crate::{Setting, Value};
use std::collections::HashMap;

/// Data source that gets the overridden feature flag or setting values from a [`HashMap`] of [`String`] and [`Value`] items.
pub struct MapDataSource {
    overrides: HashMap<String, Setting>,
}

impl MapDataSource {
    /// Creates a new [`MapDataSource`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let source = MapDataSource::new(HashMap::from([
    ///     ("flag".to_owned(), Value::Bool(true))
    /// ]));
    /// ```
    pub fn new(overrides: HashMap<String, Value>) -> Self {
        Self {
            overrides: overrides
                .iter()
                .map(|(k, v)| (k.clone(), v.clone().into()))
                .collect::<HashMap<String, Setting>>(),
        }
    }
}

impl OverrideDataSource for MapDataSource {
    fn settings(&self) -> &HashMap<String, Setting> {
        &self.overrides
    }
}
