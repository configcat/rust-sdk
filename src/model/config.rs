use crate::model::enums::{
    PrerequisiteFlagComparator, RedirectMode, SegmentComparator, SettingType, UserComparator,
};
use crate::r#override::FlagOverrides;
use crate::value::Value;
use crate::OverrideBehavior;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

const INVALID_VALUE_TXT: &str = "<invalid value>";

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON parsing failed. ({0})")]
    Parse(String),
}

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub config: Arc<Config>,
    pub cache_str: String,
    pub etag: String,
    pub fetch_time: DateTime<Utc>,
}

impl Default for ConfigEntry {
    fn default() -> Self {
        Self {
            config: Arc::new(Config::default()),
            cache_str: String::default(),
            etag: String::default(),
            fetch_time: DateTime::<Utc>::MIN_UTC,
        }
    }
}

impl PartialEq for ConfigEntry {
    fn eq(&self, other: &Self) -> bool {
        self.etag == other.etag
    }
}

impl ConfigEntry {
    pub fn is_empty(&self) -> bool {
        self.etag.is_empty() && self.cache_str.is_empty()
    }

    pub fn local() -> Self {
        Self {
            etag: "local".to_owned(),
            cache_str: "local".to_owned(),
            ..ConfigEntry::default()
        }
    }

    pub fn is_expired(&self, duration: Duration) -> bool {
        Utc::now() - duration > self.fetch_time
    }

    pub fn set_fetch_time(&mut self, fetch_time: DateTime<Utc>) {
        let Some(time_index) = self.cache_str.find('\n') else {
            return;
        };
        let without_time = &self.cache_str[time_index + 1..];
        let Some(etag_index) = without_time.find('\n') else {
            return;
        };
        let config_json = &self.cache_str[time_index + 1 + etag_index + 1..];
        self.fetch_time = fetch_time;
        self.cache_str = generate_cache_str(fetch_time, &self.etag, config_json);
    }
}

pub fn generate_cache_str(time: DateTime<Utc>, etag: &str, json: &str) -> String {
    time.timestamp_millis().to_string() + "\n" + etag + "\n" + json
}

pub fn entry_from_json(
    json: &str,
    etag: &str,
    fetch_time: DateTime<Utc>,
) -> Result<ConfigEntry, Error> {
    match serde_json::from_str::<Config>(json) {
        Ok(config) => {
            let mut entry = ConfigEntry {
                config: Arc::new(config),
                etag: etag.to_owned(),
                fetch_time,
                cache_str: generate_cache_str(fetch_time, etag, json),
            };
            if let Some(conf_mut) = Arc::get_mut(&mut entry.config) {
                post_process_config(conf_mut);
            }
            Ok(entry)
        }
        Err(err) => Err(Error::Parse(err.to_string())),
    }
}

pub fn entry_from_cached_json(cached_json: &str) -> Result<ConfigEntry, Error> {
    let Some(time_index) = cached_json.find('\n') else {
        return Err(Error::Parse(
            "Number of values is fewer than expected".to_owned(),
        ));
    };
    let without_time = &cached_json[time_index + 1..];
    let Some(etag_index) = without_time.find('\n') else {
        return Err(Error::Parse(
            "Number of values is fewer than expected".to_owned(),
        ));
    };
    let time_string = &cached_json[..time_index];
    let Ok(time) = time_string.parse::<i64>() else {
        return Err(Error::Parse(format!("Invalid fetch time: '{time_string}'")));
    };
    let Some(fetch_time) = DateTime::from_timestamp_millis(time) else {
        return Err(Error::Parse(format!(
            "Invalid unix seconds value: '{time}'"
        )));
    };

    let config_json = &cached_json[time_index + 1 + etag_index + 1..];
    let etag = &cached_json[(time_index + 1)..=(time_index + etag_index)];
    entry_from_json(config_json, etag, fetch_time)
}

