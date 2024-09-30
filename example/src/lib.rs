use wavedash_guest::{log, App};

#[no_mangle]
fn main() {
    log(App::current().world_mut().resource("x").to_string());
}
