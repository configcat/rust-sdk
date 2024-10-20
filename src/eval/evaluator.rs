use crate::eval::evaluator::ConditionResult::{
    AttrInvalid, AttrMissing, CompValInvalid, Fatal, NoUser, Success,
};
use crate::eval::log_builder::EvalLogBuilder;
use crate::value::{OptionalValueDisplay, Value};
use crate::UserComparator::{
    AfterDateTime, ArrayContainsAnyOf, ArrayContainsAnyOfHashed, ArrayNotContainsAnyOf,
    ArrayNotContainsAnyOfHashed, BeforeDateTime, Contains, EndsWithAnyOf, EndsWithAnyOfHashed, Eq,
    EqHashed, EqNum, GreaterEqNum, GreaterEqSemver, GreaterNum, GreaterSemver, LessEqNum,
    LessEqSemver, LessNum, LessSemver, NotContains, NotEndsWithAnyOf, NotEndsWithAnyOfHashed,
    NotEq, NotEqHashed, NotEqNum, NotOneOf, NotOneOfHashed, NotOneOfSemver, NotStartsWithAnyOf,
    NotStartsWithAnyOfHashed, OneOf, OneOfHashed, OneOfSemver, StartsWithAnyOf,
    StartsWithAnyOfHashed,
};
use crate::{
    utils, Condition, PercentageOption, PrerequisiteFlagComparator, PrerequisiteFlagCondition,
    SegmentComparator::{IsIn, IsNotIn},
    SegmentCondition, ServedValue, Setting, SettingType, SettingValue, TargetingRule, User,
    UserComparator, UserCondition,
};
use log::{info, log_enabled, warn};
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
const COMP_VAL_INVALID_MSG: &str = "Comparison value is missing or invalid";
const SETTING_VAL_INVALID_MSG: &str = "Setting value is missing or invalid";
const IDENTIFIER_ATTR: &str = "Identifier";

pub struct EvalResult {
    pub value: Value,
    pub variation_id: Option<String>,
    pub rule: Option<Arc<TargetingRule>>,
    pub option: Option<Arc<PercentageOption>>,
    pub setting_type: SettingType,
}

pub enum PercentageResult {
    Success(Arc<PercentageOption>),
    UserAttrMissing(String),
    Fatal(String),
}

pub enum ConditionResult {
    Success(bool),
    NoUser,
    AttrMissing(String, String),
    AttrInvalid(String, String, String),
    CompValInvalid(Option<String>),
    Fatal(String),
}

impl ConditionResult {
    fn is_match(&self) -> bool {
        match self {
            Success(matched) => *matched,
            _ => false,
        }
    }

    fn is_success(&self) -> bool {
        matches!(self, Success(_))
    }

    fn is_attr_miss(&self) -> bool {
        matches!(self, AttrMissing(_, _))
    }
}

impl Display for ConditionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Success(_) => f.write_str(""),
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

pub fn eval(
    setting: &Setting,
    key: &str,
    user: &Option<User>,
    settings: &HashMap<String, Setting>,
    default: &Option<Value>,
) -> Result<EvalResult, String> {
    let mut eval_log = EvalLogBuilder::default();
    let mut cycle_tracker = Vec::<String>::default();
    if eval_log_enabled!() {
        eval_log.append(format!("Evaluating '{key}'").as_str());
        if let Some(user) = user {
            eval_log.append(format!(" for User '{user}'").as_str());
        }
        eval_log.inc_indent();
    }
    let result = eval_setting(
        setting,
        key,
        user,
        settings,
        &mut eval_log,
        &mut cycle_tracker,
    );
    if eval_log_enabled!() {
        if let Ok(res) = &result {
            eval_log.new_ln(Some(format!("Returning '{}'.", res.value).as_str()));
        } else {
            eval_log
                .reset_indent()
                .inc_indent()
                .new_ln(Some(format!("Returning '{}'.", default.to_str()).as_str()));
        }
        eval_log.dec_indent();
        info!(event_id = 5000; "{}", eval_log.content());
    }
    result
}