pub fn post_process_config(config: &mut Config) {
    config.salt = match &config.preferences {
        Some(pref) => pref.salt.clone(),
        None => None,
    };
    for value in config.settings.values_mut() {
        value.salt.clone_from(&config.salt);

        if let Some(rules) = value.targeting_rules.as_mut() {
            for rule in rules {
                let rule_mut = Arc::get_mut(rule).unwrap();
                if let Some(conditions) = rule_mut.conditions.as_mut() {
                    for cond in conditions {
                        if let Some(segment_condition) = cond.segment_condition.as_mut() {
                            if let Some(segments) = &config.segments {
                                if let Some(segment) = segments.get(segment_condition.index) {
                                    segment_condition.segment = Some(segment.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn process_overrides(entry: &mut ConfigEntry, overrides: Option<&FlagOverrides>) {
    if let Some(ov) = overrides {
        if matches!(ov.behavior(), OverrideBehavior::LocalOverRemote) {
            if let Some(conf_mut) = Arc::get_mut(&mut entry.config) {
                conf_mut.settings.extend(ov.source().settings().clone());
            }
        }
        if matches!(ov.behavior(), OverrideBehavior::RemoteOverLocal) {
            if let Some(conf_mut) = Arc::get_mut(&mut entry.config) {
                let mut local = ov.source().settings().clone();
                local.extend(conf_mut.settings.clone());
                conf_mut.settings = local;
            }
        }
    }
}

/// Describes a ConfigCat config JSON.
#[derive(Deserialize, Debug, Default)]
pub struct Config {
    /// The map of settings.
    #[serde(rename = "f")]
    pub settings: HashMap<String, Setting>,
    /// The list of segments.
    #[serde(rename = "s")]
    pub segments: Option<Vec<Arc<Segment>>>,
    /// The salt that was used to hash sensitive comparison values.
    #[serde(skip)]
    pub salt: Option<String>,

    #[serde(rename = "p")]
    pub(crate) preferences: Option<Preferences>,
}

#[derive(Deserialize, Debug)]
pub struct Preferences {
    #[serde(rename = "u")]
    pub url: Option<String>,
    #[serde(rename = "r")]
    pub redirect: Option<RedirectMode>,
    #[serde(rename = "s")]
    pub salt: Option<String>,
}

/// Describes a feature flag or setting.
#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    /// The value that is returned when none of the targeting rules or percentage options yield a result.
    #[serde(rename = "v")]
    pub value: SettingValue,
    /// The list of percentage options.
    #[serde(rename = "p")]
    pub percentage_options: Option<Vec<Arc<PercentageOption>>>,
    /// The list of targeting rules (where there is a logical OR relation between the items).
    #[serde(rename = "r")]
    pub targeting_rules: Option<Vec<Arc<TargetingRule>>>,
    /// Variation ID (for analytical purposes).
    #[serde(rename = "i")]
    pub variation_id: Option<String>,
    /// The User Object attribute which serves as the basis of percentage options evaluation.
    #[serde(rename = "a")]
    pub percentage_attribute: Option<String>,
    /// The setting's type. It can be `bool`, `String`, `i64` or `f64`.
    #[serde(rename = "t")]
    pub setting_type: SettingType,

    #[serde(skip)]
    pub(crate) salt: Option<String>,
}

impl From<&Value> for Setting {
    fn from(value: &Value) -> Self {
        Setting {
            setting_type: value.into(),
            value: value.into(),
            variation_id: None,
            percentage_options: None,
            percentage_attribute: None,
            targeting_rules: None,
            salt: None,
        }
    }
}

#[derive(Deserialize, Debug)]
/// Describes a segment.
pub struct Segment {
    /// The name of the segment.
    #[serde(rename = "n")]
    pub name: String,
    /// The list of segment rule conditions (has a logical AND relation between the items).
    #[serde(rename = "r")]
    pub conditions: Vec<UserCondition>,
}

#[derive(Deserialize, Debug)]
/// Describes a targeting rule.
pub struct TargetingRule {
    /// The value associated with the targeting rule or nil if the targeting rule has percentage options THEN part.
    #[serde(rename = "s")]
    pub served_value: Option<ServedValue>,
    /// The list of conditions that are combined with the AND logical operator.
    #[serde(rename = "c")]
    pub conditions: Option<Vec<Condition>>,
    /// The list of percentage options associated with the targeting rule or empty if the targeting rule has a served value THEN part.
    #[serde(rename = "p")]
    pub percentage_options: Option<Vec<Arc<PercentageOption>>>,
}

#[derive(Deserialize, Debug)]
/// Describes a condition that can contain either a [`UserCondition`], a [`SegmentCondition`], or a [`PrerequisiteFlagCondition`].
pub struct Condition {
    /// Describes a condition that works with User Object attributes.
    #[serde(rename = "u")]
    pub user_condition: Option<UserCondition>,
    /// Describes a condition that works with a segment.
    #[serde(rename = "s")]
    pub segment_condition: Option<SegmentCondition>,
    /// Describes a condition that works with a prerequisite flag.
    #[serde(rename = "p")]
    pub prerequisite_flag_condition: Option<PrerequisiteFlagCondition>,
}

#[derive(Deserialize, Debug)]
/// Describes a condition that is based on a [`crate::User`] attribute.
pub struct UserCondition {
    /// The value that the User Object attribute is compared to, when the comparator works with a single text comparison value.
    #[serde(rename = "s")]
    pub string_val: Option<String>,
    /// The value that the User Object attribute is compared to, when the comparator works with a numeric comparison value.
    #[serde(rename = "d")]
    pub float_val: Option<f64>,
    /// The value that the User Object attribute is compared to, when the comparator works with an array of text comparison value.
    #[serde(rename = "l")]
    pub string_vec_val: Option<Vec<String>>,
    /// The operator which defines the relation between the comparison attribute and the comparison value.
    #[serde(rename = "c")]
    pub comparator: UserComparator,
    /// The User Object attribute that the condition is based on. Can be "Identifier", "Email", "Country" or any custom attribute.
    #[serde(rename = "a")]
    pub comp_attr: String,
}

const STRING_LIST_MAX_LENGTH: usize = 10;

impl Display for UserCondition {
    #[allow(clippy::cast_possible_truncation)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let res = write!(f, "User.{} {} ", self.comp_attr, self.comparator);
        if self.float_val.is_none() && self.string_val.is_none() && self.string_vec_val.is_none() {
            return f.write_str(INVALID_VALUE_TXT);
        }
        if let Some(num) = self.float_val {
            return if self.comparator.is_date() {
                let date =
                    DateTime::from_timestamp_millis((num * 1000.0) as i64).unwrap_or_default();
                write!(f, "'{num}' ({})", date.format("%Y-%m-%dT%H:%M:%S%.3f %Z"))
            } else {
                write!(f, "'{num}'")
            };
        }
        if let Some(text) = self.string_val.as_ref() {
            return if self.comparator.is_sensitive() {
                f.write_str("'<hashed value>'")
            } else {
                write!(f, "'{text}'")
            };
        }
        if let Some(vec) = self.string_vec_val.as_ref() {
            return if self.comparator.is_sensitive() {
                let val_t = if vec.len() > 1 { "values" } else { "value" };
                write!(f, "[<{} hashed {val_t}>]", vec.len())
            } else {
                let len = vec.len();
                let val_t = if len > (STRING_LIST_MAX_LENGTH + 1) {
                    "values"
                } else {
                    "value"
                };
                let limit = min(len, STRING_LIST_MAX_LENGTH);
                let mut arr_txt = String::default();
                for (index, item) in vec.iter().enumerate() {
                    arr_txt.push_str(format!("'{item}'").as_str());
                    if index < limit - 1 {
                        arr_txt.push_str(", ");
                    } else if len > STRING_LIST_MAX_LENGTH {
                        arr_txt.push_str(
                            format!(", ... <{} more {val_t}>", len - STRING_LIST_MAX_LENGTH)
                                .as_str(),
                        );
                        break;
                    }
                }
                write!(f, "[{arr_txt}]")
            };
        }
        res
    }
}

/// Describes a condition that is based on a [`Segment`].
#[derive(Deserialize, Debug)]
pub struct SegmentCondition {
    /// Identifies the segment that the condition is based on.
    #[serde(rename = "s")]
    pub index: usize,
    /// The operator which defines the expected result of the evaluation of the segment.
    #[serde(rename = "c")]
    pub segment_comparator: SegmentComparator,

    #[serde(skip)]
    pub(crate) segment: Option<Arc<Segment>>,
}

impl Display for SegmentCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self.segment.as_ref() {
            Some(seg) => seg.name.as_str(),
            None => "<invalid name>",
        };
        write!(f, "User {} '{name}'", self.segment_comparator)
    }
}

/// Describes a condition that is based on a prerequisite flag.
#[derive(Deserialize, Debug)]
pub struct PrerequisiteFlagCondition {
    /// The key of the prerequisite flag that the condition is based on.
    #[serde(rename = "f")]
    pub flag_key: String,
    /// The operator which defines the relation between the evaluated value of the prerequisite flag and the comparison value.
    #[serde(rename = "c")]
    pub prerequisite_comparator: PrerequisiteFlagComparator,
    /// The evaluated value of the prerequisite flag is compared to.
    #[serde(rename = "v")]
    pub flag_value: SettingValue,
}

impl Display for PrerequisiteFlagCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Flag '{}' {} '{}'",
            self.flag_key, self.prerequisite_comparator, self.flag_value
        )
    }
}

/// Describes a percentage option.
#[derive(Deserialize, Debug)]
pub struct PercentageOption {
    /// The served value of the percentage option.
    #[serde(rename = "v")]
    pub served_value: SettingValue,
    /// A number between 0 and 100 that represents a randomly allocated fraction of the users.
    #[serde(rename = "p")]
    pub percentage: i64,
    /// Variation ID (for analytical purposes).
    #[serde(rename = "i")]
    pub variation_id: Option<String>,
}

/// Describes a setting value along with related data.
#[derive(Deserialize, Debug)]
pub struct ServedValue {
    /// The value associated with the targeting rule.
    #[serde(rename = "v")]
    pub value: SettingValue,
    /// Variation ID (for analytical purposes).
    #[serde(rename = "i")]
    pub variation_id: Option<String>,
}

/// Describes a setting's value.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SettingValue {
    /// Holds a bool feature flag's value.
    #[serde(rename = "b")]
    pub bool_val: Option<bool>,
    /// Holds a string setting's value.
    #[serde(rename = "s")]
    pub string_val: Option<String>,
    /// Holds a decimal number setting's value.
    #[serde(rename = "d")]
    pub float_val: Option<f64>,
    /// Holds a whole number setting's value.
    #[serde(rename = "i")]
    pub int_val: Option<i64>,
}

impl SettingValue {
    pub(crate) fn as_val(&self, setting_type: &SettingType) -> Option<Value> {
        match setting_type {
            SettingType::Bool => {
                if let Some(bool_val) = self.bool_val {
                    return Some(Value::Bool(bool_val));
                }
                None
            }
            SettingType::String => {
                if let Some(string_val) = self.string_val.as_ref() {
                    return Some(Value::String(string_val.clone()));
                }
                None
            }
            SettingType::Int => {
                if let Some(int_val) = self.int_val {
                    return Some(Value::Int(int_val));
                }
                None
            }
            SettingType::Float => {
                if let Some(float_val) = self.float_val {
                    return Some(Value::Float(float_val));
                }
                None
            }
        }
    }
}

impl From<&Value> for SettingValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Bool(val) => SettingValue {
                bool_val: Some(*val),
                ..SettingValue::default()
            },
            Value::Int(val) => SettingValue {
                int_val: Some(*val),
                ..SettingValue::default()
            },
            Value::Float(val) => SettingValue {
                float_val: Some(*val),
                ..SettingValue::default()
            },
            Value::String(val) => SettingValue {
                string_val: Some(val.clone()),
                ..SettingValue::default()
            },
        }
    }
}

