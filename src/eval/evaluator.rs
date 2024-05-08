use crate::eval::evaluator::ConditionResult::*;
use crate::UserComparator::*;
use crate::{utils, User, UserComparator, UserCondition};
use log::warn;
use semver::Version;
use std::fmt::{Display, Formatter};

const RULE_IGNORED_MSG: &str =
    "The current targeting rule is ignored and the evaluation continues with the next rule.";
const SALT_MISSING_MSG: &str = "Config JSON salt is missing";
const INVALID_VALUE_TXT: &str = "<invalid value>";
const COMP_VAL_INVALID_MSG: &str = "Comparison value is missing or invalid";

pub enum ConditionResult {
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
            Ok(_) => f.write_str(""),
            NoUser => f.write_str("cannot evaluate, User Object is missing"),
            AttrMissing(cond) => write!(
                f,
                "cannot evaluate, the User.{} attribute is missing",
                cond.fmt_comp_attr()
            ),
            AttrInvalid(reason, cond) => write!(
                f,
                "cannot evaluate, the User.{} attribute is invalid ({reason})",
                cond.fmt_comp_attr()
            ),
            CompValInvalid(err) => write!(
                f,
                "cannot evaluate, ({})",
                err.as_ref()
                    .unwrap_or(&"comparison value is missing or invalid".to_owned())
            ),
            Fatal(err) => write!(f, "cannot evaluate ({err})"),
        }
    }
}

fn eval_user_cond(
    cond: &UserCondition,
    key: &str,
    user: &User,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let comp_attr = if let Some(cmp_a) = cond.comp_attr.as_ref() {
        cmp_a
    } else {
        return Fatal("Comparison attribute is missing".to_owned());
    };
    let user_attr = if let Some(user_attr) = user.get(comp_attr) {
        user_attr
    } else {
        return AttrMissing(cond.clone());
    };
    return match cond.comparator {
        Eq | NotEq | EqHashed | NotEqHashed => {
            let comp_val = if let Some(cmp_v) = cond.string_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_text_eq(comp_val, user_val, &cond.comparator, salt, ctx_salt)
        }
        OneOf | NotOneOf | OneOfHashed | NotOneOfHashed => {
            let comp_val = if let Some(cmp_v) = cond.string_vec_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_one_of(comp_val, user_val, &cond.comparator, salt, ctx_salt)
        }
        StartsWithAnyOf
        | StartsWithAnyOfHashed
        | NotStartsWithAnyOf
        | NotStartsWithAnyOfHashed
        | EndsWithAnyOf
        | NotEndsWithAnyOf
        | EndsWithAnyOfHashed
        | NotEndsWithAnyOfHashed => {
            let comp_val = if let Some(cmp_v) = cond.string_vec_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_starts_ends_with(comp_val, user_val, &cond.comparator, salt, ctx_salt)
        }
        Contains | NotContains => {
            let comp_val = if let Some(cmp_v) = cond.string_vec_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_contains(comp_val, user_val, &cond.comparator)
        }
        OneOfSemver | NotOneOfSemver => {
            let comp_val = if let Some(cmp_v) = cond.string_vec_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(usr_v) = user_attr.as_semver() {
                usr_v
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid semantic version"),
                    cond.clone(),
                );
            };
            eval_semver_is_one_of(comp_val, user_val, &cond.comparator)
        }
        GreaterSemver | GreaterEqSemver | LessSemver | LessEqSemver => {
            let comp_val = if let Some(cmp_v) = cond.string_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(usr_v) = user_attr.as_semver() {
                usr_v
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid semantic version"),
                    cond.clone(),
                );
            };
            eval_semver_compare(comp_val, user_val, &cond.comparator)
        }
        EqNum | NotEqNum | GreaterNum | GreaterEqNum | LessNum | LessEqNum => {
            let comp_val = if let Some(cmp_v) = cond.double_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(usr_v) = user_attr.as_float() {
                usr_v
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid decimal number"),
                    cond.clone(),
                );
            };
            eval_number_compare(comp_val, &user_val, &cond.comparator)
        }
        BeforeDateTime | AfterDateTime => {
            let comp_val = if let Some(cmp_v) = cond.double_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(usr_v) = user_attr.as_timestamp() {
                usr_v
            } else {
                return AttrInvalid(format!("{user_attr} is not a valid Unix timestamp (number of seconds elapsed since Unix epoch)"), cond.clone());
            };
            eval_date(comp_val, &user_val, &cond.comparator)
        }
        ArrayContainsAnyOf
        | ArrayNotContainsAnyOf
        | ArrayContainsAnyOfHashed
        | ArrayNotContainsAnyOfHashed => {
            let comp_val = if let Some(cmp_v) = cond.string_vec_val.as_ref() {
                cmp_v
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(usr_v) = user_attr.as_str_vec() {
                usr_v
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid string vector"),
                    cond.clone(),
                );
            };
            eval_array_contains(comp_val, &user_val, &cond.comparator, salt, ctx_salt)
        }
    };
}

