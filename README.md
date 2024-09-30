A WASM runtime for Bevy.

## Host
```rs
use bevy::prelude::*;
use serde::Serialize;
use wavedash::RuntimePlugin;
use wavedash_core::Named;

#[derive(Serialize, Resource, Named)]
struct X(i32);

fn main() {
    App::new()
        .add_plugins(RuntimePlugin::new().resource::<X>())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(X(42));
}
```

## Guest
```rs
use serde::Deserialize;
use wavedash_core::Named;
use wavedash_guest::{log, App};

#[derive(Deserialize, Named)]
struct X(i32);

#[no_mangle]
fn main() {
    log(App::current().world_mut().resource::<X>().0.to_string());
}
```
