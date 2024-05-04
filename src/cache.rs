/// A cache API used to make custom cache implementations.
pub trait ConfigCache: Sync + Send {
    /// Gets the actual value from the cache identified by the given `key`.
    fn read(&self, key: &str) -> Option<String>;

    /// Writes the given `value` to the cache by the given `key`.
    fn write(&self, key: &str, value: &str);
}

pub struct EmptyConfigCache {}

impl EmptyConfigCache {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConfigCache for EmptyConfigCache {
    fn read(&self, _: &str) -> Option<String> {
        None
    }
    fn write(&self, _: &str, _: &str) {}
}
