# Wavedash

A (WIP) WASM runtime for Bevy mods and scripting.

## Shared

First create your shared application state.

```rs
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Reflect, Resource)]
pub struct ExampleResource {
    pub value: i32,
}
```

## Game

Then setup the `WavedashPlugin` in your game's Bevy App using your shared state.

```rs
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
```

## Mod

Finally create a mod using the same shared crate from before.

```rs
use wavedash::prelude::*;
use wavedash_example_core::ExampleResource;

#[wavedash::main]
fn main(example: ResMut<ExampleResource>) {
    wavedash::dbg(&*example);
}
```
