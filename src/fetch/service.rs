use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;

use chrono::{DateTime, Utc};
use log::warn;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::builder::Options;
use crate::constants::{CONFIG_FILE_NAME, SERIALIZATION_FORMAT_VERSION};
use crate::errors::ClientError;
use crate::fetch::fetcher::{FetchResponse, Fetcher};
use crate::model::config::{entry_from_cached_json, process_overrides, Config, ConfigEntry};
use crate::model::enums::DataGovernance;
use crate::modes::PollingMode;
use crate::r#override::OptionalOverrides;
use crate::utils::sha1;
use crate::ClientCacheState::{
    HasCachedFlagDataOnly, HasLocalOverrideFlagDataOnly, HasUpToDateFlagData, NoFlagData,
};
use crate::{ClientCacheState, OverrideBehavior};

pub enum ServiceResult {
    Ok(ConfigResult),
    Err(ClientError, ConfigResult),
}

pub struct ConfigResult {
    config: Arc<Config>,
    fetch_time: DateTime<Utc>,
}

impl ConfigResult {
    fn new(config: Arc<Config>, fetch_time: DateTime<Utc>) -> Self {
        Self { config, fetch_time }
    }

    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn fetch_time(&self) -> &DateTime<Utc> {
        &self.fetch_time
    }
}

struct ServiceState {
    fetcher: Fetcher,
    cached_entry: Arc<tokio::sync::Mutex<ConfigEntry>>,
    cache_key: String,
    offline: AtomicBool,
    initialized: AtomicBool,
    init: Once,
    init_wait: Semaphore,
}

impl ServiceState {
    fn initialized(&self) {
        self.init.call_once(|| {
            self.initialized.store(true, Ordering::SeqCst);
            self.init_wait.add_permits(1);
        });
    }
}

pub struct ConfigService {
    state: Arc<ServiceState>,
    options: Arc<Options>,
    cancellation_token: CancellationToken,
    close: Once,
}

impl ConfigService {
    const GLOBAL_CDN_URL: &'static str = "https://cdn-global.configcat.com";
    const EU_CDN_URL: &'static str = "https://cdn-eu.configcat.com";

