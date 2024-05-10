use chrono::{DateTime, Utc};
use semver::Version;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub const IDENTIFIER: &str = "Identifier";
pub const EMAIL: &str = "Email";
pub const COUNTRY: &str = "Country";

/// Supported user attribute value types.
pub enum UserValue {
    /// String user attribute value.
    String(String),
    /// Signed integer user attribute value.
    Int(i64),
    /// Unsigned integer user attribute value.
    UInt(u64),
    /// Float user attribute value.
    Float(f64),
    /// Datetime user attribute value.
    DateTime(DateTime<Utc>),
    /// String vector user attribute value.
    StringVec(Vec<String>),
    /// Semantic version user attribute value.
    SemVer(Version),
}

impl Display for UserValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UserValue::String(val) => f.write_str(val),
            UserValue::Int(val) => write!(f, "{val}"),
            UserValue::UInt(val) => write!(f, "{val}"),
            UserValue::Float(val) => write!(f, "{val}"),
            UserValue::DateTime(val) => f.write_str(val.to_string().as_str()),
            UserValue::StringVec(_) => f.write_str("<vec of strings>"),
            UserValue::SemVer(val) => f.write_str(val.to_string().as_str()),
        }
    }
}

impl Serialize for UserValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            UserValue::String(val) => serializer.serialize_str(val),
            UserValue::Int(val) => serializer.serialize_i64(*val),
            UserValue::UInt(val) => serializer.serialize_u64(*val),
            UserValue::Float(val) => serializer.serialize_f64(*val),
            UserValue::DateTime(val) => serializer.serialize_str(val.to_string().as_str()),
            UserValue::StringVec(val) => {
                let mut seq = serializer.serialize_seq(Some(val.len()))?;
                for element in val {
                    seq.serialize_element(element)?;
                }
                seq.end()
            }
            UserValue::SemVer(val) => serializer.serialize_str(val.to_string().as_str()),
        }
    }
}

/// Describes a User Object. Contains user attributes which are used for evaluating targeting rules and percentage options.
///
/// All comparators support [`String`] values as User Object attribute (in some cases they need to be provided in a specific format though, see below),
/// but some of them also support other types of values. It depends on the comparator how the values will be handled. The following rules apply:
///
/// **Text-based comparators** (`EQUALS`, `IS ONE OF`, etc.)
/// * accept [`String`] values,
/// * all other values are automatically converted to [`String`] (a warning will be logged but evaluation will continue as normal).
///
/// **SemVer-based comparators** (`IS ONE OF`, `<`, `>=`, etc.)
/// * accept [`String`] values containing a properly formatted, valid semver value,
/// * all other values are considered invalid (a warning will be logged and the currently evaluated targeting rule will be skipped).
///
/// **Number-based comparators** (`=`, `<`, `>=`, etc.)
/// * accept `Int`, `UInt`, or `Float` values,
/// * accept [`String`] values containing a properly formatted, valid `Float` value,
/// * all other values are considered invalid (a warning will be logged and the currently evaluated targeting rule will be skipped).
///
/// **Date time-based comparators** (`BEFORE` / `AFTER`)
/// * accept [`DateTime`] values, which are automatically converted to a second-based Unix timestamp,
/// * accept `Int`, `UInt`, or `Float` values representing a second-based Unix timestamp,
/// * accept [`String`] values containing a properly formatted, valid `Float` value,
/// * all other values are considered invalid (a warning will be logged and the currently evaluated targeting rule will be skipped).
///
/// **String array-based comparators** (`ARRAY CONTAINS ANY OF` / `ARRAY NOT CONTAINS ANY OF`)
/// * accept [`Vec`] of [`String`]s,
/// * accept [`String`] values containing a valid JSON string which can be deserialized to an array of [`String`],
/// * all other values are considered invalid (a warning will be logged and the currently evaluated targeting rule will be skipped).
/// # Examples:
///
/// ```rust
/// use configcat::User;
///
/// use std::str::FromStr;
/// use chrono::{DateTime, Utc};
///
/// let user = User::new("user-id")
///     .email("john@example.com")
///     .custom("Rating", 4.5)
///     .custom("RegisteredAt", DateTime::from_str("2023-06-14T15:27:15.8440000Z").unwrap())
///     .custom("Roles", vec!["Role1", "Role2"]);
/// ```
#[derive(Serialize)]
pub struct User {
    attributes: HashMap<String, UserValue>,
}

impl User {
    /// Initializes a new [`User`].
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use configcat::User;
    ///
    /// let user = User::new("user-id");
    /// ```
    pub fn new(identifier: &str) -> Self {
        Self {
            attributes: HashMap::from([(IDENTIFIER.to_owned(), UserValue::from(identifier))]),
        }
    }

