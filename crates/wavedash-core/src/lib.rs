use std::borrow::Cow;

pub use wavedash_core_macros::Named;

pub trait Named {
    fn name() -> Cow<'static, str>;
}
