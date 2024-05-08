use sha1::{Digest, Sha1};
use sha2::Sha256;

pub fn sha1(payload: &str) -> String {
    let hash = Sha1::digest(payload);
    base16ct::lower::encode_string(&hash)
}

pub fn sha256(payload: &str, salt: &str, ctx_salt: &str) -> String {
    let mut cont = String::new();
    cont.push_str(payload);
    cont.push_str(salt);
    cont.push_str(ctx_salt);
    let hash = Sha256::digest(cont);
    base16ct::lower::encode_string(&hash)
}

#[cfg(test)]
pub mod test_utils {
    use log::{Level, Log, Metadata, Record};

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
}