    /// Email address of the user.
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use configcat::User;
    ///
    /// let user = User::new("user-id")
    ///     .email("john@example.com");
    /// ```
    pub fn email(mut self, email: &str) -> Self {
        self.attributes.insert(EMAIL.to_owned(), email.into());
        self
    }

    /// Country of the user.
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use configcat::User;
    ///
    /// let user = User::new("user-id")
    ///     .country("Hungary");
    /// ```
    pub fn country(mut self, country: &str) -> Self {
        self.attributes.insert(COUNTRY.to_owned(), country.into());
        self
    }

    /// Custom attribute of the user for advanced targeting rule definitions (e.g. user role, subscription type, etc.)
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use configcat::User;
    ///
    /// use std::str::FromStr;
    /// use chrono::{DateTime, Utc};
    ///
    /// let user = User::new("user-id")
    ///     .custom("Rating", 4.5)
    ///     .custom("RegisteredAt", DateTime::from_str("2023-06-14T15:27:15.8440000Z").unwrap())
    ///     .custom("Roles", vec!["Role1", "Role2"]);
    /// ```
    pub fn custom<T: Into<UserValue>>(mut self, key: &str, value: T) -> Self {
        if key == IDENTIFIER || key == EMAIL || key == COUNTRY {
            return self;
        }
        self.attributes.insert(key.to_owned(), value.into());
        self
    }

    pub(crate) fn get(&self, key: &str) -> Option<&UserValue> {
        self.attributes.get(key)
    }
}

impl UserValue {
    pub(crate) fn as_str(&self) -> (String, bool) {
        match self {
            UserValue::String(val) => (val.clone(), false),
            UserValue::Float(val) => {
                if val.is_nan() {
                    ("NaN".to_owned(), true)
                } else if val.is_infinite() && val.is_sign_positive() {
                    ("Infinity".to_owned(), true)
                } else if val.is_infinite() && val.is_sign_negative() {
                    ("-Infinity".to_owned(), true)
                } else {
                    (val.to_string(), true)
                }
            }
            UserValue::SemVer(val) => (val.to_string(), true),
            UserValue::Int(val) => (val.to_string(), true),
            UserValue::UInt(val) => (val.to_string(), true),
            UserValue::DateTime(val) => (val.timestamp_millis().to_string(), true),
            UserValue::StringVec(val) => {
                let ser = serde_json::to_string(val);
                match ser {
                    Ok(val) => (val, true),
                    Err(_) => (String::default(), true),
                }
            }
        }
    }

    pub(crate) fn as_float(&self) -> Option<f64> {
        match self {
            UserValue::String(val) => {
                let trimmed = val.trim();
                match trimmed {
                    "Infinity" | "+Infinity" => Some(f64::INFINITY),
                    "-Infinity" => Some(f64::NEG_INFINITY),
                    "NaN" => Some(f64::NAN),
                    _ => match trimmed.parse() {
                        Ok(num) => Some(num),
                        Err(_) => None,
                    },
                }
            }
            UserValue::Int(val) => Some(*val as f64),
            UserValue::UInt(val) => Some(*val as f64),
            UserValue::Float(val) => Some(*val),
            _ => None,
        }
    }

    pub(crate) fn as_timestamp(&self) -> Option<f64> {
        match self {
            UserValue::DateTime(val) => Some(val.timestamp_millis() as f64),
            _ => self.as_float(),
        }
    }

    pub(crate) fn as_semver(&self) -> Option<Version> {
        match self {
            UserValue::SemVer(val) => Some(val.clone()),
            UserValue::String(val) => match Version::parse(val) {
                Ok(version) => Some(version),
                Err(_) => None,
            },
            _ => None,
        }
    }

    pub(crate) fn as_str_vec(&self) -> Option<Vec<String>> {
        match self {
            UserValue::StringVec(val) => Some(val.clone()),
            UserValue::String(val) => {
                let result = serde_json::from_str::<Vec<String>>(val);
                match result {
                    Ok(vec) => Some(vec),
                    Err(_) => None,
                }
            }
            _ => None,
        }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(str) => write!(f, "{str}"),
            Err(_) => f.write_str("<invalid user>"),
        }
    }
}

impl From<Vec<&str>> for UserValue {
    fn from(value: Vec<&str>) -> Self {
        let str_vec = value.iter().map(|x| x.to_string()).collect();
        Self::StringVec(str_vec)
    }
}

from_val_to_enum!(UserValue String String);
from_val_to_enum!(UserValue DateTime DateTime<Utc>);
from_val_to_enum!(UserValue StringVec Vec<String>);
from_val_to_enum!(UserValue SemVer Version);
from_val_to_enum_into!(UserValue Float f64 f32);
from_val_to_enum_into!(UserValue UInt u8 u16 u32 u64);
from_val_to_enum_into!(UserValue Int i8 i16 i32 i64);
from_val_to_enum_into!(UserValue String &str);
