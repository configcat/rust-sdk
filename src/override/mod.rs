use crate::{OverrideBehavior, OverrideDataSource};
use std::borrow::Borrow;

pub mod behavior;
pub mod file;
pub mod map;
pub mod source;

pub trait OptionalOverrides {
    fn is_local(&self) -> bool;
}

pub struct FlagOverrides {
    behavior: OverrideBehavior,
    source: Box<dyn OverrideDataSource>,
}

impl FlagOverrides {
    pub fn new(source: Box<dyn OverrideDataSource>, behavior: OverrideBehavior) -> Self {
        Self { source, behavior }
    }

    pub fn behavior(&self) -> &OverrideBehavior {
        &self.behavior
    }

    pub fn source(&self) -> &dyn OverrideDataSource {
        self.source.borrow()
    }
}

impl OptionalOverrides for Option<FlagOverrides> {
    fn is_local(&self) -> bool {
        if let Some(ov) = self {
            return matches!(ov.behavior, OverrideBehavior::LocalOnly);
        }
        false
    }
}
