use crate::UserCondition;
use std::fmt::{Display, Formatter};

enum ConditionResult {
    Ok(bool),
    NoUser,
    AttrMissing(UserCondition),
    AttrInvalid(String, UserCondition),
    CompValInvalid(Option<String>),
    Fatal(String),
}

impl Display for ConditionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionResult::Ok(_) => f.write_str(""),
            ConditionResult::NoUser => f.write_str("cannot evaluate, User Object is missing"),
            ConditionResult::AttrMissing(cond) => write!(
                f,
                "cannot evaluate, the User.{} attribute is missing",
                cond.fmt_comp_attr()
            ),
            ConditionResult::AttrInvalid(reason, cond) => write!(
                f,
                "cannot evaluate, the User.{} attribute is invalid ({})",
                cond.fmt_comp_attr(),
                reason
            ),
            ConditionResult::CompValInvalid(err) => write!(
                f,
                "cannot evaluate, ({})",
                err.as_ref()
                    .unwrap_or(&"comparison value is missing or invalid".to_owned())
            ),
            ConditionResult::Fatal(err) => write!(f, "cannot evaluate ({})", err),
        }
    }
}
