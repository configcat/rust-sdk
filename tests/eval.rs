#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use chrono::{DateTime, Utc};
use configcat::OverrideBehavior::{LocalOnly, LocalOverRemote, RemoteOverLocal};
use configcat::{Client, FileDataSource, OverrideBehavior, User, UserValue};
use std::str::FromStr;

#[tokio::test]
async fn prerequisite_flag_overrides() {
    let tests: Vec<(&str, &str, &str, Option<OverrideBehavior>, Option<&str>)> = vec![
        ("stringDependsOnString", "1", "john@sensitivecompany.com", None, Some("Dog")),
        ("stringDependsOnString", "1", "john@sensitivecompany.com", Some(RemoteOverLocal), Some("Dog")),
        ("stringDependsOnString", "1", "john@sensitivecompany.com", Some(LocalOverRemote), Some("Dog")),
        ("stringDependsOnString", "1", "john@sensitivecompany.com", Some(LocalOnly), None),
        ("stringDependsOnString", "2", "john@notsensitivecompany.com", None, Some("Cat")),
        ("stringDependsOnString", "2", "john@notsensitivecompany.com", Some(RemoteOverLocal), Some("Cat")),
        ("stringDependsOnString", "2", "john@notsensitivecompany.com", Some(LocalOverRemote), Some("Dog")),
        ("stringDependsOnString", "2", "john@notsensitivecompany.com", Some(LocalOnly), None),
        ("stringDependsOnInt", "1", "john@sensitivecompany.com", None, Some("Dog")),
        ("stringDependsOnInt", "1", "john@sensitivecompany.com", Some(RemoteOverLocal), Some("Dog")),
        ("stringDependsOnInt", "1", "john@sensitivecompany.com", Some(LocalOverRemote), Some("Cat")),
        ("stringDependsOnInt", "1", "john@sensitivecompany.com", Some(LocalOnly), None),
        ("stringDependsOnInt", "2", "john@notsensitivecompany.com", None, Some("Cat")),
        ("stringDependsOnInt", "2", "john@notsensitivecompany.com", Some(RemoteOverLocal), Some("Cat")),
        ("stringDependsOnInt", "2", "john@notsensitivecompany.com", Some(LocalOverRemote), Some("Dog")),
        ("stringDependsOnInt", "2", "john@notsensitivecompany.com", Some(LocalOnly), None),
    ];

    for test in tests {
        let mut builder = Client::builder("configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/JoGwdqJZQ0K2xDy7LnbyOg");
        if test.3.is_some() {
            builder = builder.overrides(Box::new(FileDataSource::new("tests/data/test_override_flagdependency_v6.json").unwrap()), test.3.unwrap());
        }
        let client = builder.build().unwrap();

        let user = User::new(test.1).email(test.2);
        let details = client.get_flag_details(test.0, Some(user)).await;

        if test.4.is_none() {
            assert!(details.value.is_none());
        } else {
            assert_eq!(details.value.unwrap().as_str().unwrap(), test.4.unwrap());
        }
    }
}

#[tokio::test]
async fn segment_overrides() {
    let tests: Vec<(&str, &str, &str, Option<OverrideBehavior>, Option<bool>)> = vec![
        ("developerAndBetaUserSegment", "1", "john@example.com", None, Some(false)),
        ("developerAndBetaUserSegment", "1", "john@example.com", Some(RemoteOverLocal), Some(false)),
        ("developerAndBetaUserSegment", "1", "john@example.com", Some(LocalOverRemote), Some(true)),
        ("developerAndBetaUserSegment", "1", "john@example.com", Some(LocalOnly), Some(true)),
        ("notDeveloperAndNotBetaUserSegment", "2", "kate@example.com", None, Some(true)),
        ("notDeveloperAndNotBetaUserSegment", "2", "kate@example.com", Some(RemoteOverLocal), Some(true)),
        ("notDeveloperAndNotBetaUserSegment", "2", "kate@example.com", Some(LocalOverRemote), Some(true)),
        ("notDeveloperAndNotBetaUserSegment", "2", "kate@example.com", Some(LocalOnly), None),
    ];

    for test in tests {
        let mut builder = Client::builder("configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/h99HYXWWNE2bH8eWyLAVMA");
        if test.3.is_some() {
            builder = builder.overrides(Box::new(FileDataSource::new("tests/data/test_override_segments_v6.json").unwrap()), test.3.unwrap());
        }
        let client = builder.build().unwrap();

        let user = User::new(test.1).email(test.2);
        let details = client.get_flag_details(test.0, Some(user)).await;

        if test.4.is_none() {
            assert!(details.value.is_none());
        } else {
            assert_eq!(details.value.unwrap().as_bool().unwrap(), test.4.unwrap());
        }
    }
}

