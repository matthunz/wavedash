use std::borrow::Cow;

pub trait Named {
    fn name() -> Cow<'static, str>;
}
