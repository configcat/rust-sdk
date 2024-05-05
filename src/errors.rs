use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ClientError {
    #[error("JSON parsing failed. ({0})")]
    Parse(String),
    #[error("{1}")]
    Http(i64, String),
}
