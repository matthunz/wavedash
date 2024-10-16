use bevy::prelude::*;
use wavedash_example_core::ExampleResource;
use wavedash_host::WavedashPlugin;

fn main() {
    let module =
        include_bytes!("../../target/wasm32-unknown-unknown/debug/wavedash_example_client.wasm");

    App::new()
        .add_plugins((
            DefaultPlugins,
            WavedashPlugin::new(module.to_vec()).with_resource::<ExampleResource>(),
        ))
        .register_type::<ExampleResource>()
        .insert_resource(ExampleResource { value: 42 })
        .run();
}
