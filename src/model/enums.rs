use serde_repr::Deserialize_repr;
use std::fmt::{Display, Formatter};

/// Describes the location of your feature flag and setting data within the ConfigCat CDN.
#[derive(Clone, PartialEq, Debug)]
pub enum DataGovernance {
    /// Select this if your feature flags are published to all global CDN nodes.
    Global,
    /// Select this if your feature flags are published to CDN nodes only in the EU.
    EU,
}

#[derive(Debug, Deserialize_repr, PartialEq, Clone)]
#[repr(u8)]
pub enum RedirectMode {
    No,
    Should,
    Force,
}

/// The type of a feature flag or setting.
#[derive(Debug, Clone, Deserialize_repr)]
#[repr(u8)]
pub enum SettingType {
    /// The on/off type (feature flag).
    Bool = 0,
    /// The text setting type.
    String = 1,
    /// The whole number setting type.
    Int = 2,
    /// The decimal number setting type.
    Float = 3,
}

impl Display for SettingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingType::Bool => f.write_str("Bool"),
            SettingType::String => f.write_str("String"),
            SettingType::Int => f.write_str("Int"),
            SettingType::Float => f.write_str("Float"),
        }
    }
}

/// Segment comparison operator used during the evaluation process.
#[derive(Debug, PartialEq, Deserialize_repr)]
#[repr(u8)]
pub enum SegmentComparator {
    /// Checks whether the conditions of the specified segment are evaluated to true.
    IsIn = 0,
    /// Checks whether the conditions of the specified segment are evaluated to false.
    IsNotIn = 1,
}

impl Display for SegmentComparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentComparator::IsIn => f.write_str("IS IN SEGMENT"),
            SegmentComparator::IsNotIn => f.write_str("IS NOT IN SEGMENT"),
        }
    }
}

/// Prerequisite flag comparison operator used during the evaluation process.
#[derive(Debug, PartialEq, Deserialize_repr)]
#[repr(u8)]
pub enum PrerequisiteFlagComparator {
    /// Checks whether the evaluated value of the specified prerequisite flag is equal to the comparison value.
    Eq = 0,
    /// Checks whether the evaluated value of the specified prerequisite flag is not equal to the comparison value.
    NotEq = 1,
}

impl Display for PrerequisiteFlagComparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PrerequisiteFlagComparator::Eq => f.write_str("EQUALS"),
            PrerequisiteFlagComparator::NotEq => f.write_str("NOT EQUALS"),
        }
    }
}

/// User Object attribute comparison operator used during the evaluation process.
#[derive(Debug, PartialEq, Deserialize_repr)]
#[repr(u8)]
pub enum UserComparator {
    /// Checks whether the comparison attribute is equal to any of the comparison values.
    OneOf = 0,
    /// Checks whether the comparison attribute is not equal to any of the comparison values.
    NotOneOf = 1,
    /// Checks whether the comparison attribute contains any comparison values as a substring.
    Contains = 2,
    /// Checks whether the comparison attribute does not contain any comparison values as a substring.
    NotContains = 3,
    /// Checks whether the comparison attribute interpreted as a semantic version is equal to any of the comparison values.
    OneOfSemver = 4,
    /// Checks whether the comparison attribute interpreted as a semantic version is not equal to any of the comparison values.
    NotOneOfSemver = 5,
    /// Checks whether the comparison attribute interpreted as a semantic version is less than the comparison value.
    LessSemver = 6,
    /// Checks whether the comparison attribute interpreted as a semantic version is less than or equal to the comparison value.
    LessEqSemver = 7,
    /// Checks whether the comparison attribute interpreted as a semantic version is greater than the comparison value.
    GreaterSemver = 8,
    /// Checks whether the comparison attribute interpreted as a semantic version is greater than or equal to the comparison value.
    GreaterEqSemver = 9,
    /// Checks whether the comparison attribute interpreted as a decimal number is equal to the comparison value.
    EqNum = 10,
    /// Checks whether the comparison attribute interpreted as a decimal number is not equal to the comparison value.
    NotEqNum = 11,
    /// Checks whether the comparison attribute interpreted as a decimal number is less than the comparison value.
    LessNum = 12,
    /// Checks whether the comparison attribute interpreted as a decimal number is less than or equal to the comparison value.
    LessEqNum = 13,
    /// Checks whether the comparison attribute interpreted as a decimal number is greater than the comparison value.
    GreaterNum = 14,
    /// Checks whether the comparison attribute interpreted as a decimal number is greater than or equal to the comparison value.
    GreaterEqNum = 15,
    /// Checks whether the comparison attribute is equal to any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    OneOfHashed = 16,
    /// Checks whether the comparison attribute is not equal to any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    NotOneOfHashed = 17,
    /// Checks whether the comparison attribute interpreted as the seconds elapsed since Unix Epoch is less than the comparison value.
    BeforeDateTime = 18,
    /// Checks whether the comparison attribute interpreted as the seconds elapsed since Unix Epoch is greater than the comparison value.
    AfterDateTime = 19,
    /// Checks whether the comparison attribute is equal to the comparison value (where the comparison is performed using the salted SHA256 hashes of the values).
    EqHashed = 20,
    /// Checks whether the comparison attribute is not equal to the comparison value (where the comparison is performed using the salted SHA256 hashes of the values).
    NotEqHashed = 21,
    /// Checks whether the comparison attribute starts with any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    StartsWithAnyOfHashed = 22,
    /// Checks whether the comparison attribute does not start with any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    NotStartsWithAnyOfHashed = 23,
    /// Checks whether the comparison attribute ends with any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    EndsWithAnyOfHashed = 24,
    /// Checks whether the comparison attribute does not end with any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    NotEndsWithAnyOfHashed = 25,
    /// Checks whether the comparison attribute interpreted as a string list contains any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    ArrayContainsAnyOfHashed = 26,
    /// Checks whether the comparison attribute interpreted as a string list does not contain any of the comparison values (where the comparison is performed using the salted SHA256 hashes of the values).
    ArrayNotContainsAnyOfHashed = 27,
    /// Checks whether the comparison attribute is equal to the comparison value.
    Eq = 28,
    /// Checks whether the comparison attribute is not equal to the comparison value.
    NotEq = 29,
    /// Checks whether the comparison attribute starts with any of the comparison values.
    StartsWithAnyOf = 30,
    /// Checks whether the comparison attribute does not start with any of the comparison values.
    NotStartsWithAnyOf = 31,
    /// Checks whether the comparison attribute ends with any of the comparison values.
    EndsWithAnyOf = 32,
    /// Checks whether the comparison attribute does not end with any of the comparison values.
    NotEndsWithAnyOf = 33,
    /// Checks whether the comparison attribute interpreted as a string list contains any of the comparison values.
    ArrayContainsAnyOf = 34,
    /// Checks whether the comparison attribute interpreted as a string list does not contain any of the comparison values.
    ArrayNotContainsAnyOf = 35,
}

