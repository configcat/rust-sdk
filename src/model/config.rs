use crate::errors::InternalError;
use crate::model::enums::{
    PrerequisiteFlagComparator, RedirectMode, SegmentComparator, SettingType, UserComparator,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serializer};
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

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
) -> Result<ConfigEntry, InternalError> {
    match serde_json::from_str::<Config>(json) {
        Ok(config) => Ok(ConfigEntry {
            config: Arc::new(config),
            etag: etag.to_owned(),
            fetch_time,
            config_json: json.to_owned(),
        }),
        Err(err) => Err(InternalError::Parse(err.to_string())),
    }
}

pub fn entry_from_cached_json(cached_json: &str) -> Result<ConfigEntry, InternalError> {
    let time_index = if let Some(time_index) = cached_json.find('\n') {
        time_index
    } else {
        return Err(InternalError::Parse(
            "Number of values is fewer than expected".to_owned(),
        ));
    };
    let without_time = &cached_json[time_index + 1..];
    let etag_index = if let Some(etag_index) = without_time.find('\n') {
        etag_index
    } else {
        return Err(InternalError::Parse(
            "Number of values is fewer than expected".to_owned(),
        ));
    };
    let time_string = &cached_json[..time_index];
    let time = if let Ok(time) = time_string.parse::<i64>() {
        time
    } else {
        return Err(InternalError::Parse(format!(
            "Invalid fetch time: '{time_string}'"
        )));
    };
    let fetch_time = if let Some(fetch_time) = DateTime::from_timestamp_millis(time) {
        fetch_time
    } else {
        return Err(InternalError::Parse(format!(
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

/// Describes a feature flag or setting.
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
/// Describes a segment.
pub struct Segment {
    /// The name of the segment.
    #[serde(rename = "n")]
    pub name: Option<String>,
    /// The list of segment rule conditions (has a logical AND relation between the items).
    #[serde(rename = "r")]
    pub conditions: Option<Vec<UserCondition>>,
}

#[derive(Deserialize, Debug, Clone)]
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
    pub percentage_options: Option<Vec<PercentageOption>>,
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
/// Describes a condition that is based on a [`crate::User`] attribute.
pub struct UserCondition {
    /// The value that the User Object attribute is compared to, when the comparator works with a single text comparison value.
    #[serde(rename = "s")]
    pub string_val: Option<String>,
    /// The value that the User Object attribute is compared to, when the comparator works with a numeric comparison value.
    #[serde(rename = "d")]
    pub double_val: Option<f64>,
    /// The value that the User Object attribute is compared to, when the comparator works with an array of text comparison value.
    #[serde(rename = "l")]
    pub string_vec_val: Option<Vec<String>>,
    /// The operator which defines the relation between the comparison attribute and the comparison value.
    #[serde(rename = "c")]
    pub comparator: UserComparator,
    /// The User Object attribute that the condition is based on. Can be "Identifier", "Email", "Country" or any custom attribute.
    #[serde(rename = "a")]
    pub comp_attr: Option<String>,
}

impl UserCondition {
    const INVALID_ATTRIBUTE: &'static str = "<invalid attribute>";

    pub(crate) fn fmt_comp_attr(&self) -> String {
        self.comp_attr
            .clone()
            .unwrap_or(Self::INVALID_ATTRIBUTE.to_owned())
    }
}

const STRING_LIST_MAX_LENGTH: usize = 10;

impl Display for UserCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let res = write!(f, "User.{} {}", self.fmt_comp_attr(), self.comparator);
        if self.double_val.is_none() && self.string_val.is_none() && self.string_vec_val.is_none() {
            return f.write_str("<invalid value>");
        }
        if let Some(num) = self.double_val {
            return if self.comparator.is_date() {
                let date = DateTime::from_timestamp_millis(num as i64).unwrap();
                write!(f, "{num} ({date})")
            } else {
                f.serialize_f64(num)
            };
        }
        if let Some(text) = self.string_val.as_ref() {
            return if self.comparator.is_sensitive() {
                f.write_str("<hashed value>")
            } else {
                f.write_str(text.as_str())
            };
        }
        if let Some(vec) = self.string_vec_val.as_ref() {
            return if self.comparator.is_sensitive() {
                let val_t = if vec.len() > 1 { "values" } else { "value" };
                write!(f, "[<{} hashed {val_t}>]", vec.len())
            } else {
                let len = vec.len();
                let val_t = if len - STRING_LIST_MAX_LENGTH > 1 {
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
#[derive(Deserialize, Debug, Clone)]
pub struct SegmentCondition {
    /// Identifies the segment that the condition is based on.
    #[serde(rename = "s")]
    pub index: i64,
    /// The operator which defines the expected result of the evaluation of the segment.
    #[serde(rename = "c")]
    pub segment_comparator: SegmentComparator,
}

/// Describes a condition that is based on a prerequisite flag.
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

/// Describes a percentage option.
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

/// Describes a setting value along with related data.
#[derive(Deserialize, Debug, Clone)]
pub struct ServedValue {
    /// The value associated with the targeting rule.
    #[serde(rename = "v")]
    pub value: SettingValue,
    /// Variation ID (for analytical purposes).
    #[serde(rename = "i")]
    pub variation_id: Option<String>,
}

/// Describes a setting's value.
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

impl Display for SettingValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(b) = self.bool_val.as_ref() {
            f.serialize_bool(*b)
        } else if let Some(s) = self.string_val.as_ref() {
            f.write_str(s)
        } else if let Some(fl) = self.float_val.as_ref() {
            f.serialize_f64(*fl)
        } else if let Some(i) = self.int_val.as_ref() {
            f.serialize_i64(*i)
        } else {
            f.write_str("<invalid value>")
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
