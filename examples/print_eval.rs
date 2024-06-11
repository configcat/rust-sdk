use configcat::*;
use log::kv::Key;
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Info level logging helps to inspect the feature flag evaluation process.
    // Use the default Warning level to avoid too detailed logging in your application.
    log::set_max_level(LevelFilter::Info);
    log::set_logger(&PrintLog {}).unwrap();

    let client = Client::builder("PKDVCLf-Hq-h-kCzMp-L7Q/HhOWfwVtZ0mb30i9wi17GQ")
        .polling_mode(PollingMode::AutoPoll(Duration::from_secs(5)))
        .build()
        .unwrap();

    let is_awesome_enabled = client
        .get_value("isAwesomeFeatureEnabled", None, false)
        .await;

    println!("isAwesomeFeatureEnabled: {is_awesome_enabled}");

    let user = User::new("#SOME-USER-ID#").email("configcat@example.com");

    let is_poc_enabled = client
        .get_value("isPOCFeatureEnabled", Some(user), false)
        .await;

    println!("isPOCFeatureEnabled: {is_poc_enabled}");
}

// Example log implementation.
pub struct PrintLog {}

impl Log for PrintLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level() && metadata.target().contains("configcat")
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = match record.level() {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        let event_id = record.key_values().get(Key::from("event_id")).unwrap();
        println!("{level} [{event_id}] {}", record.args());
    }

    fn flush(&self) {}
}