fn eval_text_eq(
    comp_val: &str,
    user_val: String,
    comp: &UserComparator,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let needs_true = if comp.is_sensitive() {
        *comp == EqHashed
    } else {
        *comp == Eq
    };
    let mut usr_v = user_val;
    if comp.is_sensitive() {
        let st = if let Some(st) = salt {
            st
        } else {
            return Fatal(SALT_MISSING_MSG.to_owned());
        };
        usr_v = utils::sha256(usr_v.as_str(), st.as_str(), ctx_salt);
    }
    Ok((comp_val == usr_v) == needs_true)
}

fn eval_one_of(
    comp_val: &[String],
    user_val: String,
    comp: &UserComparator,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let needs_true = if comp.is_sensitive() {
        *comp == OneOfHashed
    } else {
        *comp == OneOf
    };
    let mut usr_v = user_val;
    if comp.is_sensitive() {
        let st = if let Some(st) = salt {
            st
        } else {
            return Fatal(SALT_MISSING_MSG.to_owned());
        };
        usr_v = utils::sha256(usr_v.as_str(), st.as_str(), ctx_salt);
    }
    for item in comp_val.iter() {
        if *item == usr_v {
            return Ok(needs_true);
        }
    }
    Ok(!needs_true)
}

fn eval_starts_ends_with(
    comp_val: &[String],
    user_val: String,
    comp: &UserComparator,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let needs_true = if comp.is_sensitive() {
        if comp.is_sensitive() {
            *comp == StartsWithAnyOfHashed
        } else {
            *comp == StartsWithAnyOf
        }
    } else if comp.is_sensitive() {
        *comp == EndsWithAnyOfHashed
    } else {
        *comp == EndsWithAnyOf
    };
    let user_val_len = user_val.len();
    let user_val_ref = user_val.as_str();
    for item in comp_val.iter() {
        if comp.is_sensitive() {
            let st = if let Some(st) = salt {
                st
            } else {
                return Fatal(SALT_MISSING_MSG.to_owned());
            };
            let parts = item.split('_').collect::<Vec<&str>>();
            if parts.len() < 2 || parts[1].is_empty() {
                return Fatal(COMP_VAL_INVALID_MSG.to_owned());
            }
            let length = if let Result::Ok(lg) = parts[0].trim().parse::<usize>() {
                lg
            } else {
                return Fatal(COMP_VAL_INVALID_MSG.to_owned());
            };
            if length > user_val_len {
                continue;
            }
            if comp.is_starts_with() {
                let chunk = &user_val_ref[..length];
                if utils::sha256(chunk, st, ctx_salt) == parts[1] {
                    return Ok(needs_true);
                }
            } else {
                let chunk = &user_val_ref[(user_val_len - length)..];
                if utils::sha256(chunk, st, ctx_salt) == parts[1] {
                    return Ok(needs_true);
                }
            }
        } else {
            let condition = if comp.is_starts_with() {
                user_val.starts_with(item.as_str())
            } else {
                user_val.ends_with(item.as_str())
            };
            if condition {
                return Ok(needs_true);
            }
        }
    }
    Ok(!needs_true)
}

