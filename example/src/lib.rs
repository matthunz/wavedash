use wavedash_core::ExampleResource;

#[no_mangle]
extern "C" fn run() {
    wavedash::dbg(wavedash::resource::<ExampleResource>());
}