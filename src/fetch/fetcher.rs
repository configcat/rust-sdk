use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use reqwest::header::{HeaderMap, ETAG, IF_NONE_MATCH};

use crate::constants::{CONFIG_FILE_NAME, PKG_VERSION, SDK_KEY_PROXY_PREFIX};
use crate::fetch::fetcher::FetchResponse::{Failed, Fetched, NotModified};
use crate::model::config::{entry_from_json, ConfigEntry};
use crate::model::enums::RedirectMode;

const CONFIGCAT_UA_HEADER: &str = "X-ConfigCat-UserAgent";

#[derive(Debug, PartialEq)]
pub enum FetchResponse {
    Fetched(ConfigEntry),
    NotModified,
    Failed(String, bool),
}

impl FetchResponse {
    pub fn entry(&self) -> Option<&ConfigEntry> {
        match self {
            Fetched(entry) => Some(entry),
            _ => None,
        }
    }
}

pub struct Fetcher {
    is_custom_url: bool,
    fetch_url: Arc<Mutex<String>>,
    http_client: reqwest::Client,
    sdk_key: String,
}

impl Fetcher {
    pub fn new(url: String, is_custom: bool, sdk_key: &str, mode: &str, timeout: Duration) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONFIGCAT_UA_HEADER,
            format!("ConfigCat-Rust/{mode}-{PKG_VERSION}")
                .parse()
                .unwrap(),
        );
        Self {
            sdk_key: sdk_key.to_string(),
            fetch_url: Arc::new(Mutex::new(url)),
            is_custom_url: is_custom,
            http_client: reqwest::Client::builder()
                .timeout(timeout)
                .default_headers(headers)
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch(&self, etag: &str) -> FetchResponse {
        for _ in 0..3 {
            let fetch_url = self.fetch_url();
            let response = self.fetch_http(fetch_url.as_str(), etag).await;
            match &response {
                Fetched(entry) => match &entry.config.preferences {
                    Some(pref) => {
                        if pref
                            .url
                            .clone()
                            .is_some_and(|pref_url| pref_url == fetch_url)
                        {
                            return response;
                        };

                        let redirect = pref.redirect.clone().unwrap_or(RedirectMode::No);
                        if self.is_custom_url
                            && (self.sdk_key.starts_with(SDK_KEY_PROXY_PREFIX)
                                || redirect != RedirectMode::Force)
                        {
                            return response;
                        }

                        if pref.url.is_some() {
                            self.set_fetch_url(pref.url.clone().unwrap());
                        }

                        if redirect == RedirectMode::No {
                            return response;
                        } else if redirect == RedirectMode::Should {
                            log_warn!(event_id: 3002, "The `builder.dataGovernance()` parameter specified at the client initialization is not in sync with the preferences on the ConfigCat Dashboard. Read more: https://configcat.com/docs/advanced/data-governance")
                        }
                    }
                    _ => return response,
                },
                _ => return response,
            }
        }
        let msg = "Redirection loop encountered while trying to fetch config JSON. Please contact us at https://configcat.com/support".to_string();
        log_err!(event_id: 1104, "{}", msg);
        Failed(msg, true)
    }

    async fn fetch_http(&self, url: &str, etag: &str) -> FetchResponse {
        let final_url = format!(
            "{url}/configuration-files/{sdk_key}/{config_json_name}",
            sdk_key = self.sdk_key,
            config_json_name = CONFIG_FILE_NAME
        );
        let mut builder = self.http_client.get(final_url);
        if !etag.is_empty() {
            builder = builder.header(IF_NONE_MATCH, etag.to_string());
        }

        let result = builder.send().await;

        match result {
            Ok(response) => match response.status().as_u16() {
                200 => {
                    log_debug!("Fetch was successful: new config fetched");
                    let headers = response.headers().clone();
                    let etag = if let Some(header) = headers.get(ETAG) {
                        header.to_str().unwrap_or("")
                    } else {
                        ""
                    };
                    let body_result = response.text().await;
                    match body_result {
                        Ok(body_str) => {
                            let parse_result = entry_from_json(body_str.as_str(), etag, Utc::now());
                            println!("{}", body_str);
                            match parse_result {
                                Ok(entry) => Fetched(entry),
                                Err(parse_error) => {
                                    let msg = format!("Fetching config JSON was successful but the HTTP response content was invalid. {parse_error}");
                                    log_err!(event_id: 1105, "{}", msg);
                                    Failed(msg, true)
                                }
                            }
                        }
                        Err(body_error) => {
                            let msg = format!("Fetching config JSON was successful but the HTTP response content was invalid. {body_error}");
                            log_err!(event_id: 1105, "{}", msg);
                            Failed(msg, true)
                        }
                    }
                }
                304 => {
                    log_debug!("Fetch was successful: not modified");
                    NotModified
                }
                code @ 404 | code @ 403 => {
                    let msg = format!("Your SDK Key seems to be wrong. You can find the valid SDK Key at https://app.configcat.com/sdkkey. Status code: {code}");
                    log_err!(event_id: 1100, "{}", msg);
                    Failed(msg, false)
                }
                code => {
                    let msg = format!("Unexpected HTTP response was received while trying to fetch config JSON. Status code: {code}");
                    log_err!(event_id: 1101, "{}", msg);
                    Failed(msg, true)
                }
            },
            Err(error) => {
                if error.is_timeout() {
                    let msg = "Request timed out while trying to fetch config JSON.".to_string();
                    log_err!(event_id: 1102, "{}", msg);
                    Failed(msg, true)
                } else {
                    let msg = format!("Unexpected error occurred while trying to fetch config JSON. It is most likely due to a local network issue. Please make sure your application can reach the ConfigCat CDN servers (or your proxy server) over HTTP. {error}");
                    log_err!(event_id: 1103, "{}", msg);
                    Failed(msg, true)
                }
            }
        }
    }

    fn fetch_url(&self) -> String {
        let url = self.fetch_url.lock().unwrap();
        url.to_string()
    }

    fn set_fetch_url(&self, new_url: String) {
        let mut url = self.fetch_url.lock().unwrap();
        *url = new_url
    }
}

