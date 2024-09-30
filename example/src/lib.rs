use std::borrow::Cow;
use wavedash_guest::{log, App, Named};

#[derive(serde::Deserialize)]
struct X(i32);

impl Named for X {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("x")
    }
}

#[no_mangle]
fn main() {
    log(App::current().world_mut().resource::<X>().0.to_string());
}
