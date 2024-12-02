use crate::{OverrideBehavior, OverrideDataSource};
use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};

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
        Self { behavior, source }
    }

    pub fn behavior(&self) -> &OverrideBehavior {
        &self.behavior
    }

    pub fn source(&self) -> &dyn OverrideDataSource {
        self.source.borrow()
    }
}

impl OptionalOverrides for Option<&FlagOverrides> {
    fn is_local(&self) -> bool {
        if let Some(ov) = self {
            return matches!(ov.behavior, OverrideBehavior::LocalOnly);
        }
        false
    }
}

impl OptionalOverrides for Option<FlagOverrides> {
    fn is_local(&self) -> bool {
        self.as_ref().is_local()
    }
}

impl Debug for FlagOverrides {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlagOverrides")
            .field("behavior", &self.behavior)
            .finish_non_exhaustive()
    }
}