impl Display for SettingValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(b) = self.bool_val.as_ref() {
            write!(f, "{b}")
        } else if let Some(s) = self.string_val.as_ref() {
            f.write_str(s)
        } else if let Some(fl) = self.float_val.as_ref() {
            write!(f, "{fl}")
        } else if let Some(i) = self.int_val.as_ref() {
            write!(f, "{i}")
        } else {
            f.write_str(INVALID_VALUE_TXT)
        }
    }
}

#[cfg(test)]
mod model_tests {
    use crate::model::config::entry_from_cached_json;
    use chrono::{DateTime, Utc};
    use std::str::FromStr;

    static CONFIG_JSON: &str = r#"{"p":{"u":"https://cdn-global.configcat.com","r":0,"s":"FUkC6RADjzF0vXrDSfJn7BcEBag9afw1Y6jkqjMP9BA="},"f":{"testKey":{"t":1,"v":{"s": "testValue"}}}}"#;

    #[test]
    fn parse() {
        let payload = format!("1686756435844\ntest-etag\n{CONFIG_JSON}");
        let result = entry_from_cached_json(payload.as_str()).unwrap();
        let exp_time: DateTime<Utc> = DateTime::from_str("2023-06-14T15:27:15.8440000Z").unwrap();
        assert_eq!(result.config.settings.len(), 1);
        assert_eq!(result.etag, "test-etag");
        assert_eq!(result.fetch_time, exp_time);
        assert_eq!(result.cache_str, payload);
    }

