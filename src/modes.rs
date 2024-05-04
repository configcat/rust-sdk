use std::time::Duration;

pub enum PollingMode {
    AutoPoll(Duration),
    LazyLoad(Duration),
    Manual,
}
