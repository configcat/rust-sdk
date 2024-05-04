use crate::model::enums::{
    PrerequisiteFlagComparator, RedirectMode, SegmentComparator, SettingType, UserComparator,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("JSON parsing failed. ({0})")]
    Parse(String),
}

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub config: Arc<Config>,
    pub config_json: String,
    pub etag: String,
    pub fetch_time: DateTime<Utc>,
}

impl Default for ConfigEntry {
    fn default() -> Self {
        Self {
            config: Arc::new(Default::default()),
            config_json: String::default(),
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
    pub fn serialize(&self) -> String {
        self.fetch_time.timestamp_millis().to_string()
            + "\n"
            + &self.etag
            + "\n"
            + &self.config_json
    }

    pub fn is_empty(&self) -> bool {
        self.etag.is_empty() && self.config_json.is_empty()
    }

    pub fn with_time(&self, time: DateTime<Utc>) -> Self {
        Self {
            fetch_time: time,
            etag: self.etag.clone(),
            config_json: self.config_json.clone(),
            config: self.config.clone(),
        }
    }
}

pub fn entry_from_json(
    json: &str,
    etag: &str,
    fetch_time: DateTime<Utc>,
) -> Result<ConfigEntry, Error> {
    match serde_json::from_str::<Config>(json) {
        Ok(config) => Ok(ConfigEntry {
            config: Arc::new(config),
            etag: etag.to_string(),
            fetch_time,
            config_json: json.to_string(),
        }),
        Err(err) => Err(Error::Parse(err.to_string())),
    }
}

pub fn entry_from_cached_json(cached_json: &str) -> Result<ConfigEntry, Error> {
    let time_index = if let Some(time_index) = cached_json.find('\n') {
        time_index
    } else {
        return Err(Error::Parse(
            "Number of values is fewer than expected".to_string(),
        ));
    };
    let without_time = &cached_json[time_index + 1..];
    let etag_index = if let Some(etag_index) = without_time.find('\n') {
        etag_index
    } else {
        return Err(Error::Parse(
            "Number of values is fewer than expected".to_string(),
        ));
    };
    let time_string = &cached_json[..time_index];
    let time = if let Ok(time) = time_string.parse::<i64>() {
        time
    } else {
        return Err(Error::Parse(format!("Invalid fetch time: '{time_string}'")));
    };
    let fetch_time = if let Some(fetch_time) = DateTime::from_timestamp_millis(time) {
        fetch_time
    } else {
        return Err(Error::Parse(format!(
            "Invalid unix seconds value: '{time}'"
        )));
    };

    let config_json = &cached_json[time_index + 1 + etag_index + 1..];
    let etag = &cached_json[time_index + 1..time_index + etag_index + 1];
    entry_from_json(config_json, etag, fetch_time)
}

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    /// The dictionary of settings.
    #[serde(rename = "f")]
    pub settings: HashMap<String, Setting>,
    /// The list of segments.
    #[serde(rename = "s")]
    pub segments: Option<Vec<Segment>>,
    /// The salt that was used to hash sensitive comparison values.
    #[serde(skip)]
    pub salt: Option<String>,

    #[serde(rename = "p")]
    pub preferences: Option<Preferences>,
}

#[derive(Deserialize, Debug)]
pub struct Preferences {
    #[serde(rename = "u")]
    pub url: Option<String>,
    #[serde(rename = "r")]
    pub redirect: Option<RedirectMode>,
    #[serde(rename = "s")]
    pub salt: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    /// The value that is returned when none of the targeting rules or percentage options yield a result.
    #[serde(rename = "v")]
    pub value: Option<SettingValue>,
    /// The list of percentage options.
    #[serde(rename = "p")]
    pub percentage_options: Option<Vec<PercentageOption>>,
    /// The list of targeting rules (where there is a logical OR relation between the items).
    #[serde(rename = "r")]
    pub targeting_rules: Option<Vec<TargetingRule>>,
    /// Variation ID (for analytical purposes).
    #[serde(rename = "i")]
    pub variation_id: Option<String>,
    /// The User Object attribute which serves as the basis of percentage options evaluation.
    #[serde(rename = "a")]
    pub percentage_attribute: Option<String>,
    /// The setting's type. It can be `bool`, `String`, `i64` or `f64`.
    #[serde(rename = "t")]
    pub setting_type: Option<SettingType>,
}

#[derive(Deserialize, Debug)]
pub struct Segment {
    /// The name of the segment.
    #[serde(rename = "n")]
    pub name: Option<String>,
    /// The list of segment rule conditions (has a logical AND relation between the items).
    #[serde(rename = "r")]
    pub conditions: Option<Vec<UserCondition>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TargetingRule {
    /// The value associated with the targeting rule or nil if the targeting rule has percentage options THEN part.
    #[serde(rename = "s")]
    pub served_value: Option<ServedValue>,
    /// The list of conditions that are combined with the AND logical operator.
    #[serde(rename = "c")]
    pub conditions: Option<Vec<Condition>>,
    /// The list of percentage options associated with the targeting rule or empty if the targeting rule has a served value THEN part.
    #[serde(rename = "p")]
    pub percentage_options: Option<Vec<PercentageOption>>,
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct UserCondition {
    /// The value that the User Object attribute is compared to, when the comparator works with a single text comparison value.
    #[serde(rename = "s")]
    pub string_val: Option<String>,
    /// The value that the User Object attribute is compared to, when the comparator works with a numeric comparison value.
    #[serde(rename = "d")]
    pub double_val: Option<f64>,
    /// The value that the User Object attribute is compared to, when the comparator works with an array of text comparison value.
    #[serde(rename = "l")]
    pub string_arr_val: Option<Vec<String>>,
    /// The operator which defines the relation between the comparison attribute and the comparison value.
    #[serde(rename = "c")]
    pub comparator: UserComparator,
    /// The User Object attribute that the condition is based on. Can be "Identifier", "Email", "Country" or any custom attribute.
    #[serde(rename = "a")]
    pub comp_attr: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SegmentCondition {
    /// Identifies the segment that the condition is based on.
    #[serde(rename = "s")]
    pub index: i64,
    /// The operator which defines the expected result of the evaluation of the segment.
    #[serde(rename = "c")]
    pub segment_comparator: SegmentComparator,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrerequisiteFlagCondition {
    /// The key of the prerequisite flag that the condition is based on.
    #[serde(rename = "f")]
    pub flag_key: Option<String>,
    /// The operator which defines the relation between the evaluated value of the prerequisite flag and the comparison value.
    #[serde(rename = "c")]
    pub prerequisite_comparator: PrerequisiteFlagComparator,
    /// The evaluated value of the prerequisite flag is compared to.
    #[serde(rename = "v")]
    pub flag_value: SettingValue,
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct ServedValue {
    /// The value associated with the targeting rule.
    #[serde(rename = "v")]
    pub value: SettingValue,
    /// Variation ID (for analytical purposes).
    #[serde(rename = "i")]
    pub variation_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
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
        assert_eq!(result.config_json, CONFIG_JSON);
        assert_eq!(payload, result.serialize());
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
