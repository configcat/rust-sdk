macro_rules! from_val_to_enum {
    ($ob:ident $to:ident $($t:ty)*) => ($(
        impl From<$t> for $ob {
            fn from(value: $t) -> Self {
                Self::$to(value)
            }
        }
    )*)
}

macro_rules! from_val_to_enum_into {
    ($ob:ident $to:ident $($t:ty)*) => ($(
        impl From<$t> for $ob {
            fn from(value: $t) -> Self {
                Self::$to(value.into())
            }
        }
    )*)
}
