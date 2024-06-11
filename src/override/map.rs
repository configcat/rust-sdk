use crate::r#override::source::OverrideDataSource;
use crate::{Setting, Value};
use std::collections::HashMap;

/// Data source that gets the overridden feature flag or setting values from a [`HashMap`] or a `[(&str, Value)]` array.
pub struct MapDataSource {
    overrides: HashMap<String, Setting>,
}

impl OverrideDataSource for MapDataSource {
    fn settings(&self) -> &HashMap<String, Setting> {
        &self.overrides
    }
}

impl From<HashMap<&str, Value>> for MapDataSource {
    /// Creates a new [`MapDataSource`] from a [`HashMap`] of `&str` and [`Value`].
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let source = MapDataSource::from(HashMap::from([
    ///     ("flag", Value::Bool(true))
    /// ]));
    /// ```
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let map = HashMap::from([("flag", Value::Bool(true))]);
    /// let source: MapDataSource = map.into();
    /// ```
    fn from(value: HashMap<&str, Value>) -> Self {
        Self {
            overrides: value
                .iter()
                .map(|(k, v)| (k.to_string(), v.into()))
                .collect::<HashMap<String, Setting>>(),
        }
    }
}

impl From<HashMap<String, Value>> for MapDataSource {
    /// Creates a new [`MapDataSource`] from a [`HashMap`] of [`String`] and [`Value`].
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let source = MapDataSource::from(HashMap::from([
    ///     ("flag".to_owned(), Value::Bool(true))
    /// ]));
    /// ```
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let map = HashMap::from([("flag".to_owned(), Value::Bool(true))]);
    /// let source: MapDataSource = map.into();
    /// ```
    fn from(value: HashMap<String, Value>) -> Self {
        Self {
            overrides: value
                .iter()
                .map(|(k, v)| (k.clone(), v.into()))
                .collect::<HashMap<String, Setting>>(),
        }
    }
}

impl<const N: usize> From<[(&str, Value); N]> for MapDataSource {
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let source = MapDataSource::from([
    ///     ("flag", Value::Bool(true))
    /// ]);
    /// ```
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use configcat::{MapDataSource, Value};
    ///
    /// let arr = [("flag", Value::Bool(true))];
    /// let source: MapDataSource = arr.into();
    /// ```
    fn from(arr: [(&str, Value); N]) -> Self {
        Self {
            overrides: HashMap::from_iter(arr.iter().map(|(k, v)| (k.to_string(), v.into()))),
        }
    }
}
