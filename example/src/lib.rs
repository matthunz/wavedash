use wavedash::{App, ResMut, Update};
use wavedash_core::ExampleResource;

#[no_mangle]
extern "C" fn run() {
    let mut app = unsafe { App::current() };

    app.add_system(Update, |mut example: ResMut<ExampleResource>| {
        example.value += 1;
        wavedash::dbg(&*example);
    });
}
