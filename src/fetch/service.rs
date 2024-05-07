use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::constants::{
    CONFIG_FILE_NAME, EU_CDN_URL, GLOBAL_CDN_URL, SERIALIZATION_FORMAT_VERSION,
};
use crate::errors::ClientError;
use crate::fetch::fetcher::{FetchResponse, Fetcher};
use crate::model::config::{entry_from_cached_json, Config, ConfigEntry};
use crate::model::enums::DataGovernance;
use crate::modes::PollingMode;
use crate::options::Options;
use crate::utils::sha1;

pub enum ServiceResult {
    Ok(Arc<Config>, DateTime<Utc>),
    Failed(ClientError, Arc<Config>, DateTime<Utc>),
}

impl ServiceResult {
    pub fn config(&self) -> &Arc<Config> {
        match self {
            ServiceResult::Ok(entry, _) => entry,
            ServiceResult::Failed(_, entry, _) => entry,
        }
    }
}

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
                    opts.base_url()
                        .clone()
                        .unwrap_or_else(|| match *opts.data_governance() {
                            DataGovernance::Global => GLOBAL_CDN_URL.to_owned(),
                            DataGovernance::EU => EU_CDN_URL.to_owned(),
                        }),
                    !opts.base_url().is_none(),
                    opts.sdk_key(),
                    opts.polling_mode().mode_identifier(),
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

    pub async fn get_config(&self) -> ServiceResult {
        let initialized = self.state.initialized.load(Ordering::SeqCst);
        let threshold = match self.options.polling_mode() {
            PollingMode::LazyLoad(cache_ttl) => Utc::now() - *cache_ttl,
            PollingMode::AutoPoll(interval) if !initialized => Utc::now() - *interval,
            _ => DateTime::<Utc>::MIN_UTC,
        };
        let prefer_cached = match self.options.polling_mode() {
            PollingMode::LazyLoad(_) => false,
            _ => initialized,
        };
        fetch_if_older(&self.state, &self.options, threshold, prefer_cached).await
    }

    pub async fn refresh(&self) -> Result<(), ClientError> {
        let result =
            fetch_if_older(&self.state, &self.options, DateTime::<Utc>::MAX_UTC, false).await;
        match result {
            ServiceResult::Ok(_, _) => Ok(()),
            ServiceResult::Failed(err, _, _) => Err(err),
        }
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
) -> ServiceResult {
    let mut entry = state.cached_entry.lock().await;
    let from_cache = read_cache(state, options, &entry.config_json).unwrap_or_default();

    if !from_cache.is_empty() && *entry != from_cache {
        *entry = from_cache;
    }

    if entry.fetch_time > threshold || state.offline.load(Ordering::SeqCst) || prefer_cached {
        state.initialized();
        return ServiceResult::Ok(entry.config.clone(), entry.fetch_time);
    }

    let response = state.fetcher.fetch(&entry.etag).await;
    state.initialized();
    match response {
        FetchResponse::Fetched(new_entry) => {
            *entry = new_entry;
            options
                .cache()
                .write(&state.cache_key, entry.serialize().as_str());
            ServiceResult::Ok(entry.config.clone(), entry.fetch_time)
        }
        FetchResponse::NotModified => {
            *entry = entry.with_time(Utc::now());
            options
                .cache()
                .write(&state.cache_key, entry.serialize().as_str());
            ServiceResult::Ok(entry.config.clone(), entry.fetch_time)
        }
        FetchResponse::Failed(err, transient) => {
            if !transient && !entry.is_empty() {
                *entry = entry.with_time(Utc::now());
                options
                    .cache()
                    .write(&state.cache_key, entry.serialize().as_str());
            }
            ServiceResult::Failed(
                ClientError::Fetch(err.to_string()),
                entry.config.clone(),
                entry.fetch_time,
            )
        }
    }
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
            log_err!(event_id: 2201, "Error occurred while reading the cache. ({err})");
            None
        }
    }
}

