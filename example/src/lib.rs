use wavedash::App;
use wavedash_core::ExampleResource;

#[no_mangle]
extern "C" fn run() {
    let mut app = unsafe { App::current() };

    wavedash::dbg(app.resource::<ExampleResource>());

    app.resource_mut::<ExampleResource>().value += 1;

    wavedash::dbg(app.resource::<ExampleResource>());
}