#[cfg(test)]
mod fetch_tests {
    use std::time::Duration;

    use reqwest::header::{ETAG, IF_NONE_MATCH};

    use crate::constants::test_constants::{MOCK_KEY, MOCK_PATH};
    use crate::constants::PKG_VERSION;
    use crate::fetch::fetcher::FetchResponse::{Fetched, NotModified};
    use crate::fetch::fetcher::{FetchResponse, Fetcher, CONFIGCAT_UA_HEADER};

    #[tokio::test]
    async fn fetch_http() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .match_header(
                CONFIGCAT_UA_HEADER,
                format!("ConfigCat-Rust/mode-{PKG_VERSION}").as_str(),
            )
            .with_body(r#"{"f": {}, "s": []}"#)
            .create_async()
            .await;

        let fetcher = Fetcher::new(
            server.url(),
            false,
            MOCK_KEY,
            "mode",
            Duration::from_secs(30),
        );
        let response = fetcher.fetch("").await;
        assert!(matches!(response, Fetched(_)));
    }

    #[tokio::test]
    async fn fetch_http_etag() {
        let mut server = mockito::Server::new_async().await;
        let m1 = server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_header(ETAG.as_str(), "etag1")
            .with_body(r#"{"f": {}, "s": []}"#)
            .create_async()
            .await;

        let m2 = server
            .mock("GET", MOCK_PATH)
            .match_header(IF_NONE_MATCH.as_str(), "etag1")
            .with_status(304)
            .with_header(ETAG.as_str(), "etag2")
            .create_async()
            .await;

        let fetcher = Fetcher::new(server.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        let response = fetcher.fetch("").await;
        assert!(matches!(response, Fetched(_)));

        let entry = response.entry().unwrap();
        assert_eq!("etag1", entry.etag);

        let response = fetcher.fetch(entry.etag.as_str()).await;
        assert!(matches!(response, NotModified));

        m1.assert_async().await;
        m2.assert_async().await;
    }

    #[tokio::test]
    async fn fetch_http_failed() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", MOCK_PATH)
            .with_status(404)
            .create_async()
            .await;

        server
            .mock("GET", MOCK_PATH)
            .with_status(403)
            .create_async()
            .await;

        server
            .mock("GET", MOCK_PATH)
            .with_status(500)
            .create_async()
            .await;

        let fetcher = Fetcher::new(server.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        let response = fetcher.fetch("").await;
        match response {
            FetchResponse::Failed(err, transient) => {
                assert!(!transient);
                assert_eq!(err, "Your SDK Key seems to be wrong. You can find the valid SDK Key at https://app.configcat.com/sdkkey. Status code: 404");
            }
            _ => panic!(),
        }

        let response = fetcher.fetch("").await;
        match response {
            FetchResponse::Failed(err, transient) => {
                assert!(!transient);
                assert_eq!(err, "Your SDK Key seems to be wrong. You can find the valid SDK Key at https://app.configcat.com/sdkkey. Status code: 403");
            }
            _ => panic!(),
        }

        let response = fetcher.fetch("").await;
        match response {
            FetchResponse::Failed(err, transient) => {
                assert!(transient);
                assert_eq!(err, "Unexpected HTTP response was received while trying to fetch config JSON. Status code: 500");
            }
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn fetch_http_body_error() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(r#"{"f": {}"#)
            .create_async()
            .await;

        server
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .create_async()
            .await;

        let fetcher = Fetcher::new(server.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        let response = fetcher.fetch("").await;
        match response {
            FetchResponse::Failed(err, transient) => {
                assert!(transient);
                assert_eq!(err, "Fetching config JSON was successful but the HTTP response content was invalid. JSON parsing failed. (EOF while parsing an object at line 1 column 8)");
            }
            _ => panic!(),
        }

        let response = fetcher.fetch("").await;
        match response {
            FetchResponse::Failed(err, transient) => {
                assert!(transient);
                assert_eq!(err, "Fetching config JSON was successful but the HTTP response content was invalid. JSON parsing failed. (EOF while parsing a value at line 1 column 0)");
            }
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod data_governance_tests {
    use std::time::Duration;

    use crate::constants::test_constants::{MOCK_KEY, MOCK_PATH};
    use crate::constants::SDK_KEY_PROXY_PREFIX;
    use crate::fetch::fetcher::Fetcher;

    #[tokio::test]
    async fn stay_on_server() {
        let mut global = mockito::Server::new_async().await;
        let mut eu = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(eu.url(), 0))
            .create_async()
            .await;
        let eu_mock = eu.mock("GET", MOCK_PATH).expect(0).create_async().await;

        let fetcher = Fetcher::new(global.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        eu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn stay_on_same_url() {
        let mut global = mockito::Server::new_async().await;
        let mut eu = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 1))
            .create_async()
            .await;
        let eu_mock = eu.mock("GET", MOCK_PATH).expect(0).create_async().await;

        let fetcher = Fetcher::new(global.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        eu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn stay_on_same_url_even_with_force() {
        let mut global = mockito::Server::new_async().await;
        let mut eu = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 2))
            .create_async()
            .await;
        let eu_mock = eu.mock("GET", MOCK_PATH).expect(0).create_async().await;

        let fetcher = Fetcher::new(global.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        eu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_redirect() {
        let mut global = mockito::Server::new_async().await;
        let mut eu = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(eu.url(), 1))
            .create_async()
            .await;
        let eu_mock = eu
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(eu.url(), 0))
            .create_async()
            .await;

        let fetcher = Fetcher::new(global.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        eu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_redirect_when_forced() {
        let mut global = mockito::Server::new_async().await;
        let mut eu = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(eu.url(), 2))
            .create_async()
            .await;
        let eu_mock = eu
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(eu.url(), 0))
            .create_async()
            .await;

        let fetcher = Fetcher::new(global.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        eu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_break_redirect_loop() {
        let mut global = mockito::Server::new_async().await;
        let mut eu = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(eu.url(), 1))
            .expect(2)
            .create_async()
            .await;
        let eu_mock = eu
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 1))
            .create_async()
            .await;

        let fetcher = Fetcher::new(global.url(), false, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        eu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_respect_custom_url() {
        let mut global = mockito::Server::new_async().await;
        let mut custom = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 0))
            .expect(0)
            .create_async()
            .await;
        let cu_mock = custom
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 1))
            .expect(2)
            .create_async()
            .await;

        let fetcher = Fetcher::new(custom.url(), true, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        cu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_not_respect_custom_url_when_forced() {
        let mut global = mockito::Server::new_async().await;
        let mut custom = mockito::Server::new_async().await;
        let g_mock = global
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 0))
            .expect(2)
            .create_async()
            .await;
        let cu_mock = custom
            .mock("GET", MOCK_PATH)
            .with_status(200)
            .with_body(format_body(global.url(), 2))
            .expect(1)
            .create_async()
            .await;

        let fetcher = Fetcher::new(custom.url(), true, MOCK_KEY, "", Duration::from_secs(30));
        fetcher.fetch("").await;
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        cu_mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_respect_proxy_even_when_forced() {
        let mut global = mockito::Server::new_async().await;
        let mut custom = mockito::Server::new_async().await;
        let g_mock = global
            .mock(
                "GET",
                format!("/configuration-files/{SDK_KEY_PROXY_PREFIX}{MOCK_KEY}/config_v6.json")
                    .as_str(),
            )
            .with_status(200)
            .with_body(format_body(global.url(), 0))
            .expect(0)
            .create_async()
            .await;
        let cu_mock = custom
            .mock(
                "GET",
                format!("/configuration-files/{SDK_KEY_PROXY_PREFIX}{MOCK_KEY}/config_v6.json")
                    .as_str(),
            )
            .with_status(200)
            .with_body(format_body(global.url(), 2))
            .expect(2)
            .create_async()
            .await;

        let fetcher = Fetcher::new(
            custom.url(),
            true,
            format!("{SDK_KEY_PROXY_PREFIX}{MOCK_KEY}").as_str(),
            "",
            Duration::from_secs(30),
        );
        fetcher.fetch("").await;
        fetcher.fetch("").await;

        g_mock.assert_async().await;
        cu_mock.assert_async().await;
    }

    fn format_body(url: String, redirect_mode: u8) -> String {
        return "{ \"p\": { \"u\": \"".to_string()
            + url.as_str()
            + "\", \"r\": "
            + redirect_mode.to_string().as_str()
            + ", \"s\": \"test-salt\" }, \"f\": {}, \"s\":[] }";
    }
}