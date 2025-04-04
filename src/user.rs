use crate::utils;
use chrono::{DateTime, Utc};
use semver::Version;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Index;

#[derive(Clone, Debug)]
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
///     .custom("Roles", ["Role1", "Role2"]);
///
/// assert_eq!("user-id", user[User::IDENTIFIER].to_string().as_str());
/// ```
#[derive(Serialize, Clone, Debug)]
pub struct User {
    attributes: HashMap<String, UserValue>,
}

impl User {
    /// The predefined attribute key of the user's identifier.
    pub const IDENTIFIER: &'static str = "Identifier";
    /// The predefined attribute key of the user's email.
    pub const EMAIL: &'static str = "Email";
    /// The predefined attribute key of the user's country.
    pub const COUNTRY: &'static str = "Country";

    /// Creates a new [`User`].
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
            attributes: HashMap::from([(Self::IDENTIFIER.to_owned(), UserValue::from(identifier))]),
        }
    }

    pub(crate) fn from_map(map: HashMap<String, UserValue>) -> Self {
        Self { attributes: map }
    }

    /// Sets the email address of the user.
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
        self.attributes.insert(Self::EMAIL.to_owned(), email.into());
        self
    }

    /// Sets the country of the user.
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
        self.attributes
            .insert(Self::COUNTRY.to_owned(), country.into());
        self
    }

    /// Sets a custom attribute of the user for advanced targeting rule definitions (e.g. user role, subscription type, etc.)
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
    ///     .custom("Roles", ["Role1", "Role2"]);
    /// ```
    pub fn custom<T: Into<UserValue>>(mut self, key: &str, value: T) -> Self {
        if key == Self::IDENTIFIER || key == Self::EMAIL || key == Self::COUNTRY {
            return self;
        }
        self.attributes.insert(key.to_owned(), value.into());
        self
    }

    /// Returns a user attribute's [`UserValue`] identified by the given `key`.
    ///
    /// If the attribute doesn't exist, [`None`] is returned.
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use configcat::User;
    ///
    /// let user = User::new("user-id");
    ///
    /// assert_eq!("user-id", user.get(User::IDENTIFIER).unwrap().to_string().as_str());
    /// ```
    pub fn get(&self, key: &str) -> Option<&UserValue> {
        self.attributes.get(key)
    }
}

impl UserValue {
    #![allow(clippy::cast_precision_loss)]
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
                } else if (1e-6..1e21).contains(&val.abs()) {
                    (val.to_string(), true)
                } else {
                    let sc = format!("{val:+e}");
                    if val.abs() > 1.0 {
                        (sc.replace('e', "e+"), true)
                    } else {
                        (sc, true)
                    }
                }
            }
            UserValue::SemVer(val) => (val.to_string(), true),
            UserValue::Int(val) => (val.to_string(), true),
            UserValue::UInt(val) => (val.to_string(), true),
            UserValue::DateTime(val) => {
                (((val.timestamp_millis() as f64) / 1000.0).to_string(), true)
            }
            UserValue::StringVec(val) => {
                let ser = serde_json::to_string(val);
                match ser {
                    Ok(val) => (val, true),
                    Err(_) => (String::default(), true),
                }
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn as_float(&self) -> Option<f64> {
        match self {
            UserValue::String(val) => {
                let trimmed = val.trim();
                match trimmed {
                    "Infinity" | "+Infinity" => Some(f64::INFINITY),
                    "-Infinity" => Some(f64::NEG_INFINITY),
                    "NaN" => Some(f64::NAN),
                    _ => trimmed.replace(',', ".").parse().ok(),
                }
            }
            UserValue::Int(val) => Some(*val as f64),
            UserValue::UInt(val) => Some(*val as f64),
            UserValue::Float(val) => Some(*val),
            _ => None,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn as_timestamp(&self) -> Option<f64> {
        match self {
            UserValue::DateTime(val) => Some((val.timestamp_millis() as f64) / 1000.0),
            _ => self.as_float(),
        }
    }

    pub(crate) fn as_semver(&self) -> Option<Version> {
        match self {
            UserValue::SemVer(val) => Some(val.clone()),
            UserValue::String(val) => utils::parse_semver(val).ok(),
            _ => None,
        }
    }

    pub(crate) fn as_str_vec(&self) -> Option<Vec<String>> {
        match self {
            UserValue::StringVec(val) => Some(val.clone()),
            UserValue::String(val) => {
                let result = serde_json::from_str::<Vec<String>>(val);
                result.ok()
            }
            _ => None,
        }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(&self.attributes) {
            Ok(str) => f.write_str(str.as_str()),
            Err(_) => f.write_str("<invalid user>"),
        }
    }
}

impl From<HashMap<String, UserValue>> for User {
    fn from(value: HashMap<String, UserValue>) -> Self {
        Self::from_map(value)
    }
}

impl Index<&str> for User {
    type Output = UserValue;

    fn index(&self, index: &str) -> &Self::Output {
        &self.attributes[index]
    }
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

impl From<Vec<&str>> for UserValue {
    fn from(value: Vec<&str>) -> Self {
        let str_vec = value.iter().map(|v| (*v).to_string()).collect();
        Self::StringVec(str_vec)
    }
}

impl<const N: usize> From<[&str; N]> for UserValue {
    fn from(arr: [&str; N]) -> Self {
        arr.to_vec().into()
    }
}

from_val_to_enum!(UserValue String String);
from_val_to_enum!(UserValue DateTime DateTime<Utc>);
from_val_to_enum!(UserValue SemVer Version);
from_val_to_enum!(UserValue StringVec Vec<String>);
from_val_to_enum_into!(UserValue Float f64 f32);
from_val_to_enum_into!(UserValue UInt u8 u16 u32 u64);
from_val_to_enum_into!(UserValue Int i8 i16 i32 i64);
from_val_to_enum_into!(UserValue String &str);
