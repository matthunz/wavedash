use serde::Deserialize;
use wavedash_core::Named;
use wavedash_guest::{log, App};

#[derive(Deserialize, Named)]
struct X(i32);

#[no_mangle]
fn main() {
    log(App::current().world_mut().resource::<X>().0.to_string());
}
