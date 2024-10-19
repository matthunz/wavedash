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
use wavedash_host::WavedashPlugin;

fn main() {
    let module = include_bytes!("../../target/wasm32-unknown-unknown/debug/wavedash_example_client.wasm");

    App::new()
        .add_plugins((
            DefaultPlugins,
            WavedashPlugin::new(module.to_vec()).with_resource::<ExampleResource>(),
        ))
        .register_type::<ExampleResource>()
        .insert_resource(ExampleResource { value: 42 })
        .run();
}
```

## Mod

Finally create a mod using the same shared crate from before.

```rs
use wavedash::prelude::*;
use wavedash_example_core::ExampleResource;

#[wavedash::main]
fn main() {
    App::current().add_system(Update, on_update);
}

fn on_update(mut example: ResMut<ExampleResource>) {
    example.value += 1;

    wavedash::dbg(&*example);
}
```
