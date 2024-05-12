#![allow(dead_code)]

use crate::utils::RecordingLogger;
use configcat::OverrideBehavior::LocalOnly;
use configcat::{Client, FileDataSource};
use log::set_max_level;

mod utils;

fn init() {
    set_max_level(log::LevelFilter::Info);
    _ = log::set_logger(&RecordingLogger {});
}

#[tokio::test]
async fn prerequisite_circular_deps() {
    init();

    let tests = vec![
        ("key1", "'key1' -> 'key1'"),
        ("key2", "'key2' -> 'key3' -> 'key2'"),
        ("key4", "'key4' -> 'key3' -> 'key2' -> 'key3'"),
    ];

    let client = Client::builder("local")
        .overrides(
            Box::new(FileDataSource::new("tests/data/test_circulardependency_v6.json").unwrap()),
            LocalOnly,
        )
        .build()
        .unwrap();

    for test in tests {
        _ = client.get_flag_details(test.0, None).await;
        let logs = RecordingLogger::LOGS.take();
        assert!(logs.contains(test.1));
    }
}
