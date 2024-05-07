use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ClientError {
    #[error("SDK key is invalid. ({0})")]
    InvalidSdkKey(String),
    #[error("{0}")]
    Fetch(String),
}

#[derive(Error, PartialEq, Debug)]
pub enum InternalError {
    #[error("JSON parsing failed. ({0})")]
    Parse(String),
    #[error("{0}")]
    Http(String),
}
