use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::constants::{
    CONFIG_FILE_NAME, EU_CDN_URL, GLOBAL_CDN_URL, SERIALIZATION_FORMAT_VERSION,
};
use crate::fetch::fetcher::{FetchResponse, Fetcher};
use crate::model::config::{entry_from_cached_json, Config, ConfigEntry};
use crate::model::enums::DataGovernance;
use crate::modes::PollingMode;
use crate::options::Options;
use crate::utils::sha1;

struct ServiceState {
    fetcher: Fetcher,
    cached_entry: Arc<tokio::sync::Mutex<ConfigEntry>>,
    cache_key: String,
    offline: AtomicBool,
    initialized: AtomicBool,
    init: Once,
}

impl ServiceState {
    fn initialized(&self) {
        self.init
            .call_once(|| self.initialized.store(true, Ordering::SeqCst));
    }
}

pub struct ConfigService {
    state: Arc<ServiceState>,
    options: Arc<Options>,
    cancellation_token: CancellationToken,
    close: Once,
}

impl ConfigService {
    pub fn new(opts: &Arc<Options>) -> Self {
        let service = Self {
            state: Arc::new(ServiceState {
                cache_key: sha1(format!(
                    "{sdk_key}_{CONFIG_FILE_NAME}_{SERIALIZATION_FORMAT_VERSION}",
                    sdk_key = opts.sdk_key()
                )),
                fetcher: Fetcher::new(
                    opts.base_url().clone().unwrap_or_else(|| {
                        if *opts.data_governance() == DataGovernance::Global {
                            GLOBAL_CDN_URL.to_string()
                        } else {
                            EU_CDN_URL.to_string()
                        }
                    }),
                    !opts.base_url().is_none(),
                    opts.sdk_key(),
                    "",
                    *opts.http_timeout(),
                ),
                offline: AtomicBool::new(opts.offline()),
                initialized: AtomicBool::new(false),
                init: Once::new(),
                cached_entry: Arc::new(tokio::sync::Mutex::new(ConfigEntry::default())),
            }),
            options: Arc::clone(opts),
            cancellation_token: CancellationToken::new(),
            close: Once::new(),
        };

        match opts.polling_mode() {
            PollingMode::AutoPoll(interval) if !opts.offline() => service.start_poll(*interval),
            _ => service.state.initialized(),
        }

        service
    }

    pub async fn get_config(&self) -> Arc<Config> {
        let threshold = match self.options.polling_mode() {
            PollingMode::LazyLoad(cache_ttl) => Utc::now() - *cache_ttl,
            _ => DateTime::<Utc>::MIN_UTC,
        };
        let prefer_cached = match self.options.polling_mode() {
            PollingMode::LazyLoad(_) => false,
            _ => self.state.initialized.load(Ordering::SeqCst),
        };

        fetch_if_older(&self.state, &self.options, threshold, prefer_cached).await;
        let entry = self.state.cached_entry.lock().await;
        entry.config.clone()
    }

    pub async fn refresh(&self) {
        fetch_if_older(&self.state, &self.options, DateTime::<Utc>::MIN_UTC, false).await;
    }

    pub fn close(&self) {
        self.close.call_once(|| self.cancellation_token.cancel());
    }

    fn start_poll(&self, interval: Duration) {
        let state = Arc::clone(&self.state);
        let opts = Arc::clone(&self.options);
        let token = self.cancellation_token.clone();

        tokio::spawn(async move {
            let mut int = tokio::time::interval(interval);
            loop {
                tokio::select! {
                    _ = int.tick() => {
                        fetch_if_older(&state, &opts, Utc::now() - (interval / 2), false).await;
                    },
                    _ = token.cancelled() => break
                }
            }
        });
    }
}

impl Drop for ConfigService {
    fn drop(&mut self) {
        self.close();
    }
}

async fn fetch_if_older(
    state: &Arc<ServiceState>,
    options: &Arc<Options>,
    threshold: DateTime<Utc>,
    prefer_cached: bool,
) {
    let mut entry = state.cached_entry.lock().await;
    let from_cache = read_cache(state, options, &entry.config_json).unwrap_or_default();

    if !from_cache.is_empty() && *entry != from_cache {
        *entry = from_cache;
    }

    if entry.fetch_time > threshold || state.offline.load(Ordering::SeqCst) || prefer_cached {
        state.initialized();
        return;
    }

    let response = state.fetcher.fetch(&entry.etag).await;
    match response {
        FetchResponse::Fetched(new_entry) => {
            *entry = new_entry;
            println!("{:?}", *entry);
            options
                .cache()
                .write(&state.cache_key, entry.serialize().as_str())
        }
        FetchResponse::NotModified => {
            *entry = entry.with_time(Utc::now());
            options
                .cache()
                .write(&state.cache_key, entry.serialize().as_str())
        }
        FetchResponse::Failed(_, transient) if !transient && !entry.is_empty() => {
            *entry = entry.with_time(Utc::now());
            options
                .cache()
                .write(&state.cache_key, entry.serialize().as_str())
        }
        _ => {}
    }
    state.initialized();
}

