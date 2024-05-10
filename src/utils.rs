use semver::{Error, Version};
use sha1::{Digest, Sha1};
use sha2::Sha256;

pub fn sha1(payload: &str) -> String {
    let hash = Sha1::digest(payload);
    base16ct::lower::encode_string(&hash)
}

pub fn sha256(payload: &str, salt: &str, ctx_salt: &str) -> String {
    let mut cont = String::with_capacity(payload.len() + salt.len() + ctx_salt.len());
    cont.push_str(payload);
    cont.push_str(salt);
    cont.push_str(ctx_salt);
    let hash = Sha256::digest(cont);
    base16ct::lower::encode_string(&hash)
}

pub fn parse_semver(input: &str) -> Result<Version, Error> {
    let mut input_mut = input;
    if let Some((first, _)) = input.split_once('+') {
        input_mut = first;
    }
    Version::parse(input_mut)
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
