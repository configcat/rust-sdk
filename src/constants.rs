/// The ConfigCat Rust SDK's version.
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SDK_KEY_PROXY_PREFIX: &str = "configcat-proxy/";
pub const SDK_KEY_SECTION_LENGTH: usize = 22;
pub const SDK_KEY_PREFIX: &str = "configcat-sdk-1";
pub const CONFIG_FILE_NAME: &str = "config_v6.json";
pub const SERIALIZATION_FORMAT_VERSION: &str = "v2";

#[cfg(test)]
pub mod test_constants {
    pub const MOCK_PATH: &str = "/configuration-files/key/config_v6.json";
    pub const MOCK_KEY: &str = "key";
}
