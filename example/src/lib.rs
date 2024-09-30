#[no_mangle]
fn main() {
    wavedash_guest::log("Hello, World!");

    wavedash_guest::world_resource("x");
}
