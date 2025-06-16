use chrono::{DateTime, Utc};
use configcat::{Client, ConfigCache, PollingMode};
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use std::sync::Arc;
use tokio::runtime::Runtime;

struct SingleValueCache {
    value: String,
}

impl SingleValueCache {
    pub fn new(val: String) -> Self {
        Self { value: val }
    }
}

impl ConfigCache for SingleValueCache {
    fn read(&self, _: &str) -> Option<String> {
        Some(self.value.clone())
    }
    fn write(&self, _: &str, _: &str) {}
}

fn get_value_bench(c: &mut Criterion) {
    let client = Arc::new(
        Client::builder("PKDVCLf-Hq-h-kCzMp-L7Q/HhOWfwVtZ0mb30i9wi17GQ")
            .polling_mode(PollingMode::Manual)
            // We benchmark on a cache to bypass the first HTTP request which
            // heavily influences the measurements.
            .cache(Box::new(SingleValueCache::new(construct_cache_payload(
                true,
                Utc::now(),
                "tag",
            ))))
            .build()
            .unwrap(),
    );
    c.bench_function("get_value", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            let mut handles = Vec::new();
            for _ in 0..200 {
                let cl = client.clone();
                handles.push(tokio::spawn(async move {
                    cl.get_value("testKey", false, None).await;
                }));
            }
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
}

fn construct_cache_payload(val: bool, time: DateTime<Utc>, etag: &str) -> String {
    time.timestamp_millis().to_string() + "\n" + etag + "\n" + &construct_json_payload(val)
}

fn construct_json_payload(val: bool) -> String {
    format!(r#"{{"f": {{"testKey":{{"t":0,"v":{{"b": {val}}}}}}}, "s": []}}"#)
}

criterion_group!(benches, get_value_bench);
criterion_main!(benches);
