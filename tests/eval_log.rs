#![allow(dead_code)]

use log::set_max_level;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

use configcat::OverrideBehavior::{LocalOnly, LocalOverRemote};
use configcat::{Client, FileDataSource, MapDataSource, PollingMode, User, UserValue, Value};

use crate::utils::{rand_sdk_key, RecordingLogger};

mod utils;

fn init() {
    set_max_level(log::LevelFilter::Info);
    _ = log::set_logger(&RecordingLogger {});
}

#[tokio::test]
async fn prerequisite_circular_deps() {
    init();

    let tests = vec![("key1", "'key1' -> 'key1'"), ("key2", "'key2' -> 'key3' -> 'key2'"), ("key4", "'key4' -> 'key3' -> 'key2' -> 'key3'")];

    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/test_circulardependency_v6.json").unwrap()), LocalOnly).build().unwrap();

    for test in tests {
        _ = client.get_flag_details(test.0, None).await;
        let logs = RecordingLogger::LOGS.take();
        assert!(logs.contains(test.1));
    }
}

#[tokio::test]
async fn prerequisite_comp_val_mismatch() {
    init();

    let tests: Vec<(&str, &str, Value, Option<&str>)> = vec![
        ("stringDependsOnBool", "mainBoolFlag", Value::Bool(true), Some("Dog")),
        ("stringDependsOnBool", "mainBoolFlag", Value::Bool(false), Some("Cat")),
        ("stringDependsOnBool", "mainBoolFlag", "1".into(), None),
        ("stringDependsOnBool", "mainBoolFlag", Value::Int(1), None),
        ("stringDependsOnBool", "mainBoolFlag", Value::Float(1.0), None),
        ("stringDependsOnString", "mainStringFlag", "private".into(), Some("Dog")),
        ("stringDependsOnString", "mainStringFlag", "Private".into(), Some("Cat")),
        ("stringDependsOnString", "mainStringFlag", Value::Bool(true), None),
        ("stringDependsOnString", "mainStringFlag", Value::Int(1), None),
        ("stringDependsOnString", "mainStringFlag", Value::Float(1.0), None),
        ("stringDependsOnInt", "mainIntFlag", Value::Int(2), Some("Dog")),
        ("stringDependsOnInt", "mainIntFlag", Value::Int(1), Some("Cat")),
        ("stringDependsOnInt", "mainIntFlag", "2".into(), None),
        ("stringDependsOnInt", "mainIntFlag", Value::Bool(true), None),
        ("stringDependsOnInt", "mainIntFlag", Value::Float(2.0), None),
        ("stringDependsOnDouble", "mainDoubleFlag", Value::Float(0.1), Some("Dog")),
        ("stringDependsOnDouble", "mainDoubleFlag", Value::Float(0.11), Some("Cat")),
        ("stringDependsOnDouble", "mainDoubleFlag", "0.1".into(), None),
        ("stringDependsOnDouble", "mainDoubleFlag", Value::Bool(true), None),
        ("stringDependsOnDouble", "mainDoubleFlag", Value::Int(1), None),
    ];

    for test in tests {
        let client = Client::builder("configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/JoGwdqJZQ0K2xDy7LnbyOg").overrides(Box::new(MapDataSource::from([(test.1, test.2)])), LocalOverRemote).build().unwrap();

        let details = client.get_flag_details(test.0, None).await;
        if test.3.is_none() {
            assert!(details.value.is_none());

            let logs = RecordingLogger::LOGS.take();
            assert!(logs.contains("Type mismatch between comparison value"));
        } else {
            assert_eq!(details.value.unwrap().as_str().unwrap(), test.3.unwrap());
        }
    }
}