#[allow(clippy::too_many_lines)]
fn eval_setting(
    setting: &Setting,
    key: &str,
    user: &Option<User>,
    settings: &HashMap<String, Setting>,
    log: &mut EvalLogBuilder,
    cycle_tracker: &mut Vec<String>,
) -> Result<EvalResult, String> {
    let mut user_missing_logged = false;
    if let Some(targeting_rules) = setting.targeting_rules.as_ref() {
        if eval_log_enabled!() {
            log.new_ln(Some(
                "Evaluating targeting rules and applying the first match if any:",
            ));
        }
        for rule in targeting_rules {
            if let Some(conditions) = rule.conditions.as_ref() {
                let result = eval_conditions(
                    conditions,
                    &rule.served_value,
                    key,
                    user,
                    &setting.salt,
                    key,
                    log,
                    settings,
                    cycle_tracker,
                );
                if eval_log_enabled!() && !result.is_success() {
                    log.inc_indent().new_ln(Some(RULE_IGNORED_MSG)).dec_indent();
                }
                match result {
                    Success(true) => {
                        if let Some(served_val) = rule.served_value.as_ref() {
                            return produce_result(
                                &served_val.value,
                                &setting.setting_type,
                                &served_val.variation_id,
                                Some(rule.clone()),
                                None,
                            );
                        }
                        if eval_log_enabled!() {
                            log.inc_indent();
                        }
                        match rule.percentage_options.as_ref() {
                            Some(percentage_opts) => if let Some(u) = user {
                                let percentage_result = eval_percentage(
                                    percentage_opts,
                                    u,
                                    key,
                                    &setting.percentage_attribute,
                                    log,
                                );
                                match percentage_result {
                                    PercentageResult::Success(opt) => {
                                        if eval_log_enabled!() {
                                            log.dec_indent();
                                        }
                                        return produce_result(
                                            &opt.served_value,
                                            &setting.setting_type,
                                            &opt.variation_id,
                                            Some(rule.clone()),
                                            Some(opt.clone()),
                                        );
                                    }
                                    PercentageResult::UserAttrMissing(attr) => {
                                        log_attr_missing_percentage(key, attr.as_str());
                                    }
                                    PercentageResult::Fatal(err) => return Err(err),
                                }
                            } else {
                                if !user_missing_logged {
                                    user_missing_logged = true;
                                    log_user_missing(key);
                                }
                                if eval_log_enabled!() {
                                    log.new_ln(Some("Skipping % options because the User Object is missing."));
                                }
                            },
                            None => {
                                return Err(
                                    "Targeting rule THEN part is missing or invalid".to_owned()
                                )
                            }
                        }
                        if eval_log_enabled!() {
                            log.new_ln(Some(RULE_IGNORED_MSG)).dec_indent();
                        }
                    }
                    Success(false) => continue,
                    Fatal(err) => return Err(err),
                    NoUser => {
                        if !user_missing_logged {
                            user_missing_logged = true;
                            log_user_missing(key);
                        }
                        continue;
                    }
                    AttrMissing(attr, cond_str) => {
                        log_attr_missing(key, attr.as_str(), cond_str.as_str());
                        continue;
                    }
                    AttrInvalid(reason, attr, cond_str) => {
                        log_attr_invalid(key, attr.as_str(), reason.as_str(), cond_str.as_str());
                        continue;
                    }
                    CompValInvalid(error) => {
                        return match error {
                            None => Err("Comparison value is missing or invalid".to_owned()),
                            Some(err) => Err(err),
                        }
                    }
                }
            }
        }
    }

    if let Some(percentage_opts) = setting.percentage_options.as_ref() {
        if let Some(u) = user {
            let percentage_result =
                eval_percentage(percentage_opts, u, key, &setting.percentage_attribute, log);
            match percentage_result {
                PercentageResult::Success(opt) => {
                    return produce_result(
                        &opt.served_value,
                        &setting.setting_type,
                        &opt.variation_id,
                        None,
                        Some(opt.clone()),
                    );
                }
                PercentageResult::UserAttrMissing(attr) => {
                    log_attr_missing_percentage(key, attr.as_str());
                }
                PercentageResult::Fatal(err) => return Err(err),
            }
        } else {
            if !user_missing_logged {
                log_user_missing(key);
            }
            if eval_log_enabled!() {
                log.new_ln(Some(
                    "Skipping % options because the User Object is missing.",
                ));
            }
        }
    }
    produce_result(
        &setting.value,
        &setting.setting_type,
        &setting.variation_id,
        None,
        None,
    )
}

