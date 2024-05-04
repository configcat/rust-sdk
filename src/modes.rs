use std::time::Duration;

pub enum PollingMode {
    AutoPoll(Duration),
    LazyLoad(Duration),
    Manual,
}

impl PollingMode {
    pub fn mode_identifier(&self) -> &str {
        match self {
            PollingMode::AutoPoll(_) => "a",
            PollingMode::LazyLoad(_) => "l",
            PollingMode::Manual => "m",
        }
    }
}
