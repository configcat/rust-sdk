use crate::eval::evaluator::ConditionResult::*;
use crate::eval::evaluator::EvalResult::*;
use crate::eval::log_builder::EvalLogBuilder;
use crate::UserComparator::*;
use crate::{
    utils, Condition, PercentageOption, PrerequisiteFlagComparator, PrerequisiteFlagCondition,
    SegmentComparator, SegmentComparator::*, SegmentCondition, ServedValue, Setting, SettingValue,
    TargetingRule, User, UserComparator, UserCondition,
};
use log::{log_enabled, warn};
use semver::Version;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

macro_rules! eval_log_enabled {
    () => {
        log_enabled!(log::Level::Info)
    };
}

const RULE_IGNORED_MSG: &str =
    "The current targeting rule is ignored and the evaluation continues with the next rule.";
const SALT_MISSING_MSG: &str = "Config JSON salt is missing";
const INVALID_VALUE_TXT: &str = "<invalid value>";
const COMP_VAL_INVALID_MSG: &str = "Comparison value is missing or invalid";

pub enum EvalResult {
    Success(
        SettingValue,
        Option<String>,
        Option<Arc<TargetingRule>>,
        Option<Arc<PercentageOption>>,
    ),
    Error(String),
}

pub enum ConditionResult {
    Done(bool),
    NoUser,
    AttrMissing(String, String),
    AttrInvalid(String, String, String),
    CompValInvalid(Option<String>),
    Fatal(String),
}

impl ConditionResult {
    fn is_match(&self) -> bool {
        match self {
            Done(matched) => *matched,
            _ => false,
        }
    }

    fn is_ok(&self) -> bool {
        matches!(self, Done(_))
    }

    fn is_attr_miss(&self) -> bool {
        matches!(self, AttrMissing(_, _))
    }
}

