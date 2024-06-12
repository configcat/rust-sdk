#![allow(dead_code)]

use crate::utils::{construct_bool_json_payload, produce_mock_path};
use configcat::OverrideBehavior::{LocalOnly, LocalOverRemote, RemoteOverLocal};
use configcat::Value::{Bool, Float, Int};
use configcat::{Client, ClientCacheState, FileDataSource, MapDataSource, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

mod utils;

#[tokio::test]
async fn file_simple() {
    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/test_json_simple.json").unwrap()), LocalOnly).build().unwrap();

    assert!(client.get_value("enabledFeature", false, None).await);
    assert!(!client.get_value("disabledFeature", true, None).await);
    assert_eq!(client.get_value("intSetting", 0, None).await, 5);
    assert_eq!(client.get_value("doubleSetting", 0.0, None).await, 1.2);
    assert_eq!(client.get_value("stringSetting", String::default(), None).await, "test".to_owned());
}

#[tokio::test]
async fn file_complex() {
    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/test_json_complex.json").unwrap()), LocalOnly).build().unwrap();

    assert!(client.get_value("enabledFeature", false, None).await);
    assert!(!client.get_value("disabledFeature", true, None).await);
    assert_eq!(client.get_value("intSetting", 0, None).await, 5);
    assert_eq!(client.get_value("doubleSetting", 0.0, None).await, 1.2);
    assert_eq!(client.get_value("stringSetting", String::default(), None).await, "test".to_owned());
}

#[tokio::test]
async fn map() {
    let mut server = mockito::Server::new_async().await;
    let (sdk_key, path) = produce_mock_path();
    let m = server.mock("GET", path.as_str()).with_status(200).expect(0).create_async().await;

    let client = Client::builder(sdk_key.as_str())
        .overrides(
            Box::new(MapDataSource::from([
                ("enabledFeature", Bool(true)),
                ("disabledFeature", Bool(false)),
                ("intSetting", Int(5)),
                ("doubleSetting", Float(1.2)),
                ("stringSetting", Value::String("test".to_owned())),
            ])),
            LocalOnly,
        )
        .build()
        .unwrap();

    assert!(matches!(client.wait_for_ready(Duration::from_secs(5)).await.unwrap(), ClientCacheState::HasLocalOverrideFlagDataOnly));
    assert!(client.get_value("enabledFeature", false, None).await);
    assert!(!client.get_value("disabledFeature", true, None).await);
    assert_eq!(client.get_value("intSetting", 0, None).await, 5);
    assert_eq!(client.get_value("doubleSetting", 0.0, None).await, 1.2);
    assert_eq!(client.get_value("stringSetting", String::default(), None).await, "test".to_owned());

    m.assert_async().await;
}

#[tokio::test]
async fn local_over_remote() {
    let mut server = mockito::Server::new_async().await;
    let (sdk_key, path) = produce_mock_path();
    let m = server.mock("GET", path.as_str()).with_status(200).with_body(construct_bool_json_payload("fakeKey", false)).create_async().await;

    let client = Client::builder(sdk_key.as_str())
        .base_url(server.url().as_str())
        .overrides(Box::new(MapDataSource::from([("fakeKey", Bool(true)), ("nonexisting", Bool(true))])), LocalOverRemote)
        .build()
        .unwrap();

    assert!(client.get_value("fakeKey", false, None).await);
    assert!(client.get_value("nonexisting", false, None).await);

    m.assert_async().await;
}

#[tokio::test]
async fn remote_over_local() {
    let mut server = mockito::Server::new_async().await;
    let (sdk_key, path) = produce_mock_path();
    let m = server.mock("GET", path.as_str()).with_status(200).with_body(construct_bool_json_payload("fakeKey", false)).create_async().await;

    let client = Client::builder(sdk_key.as_str())
        .base_url(server.url().as_str())
        .overrides(Box::new(MapDataSource::from([("fakeKey", Bool(true)), ("nonexisting", Bool(true))])), RemoteOverLocal)
        .build()
        .unwrap();

    assert!(!client.get_value("fakeKey", false, None).await);
    assert!(client.get_value("nonexisting", false, None).await);

    m.assert_async().await;
}

#[tokio::test]
async fn external_serde() {
    let content_result = fs::read_to_string("tests/data/test_yaml.yml").unwrap();
    let overrides = serde_yaml::from_str::<YamlOverrides>(content_result.as_str()).unwrap();

    let map: MapDataSource = overrides.flag_overrides.into();
    let client = Client::builder("local").overrides(Box::new(map), LocalOnly).build().unwrap();

    assert!(client.get_value("flag_1", false, None).await);
    assert!(!client.get_value("flag_2", true, None).await);
    assert_eq!(client.get_value("flag_3", String::default(), None).await, "some string".to_owned());
    assert_eq!(client.get_value("flag_4", 0, None).await, 1);
    assert_eq!(client.get_value("flag_5", 0, None).await, -1);
    assert_eq!(client.get_value("flag_6", 0.0, None).await, 0.5);
}

#[derive(Serialize, Deserialize)]
struct YamlOverrides {
    pub flag_overrides: HashMap<String, Value>,
}