#[cfg(test)]
mod service_tests {
    use crate::cache::EmptyConfigCache;
    use crate::ConfigCache;
    use chrono::{DateTime, Utc};
    use mockito::{Mock, ServerGuard};
    use reqwest::header::{ETAG, IF_NONE_MATCH};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::constants::test_constants::{MOCK_KEY, MOCK_PATH};
    use crate::fetch::service::ConfigService;
    use crate::model::config::entry_from_cached_json;
    use crate::modes::PollingMode;
    use crate::options::{Options, OptionsBuilder};

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
    async fn auto_poll() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2, m3) = create_success_mock_sequence(&mut server).await;

        let opts = create_options(
            server.url(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }

    #[tokio::test]
    async fn auto_poll_failed() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2) = create_success_then_failure_mock(&mut server).await;

        let opts = create_options(
            server.url(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        m1.assert_async().await;
        m2.assert_async().await;
    }

    #[tokio::test]
    async fn lazy_load() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2, m3) = create_success_mock_sequence(&mut server).await;

        let opts = create_options(
            server.url(),
            PollingMode::LazyLoad(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }

    #[tokio::test]
    async fn lazy_load_failed() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2) = create_success_then_failure_mock(&mut server).await;

        let opts = create_options(
            server.url(),
            PollingMode::LazyLoad(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        m1.assert_async().await;
        m2.assert_async().await;
    }

    #[tokio::test]
    async fn manual_poll() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2, m3) = create_success_mock_sequence(&mut server).await;

        let opts = create_options(server.url(), PollingMode::Manual, None);
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        assert!(result.config().settings.is_empty());

        _ = service.refresh().await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        _ = service.refresh().await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        _ = service.refresh().await;

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }

    #[tokio::test]
    async fn fail_http_reload_from_cache() {
        let mut server = mockito::Server::new_async().await;
        let m = create_failure_mock(&mut server, 1).await;

        let opts = create_options(
            server.url(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test1",
                Utc::now() - Duration::from_secs(1),
                "etag1",
            )))),
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        opts.cache().write(
            service.state.clone().cache_key.as_str(),
            construct_cache_payload("test2", Utc::now(), "etag2").as_str(),
        );

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        m.assert_async().await;
    }

    #[tokio::test]
    async fn poll_respects_cache_expiration() {
        let mut server = mockito::Server::new_async().await;
        let m1 = create_success_mock_with_etag(&mut server, "etag1", 0).await;
        let m2 = create_success_mock_with_etag(&mut server, "etag2", 0).await;

        let opts = create_options(
            server.url(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test1",
                Utc::now(),
                "etag1",
            )))),
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        opts.cache().write(
            service.state.clone().cache_key.as_str(),
            construct_cache_payload("test2", Utc::now(), "etag2").as_str(),
        );

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
    }

    #[tokio::test]
    async fn poll_cache_write() {
        let mut server = mockito::Server::new_async().await;
        let m = create_success_mock(&mut server, 1).await;

        let opts = create_options(
            server.url(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(String::default()))),
        );
        let service = ConfigService::new(&opts);

        let result = service.get_config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().unwrap().string_val.unwrap(), "test1");

        let cached = opts.cache().read("").unwrap();
        let entry = entry_from_cached_json(cached.as_str()).unwrap();

        assert_eq!(entry.etag, "etag1");
        assert_eq!(
            entry.config_json,
            r#"{"f": {"testKey":{"t":1,"v":{"s": "test1"}}}, "s": []}"#
        );

        m.assert_async().await;
    }

    #[tokio::test]
    async fn offline() {
        let mut server = mockito::Server::new_async().await;
        let m = create_success_mock(&mut server, 0).await;

        let opts = Arc::new(
            OptionsBuilder::new(MOCK_KEY)
                .base_url(server.url().as_str())
                .polling_mode(PollingMode::AutoPoll(Duration::from_millis(100)))
                .offline(true)
                .build()
                .unwrap(),
        );
        let service = ConfigService::new(&opts);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = service.get_config().await;
        assert!(result.config().settings.is_empty());

        m.assert_async().await;
    }

    fn create_options(
        url: String,
        mode: PollingMode,
        cache: Option<Box<dyn ConfigCache>>,
    ) -> Arc<Options> {
        Arc::new(
            OptionsBuilder::new(MOCK_KEY)
                .cache(cache.unwrap_or(Box::new(EmptyConfigCache::new())))
                .base_url(url.as_str())
                .polling_mode(mode)
                .build()
                .unwrap(),
        )
    }

    async fn create_success_mock_sequence(server: &mut ServerGuard) -> (Mock, Mock, Mock) {
        let m1 = create_success_mock(server, 1).await;

        let m2 = server
            .mock("GET", MOCK_PATH)
            .match_header(IF_NONE_MATCH.as_str(), "etag1")
            .with_status(200)
            .with_body(construct_json_payload("test2"))
            .with_header(ETAG.as_str(), "etag2")
            .expect(1)
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

        (m1, m2, m3)
    }

    async fn create_success_then_failure_mock(server: &mut ServerGuard) -> (Mock, Mock) {
        let m1 = create_success_mock(server, 1).await;
        let m2 = create_failure_mock(server, 1).await;
        (m1, m2)
    }

    async fn create_success_mock(server: &mut ServerGuard, expect: usize) -> Mock {
        server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(construct_json_payload("test1"))
            .with_header(ETAG.as_str(), "etag1")
            .expect(expect)
            .create_async()
            .await
    }

    async fn create_success_mock_with_etag(
        server: &mut ServerGuard,
        etag: &str,
        expect: usize,
    ) -> Mock {
        server
            .mock("GET", MOCK_PATH)
            .match_header(IF_NONE_MATCH.as_str(), etag)
            .with_status(200)
            .with_body(construct_json_payload("test1"))
            .with_header(ETAG.as_str(), etag)
            .expect(expect)
            .create_async()
            .await
    }

    async fn create_failure_mock(server: &mut ServerGuard, expect: usize) -> Mock {
        server
            .mock("GET", MOCK_PATH)
            .match_header(IF_NONE_MATCH.as_str(), "etag1")
            .with_status(502)
            .expect_at_least(expect)
            .create_async()
            .await
    }

    fn construct_cache_payload(val: &str, time: DateTime<Utc>, etag: &str) -> String {
        time.timestamp_millis().to_string() + "\n" + etag + "\n" + &construct_json_payload(val)
    }

    fn construct_json_payload(val: &str) -> String {
        format!(r#"{{"f": {{"testKey":{{"t":1,"v":{{"s": "{val}"}}}}}}, "s": []}}"#)
    }

    struct SingleValueCache {
        pub val: Mutex<String>,
    }

    impl SingleValueCache {
        fn new(val: String) -> Self {
            Self {
                val: Mutex::new(val),
            }
        }
    }

    impl ConfigCache for SingleValueCache {
        fn read(&self, _: &str) -> Option<String> {
            Some(self.val.lock().unwrap().clone())
        }

        fn write(&self, _: &str, value: &str) {
            let mut val = self.val.lock().unwrap();
            *val = value.to_owned()
        }
    }
}
