use bevy::prelude::*;
use wavedash_example_core::ExampleResource;
use wavedash_host::WasmModule;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<ExampleResource>()
        .insert_resource(ExampleResource { value: 42 })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let module =
        include_bytes!("../../target/wasm32-unknown-unknown/debug/wavedash_example_client.wasm");

    let wasm = WasmModule::new(module.to_vec()).with_resource::<ExampleResource>();

    commands.spawn(wasm);
}