impl Display for ConditionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Done(_) => f.write_str(""),
            NoUser => f.write_str("cannot evaluate, User Object is missing"),
            AttrMissing(attr, _) => {
                write!(f, "cannot evaluate, the User.{attr} attribute is missing")
            }
            AttrInvalid(reason, attr, _) => write!(
                f,
                "cannot evaluate, the User.{attr} attribute is invalid ({reason})"
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

fn eval_setting(
    setting: &Setting,
    key: &str,
    user: &Option<User>,
    log: &mut EvalLogBuilder,
    settings: &HashMap<String, Setting>,
    cycle_tracker: &mut Vec<String>,
) -> EvalResult {
    Error("".to_owned())
}

fn eval_conditions(
    conditions: &[Condition],
    rule_srv_value: &Option<ServedValue>,
    key: &str,
    user: &Option<User>,
    salt: &Option<String>,
    ctx_salt: &str,
    log: &mut EvalLogBuilder,
    settings: &HashMap<String, Setting>,
    cycle_tracker: &mut Vec<String>,
) -> ConditionResult {
    if eval_log_enabled!() {
        log.new_ln(Some("- "));
    }
    let mut new_line_before_then = false;
    for (index, condition) in conditions.iter().enumerate() {
        let mut cond_result = Fatal(
            "Condition isn't a type of user, segment, or prerequisite flag condition".to_owned(),
        );
        if eval_log_enabled!() {
            if index == 0 {
                log.append("IF ").inc_indent();
            } else {
                log.inc_indent().new_ln(Some("AND "));
            }
        }
        if let Some(user_condition) = condition.user_condition.as_ref() {
            if eval_log_enabled!() {
                log.append(format!("{user_condition}").as_str());
            }
            if let Some(user) = user {
                cond_result = eval_user_cond(user_condition, key, user, salt, ctx_salt);
            } else {
                cond_result = NoUser;
            }
            new_line_before_then = conditions.len() > 1;
        } else if let Some(segment_condition) = condition.segment_condition.as_ref() {
            if eval_log_enabled!() {
                log.append(format!("{segment_condition}").as_str());
            }
            if let Some(user) = user {
                cond_result = eval_segment_cond(segment_condition, key, user, salt, log);
            } else {
                cond_result = NoUser;
            }
            new_line_before_then =
                cond_result.is_ok() || cond_result.is_attr_miss() || conditions.len() > 1;
        } else if let Some(prerequisite_condition) = condition.prerequisite_flag_condition.as_ref()
        {
            cond_result = eval_prerequisite_cond(
                prerequisite_condition,
                key,
                user,
                log,
                settings,
                cycle_tracker,
            );
            new_line_before_then = true;
        }
        if eval_log_enabled!() {
            if conditions.len() > 1 {
                let res_msg = if cond_result.is_match() {
                    "true"
                } else {
                    "false"
                };
                let conclusion = if cond_result.is_match() {
                    ""
                } else {
                    ", skipping the remaining AND conditions"
                };
                log.append(format!(" => {res_msg}{conclusion}").as_str());
            }
            log.dec_indent();
        }
        let matched = match cond_result {
            Done(is_match) => is_match,
            _ => false,
        };
        if !matched {
            if eval_log_enabled!() {
                log.append_then_clause(new_line_before_then, &cond_result, rule_srv_value);
            }
            return cond_result;
        }
    }
    if eval_log_enabled!() {
        log.append_then_clause(new_line_before_then, &Done(true), rule_srv_value);
    }
    Done(true)
}

fn eval_prerequisite_cond(
    cond: &PrerequisiteFlagCondition,
    key: &str,
    user: &Option<User>,
    log: &mut EvalLogBuilder,
    settings: &HashMap<String, Setting>,
    cycle_tracker: &mut Vec<String>,
) -> ConditionResult {
    if eval_log_enabled!() {
        log.append(format!("{cond}").as_str());
    }
    let prerequisite = if let Some(prerequisite) = settings.get(&cond.flag_key) {
        prerequisite
    } else {
        return Fatal("Prerequisite flag is missing".to_owned());
    };
    if !cond.flag_value.is_valid(&prerequisite.setting_type) {
        return Fatal(format!(
            "Type mismatch between comparison value '{}' and prerequisite flag '{}'",
            cond.flag_value, cond.flag_key
        ));
    }

    cycle_tracker.push(key.to_owned());
    if cycle_tracker.contains(&cond.flag_key) {
        cycle_tracker.push(cond.flag_key.clone());
        let output = cycle_tracker
            .iter()
            .map(|k| format!("'{k}'"))
            .collect::<Vec<String>>()
            .join(" => ");
        return Fatal(output);
    }

    let needs_true = cond.prerequisite_comparator == PrerequisiteFlagComparator::Eq;
    if eval_log_enabled!() {
        log.new_ln(Some("(")).inc_indent().new_ln(Some(
            format!("Evaluating prerequisite flag '{}':", cond.flag_key).as_str(),
        ));
    }

    let result = eval_setting(
        prerequisite,
        cond.flag_key.as_str(),
        user,
        log,
        settings,
        cycle_tracker,
    );
    cycle_tracker.pop();

    match result {
        Success(sv, _, _, _) => {
            let matched = needs_true == cond.flag_value.eq_by_type(&sv, &prerequisite.setting_type);
            if eval_log_enabled!() {
                let msg = if matched { "true" } else { "false" };
                log.new_ln(Some(
                    format!("Condition ({cond}) evaluates to {msg}.").as_str(),
                ))
                .dec_indent()
                .new_ln(Some(")"));
            }
            Done(matched)
        }
        Error(err) => Fatal(err),
    }
}

fn eval_segment_cond(
    cond: &SegmentCondition,
    key: &str,
    user: &User,
    salt: &Option<String>,
    log: &mut EvalLogBuilder,
) -> ConditionResult {
    let segment = if let Some(segment) = cond.segment.as_ref() {
        segment
    } else {
        return Fatal("Segment reference is invalid".to_owned());
    };

    if eval_log_enabled!() {
        log.new_ln(Some("(")).inc_indent().new_ln(Some(
            format!("Evaluating segment '{}':", segment.name).as_str(),
        ));
    }

    let mut result = Fatal(String::default());
    let needs_true = cond.segment_comparator == IsIn;

    for (index, user_condition) in segment.conditions.iter().enumerate() {
        if eval_log_enabled!() {
            log.new_ln(Some("- "));
            if index == 0 {
                log.append("IF ").inc_indent();
            } else {
                log.inc_indent().new_ln(Some("AND "));
            }
            log.append(format!("{user_condition}").as_str());
        }
        result = eval_user_cond(user_condition, key, user, salt, &segment.name);
        if eval_log_enabled!() {
            let end = if result.is_match() {
                ""
            } else {
                ", skipping the remaining AND conditions"
            };
            let match_msg = if result.is_match() { "true" } else { "false" };
            log.append(" = >")
                .append(match_msg)
                .append(end)
                .dec_indent();
        }
        if !result.is_ok() || !result.is_match() {
            break;
        }
    }
    if eval_log_enabled!() {
        log.new_ln(Some("Segment evaluation result: "));
        if result.is_ok() {
            let msg = if result.is_match() {
                format!("{}", IsIn)
            } else {
                format!("{}", IsNotIn)
            };
            log.append(format!("User {msg}.").as_str());
        } else {
            log.append(format!("{result}.").as_str());
        }
        log.new_ln(Some("Condition ("))
            .append(format!("{cond}").as_str())
            .append(")");
        if !result.is_ok() {
            log.append(" failed to evaluate.");
        } else {
            let msg = if result.is_match() == needs_true {
                "true"
            } else {
                "false"
            };
            log.append(format!("evaluates to {msg}.").as_str());
        }
        log.dec_indent().new_ln(Some(")"));
    }
    match result {
        Done(matched) => Done(matched == needs_true),
        _ => result,
    }
}

fn eval_user_cond(
    cond: &UserCondition,
    key: &str,
    user: &User,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let user_attr = if let Some(user_attr) = user.get(&cond.comp_attr) {
        user_attr
    } else {
        return AttrMissing(cond.comp_attr.clone(), format!("{cond}"));
    };
    return match cond.comparator {
        Eq | NotEq | EqHashed | NotEqHashed => {
            let comp_val = if let Some(comp_val) = cond.string_val.as_ref() {
                comp_val
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
            let comp_val = if let Some(comp_val) = cond.string_vec_val.as_ref() {
                comp_val
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
            let comp_val = if let Some(comp_val) = cond.string_vec_val.as_ref() {
                comp_val
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
            let comp_val = if let Some(comp_val) = cond.string_vec_val.as_ref() {
                comp_val
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
            let comp_val = if let Some(comp_val) = cond.string_vec_val.as_ref() {
                comp_val
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(user_val) = user_attr.as_semver() {
                user_val
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid semantic version"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
                );
            };
            eval_semver_is_one_of(comp_val, user_val, &cond.comparator)
        }
        GreaterSemver | GreaterEqSemver | LessSemver | LessEqSemver => {
            let comp_val = if let Some(comp_val) = cond.string_val.as_ref() {
                comp_val
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(user_val) = user_attr.as_semver() {
                user_val
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid semantic version"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
                );
            };
            eval_semver_compare(comp_val, user_val, &cond.comparator)
        }
        EqNum | NotEqNum | GreaterNum | GreaterEqNum | LessNum | LessEqNum => {
            let comp_val = if let Some(comp_val) = cond.double_val.as_ref() {
                comp_val
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(user_val) = user_attr.as_float() {
                user_val
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid decimal number"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
                );
            };
            eval_number_compare(comp_val, &user_val, &cond.comparator)
        }
        BeforeDateTime | AfterDateTime => {
            let comp_val = if let Some(comp_val) = cond.double_val.as_ref() {
                comp_val
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(user_val) = user_attr.as_timestamp() {
                user_val
            } else {
                return AttrInvalid(format!("{user_attr} is not a valid Unix timestamp (number of seconds elapsed since Unix epoch)"),
                                   cond.comp_attr.clone(),
                                   format!("{cond}")
                );
            };
            eval_date(comp_val, &user_val, &cond.comparator)
        }
        ArrayContainsAnyOf
        | ArrayNotContainsAnyOf
        | ArrayContainsAnyOfHashed
        | ArrayNotContainsAnyOfHashed => {
            let comp_val = if let Some(comp_val) = cond.string_vec_val.as_ref() {
                comp_val
            } else {
                return CompValInvalid(None);
            };
            let user_val = if let Some(user_val) = user_attr.as_str_vec() {
                user_val
            } else {
                return AttrInvalid(
                    format!("{user_attr} is not a valid string vector"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
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
    Done((comp_val == usr_v) == needs_true)
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
            return Done(needs_true);
        }
    }
    Done(!needs_true)
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
            let length = if let Ok(lg) = parts[0].trim().parse::<usize>() {
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
                    return Done(needs_true);
                }
            } else {
                let chunk = &user_val_ref[(user_val_len - length)..];
                if utils::sha256(chunk, st, ctx_salt) == parts[1] {
                    return Done(needs_true);
                }
            }
        } else {
            let condition = if comp.is_starts_with() {
                user_val.starts_with(item.as_str())
            } else {
                user_val.ends_with(item.as_str())
            };
            if condition {
                return Done(needs_true);
            }
        }
    }
    Done(!needs_true)
}

fn eval_contains(comp_val: &[String], user_val: String, comp: &UserComparator) -> ConditionResult {
    let needs_true = *comp == Contains;
    for item in comp_val.iter() {
        if user_val.contains(item) {
            return Done(needs_true);
        }
    }
    Done(!needs_true)
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
        let comp_ver = if let Ok(ver) = Version::parse(trimmed) {
            ver
        } else {
            // NOTE: Previous versions of the evaluation algorithm ignored invalid comparison values.
            // We keep this behavior for backward compatibility.
            return Done(false);
        };
        if user_val.eq(&comp_ver) {
            matched = true;
        }
    }
    Done(matched == needs_true)
}

fn eval_semver_compare(
    comp_val: &str,
    user_val: Version,
    comp: &UserComparator,
) -> ConditionResult {
    let comp_ver = if let Ok(ver) = Version::parse(comp_val) {
        ver
    } else {
        // NOTE: Previous versions of the evaluation algorithm ignored invalid comparison values.
        // We keep this behavior for backward compatibility.
        return Done(false);
    };
    match comp {
        GreaterSemver => Done(user_val.gt(&comp_ver)),
        GreaterEqSemver => Done(user_val.ge(&comp_ver)),
        LessSemver => Done(user_val.lt(&comp_ver)),
        LessEqSemver => Done(user_val.le(&comp_ver)),
        _ => Fatal("wrong semver comparator".to_owned()),
    }
}

fn eval_number_compare(comp_val: &f64, user_val: &f64, comp: &UserComparator) -> ConditionResult {
    match comp {
        EqNum => Done(user_val == comp_val),
        NotEqNum => Done(user_val != comp_val),
        GreaterNum => Done(user_val > comp_val),
        GreaterEqNum => Done(user_val >= comp_val),
        LessNum => Done(user_val < comp_val),
        LessEqNum => Done(user_val <= comp_val),
        _ => Fatal("wrong number comparator".to_owned()),
    }
}

fn eval_date(comp_val: &f64, user_val: &f64, comp: &UserComparator) -> ConditionResult {
    match comp {
        BeforeDateTime => Done(user_val < comp_val),
        _ => Done(user_val > comp_val),
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
                    return Done(needs_true);
                }
            }
        }
        for comp_item in comp_val.iter() {
            if user_item == comp_item {
                return Done(needs_true);
            }
        }
    }
    Done(!needs_true)
}

fn log_conv(cond: &UserCondition, key: &str, attr_val: &str) {
    warn!(event_id = 3005; "Evaluation of condition ({cond}) for setting '{key}' may not produce the expected result (the User.{} attribute is not a string value, thus it was automatically converted to the string value '{attr_val}'). Please make sure that using a non-string value was intended.", cond.comp_attr)
}