fn produce_result(
    sv: &SettingValue,
    setting_type: &SettingType,
    variation: &Option<String>,
    rule: Option<Arc<TargetingRule>>,
    option: Option<Arc<PercentageOption>>,
) -> Result<EvalResult, String> {
    if let Some(value) = sv.as_val(setting_type) {
        return Ok(EvalResult {
            value,
            rule,
            option,
            variation_id: variation.clone(),
            setting_type: setting_type.clone(),
        });
    }
    Err(SETTING_VAL_INVALID_MSG.to_owned())
}

fn eval_percentage(
    opts: &[Arc<PercentageOption>],
    user: &User,
    key: &str,
    percentage_attr: &Option<String>,
    log: &mut EvalLogBuilder,
) -> PercentageResult {
    let attr = if let Some(percentage_attr) = percentage_attr {
        percentage_attr
    } else {
        IDENTIFIER_ATTR
    };
    let Some(user_attr) = user.get(attr) else {
        if eval_log_enabled!() {
            log.new_ln(Some(
                format!("Skipping % options because the User.{attr} attribute is missing.")
                    .as_str(),
            ));
        }
        return PercentageResult::UserAttrMissing(attr.to_owned());
    };
    if eval_log_enabled!() {
        log.new_ln(Some(
            format!("Evaluating % options based on the User.{attr} attribute:").as_str(),
        ));
    }
    let (str_attr_val, _) = user_attr.as_str();
    let mut hash_candidate = String::with_capacity(key.len() + str_attr_val.len());
    hash_candidate.push_str(key);
    hash_candidate.push_str(str_attr_val.as_str());
    let hash = &utils::sha1(hash_candidate.as_str())[..7];
    if let Ok(num) = i64::from_str_radix(hash, 16) {
        let scaled = num % 100;
        if eval_log_enabled!() {
            log.new_ln(Some(format!("- Computing hash in the [0..99] range from User.{attr} => {scaled} (this value is sticky and consistent across all SDKs)").as_str()));
        }
        let mut bucket = 0;
        for (index, opt) in opts.iter().enumerate() {
            bucket += opt.percentage;
            if scaled < bucket {
                if eval_log_enabled!() {
                    log.new_ln(Some(
                        format!(
                            "- Hash value {scaled} selects % option {} ({}%), '{}'.",
                            index + 1,
                            opt.percentage,
                            opt.served_value
                        )
                        .as_str(),
                    ));
                }
                return PercentageResult::Success(opt.clone());
            }
        }
    }
    PercentageResult::Fatal("Sum of percentage option percentages is less than 100".to_owned())
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
                cond_result.is_success() || cond_result.is_attr_miss() || conditions.len() > 1;
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
                let res_msg = format!("{}", cond_result.is_match());
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
            Success(is_match) => is_match,
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
        log.append_then_clause(new_line_before_then, &Success(true), rule_srv_value);
    }
    Success(true)
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
    let Some(prerequisite) = settings.get(&cond.flag_key) else {
        return Fatal("Prerequisite flag is missing".to_owned());
    };
    let Some(checked) = cond.flag_value.as_val(&prerequisite.setting_type) else {
        return Fatal(format!(
            "Type mismatch between comparison value '{}' and prerequisite flag '{}'",
            cond.flag_value, cond.flag_key
        ));
    };

    cycle_tracker.push(key.to_owned());
    if cycle_tracker.contains(&cond.flag_key) {
        cycle_tracker.push(cond.flag_key.clone());
        let output = cycle_tracker
            .iter()
            .map(|k| format!("'{k}'"))
            .collect::<Vec<String>>()
            .join(" -> ");
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
        settings,
        log,
        cycle_tracker,
    );
    cycle_tracker.pop();

    match result {
        Ok(result) => {
            let matched = needs_true == (result.value == checked);
            if eval_log_enabled!() {
                let msg = format!("{matched}");
                log.new_ln(Some(
                    format!("Prerequisite flag evaluation result: '{}'.", result.value).as_str(),
                ))
                .new_ln(Some(
                    format!("Condition ({cond}) evaluates to {msg}.").as_str(),
                ))
                .dec_indent()
                .new_ln(Some(")"));
            }
            Success(matched)
        }
        Err(err) => Fatal(err),
    }
}