#[tokio::test]
async fn matched_eval_rule_percentage_opts() {
    let tests: Vec<(&str, Option<&str>, Option<&str>, Option<&str>, &str, bool, bool)> = vec![
        ("stringMatchedTargetingRuleAndOrPercentageOption", None, None, None, "Cat", false, false),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), None, None, "Cat", false, false),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("a@example.com"), None, "Dog", true, false),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("a@configcat.com"), None, "Cat", false, false),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("a@configcat.com"), Some(""), "Frog", true, true),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("a@configcat.com"), Some("US"), "Fish", true, true),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("b@configcat.com"), None, "Cat", false, false),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("b@configcat.com"), Some(""), "Falcon", false, true),
        ("stringMatchedTargetingRuleAndOrPercentageOption", Some("12345"), Some("b@configcat.com"), Some("US"), "Spider", false, true),
    ];

    let client = Client::new("configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/P4e3fAz_1ky2-Zg2e4cbkw").unwrap();

    for test in tests {
        let mut user = None;
        if test.1.is_some() {
            let mut u = User::new(test.1.unwrap());
            if test.2.is_some() {
                u = u.email(test.2.unwrap());
            }
            if test.3.is_some() {
                u = u.custom("PercentageBase", test.3.unwrap());
            }
            user = Some(u);
        }

        let details = client.get_flag_details(test.0, user).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.4);
        assert_eq!(details.matched_targeting_rule.is_some(), test.5);
        assert_eq!(details.matched_percentage_option.is_some(), test.6);
    }
}

#[tokio::test]
async fn comp_attr_canonical_str_representation() {
    let tests: Vec<(&str, UserValue, &str)> = vec![
        ("numberToStringConversion", UserValue::Float(0.12345), "1"),
        ("numberToStringConversionInt", UserValue::Int(125), "4"),
        ("numberToStringConversionInt", UserValue::UInt(125), "4"),
        ("numberToStringConversionPositiveExp", UserValue::Float(-1.23456789e96), "2"),
        ("numberToStringConversionNegativeExp", UserValue::Float(-12345.6789E-100), "4"),
        ("numberToStringConversionNaN", UserValue::Float(f64::NAN), "3"),
        ("numberToStringConversionPositiveInf", UserValue::Float(f64::INFINITY), "4"),
        ("numberToStringConversionNegativeInf", UserValue::Float(f64::NEG_INFINITY), "3"),
        ("dateToStringConversion", DateTime::<Utc>::from_str("2023-03-31T23:59:59.999Z").unwrap().into(), "3"),
        ("dateToStringConversion", UserValue::Float(1680307199.999), "3"),
        ("dateToStringConversionNaN", UserValue::Float(f64::NAN), "3"),
        ("dateToStringConversionPositiveInf", UserValue::Float(f64::INFINITY), "1"),
        ("dateToStringConversionNegativeInf", UserValue::Float(f64::NEG_INFINITY), "5"),
        ("stringArrayToStringConversion", vec!["read", "Write", " eXecute "].into(), "4"),
        ("stringArrayToStringConversionEmpty", UserValue::StringVec(vec![]), "5"),
        ("stringArrayToStringConversionSpecialChars", vec!["+<>%\"'\\/\t\r\n"].into(), "3"),
        ("stringArrayToStringConversionUnicode", vec!["äöüÄÖÜçéèñışğâ¢™✓😀"].into(), "2"),
    ];

    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/comparison_attribute_conversion.json").unwrap()), LocalOnly).build().unwrap();

    for test in tests {
        let user = User::new("12345").custom("Custom1", test.1.clone());
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.2);
    }
}

#[tokio::test]
async fn spec_chars() {
    let tests: Vec<(&str, &str, &str)> = vec![("specialCharacters", "äöüÄÖÜçéèñışğâ¢™✓😀", "äöüÄÖÜçéèñışğâ¢™✓😀"), ("specialCharactersHashed", "äöüÄÖÜçéèñışğâ¢™✓😀", "äöüÄÖÜçéèñışğâ¢™✓😀")];

    let client = Client::new("configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/u28_1qNyZ0Wz-ldYHIU7-g").unwrap();

    for test in tests {
        let user = User::new(test.1);
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.2);
    }
}

