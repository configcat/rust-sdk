use std::fmt::{Display, Formatter};

#[derive(PartialEq, Debug)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(val) = self {
            return Some(*val);
        }
        None
    }

    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(val) = self {
            return Some(*val);
        }
        None
    }

    pub fn as_float(&self) -> Option<f64> {
        if let Value::Float(val) = self {
            return Some(*val);
        }
        None
    }

    pub fn as_str(&self) -> Option<String> {
        if let Value::String(val) = self {
            return Some(val.clone());
        }
        None
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
from_val_to_enum!(Value Bool bool);
from_val_to_enum!(Value Float f64);
from_val_to_enum!(Value Int i64);
