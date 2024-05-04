#[cfg(not(test))]
macro_rules! log_ext {
    (warn, event_id: $event_id:expr, $($arg:expr),*) => { log::warn!(event_id = $event_id; $($arg),*) };
    (info, event_id: $event_id:expr, $($arg:expr),*) => { log::info!(event_id = $event_id; $($arg),*) };
    (error, event_id: $event_id:expr, $($arg:expr),*) => { log::error!(event_id = $event_id; $($arg),*) };
    (debug, $($arg:expr),*) => { log::debug!($($arg),*) };
}

#[cfg(test)]
macro_rules! log_ext {
    (warn, event_id: $event_id:expr, $($arg:expr),*) => { println!($($arg),*) };
    (info, event_id: $event_id:expr, $($arg:expr),*) => { println!($($arg),*) };
    (error, event_id: $event_id:expr, $($arg:expr),*) => { println!($($arg),*) };
    (debug, $($arg:expr),*) => { println!($($arg),*) };
}

macro_rules! log_warn {
    (event_id: $event_id:expr, $($arg:expr),*) => { log_ext!(warn, event_id: $event_id, $($arg),*) };
}

macro_rules! log_info {
    (event_id: $event_id:expr, $($arg:expr),*) => { log_ext!(info, event_id: $event_id, $($arg),*) };
}

macro_rules! log_err {
    (event_id: $event_id:expr, $($arg:expr),*) => { log_ext!(error, event_id: $event_id, $($arg),*) };
}

macro_rules! log_debug {
    ($($arg:expr),*) => { log_ext!(debug, $($arg),*) };
}
