use wavedash::{App, ResMut, Update};
use wavedash_example_core::ExampleResource;

#[wavedash::main]
fn main() {
    App::current().add_system(Update, on_update);
}

fn on_update(mut example: ResMut<ExampleResource>) {
    example.value += 1;
    wavedash::dbg(&*example);
}