    #[test]
    fn set_fetch_time() {
        let payload = format!("1686756435844\ntest-etag\n{CONFIG_JSON}");
        let mut entry = entry_from_cached_json(payload.as_str()).unwrap();
        let updated_time = Utc::now();
        entry.set_fetch_time(updated_time);
        assert_eq!(entry.config.settings.len(), 1);
        assert_eq!(entry.fetch_time, updated_time);
        assert_eq!(entry.etag, "test-etag");
        assert_eq!(
            entry.cache_str,
            format!(
                "{}\ntest-etag\n{CONFIG_JSON}",
                updated_time.timestamp_millis()
            )
        );
    }

    #[test]
    fn parse_invalid() {
        match entry_from_cached_json("") {
            Ok(_) => panic!(),
            Err(msg) => assert_eq!(
                msg.to_string(),
                "JSON parsing failed. (Number of values is fewer than expected)"
            ),
        }
        match entry_from_cached_json("\n") {
            Ok(_) => panic!(),
            Err(msg) => assert_eq!(
                msg.to_string(),
                "JSON parsing failed. (Number of values is fewer than expected)"
            ),
        }
        match entry_from_cached_json("\n\n") {
            Ok(_) => panic!(),
            Err(msg) => assert_eq!(
                msg.to_string(),
                "JSON parsing failed. (Invalid fetch time: '')"
            ),
        }
        match entry_from_cached_json("1686756435844\ntest-etag") {
            Ok(_) => panic!(),
            Err(msg) => assert_eq!(
                msg.to_string(),
                "JSON parsing failed. (Number of values is fewer than expected)"
            ),
        }
        match entry_from_cached_json(format!("abc\ntest-etag\n{CONFIG_JSON}").as_str()) {
            Ok(_) => panic!(),
            Err(msg) => assert_eq!(
                msg.to_string(),
                "JSON parsing failed. (Invalid fetch time: 'abc')"
            ),
        }
        match entry_from_cached_json("1686756435844\ntest-etag\n{\"a\":\"b\"}") {
            Ok(_) => panic!(),
            Err(msg) => assert_eq!(
                msg.to_string(),
                "JSON parsing failed. (missing field `f` at line 1 column 9)"
            ),
        }
    }
}
