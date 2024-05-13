#![allow(dead_code)]

use log::set_max_level;

use configcat::OverrideBehavior::{LocalOnly, LocalOverRemote};
use configcat::{Client, FileDataSource, MapDataSource, Value};

use crate::utils::RecordingLogger;

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
