use bevy::prelude::*;
use wavedash_example_core::Example;

pub fn main() {
    let mut app = App::new();
    app.register_type::<Example>();
    app.world_mut().insert_resource(Example { value: 42 });
    app.add_systems(Startup, setup);
    app.run();
}

fn setup(world: &mut World) {
    let module =
        include_bytes!("../../../../target/wasm32-unknown-unknown/debug/wavedash_example.wasm");

    let mut wasm = wavedash_host::Wasm::new(module);
    wasm.insert_resource::<Example>();
    wasm.run(world);
}