fn eval_segment_cond(
    cond: &SegmentCondition,
    key: &str,
    user: &User,
    salt: &Option<String>,
    log: &mut EvalLogBuilder,
) -> ConditionResult {
    let Some(segment) = cond.segment.as_ref() else {
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
        result = eval_user_cond(user_condition, key, user, salt, segment.name.as_str());
        if eval_log_enabled!() {
            let end = if result.is_match() {
                ""
            } else {
                ", skipping the remaining AND conditions"
            };
            let match_msg = format!("{}", result.is_match());
            log.append(" => ")
                .append(match_msg.as_str())
                .append(end)
                .dec_indent();
        }
        if !result.is_success() || !result.is_match() {
            break;
        }
    }
    if eval_log_enabled!() {
        log.new_ln(Some("Segment evaluation result: "));
        if result.is_success() {
            let msg = if result.is_match() {
                format!("{IsIn}")
            } else {
                format!("{IsNotIn}")
            };
            log.append(format!("User {msg}.").as_str());
        } else {
            log.append(format!("{result}.").as_str());
        }
        log.new_ln(Some("Condition ("))
            .append(format!("{cond}").as_str())
            .append(")");
        if result.is_success() {
            let msg = format!("{}", result.is_match() == needs_true);
            log.append(format!(" evaluates to {msg}.").as_str());
        } else {
            log.append(" failed to evaluate.");
        }
        log.dec_indent().new_ln(Some(")"));
    }
    match result {
        Success(matched) => Success(matched == needs_true),
        _ => result,
    }
}