#[tokio::test]
async fn attr_trim() {
    let tests: Vec<(&str, &str)> = vec![
        ("isoneof", "no trim"),
        ("isnotoneof", "no trim"),
        ("isoneofhashed", "no trim"),
        ("isnotoneofhashed", "no trim"),
        ("equalshashed", "no trim"),
        ("notequalshashed", "no trim"),
        ("arraycontainsanyofhashed", "no trim"),
        ("arraynotcontainsanyofhashed", "no trim"),
        ("equals", "no trim"),
        ("notequals", "no trim"),
        ("startwithanyof", "no trim"),
        ("notstartwithanyof", "no trim"),
        ("endswithanyof", "no trim"),
        ("notendswithanyof", "no trim"),
        ("arraycontainsanyof", "no trim"),
        ("arraynotcontainsanyof", "no trim"),
        ("startwithanyofhashed", "no trim"),
        ("notstartwithanyofhashed", "no trim"),
        ("endswithanyofhashed", "no trim"),
        ("notendswithanyofhashed", "no trim"),
        ("semverisoneof", "4 trim"),
        ("semverisnotoneof", "5 trim"),
        ("semverless", "6 trim"),
        ("semverlessequals", "7 trim"),
        ("semvergreater", "8 trim"),
        ("semvergreaterequals", "9 trim"),
        ("numberequals", "10 trim"),
        ("numbernotequals", "11 trim"),
        ("numberless", "12 trim"),
        ("numberlessequals", "13 trim"),
        ("numbergreater", "14 trim"),
        ("numbergreaterequals", "15 trim"),
        ("datebefore", "18 trim"),
        ("dateafter", "19 trim"),
        ("containsanyof", "no trim"),
        ("notcontainsanyof", "no trim"),
    ];

    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/comparison_attribute_trimming.json").unwrap()), LocalOnly).build().unwrap();

    for test in tests {
        let user = User::new(" 12345 ").country("[\" USA \"]").custom("Version", " 1.0.0 ").custom("Number", " 3 ").custom("Date", " 1705253400 ");
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.1, "{}", test.0);
    }
}

#[tokio::test]
async fn comp_val_trim() {
    let tests: Vec<(&str, &str)> = vec![
        ("isoneof", "no trim"),
        ("isnotoneof", "no trim"),
        ("containsanyof", "no trim"),
        ("notcontainsanyof", "no trim"),
        ("isoneofhashed", "no trim"),
        ("isnotoneofhashed", "no trim"),
        ("equalshashed", "no trim"),
        ("notequalshashed", "no trim"),
        ("arraycontainsanyofhashed", "no trim"),
        ("arraynotcontainsanyofhashed", "no trim"),
        ("equals", "no trim"),
        ("notequals", "no trim"),
        ("startwithanyof", "no trim"),
        ("notstartwithanyof", "no trim"),
        ("endswithanyof", "no trim"),
        ("notendswithanyof", "no trim"),
        ("arraycontainsanyof", "no trim"),
        ("arraynotcontainsanyof", "no trim"),
        ("startwithanyofhashed", "no trim"),
        ("notstartwithanyofhashed", "no trim"),
        ("endswithanyofhashed", "no trim"),
        ("notendswithanyofhashed", "no trim"),
        ("semverisoneof", "4 trim"),
        ("semverisnotoneof", "5 trim"),
        ("semverless", "6 trim"),
        ("semverlessequals", "7 trim"),
        ("semvergreater", "8 trim"),
        ("semvergreaterequals", "9 trim"),
    ];

    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/comparison_value_trimming.json").unwrap()), LocalOnly).build().unwrap();

    for test in tests {
        let user = User::new("12345").country("[\"USA\"]").custom("Version", "1.0.0").custom("Number", "3").custom("Date", "1705253400");
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.1, "{}", test.0);
    }
}

#[tokio::test]
async fn attr_val_conv_str() {
    let tests: Vec<(&str, &str, &str, UserValue, &str)> = vec![
        ("lessThanWithPercentage", "12345", "Custom1", "0.0".into(), "20%"),
        ("lessThanWithPercentage", "12345", "Custom1", "0.9.9".into(), "< 1.0.0"),
        ("lessThanWithPercentage", "12345", "Custom1", "1.0.0".into(), "20%"),
        ("lessThanWithPercentage", "12345", "Custom1", "1.1".into(), "20%"),
        ("lessThanWithPercentage", "12345", "Custom1", 0.into(), "20%"),
        ("lessThanWithPercentage", "12345", "Custom1", 0.9.into(), "20%"),
        ("lessThanWithPercentage", "12345", "Custom1", 2.into(), "20%"),
    ];

    let client = Client::new("configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/iV8vH2MBakKxkFZylxHmTg").unwrap();

    for test in tests {
        let user = User::new(test.1).custom(test.2, test.3);
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.4);
    }
}

