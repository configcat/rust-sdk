use crate::options::{Options, OptionsBuilder};
use crate::ClientError;
use std::sync::Arc;

pub struct Client {
    options: Arc<Options>,
}

impl Client {
    pub fn from_builder(builder: OptionsBuilder) -> Result<Self, ClientError> {
        let result = builder.build();
        match result {
            Ok(opts) => Ok(Client::new(opts)),
            Err(err) => Err(err),
        }
    }

    pub fn new(options: Options) -> Self {
        Self {
            options: Arc::new(options),
        }
    }

    pub fn refresh() -> Result<(), ClientError> {
        Ok(())
    }
}