#[allow(clippy::too_many_lines)]
fn eval_user_cond(
    cond: &UserCondition,
    key: &str,
    user: &User,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let Some(user_attr) = user.get(&cond.comp_attr) else {
        return AttrMissing(cond.comp_attr.clone(), format!("{cond}"));
    };
    return match cond.comparator {
        Eq | NotEq | EqHashed | NotEqHashed => {
            let Some(comp_val) = cond.string_val.as_ref() else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_text_eq(comp_val, user_val, &cond.comparator, salt, ctx_salt)
        }
        OneOf | NotOneOf | OneOfHashed | NotOneOfHashed => {
            let Some(comp_val) = cond.string_vec_val.as_ref() else {
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
            let Some(comp_val) = cond.string_vec_val.as_ref() else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_starts_ends_with(
                comp_val,
                user_val.as_str(),
                &cond.comparator,
                salt,
                ctx_salt,
            )
        }
        Contains | NotContains => {
            let Some(comp_val) = cond.string_vec_val.as_ref() else {
                return CompValInvalid(None);
            };
            let (user_val, converted) = user_attr.as_str();
            if converted {
                log_conv(cond, key, user_val.as_str());
            }
            eval_contains(comp_val, user_val.as_str(), &cond.comparator)
        }
        OneOfSemver | NotOneOfSemver => {
            let Some(comp_val) = cond.string_vec_val.as_ref() else {
                return CompValInvalid(None);
            };
            let Some(user_val) = user_attr.as_semver() else {
                return AttrInvalid(
                    format!("'{user_attr}' is not a valid semantic version"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
                );
            };
            eval_semver_is_one_of(comp_val, &user_val, &cond.comparator)
        }
        GreaterSemver | GreaterEqSemver | LessSemver | LessEqSemver => {
            let Some(comp_val) = cond.string_val.as_ref() else {
                return CompValInvalid(None);
            };
            let Some(user_val) = user_attr.as_semver() else {
                return AttrInvalid(
                    format!("'{user_attr}' is not a valid semantic version"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
                );
            };
            eval_semver_compare(comp_val, &user_val, &cond.comparator)
        }
        EqNum | NotEqNum | GreaterNum | GreaterEqNum | LessNum | LessEqNum => {
            let Some(comp_val) = cond.float_val else {
                return CompValInvalid(None);
            };
            let Some(user_val) = user_attr.as_float() else {
                return AttrInvalid(
                    format!("'{user_attr}' is not a valid decimal number"),
                    cond.comp_attr.clone(),
                    format!("{cond}"),
                );
            };
            eval_number_compare(comp_val, user_val, &cond.comparator)
        }
        BeforeDateTime | AfterDateTime => {
            let Some(comp_val) = cond.float_val else {
                return CompValInvalid(None);
            };
            let Some(user_val) = user_attr.as_timestamp() else {
                return AttrInvalid(format!("'{user_attr}' is not a valid Unix timestamp (number of seconds elapsed since Unix epoch)"),
                                   cond.comp_attr.clone(),
                                   format!("{cond}")
                );
            };
            eval_date(comp_val, user_val, &cond.comparator)
        }
        ArrayContainsAnyOf
        | ArrayNotContainsAnyOf
        | ArrayContainsAnyOfHashed
        | ArrayNotContainsAnyOfHashed => {
            let Some(comp_val) = cond.string_vec_val.as_ref() else {
                return CompValInvalid(None);
            };
            let Some(user_val) = user_attr.as_str_vec() else {
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
        let Some(st) = salt else {
            return Fatal(SALT_MISSING_MSG.to_owned());
        };
        usr_v = utils::sha256(usr_v.as_str(), st.as_str(), ctx_salt);
    }
    Success((comp_val == usr_v) == needs_true)
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
        let Some(st) = salt else {
            return Fatal(SALT_MISSING_MSG.to_owned());
        };
        usr_v = utils::sha256(usr_v.as_str(), st.as_str(), ctx_salt);
    }
    for item in comp_val {
        if *item == usr_v {
            return Success(needs_true);
        }
    }
    Success(!needs_true)
}

fn eval_starts_ends_with(
    comp_val: &[String],
    user_val: &str,
    comp: &UserComparator,
    salt: &Option<String>,
    ctx_salt: &str,
) -> ConditionResult {
    let needs_true = if comp.is_starts_with() {
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
    if comp.is_sensitive() {
        let user_val_len = user_val.len();
        for item in comp_val {
            let Some(st) = salt else {
                return Fatal(SALT_MISSING_MSG.to_owned());
            };
            let parts: Vec<&str> = item.split('_').collect();
            if parts.len() < 2 || parts[1].is_empty() {
                return Fatal(COMP_VAL_INVALID_MSG.to_owned());
            }
            let Ok(length) = parts[0].trim().parse::<usize>() else {
                return Fatal(COMP_VAL_INVALID_MSG.to_owned());
            };
            if length > user_val_len {
                continue;
            }
            if comp.is_starts_with() {
                if user_val.is_char_boundary(length) {
                    let chunk = &user_val[..length];
                    if utils::sha256(chunk, st, ctx_salt) == parts[1] {
                        return Success(needs_true);
                    }
                }
            } else {
                let index = user_val_len - length;
                if user_val.is_char_boundary(index) {
                    let chunk = &user_val[index..];
                    if utils::sha256(chunk, st, ctx_salt) == parts[1] {
                        return Success(needs_true);
                    }
                }
            }
        }
    } else {
        for item in comp_val {
            let condition = if comp.is_starts_with() {
                user_val.starts_with(item.as_str())
            } else {
                user_val.ends_with(item.as_str())
            };
            if condition {
                return Success(needs_true);
            }
        }
    }
    Success(!needs_true)
}

fn eval_contains(comp_val: &[String], user_val: &str, comp: &UserComparator) -> ConditionResult {
    let needs_true = *comp == Contains;
    for item in comp_val {
        if user_val.contains(item) {
            return Success(needs_true);
        }
    }
    Success(!needs_true)
}

fn eval_semver_is_one_of(
    comp_val: &[String],
    user_val: &Version,
    comp: &UserComparator,
) -> ConditionResult {
    let needs_true = *comp == OneOfSemver;
    let mut matched = false;
    for item in comp_val {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(comp_ver) = utils::parse_semver(trimmed) else {
            // NOTE: Previous versions of the evaluation algorithm ignored invalid comparison values.
            // We keep this behavior for backward compatibility.
            return Success(false);
        };
        if user_val.eq(&comp_ver) {
            matched = true;
        }
    }
    Success(matched == needs_true)
}

fn eval_semver_compare(
    comp_val: &str,
    user_val: &Version,
    comp: &UserComparator,
) -> ConditionResult {
    let Ok(comp_ver) = utils::parse_semver(comp_val) else {
        // NOTE: Previous versions of the evaluation algorithm ignored invalid comparison values.
        // We keep this behavior for backward compatibility.
        return Success(false);
    };
    match comp {
        GreaterSemver => Success(user_val.gt(&comp_ver)),
        GreaterEqSemver => Success(user_val.ge(&comp_ver)),
        LessSemver => Success(user_val.lt(&comp_ver)),
        LessEqSemver => Success(user_val.le(&comp_ver)),
        _ => Fatal("wrong semver comparator".to_owned()),
    }
}

#[allow(clippy::float_cmp)]
fn eval_number_compare(comp_val: f64, user_val: f64, comp: &UserComparator) -> ConditionResult {
    match comp {
        EqNum => Success(user_val == comp_val),
        NotEqNum => Success(user_val != comp_val),
        GreaterNum => Success(user_val > comp_val),
        GreaterEqNum => Success(user_val >= comp_val),
        LessNum => Success(user_val < comp_val),
        LessEqNum => Success(user_val <= comp_val),
        _ => Fatal("wrong number comparator".to_owned()),
    }
}

fn eval_date(comp_val: f64, user_val: f64, comp: &UserComparator) -> ConditionResult {
    match comp {
        BeforeDateTime => Success(user_val < comp_val),
        _ => Success(user_val > comp_val),
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
    for user_item in user_val {
        if comp.is_sensitive() {
            let Some(st) = salt else {
                return Fatal(SALT_MISSING_MSG.to_owned());
            };
            let user_hashed = utils::sha256(user_item.as_str(), st.as_str(), ctx_salt);
            for comp_item in comp_val {
                if user_hashed == *comp_item {
                    return Success(needs_true);
                }
            }
        }
        for comp_item in comp_val {
            if user_item == comp_item {
                return Success(needs_true);
            }
        }
    }
    Success(!needs_true)
}

fn log_user_missing(key: &str) {
    warn!(event_id = 3001; "Cannot evaluate targeting rules and % options for setting '{key}' (User Object is missing). You should pass a User Object to the evaluation methods like `get_value()`/`get_value_details()` in order to make targeting work properly. Read more: https://configcat.com/docs/advanced/user-object/");
}

fn log_attr_missing(key: &str, attr: &str, cond_str: &str) {
    warn!(event_id = 3003; "Cannot evaluate condition ({cond_str}) for setting '{key}' (the User.{attr} attribute is missing). You should set the User.{attr} attribute in order to make targeting work properly. Read more: https://configcat.com/docs/advanced/user-object/");
}

fn log_attr_missing_percentage(key: &str, attr: &str) {
    warn!(event_id = 3003; "Cannot evaluate % options for setting '{key}' (the User.{attr} attribute is missing). You should set the User.{attr} attribute in order to make targeting work properly. Read more: https://configcat.com/docs/advanced/user-object/");
}

fn log_attr_invalid(key: &str, attr: &str, reason: &str, cond_str: &str) {
    warn!(event_id = 3004; "Cannot evaluate condition ({cond_str}) for setting '{key}' ({reason}). Please check the User.{attr} attribute and make sure that its value corresponds to the comparison operator.");
}

fn log_conv(cond: &UserCondition, key: &str, attr_val: &str) {
    warn!(event_id = 3005; "Evaluation of condition ({cond}) for setting '{key}' may not produce the expected result (the User.{} attribute is not a string value, thus it was automatically converted to the string value '{attr_val}'). Please make sure that using a non-string value was intended.", cond.comp_attr);
}
