use crate::cache::EmptyConfigCache;
use crate::model::enums::DataGovernance;
use crate::modes::PollingMode;
use crate::ConfigCache;
use std::borrow::Borrow;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OptionsError {
    #[error("SDK key is invalid. ({0})")]
    InvalidSdkKey(String),
}

pub struct Options {
    sdk_key: String,
    offline: bool,
    base_url: Option<String>,
    data_governance: DataGovernance,
    http_timeout: Duration,
    cache: Box<dyn ConfigCache>,
    polling_mode: PollingMode,
}

impl Options {
    pub fn sdk_key(&self) -> &str {
        &self.sdk_key
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn base_url(&self) -> &Option<String> {
        &self.base_url
    }

    pub fn data_governance(&self) -> &DataGovernance {
        &self.data_governance
    }

    pub fn http_timeout(&self) -> &Duration {
        &self.http_timeout
    }

    pub fn cache(&self) -> &dyn ConfigCache {
        self.cache.borrow()
    }

    pub fn polling_mode(&self) -> &PollingMode {
        &self.polling_mode
    }
}

pub struct OptionsBuilder {
    sdk_key: String,
    base_url: Option<String>,
    data_governance: Option<DataGovernance>,
    http_timeout: Option<Duration>,
    cache: Option<Box<dyn ConfigCache>>,
    offline: bool,
    polling_mode: Option<PollingMode>,
}

impl OptionsBuilder {
    pub fn new(sdk_key: &str) -> Self {
        Self {
            sdk_key: sdk_key.to_string(),
            offline: false,
            http_timeout: None,
            base_url: None,
            cache: None,
            polling_mode: None,
            data_governance: None,
        }
    }

    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    pub fn http_timeout(mut self, timeout: Duration) -> Self {
        self.http_timeout = Some(timeout);
        self
    }

    pub fn base_url(mut self, base_url: &str) -> Self {
        self.base_url = Some(base_url.to_string());
        self
    }

    pub fn data_governance(mut self, data_governance: DataGovernance) -> Self {
        self.data_governance = Some(data_governance);
        self
    }

    pub fn cache(mut self, cache: Box<dyn ConfigCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn polling_mode(mut self, polling_mode: PollingMode) -> Self {
        self.polling_mode = Some(polling_mode);
        self
    }

    pub fn build(self) -> Result<Options, OptionsError> {
        if self.sdk_key.is_empty() {
            return Err(OptionsError::InvalidSdkKey(
                "SDK Key cannot be empty".into(),
            ));
        }
        Ok(Options {
            sdk_key: self.sdk_key.clone(),
            offline: self.offline,
            cache: self.cache.unwrap_or(Box::new(EmptyConfigCache::new())),
            polling_mode: self
                .polling_mode
                .unwrap_or(PollingMode::AutoPoll(Duration::from_secs(60))),
            base_url: self.base_url,
            data_governance: self.data_governance.unwrap_or(DataGovernance::Global),
            http_timeout: self.http_timeout.unwrap_or(Duration::from_secs(30)),
        })
    }
}