#[tokio::test]
async fn eval_log() {
    init();

    let suites: Vec<&str> = vec![
        "1_targeting_rule",
        "2_targeting_rules",
        "and_rules",
        "comparators",
        "epoch_date_validation",
        "list_truncation",
        "number_validation",
        "options_after_targeting_rule",
        "options_based_on_custom_attr",
        "options_based_on_user_id",
        "options_within_targeting_rule",
        "prerequisite_flag",
        "segment",
        "semver_validation",
        "simple_value",
    ];

    for suite_name in suites {
        let json = fs::read_to_string(format!("tests/data/evaluationlog/{suite_name}.json")).unwrap();
        let suite = serde_json::from_str::<TestSuite>(json.as_str()).unwrap();

        let sdk_key = if let Some(key) = suite.sdk_key { key } else { rand_sdk_key() };

        let mut builder = Client::builder(sdk_key.as_str()).polling_mode(PollingMode::Manual);

        if let Some(overrides) = suite.overrides.as_ref() {
            builder = builder.overrides(Box::new(FileDataSource::new(format!("tests/data/evaluationlog/_overrides/{overrides}").as_str()).unwrap()), LocalOnly);
        }

        let client = builder.build().unwrap();
        if suite.overrides.is_none() {
            client.refresh().await.unwrap();
        }

        for test in suite.tests {
            let mut log_content = fs::read_to_string(format!("tests/data/evaluationlog/{suite_name}/{}", test.exp_log)).unwrap();
            let has_user = test.user.is_some();
            if has_user {
                trim_user_section(&mut log_content);
            }
            let user: Option<User> = test.user.map(user_from_json);

            let def_val = Value::from_json_val(&test.default_val).unwrap();
            match def_val {
                Value::Bool(val) => {
                    let result = client.get_bool_value(test.key.as_str(), user, val).await;
                    assert_eq!(result, Value::from_json_val(&test.return_val).unwrap().as_bool().unwrap())
                }
                Value::Int(val) => {
                    let result = client.get_int_value(test.key.as_str(), user, val).await;
                    assert_eq!(result, Value::from_json_val(&test.return_val).unwrap().as_int().unwrap())
                }
                Value::Float(val) => {
                    let result = client.get_float_value(test.key.as_str(), user, val).await;
                    assert_eq!(result, Value::from_json_val(&test.return_val).unwrap().as_float().unwrap())
                }
                Value::String(val) => {
                    let result = client.get_str_value(test.key.as_str(), user, val).await;
                    assert_eq!(result, Value::from_json_val(&test.return_val).unwrap().as_str().unwrap())
                }
            }

            let mut logs = RecordingLogger::LOGS.take();
            if has_user {
                trim_user_section(&mut logs);
            }
            assert_eq!(logs, log_content, "{}", suite_name);
        }
    }
}

#[derive(Deserialize)]
struct TestCase {
    #[serde(rename = "key")]
    key: String,
    #[serde(rename = "defaultValue")]
    default_val: serde_json::Value,
    #[serde(rename = "returnValue")]
    return_val: serde_json::Value,
    #[serde(rename = "expectedLog")]
    exp_log: String,
    #[serde(rename = "user")]
    user: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
struct TestSuite {
    #[serde(rename = "sdkKey")]
    sdk_key: Option<String>,
    #[serde(rename = "jsonOverride")]
    overrides: Option<String>,
    #[serde(rename = "tests")]
    tests: Vec<TestCase>,
}

fn user_from_json(map: HashMap<String, serde_json::Value>) -> User {
    let mut usr_map = HashMap::<String, UserValue>::new();
    for (k, v) in map.iter() {
        let val = usr_val_from_json(v).unwrap();
        usr_map.insert(k.to_owned(), val);
    }
    usr_map.into()
}

fn usr_val_from_json(json_val: &serde_json::Value) -> Option<UserValue> {
    match json_val {
        serde_json::Value::String(val) => Some(UserValue::String(val.clone())),
        serde_json::Value::Number(val) => {
            if let Some(float_val) = val.as_f64() {
                return Some(UserValue::Float(float_val));
            }
            None
        }
        serde_json::Value::Array(val) => {
            let mut vec = Vec::<String>::with_capacity(val.len());
            for item in val {
                vec.push(item.as_str().unwrap().to_owned());
            }
            Some(UserValue::StringVec(vec))
        }
        _ => None,
    }
}

fn trim_user_section(content: &mut String) {
    let index = content.find("for User").unwrap();
    let rest = &content[index..];
    let newline_index = rest.find('\n').unwrap();
    content.replace_range(index..(index + newline_index), "");
}