    pub fn new(opts: Arc<Options>) -> Result<Self, ClientError> {
        let url = if let Some(base_url) = opts.base_url() {
            base_url.as_str()
        } else {
            match *opts.data_governance() {
                DataGovernance::Global => Self::GLOBAL_CDN_URL,
                DataGovernance::EU => Self::EU_CDN_URL,
            }
        };
        match Fetcher::new(
            url,
            opts.base_url().is_some(),
            opts.sdk_key(),
            opts.polling_mode().mode_identifier(),
            *opts.http_timeout(),
        ) {
            Ok(fetcher) => {
                let service = Self {
                    state: Arc::new(ServiceState {
                        cache_key: sha1(
                            format!(
                                "{}_{CONFIG_FILE_NAME}_{SERIALIZATION_FORMAT_VERSION}",
                                opts.sdk_key()
                            )
                            .as_str(),
                        ),
                        fetcher,
                        offline: AtomicBool::new(opts.offline()),
                        initialized: AtomicBool::new(false),
                        init: Once::new(),
                        init_wait: Semaphore::new(0),
                        cached_entry: Arc::new(tokio::sync::Mutex::new(ConfigEntry::default())),
                    }),
                    options: opts,
                    cancellation_token: CancellationToken::new(),
                    close: Once::new(),
                };
                match service.options.polling_mode() {
                    PollingMode::AutoPoll(interval)
                        if !service.options.offline()
                            && !service.options.overrides().is_local() =>
                    {
                        service.start_poll(*interval);
                    }
                    _ => service.state.initialized(),
                }
                Ok(service)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn config(&self) -> ConfigResult {
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
        let result = fetch_if_older(&self.state, &self.options, threshold, prefer_cached).await;
        match result {
            ServiceResult::Ok(config_result) | ServiceResult::Err(_, config_result) => {
                config_result
            }
        }
    }

    pub async fn refresh(&self) -> Result<(), ClientError> {
        let result =
            fetch_if_older(&self.state, &self.options, DateTime::<Utc>::MAX_UTC, false).await;
        match result {
            ServiceResult::Ok(_) => Ok(()),
            ServiceResult::Err(err, _) => Err(err),
        }
    }

    pub fn close(&self) {
        self.close.call_once(|| self.cancellation_token.cancel());
    }

    pub fn set_mode(&self, offline: bool) {
        self.state.offline.store(offline, Ordering::SeqCst);
    }

    pub fn is_offline(&self) -> bool {
        self.state.offline.load(Ordering::SeqCst)
    }

    pub async fn wait_for_init(&self) -> ClientCacheState {
        if !self.state.initialized.load(Ordering::SeqCst) {
            _ = self.state.init_wait.acquire().await;
        }
        self.determine_cache_state().await
    }

    async fn determine_cache_state(&self) -> ClientCacheState {
        if self.options.overrides().is_local() {
            return HasLocalOverrideFlagDataOnly;
        }

        let mut entry = self.state.cached_entry.lock().await;

        if let PollingMode::AutoPoll(interval) = self.options.polling_mode() {
            if !entry.is_expired(*interval) {
                return HasUpToDateFlagData;
            }
            if entry.is_empty() {
                return NoFlagData;
            }
            HasCachedFlagDataOnly
        } else {
            let from_cache =
                read_cache(&self.state, &self.options, &entry.cache_str).unwrap_or_default();
            if !from_cache.is_empty() && *entry != from_cache {
                *entry = from_cache;
            }
            if let PollingMode::LazyLoad(interval) = self.options.polling_mode() {
                if !entry.is_expired(*interval) {
                    return HasUpToDateFlagData;
                }
            }
            if entry.is_empty() {
                return NoFlagData;
            }
            HasCachedFlagDataOnly
        }
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
                    () = token.cancelled() => break
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
    if let Some(ov) = options.overrides() {
        if matches!(ov.behavior(), OverrideBehavior::LocalOnly) {
            if entry.is_empty() {
                *entry = ConfigEntry {
                    config: Arc::new(Config {
                        settings: ov.source().settings().clone(),
                        ..Config::default()
                    }),
                    ..ConfigEntry::local()
                };
            }
            return ServiceResult::Ok(ConfigResult::new(
                entry.config.clone(),
                DateTime::<Utc>::MIN_UTC,
            ));
        }
    }

    let from_cache = read_cache(state, options, &entry.cache_str).unwrap_or_default();

    if !from_cache.is_empty() && *entry != from_cache {
        *entry = from_cache;
    }

    if entry.fetch_time > threshold || state.offline.load(Ordering::SeqCst) || prefer_cached {
        state.initialized();
        return ServiceResult::Ok(ConfigResult::new(entry.config.clone(), entry.fetch_time));
    }

    let response = state.fetcher.fetch(&entry.etag).await;
    state.initialized();
    match response {
        FetchResponse::Fetched(mut new_entry) => {
            process_overrides(&mut new_entry, options.overrides());
            *entry = new_entry;
            options
                .cache()
                .write(&state.cache_key, entry.cache_str.as_str());
            ServiceResult::Ok(ConfigResult::new(entry.config.clone(), entry.fetch_time))
        }
        FetchResponse::NotModified => {
            entry.set_fetch_time(Utc::now());
            options
                .cache()
                .write(&state.cache_key, entry.cache_str.as_str());
            ServiceResult::Ok(ConfigResult::new(entry.config.clone(), entry.fetch_time))
        }
        FetchResponse::Failed(err, transient) => {
            if !transient && !entry.is_empty() {
                entry.set_fetch_time(Utc::now());
                options
                    .cache()
                    .write(&state.cache_key, entry.cache_str.as_str());
            }
            ServiceResult::Err(
                err,
                ConfigResult::new(entry.config.clone(), entry.fetch_time),
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
    if from_cache_str.is_empty() || from_cache_str.as_str() == from_memory_str {
        return None;
    }
    let parsed = entry_from_cached_json(from_cache_str.as_str());
    match parsed {
        Ok(mut entry) => {
            process_overrides(&mut entry, options.overrides());
            Some(entry)
        }
        Err(err) => {
            warn!(event_id = 2201; "Error occurred while reading the cache. ({err})");
            None
        }
    }
}

#[cfg(test)]
mod service_tests {
    use crate::cache::EmptyConfigCache;
    use crate::{ClientCacheState, ConfigCache};
    use chrono::{DateTime, Utc};
    use mockito::{Mock, ServerGuard};
    use reqwest::header::{ETAG, IF_NONE_MATCH};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::builder::{ClientBuilder, Options};
    use crate::constants::test_constants::{MOCK_KEY, MOCK_PATH};
    use crate::fetch::service::ConfigService;
    use crate::model::config::entry_from_cached_json;
    use crate::modes::PollingMode;

    #[test]
    fn cache_key_generation() {
        {
            let opts = Arc::new(
                ClientBuilder::new("configcat-sdk-1/TEST_KEY-0123456789012/1234567890123456789012")
                    .polling_mode(PollingMode::Manual)
                    .build_options(),
            );
            let service = ConfigService::new(opts).unwrap();
            assert_eq!(
                service.state.cache_key.as_str(),
                "f83ba5d45bceb4bb704410f51b704fb6dfa19942"
            );
        }
        {
            let opts = Arc::new(
                ClientBuilder::new("configcat-sdk-1/TEST_KEY2-123456789012/1234567890123456789012")
                    .polling_mode(PollingMode::Manual)
                    .build_options(),
            );
            let service = ConfigService::new(opts).unwrap();
            assert_eq!(
                service.state.cache_key.as_str(),
                "da7bfd8662209c8ed3f9db96daed4f8d91ba5876"
            );
        }
    }

    #[tokio::test]
    async fn auto_poll() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2, m3) = create_success_mock_sequence(&mut server).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }

    #[tokio::test]
    async fn auto_poll_failed() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2) = create_success_then_failure_mock(&mut server).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        m1.assert_async().await;
        m2.assert_async().await;
    }

    #[tokio::test]
    async fn lazy_load() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2, m3) = create_success_mock_sequence(&mut server).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::LazyLoad(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test2");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }

    #[tokio::test]
    async fn lazy_load_failed() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2) = create_success_then_failure_mock(&mut server).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::LazyLoad(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        m1.assert_async().await;
        m2.assert_async().await;
    }

    #[tokio::test]
    async fn manual_poll() {
        let mut server = mockito::Server::new_async().await;
        let (m1, m2, m3) = create_success_mock_sequence(&mut server).await;

        let opts = create_options(server.url().as_str(), PollingMode::Manual, None);
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        assert!(result.config().settings.is_empty());

        _ = service.refresh().await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        _ = service.refresh().await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test2");

        _ = service.refresh().await;

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test2");

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;
    }

    #[tokio::test]
    async fn fail_http_reload_from_cache() {
        let mut server = mockito::Server::new_async().await;
        let m = create_failure_mock(&mut server, 1).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test1",
                Utc::now() - Duration::from_secs(1),
                "etag1",
            )))),
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        service.options.cache().write(
            service.state.clone().cache_key.as_str(),
            construct_cache_payload("test2", Utc::now(), "etag2").as_str(),
        );

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test2");

        m.assert_async().await;
    }

    #[tokio::test]
    async fn poll_respects_cache_expiration() {
        let mut server = mockito::Server::new_async().await;
        let m1 = create_success_mock_with_etag(&mut server, "etag1", 0).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(500)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test1",
                Utc::now(),
                "etag1",
            )))),
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        m1.assert_async().await;
    }

    #[tokio::test]
    async fn poll_cache_write() {
        let mut server = mockito::Server::new_async().await;
        let m = create_success_mock(&mut server, 1).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(String::default()))),
        );
        let service = ConfigService::new(opts).unwrap();

        let result = service.config().await;
        let setting = &result.config().settings["testKey"];
        assert_eq!(setting.value.clone().string_val.unwrap(), "test1");

        let cached = service.options.cache().read("").unwrap();
        let entry = entry_from_cached_json(cached.as_str()).unwrap();

        assert_eq!(entry.etag, "etag1");

        m.assert_async().await;
    }

    #[tokio::test]
    async fn offline() {
        let mut server = mockito::Server::new_async().await;
        let m = create_success_mock(&mut server, 0).await;

        let opts = Arc::new(
            ClientBuilder::new(MOCK_KEY)
                .base_url(server.url().as_str())
                .polling_mode(PollingMode::AutoPoll(Duration::from_millis(100)))
                .offline(true)
                .build_options(),
        );
        let service = ConfigService::new(opts).unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = service.config().await;
        assert!(result.config().settings.is_empty());

        m.assert_async().await;
    }

    #[tokio::test]
    async fn online_offline() {
        let mut server = mockito::Server::new_async().await;
        let mut m = create_success_mock(&mut server, 1).await;

        let opts = Arc::new(
            ClientBuilder::new(MOCK_KEY)
                .base_url(server.url().as_str())
                .polling_mode(PollingMode::Manual)
                .build_options(),
        );
        let service = ConfigService::new(opts).unwrap();
        assert!(!service.is_offline());

        _ = service.refresh().await;
        m.assert_async().await;

        service.set_mode(true);
        assert!(service.is_offline());

        m.remove_async().await;
        m = create_success_mock(&mut server, 0).await;

        _ = service.refresh().await;
        m.assert_async().await;

        service.set_mode(false);
        assert!(!service.is_offline());

        m.remove_async().await;
        m = create_success_mock(&mut server, 1).await;

        _ = service.refresh().await;
        m.assert_async().await;
    }

    #[tokio::test]
    async fn wait_for_init_cached() {
        let mut server = mockito::Server::new_async().await;
        let m = create_success_mock(&mut server, 0).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_secs(1)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test",
                Utc::now(),
                "etag1",
            )))),
        );
        let service = ConfigService::new(opts).unwrap();
        let state = service.wait_for_init().await;

        assert!(matches!(state, ClientCacheState::HasUpToDateFlagData));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn wait_for_init_expired_fetch() {
        let mut server = mockito::Server::new_async().await;
        let m = create_success_mock(&mut server, 1).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test",
                Utc::now() - Duration::from_secs(5),
                "etag1",
            )))),
        );
        let service = ConfigService::new(opts).unwrap();
        let state = service.wait_for_init().await;

        assert!(matches!(state, ClientCacheState::HasUpToDateFlagData));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn wait_for_init_expired_fetch_fail() {
        let mut server = mockito::Server::new_async().await;
        let m = create_failure_mock(&mut server, 1).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test",
                Utc::now() - Duration::from_secs(5),
                "etag1",
            )))),
        );
        let service = ConfigService::new(opts).unwrap();
        let state = service.wait_for_init().await;

        assert!(matches!(state, ClientCacheState::HasCachedFlagDataOnly));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn wait_for_init_no_cache_fail() {
        let mut server = mockito::Server::new_async().await;
        let m = create_failure_mock_without_etag(&mut server, 1).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::AutoPoll(Duration::from_millis(100)),
            None,
        );
        let service = ConfigService::new(opts).unwrap();
        let state = service.wait_for_init().await;

        assert!(matches!(state, ClientCacheState::NoFlagData));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn wait_for_init_manual() {
        let mut server = mockito::Server::new_async().await;
        let m = create_failure_mock_without_etag(&mut server, 0).await;

        let opts = create_options(
            server.url().as_str(),
            PollingMode::Manual,
            Some(Box::new(SingleValueCache::new(construct_cache_payload(
                "test",
                Utc::now(),
                "etag1",
            )))),
        );
        let service = ConfigService::new(opts).unwrap();
        let state = service.wait_for_init().await;

        assert!(matches!(state, ClientCacheState::HasCachedFlagDataOnly));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn wait_for_init_manual_fail() {
        let mut server = mockito::Server::new_async().await;
        let m = create_failure_mock_without_etag(&mut server, 0).await;

        let opts = create_options(server.url().as_str(), PollingMode::Manual, None);
        let service = ConfigService::new(opts).unwrap();
        let state = service.wait_for_init().await;

        assert!(matches!(state, ClientCacheState::NoFlagData));

        m.assert_async().await;
    }

    fn create_options(
        url: &str,
        mode: PollingMode,
        cache: Option<Box<dyn ConfigCache>>,
    ) -> Arc<Options> {
        Arc::new(
            ClientBuilder::new(MOCK_KEY)
                .cache(cache.unwrap_or(Box::new(EmptyConfigCache::new())))
                .base_url(url)
                .polling_mode(mode)
                .build_options(),
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

    async fn create_failure_mock_without_etag(server: &mut ServerGuard, expect: usize) -> Mock {
        server
            .mock("GET", MOCK_PATH)
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
            *val = value.to_owned();
        }
    }
}
