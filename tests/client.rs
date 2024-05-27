#![allow(dead_code)]

use crate::utils::rand_sdk_key;
use configcat::OverrideBehavior::LocalOnly;
use configcat::{Client, ClientBuilder, FileDataSource, PollingMode, User};

mod utils;

#[tokio::test]
async fn default_user_flag() {
    let client = client_builder().default_user(User::new("id1")).build().unwrap();
    let details_without_user = client.get_flag_details("disabledFeature", None).await;

    assert_eq!("id1", details_without_user.user.unwrap()[User::IDENTIFIER].to_string().as_str());

    let details = client.get_flag_details("disabledFeature", Some(User::new("id2"))).await;

    assert_eq!("id2", details.user.unwrap()[User::IDENTIFIER].to_string().as_str());
}

#[tokio::test]
async fn default_user_value() {
    let client = client_builder().default_user(User::new("id1")).build().unwrap();
    let details_without_user = client.get_value_details("disabledFeature", None, false).await;

    assert_eq!("id1", details_without_user.user.unwrap()[User::IDENTIFIER].to_string().as_str());

    let details = client.get_value_details("disabledFeature", Some(User::new("id2")), false).await;

    assert_eq!("id2", details.user.unwrap()[User::IDENTIFIER].to_string().as_str());
}

#[tokio::test]
async fn default_value_type_mismatch() {
    let client = client_builder().build().unwrap();
    let value = client.get_value("disabledFeature", None, "def".to_owned()).await;

    assert_eq!(value, "def");
}

#[tokio::test]
async fn get_all_keys() {
    let client = client_builder().build().unwrap();
    let mut keys = client.get_all_keys().await;
    keys.sort();
    let mut exp = vec!["stringSetting", "intSetting", "doubleSetting", "disabledFeature", "enabledFeature"];
    exp.sort();

    assert_eq!(keys, exp);
}

#[tokio::test]
async fn get_all_keys_empty() {
    let client = Client::builder(rand_sdk_key().as_str()).polling_mode(PollingMode::Manual).build().unwrap();
    let keys = client.get_all_keys().await;

    assert!(keys.is_empty());
}

#[tokio::test]
async fn get_all_values() {
    let client = client_builder().build().unwrap();
    let values = client.get_all_values(None).await;

    assert!(!values["disabledFeature"].as_bool().unwrap());
    assert!(values["enabledFeature"].as_bool().unwrap());
    assert_eq!(values["stringSetting"].as_str().unwrap(), "test");
    assert_eq!(values["intSetting"].as_int().unwrap(), 5);
    assert_eq!(values["doubleSetting"].as_float().unwrap(), 1.2);
}

#[tokio::test]
async fn get_all_values_with_user() {
    let client = client_builder().build().unwrap();
    let values = client.get_all_values(Some(User::new("a@matching.com"))).await;

    assert!(values["disabledFeature"].as_bool().unwrap());
}

fn client_builder() -> ClientBuilder {
    Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/test_json_complex.json").unwrap()), LocalOnly)
}