impl Display for UserComparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UserComparator::OneOf => f.write_str("IS ONE OF"),
            UserComparator::NotOneOf => f.write_str("IS NOT ONE OF"),
            UserComparator::Contains => f.write_str("CONTAINS ANY OF"),
            UserComparator::NotContains => f.write_str("NOT CONTAINS ANY OF"),
            UserComparator::OneOfSemver => f.write_str("IS ONE OF"),
            UserComparator::NotOneOfSemver => f.write_str("IS NOT ONE OF"),
            UserComparator::LessSemver => f.write_str("<"),
            UserComparator::LessEqSemver => f.write_str("<="),
            UserComparator::GreaterSemver => f.write_str(">"),
            UserComparator::GreaterEqSemver => f.write_str(">="),
            UserComparator::EqNum => f.write_str("="),
            UserComparator::NotEqNum => f.write_str("!="),
            UserComparator::LessNum => f.write_str("<"),
            UserComparator::LessEqNum => f.write_str("<="),
            UserComparator::GreaterNum => f.write_str(">"),
            UserComparator::GreaterEqNum => f.write_str(">="),
            UserComparator::OneOfHashed => f.write_str("IS ONE OF"),
            UserComparator::NotOneOfHashed => f.write_str("IS NOT ONE OF"),
            UserComparator::BeforeDateTime => f.write_str("BEFORE"),
            UserComparator::AfterDateTime => f.write_str("AFTER"),
            UserComparator::EqHashed => f.write_str("EQUALS"),
            UserComparator::NotEqHashed => f.write_str("NOT EQUALS"),
            UserComparator::StartsWithAnyOfHashed => f.write_str("STARTS WITH ANY OF"),
            UserComparator::NotStartsWithAnyOfHashed => f.write_str("NOT STARTS WITH ANY OF"),
            UserComparator::EndsWithAnyOfHashed => f.write_str("ENDS WITH ANY OF"),
            UserComparator::NotEndsWithAnyOfHashed => f.write_str("NOT ENDS WITH ANY OF"),
            UserComparator::ArrayContainsAnyOfHashed => f.write_str("ARRAY CONTAINS ANY OF"),
            UserComparator::ArrayNotContainsAnyOfHashed => f.write_str("ARRAY NOT CONTAINS ANY OF"),
            UserComparator::Eq => f.write_str("EQUALS"),
            UserComparator::NotEq => f.write_str("NOT EQUALS"),
            UserComparator::StartsWithAnyOf => f.write_str("STARTS WITH ANY OF"),
            UserComparator::NotStartsWithAnyOf => f.write_str("NOT STARTS WITH ANY OF"),
            UserComparator::EndsWithAnyOf => f.write_str("ENDS WITH ANY OF"),
            UserComparator::NotEndsWithAnyOf => f.write_str("NOT ENDS WITH ANY OF"),
            UserComparator::ArrayContainsAnyOf => f.write_str("ARRAY CONTAINS ANY OF"),
            UserComparator::ArrayNotContainsAnyOf => f.write_str("ARRAY NOT CONTAINS ANY OF"),
        }
    }
}

impl UserComparator {
    pub(crate) fn is_sensitive(&self) -> bool {
        matches!(
            self,
            UserComparator::OneOfHashed
                | UserComparator::NotOneOfHashed
                | UserComparator::EqHashed
                | UserComparator::NotEqHashed
                | UserComparator::StartsWithAnyOfHashed
                | UserComparator::NotStartsWithAnyOfHashed
                | UserComparator::EndsWithAnyOfHashed
                | UserComparator::NotEndsWithAnyOfHashed
                | UserComparator::ArrayContainsAnyOfHashed
                | UserComparator::ArrayNotContainsAnyOfHashed
        )
    }

    pub(crate) fn is_date(&self) -> bool {
        matches!(
            self,
            UserComparator::AfterDateTime | UserComparator::BeforeDateTime
        )
    }

    pub(crate) fn is_starts_with(&self) -> bool {
        matches!(
            self,
            UserComparator::StartsWithAnyOf
                | UserComparator::StartsWithAnyOfHashed
                | UserComparator::NotStartsWithAnyOf
                | UserComparator::NotStartsWithAnyOfHashed
        )
    }
}
