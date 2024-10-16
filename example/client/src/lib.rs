use wavedash_example_core::ExampleResource;
use wavedash::{App, ResMut, Update};

#[no_mangle]
extern "C" fn run() {
    let mut app = unsafe { App::current() };

    app.add_system(Update, |mut example: ResMut<ExampleResource>| {
        example.value += 1;
        wavedash::dbg(&*example);
    });
}
