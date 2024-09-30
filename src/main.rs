use std::fs;
use bevy::prelude::*;
use serde::Serialize;
use wavedash::RuntimePlugin;
use wavedash_core::Named;

#[derive(Serialize, Resource, Named)]
struct X(i32);

fn main() {
    let wasm = fs::read("target/wasm32-unknown-unknown/debug/example.wasm").unwrap();

    App::new()
        .add_plugins(RuntimePlugin::new().module(wasm).resource::<X>())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(X(42));
}
