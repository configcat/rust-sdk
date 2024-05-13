use crate::utils::{construct_bool_json_payload, produce_mock_path};
use configcat::OverrideBehavior::{LocalOnly, LocalOverRemote, RemoteOverLocal};
use configcat::Value::{Bool, Float, Int};
use configcat::{Client, FileDataSource, MapDataSource, Value};

mod utils;

#[tokio::test]
async fn file_simple() {
    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/test_json_simple.json").unwrap()), LocalOnly).build().unwrap();

    assert!(client.get_bool_value("enabledFeature", None, false).await);
    assert!(!client.get_bool_value("disabledFeature", None, true).await);
    assert_eq!(client.get_int_value("intSetting", None, 0).await, 5);
    assert_eq!(client.get_float_value("doubleSetting", None, 0.0).await, 1.2);
    assert_eq!(client.get_str_value("stringSetting", None, String::default()).await, "test".to_owned());
}

#[tokio::test]
async fn file_complex() {
    let client = Client::builder("local").overrides(Box::new(FileDataSource::new("tests/data/test_json_complex.json").unwrap()), LocalOnly).build().unwrap();

    assert!(client.get_bool_value("enabledFeature", None, false).await);
    assert!(!client.get_bool_value("disabledFeature", None, true).await);
    assert_eq!(client.get_int_value("intSetting", None, 0).await, 5);
    assert_eq!(client.get_float_value("doubleSetting", None, 0.0).await, 1.2);
    assert_eq!(client.get_str_value("stringSetting", None, String::default()).await, "test".to_owned());
}

#[tokio::test]
async fn map() {
    let client = Client::builder("local")
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

    assert!(client.get_bool_value("enabledFeature", None, false).await);
    assert!(!client.get_bool_value("disabledFeature", None, true).await);
    assert_eq!(client.get_int_value("intSetting", None, 0).await, 5);
    assert_eq!(client.get_float_value("doubleSetting", None, 0.0).await, 1.2);
    assert_eq!(client.get_str_value("stringSetting", None, String::default()).await, "test".to_owned());
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

    assert!(client.get_bool_value("fakeKey", None, false).await);
    assert!(client.get_bool_value("nonexisting", None, false).await);

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

    assert!(!client.get_bool_value("fakeKey", None, false).await);
    assert!(client.get_bool_value("nonexisting", None, false).await);

    m.assert_async().await;
}
