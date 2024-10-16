use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Reflect, Resource)]
pub struct ExampleResource {
    pub value: i32,
}
