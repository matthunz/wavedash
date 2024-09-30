use bevy::prelude::*;
use serde::Serialize;
use std::borrow::Cow;
use wavedash::RuntimePlugin;
use wavedash_core::Named;

#[derive(Serialize, Resource)]
struct X(i32);

impl Named for X {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("x")
    }
}

fn main() {
    App::new()
        .add_plugins(RuntimePlugin::new().resource::<X>())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(X(42));
}