fn read_cache(
    state: &Arc<ServiceState>,
    options: &Arc<Options>,
    from_memory_str: &String,
) -> Option<ConfigEntry> {
    let from_cache_str = options.cache().read(&state.cache_key).unwrap_or_default();
    if from_cache_str.is_empty() || from_cache_str == *from_memory_str {
        return None;
    }
    let parsed = entry_from_cached_json(from_cache_str.as_str());
    match parsed {
        Ok(entry) => Some(entry),
        Err(err) => {
            log_err!(event_id: 2201, "{err}");
            None
        }
    }
}

#[cfg(test)]
mod service_tests {
    use reqwest::header::{ETAG, IF_NONE_MATCH};
    use std::sync::Arc;
    use std::time::Duration;

    use crate::cache::EmptyConfigCache;
    use crate::constants::test_constants::{MOCK_KEY, MOCK_PATH};
    use crate::fetch::service::ConfigService;
    use crate::modes::PollingMode;
    use crate::options::OptionsBuilder;

    #[test]
    fn cache_key_generation() {
        {
            let opts = Arc::new(
                OptionsBuilder::new(
                    "configcat-sdk-1/TEST_KEY-0123456789012/1234567890123456789012",
                )
                .polling_mode(PollingMode::Manual)
                .build()
                .unwrap(),
            );
            let service = ConfigService::new(&opts);
            assert_eq!(
                service.state.cache_key.as_str(),
                "f83ba5d45bceb4bb704410f51b704fb6dfa19942"
            )
        }
        {
            let opts = Arc::new(
                OptionsBuilder::new(
                    "configcat-sdk-1/TEST_KEY2-123456789012/1234567890123456789012",
                )
                .polling_mode(PollingMode::Manual)
                .build()
                .unwrap(),
            );
            let service = ConfigService::new(&opts);
            assert_eq!(
                service.state.cache_key.as_str(),
                "da7bfd8662209c8ed3f9db96daed4f8d91ba5876"
            )
        }
    }

    #[tokio::test]
    async fn get_config() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(r#"{"f": {"testKey":{"t":1,"v":{"s": "testValue"}}}, "s": []}"#)
            .create_async()
            .await;

        let opts = Arc::new(
            OptionsBuilder::new(MOCK_KEY)
                .base_url(server.url().as_str())
                .cache(Box::new(EmptyConfigCache::new()))
                .build()
                .unwrap(),
        );
        let service = ConfigService::new(&opts);
        let config = service.get_config().await;
        assert_eq!(config.settings.len(), 1);
    }

    #[tokio::test]
    async fn auto_poll() {
        let mut server = mockito::Server::new_async().await;
        let m1 = server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(r#"{"f": {"testKey":{"t":1,"v":{"s": "test1"}}}, "s": []}"#)
            .with_header(ETAG.as_str(), "etag1")
            .expect_at_least(1)
            .create_async()
            .await;

        let m2 = server
            .mock("GET", MOCK_PATH)
            .match_header(IF_NONE_MATCH.as_str(), "etag1")
            .with_status(200)
            .with_body(r#"{"f": {"testKey":{"t":1,"v":{"s": "test2"}}}, "s": []}"#)
            .with_header(ETAG.as_str(), "etag2")
            .expect_at_least(1)
            .create_async()
            .await;

        let m3 = server
            .mock("GET", MOCK_PATH)
            .match_header(IF_NONE_MATCH.as_str(), "etag2")
            .with_status(304)
            .with_header(ETAG.as_str(), "etag2")
            .expect_at_least(1)
            .create_async()
            .await;

        let opts = Arc::new(
            OptionsBuilder::new(MOCK_KEY)
                .base_url(server.url().as_str())
                .polling_mode(PollingMode::AutoPoll(Duration::from_millis(200)))
                .build()
                .unwrap(),
        );
        let service = ConfigService::new(&opts);
        let config1 = service.get_config().await;

        let setting1 = &config1.settings["testKey"];
        assert_eq!(setting1.value.clone().unwrap().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_secs(1)).await;

        let config2 = service.get_config().await;

        let setting2 = &config2.settings["testKey"];
        assert_eq!(setting2.value.clone().unwrap().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }
}
