use log::{Level, Log, Metadata, Record};
use rand::distributions::{Alphanumeric, DistString};

pub fn produce_mock_path() -> (String, String) {
    let sdk_key = rand_sdk_key();
    (
        sdk_key.clone(),
        format!("/configuration-files/{sdk_key}/config_v6.json"),
    )
}

pub fn rand_sdk_key() -> String {
    format!("{}/{}", rand_str(22), rand_str(22))
}

pub fn construct_bool_json_payload(key: &str, val: bool) -> String {
    format!(r#"{{"f": {{"{key}":{{"t":0,"v":{{"b": {val}}}}}}}, "s": []}}"#)
}

fn rand_str(len: usize) -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}

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
            Level::Error => "[ERROR]",
            Level::Warn => "[WARN]",
            Level::Info => "[INFO]",
            Level::Debug => "[DEBUG]",
            Level::Trace => "[TRACE]",
        };
        println!("{level} {}", record.args());
    }

    fn flush(&self) {}
}