#[tokio::test]
async fn attr_val_conv_num() {
    let tests: Vec<(&str, &str, &str, UserValue, &str)> = vec![
        ("numberWithPercentage", "12345", "Custom1", (-1_i8).into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 2_i8.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_i8.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_i8.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", 2_u8.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_u8.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_u8.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", 2_u16.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_u16.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_u16.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", (-1_i64).into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 2_i32.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_i32.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_i32.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", 2_u32.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_u32.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_u32.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", i32::MIN.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 2_i64.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_i64.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_i64.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", i64::MAX.into(), ">5"),
        ("numberWithPercentage", "12345", "Custom1", 2_u64.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_u64.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_u64.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", u64::MAX.into(), ">5"),
        ("numberWithPercentage", "12345", "Custom1", f64::NEG_INFINITY.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", (-1_f32).into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 2_f32.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 2.1_f32.into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", 3_f32.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", 5_f32.into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", f64::INFINITY.into(), ">5"),
        ("numberWithPercentage", "12345", "Custom1", f64::NAN.into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", "-Infinity".into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", "-1".into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", "2".into(), "<2.1"),
        ("numberWithPercentage", "12345", "Custom1", "2.1".into(), "<=2,1"),
        ("numberWithPercentage", "12345", "Custom1", "2,1".into(), "<=2,1"),
        ("numberWithPercentage", "12345", "Custom1", "3".into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", "5".into(), ">=5"),
        ("numberWithPercentage", "12345", "Custom1", "Infinity".into(), ">5"),
        ("numberWithPercentage", "12345", "Custom1", "NaN".into(), "<>4.2"),
        ("numberWithPercentage", "12345", "Custom1", "NaNa".into(), "80%"),
    ];

    let client = Client::new("configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/FCWN-k1dV0iBf8QZrDgjdw").unwrap();

    for test in tests {
        let user = User::new(test.1).custom(test.2, test.3);
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.4);
    }
}

#[tokio::test]
async fn attr_val_conv_date() {
    let tests: Vec<(&str, &str, &str, UserValue, bool)> = vec![
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-03-31T23:59:59.999Z").unwrap().into(), false),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-04-01T01:59:59.999+02:00").unwrap().into(), false),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-04-01T00:00:00.001Z").unwrap().into(), true),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-04-01T02:00:00.0010000+02:00").unwrap().into(), true),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-04-30T23:59:59.999Z").unwrap().into(), true),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-05-01T01:59:59.999+02:00").unwrap().into(), true),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-05-01T00:00:00.001Z").unwrap().into(), false),
        ("boolTrueIn202304", "12345", "Custom1", DateTime::<Utc>::from_str("2023-05-01T02:00:00.001+02:00").unwrap().into(), false),
        ("boolTrueIn202304", "12345", "Custom1", f64::NEG_INFINITY.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", 1680307199.999.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", 1680307200.001.into(), true),
        ("boolTrueIn202304", "12345", "Custom1", 1682899199.999.into(), true),
        ("boolTrueIn202304", "12345", "Custom1", 1682899200.001.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", f64::INFINITY.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", f64::NAN.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", 1680307199.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", 1680307201.into(), true),
        ("boolTrueIn202304", "12345", "Custom1", 1682899199.into(), true),
        ("boolTrueIn202304", "12345", "Custom1", 1682899201.into(), false),
        ("boolTrueIn202304", "12345", "Custom1", "-Infinity".into(), false),
        ("boolTrueIn202304", "12345", "Custom1", "1680307199.999".into(), false),
        ("boolTrueIn202304", "12345", "Custom1", "1680307200.001".into(), true),
        ("boolTrueIn202304", "12345", "Custom1", "1682899199.999".into(), true),
        ("boolTrueIn202304", "12345", "Custom1", "1682899200.001".into(), false),
        ("boolTrueIn202304", "12345", "Custom1", "+Infinity".into(), false),
        ("boolTrueIn202304", "12345", "Custom1", "NaN".into(), false),
    ];

    let client = Client::new("configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/OfQqcTjfFUGBwMKqtyEOrQ").unwrap();

    for test in tests {
        let user = User::new(test.1).custom(test.2, test.3.clone());
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_bool().unwrap(), test.4, "{:?}", test.3);
    }
}

#[tokio::test]
async fn attr_val_conv_str_vec() {
    let tests: Vec<(&str, &str, &str, UserValue, &str)> = vec![
        ("stringArrayContainsAnyOfDogDefaultCat", "12345", "Custom1", ["x", "read"].into(), "Dog"),
        ("stringArrayContainsAnyOfDogDefaultCat", "12345", "Custom1", ["x", "Read"].into(), "Cat"),
        ("stringArrayContainsAnyOfDogDefaultCat", "12345", "Custom1", "[\"x\", \"read\"]".into(), "Dog"),
        ("stringArrayContainsAnyOfDogDefaultCat", "12345", "Custom1", "[\"x\", \"Read\"]".into(), "Cat"),
        ("stringArrayContainsAnyOfDogDefaultCat", "12345", "Custom1", "x, read".into(), "Cat"),
    ];

    let client = Client::new("configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/OfQqcTjfFUGBwMKqtyEOrQ").unwrap();

    for test in tests {
        let user = User::new(test.1).custom(test.2, test.3);
        let details = client.get_flag_details(test.0, Some(user)).await;

        assert_eq!(details.value.unwrap().as_str().unwrap(), test.4);
    }
}
