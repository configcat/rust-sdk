use std::fmt::{Display, Formatter};

/// Represents the value of a feature flag or setting.
///
/// # Examples
///
/// ```rust
/// use configcat::Value;
///
/// let bool_val = Value::Bool(true);
/// let int_val = Value::Int(42);
/// ```
#[derive(PartialEq, Debug, Clone)]
pub enum Value {
    /// A bool feature flag's value.
    Bool(bool),
    /// A whole number setting's value.
    Int(i64),
    /// A decimal number setting's value.
    Float(f64),
    /// A text setting's value.
    String(String),
}

impl Value {
    /// Reads the value as `bool`. Returns [`None`] if it's not a [`Value::Bool`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::Value;
    ///
    /// let value = Value::Bool(true);
    /// assert!(value.as_bool().unwrap());
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(val) = self {
            return Some(*val);
        }
        None
    }

    /// Reads the value as `i64`. Returns [`None`] if it's not a [`Value::Int`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::Value;
    ///
    /// let value = Value::Int(42);
    /// assert_eq!(value.as_int().unwrap(), 42);
    /// ```
    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(val) = self {
            return Some(*val);
        }
        None
    }

    /// Reads the value as `f64`. Returns [`None`] if it's not a [`Value::Float`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::Value;
    ///
    /// let value = Value::Float(3.14);
    /// assert_eq!(value.as_float().unwrap(), 3.14);
    /// ```
    pub fn as_float(&self) -> Option<f64> {
        if let Value::Float(val) = self {
            return Some(*val);
        }
        None
    }

    /// Reads the value as [`String`]. Returns [`None`] if it's not a [`Value::String`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configcat::Value;
    ///
    /// let value = Value::String("foo".to_owned());
    /// assert_eq!(value.as_str().unwrap(), "foo".to_owned());
    /// ```
    pub fn as_str(&self) -> Option<String> {
        if let Value::String(val) = self {
            return Some(val.clone());
        }
        None
    }

    pub(crate) fn from_json(json_val: &serde_json::Value) -> Option<Value> {
        match json_val {
            serde_json::Value::Bool(val) => Some(Value::Bool(*val)),
            serde_json::Value::String(val) => Some(Value::String(val.clone())),
            serde_json::Value::Number(val) => {
                if let Some(int_val) = val.as_i64() {
                    return Some(Value::Int(int_val));
                }
                if let Some(float_val) = val.as_f64() {
                    return Some(Value::Float(float_val));
                }
                None
            }
            _ => None,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(val) => write!(f, "{val}"),
            Value::Int(val) => write!(f, "{val}"),
            Value::Float(val) => write!(f, "{val}"),
            Value::String(val) => f.write_str(val),
        }
    }
}

pub trait OptionalValueDisplay {
    fn to_str(&self) -> String;
}

impl OptionalValueDisplay for Option<Value> {
    fn to_str(&self) -> String {
        match self {
            None => "none".to_owned(),
            Some(value) => format!("{value}"),
        }
    }
}

from_val_to_enum!(Value String String);
from_val_to_enum_into!(Value String &str);
from_val_to_enum!(Value Bool bool);
from_val_to_enum!(Value Float f64);
from_val_to_enum!(Value Int i64);
