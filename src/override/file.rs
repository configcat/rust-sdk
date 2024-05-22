use crate::model::config::{post_process_config, Config};
use crate::r#override::source::OverrideDataSource;
use crate::{Setting, Value};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// Represents feature flag and setting overrides in a simple JSON map format.
///
/// # Examples
///
/// ```no_run
/// use configcat::{FileDataSource, Value};
///
/// // The following JSON format is also supported to describe overrides:
/// // {
/// //   "flags": [
/// //     "bool_flag": true,
/// //     "string_setting": "example",
/// //     "number_setting": 3.14
/// //   ]
/// // }
///
/// let source = FileDataSource::new("path/to/file.json").unwrap();
/// ```
#[derive(Deserialize)]
pub struct SimplifiedConfig {
    /// The feature flag override JSON map.
    pub flags: HashMap<String, serde_json::Value>,
}

/// Data source that gets the overridden feature flag or setting values from a JSON file.
pub struct FileDataSource {
    config: Config,
}

impl FileDataSource {
    /// Creates a new [`FileDataSource`].
    ///
    /// # Errors
    ///
    /// This method fails in the following cases:
    /// - The given file doesn't exist.
    /// - The given file's content is not deserializable to [`SimplifiedConfig`] or [`Config`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use configcat::{FileDataSource, Value};
    ///
    /// let source = FileDataSource::new("path/to/file.json").unwrap();
    /// ```
    pub fn new(file_path: &str) -> Result<Self, String> {
        let content_result = fs::read_to_string(file_path);
        match content_result {
            Ok(content) => {
                let simple_result = serde_json::from_str::<SimplifiedConfig>(content.as_str());
                match simple_result {
                    Ok(simple_config) => {
                        let mut map: HashMap<String, Setting> = HashMap::new();
                        for (k, v) in simple_config.flags.iter() {
                            let val_result = Value::from_json_val(v);
                            if let Some(val) = val_result {
                                map.insert(k.clone(), val.into());
                            } else {
                                return Err(format!("Value of override '{k}' is invalid."));
                            }
                        }
                        Ok(FileDataSource {
                            config: Config {
                                settings: map,
                                salt: None,
                                segments: None,
                                preferences: None,
                            },
                        })
                    }
                    Err(_) => match serde_json::from_str::<Config>(content.as_str()) {
                        Ok(mut config) => {
                            post_process_config(&mut config);
                            Ok(FileDataSource { config })
                        }
                        Err(err) => Err(err.to_string()),
                    },
                }
            }
            Err(err) => Err(err.to_string()),
        }
    }
}

impl OverrideDataSource for FileDataSource {
    fn settings(&self) -> &HashMap<String, Setting> {
        &self.config.settings
    }
}