fn eval_contains(comp_val: &[String], user_val: String, comp: &UserComparator) -> ConditionResult {
    let needs_true = *comp == Contains;
    for item in comp_val.iter() {
        if user_val.contains(item) {
            return Ok(needs_true);
        }
    }
    Ok(!needs_true)
}

fn eval_semver_is_one_of(
    comp_val: &[String],
    user_val: Version,
    comp: &UserComparator,
) -> ConditionResult {
    let needs_true = *comp == OneOfSemver;
    let mut matched = false;
    for item in comp_val.iter() {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let comp_ver = if let Result::Ok(ver) = Version::parse(trimmed) {
            ver
        } else {
            // NOTE: Previous versions of the evaluation algorithm ignored invalid comparison values.
            // We keep this behavior for backward compatibility.
            return Ok(false);
        };
        if user_val.eq(&comp_ver) {
            matched = true;
        }
    }
    Ok(matched == needs_true)
}

fn eval_semver_compare(
    comp_val: &str,
    user_val: Version,
    comp: &UserComparator,
) -> ConditionResult {
    let comp_ver = if let Result::Ok(ver) = Version::parse(comp_val) {
        ver
    } else {
        // NOTE: Previous versions of the evaluation algorithm ignored invalid comparison values.
        // We keep this behavior for backward compatibility.
        return Ok(false);
    };
    match comp {
        GreaterSemver => Ok(user_val.gt(&comp_ver)),
        GreaterEqSemver => Ok(user_val.ge(&comp_ver)),
        LessSemver => Ok(user_val.lt(&comp_ver)),
        LessEqSemver => Ok(user_val.le(&comp_ver)),
        _ => Fatal("wrong semver comparator".to_owned()),
    }
}

fn eval_number_compare(comp_val: &f64, user_val: &f64, comp: &UserComparator) -> ConditionResult {
    match comp {
        EqNum => Ok(user_val == comp_val),
        NotEqNum => Ok(user_val != comp_val),
        GreaterNum => Ok(user_val > comp_val),
        GreaterEqNum => Ok(user_val >= comp_val),
        LessNum => Ok(user_val < comp_val),
        LessEqNum => Ok(user_val <= comp_val),
        _ => Fatal("wrong number comparator".to_owned()),
    }
}

fn eval_date(comp_val: &f64, user_val: &f64, comp: &UserComparator) -> ConditionResult {
    match comp {
        BeforeDateTime => Ok(user_val < comp_val),
        _ => Ok(user_val > comp_val),
    }
}

fn eval_array_contains(
    comp_val: &[String],
    user_val: &[String],
    comp: &UserComparator,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let needs_true = if comp.is_sensitive() {
        *comp == ArrayContainsAnyOfHashed
    } else {
        *comp == ArrayContainsAnyOf
    };
    for user_item in user_val.iter() {
        if comp.is_sensitive() {
            let st = if let Some(st) = salt {
                st
            } else {
                return Fatal(SALT_MISSING_MSG.to_owned());
            };
            let user_hashed = utils::sha256(user_item.as_str(), st.as_str(), ctx_salt);
            for comp_item in comp_val.iter() {
                if user_hashed == *comp_item {
                    return Ok(needs_true);
                }
            }
        }
        for comp_item in comp_val.iter() {
            if user_item == comp_item {
                return Ok(needs_true);
            }
        }
    }
    Ok(!needs_true)
}

fn log_conv(cond: &UserCondition, key: &str, attr_val: &str) {
    warn!(event_id = 3005; "Evaluation of condition ({cond}) for setting '{key}' may not produce the expected result (the User.{} attribute is not a string value, thus it was automatically converted to the string value '{attr_val}'). Please make sure that using a non-string value was intended.", cond.fmt_comp_attr())
}
