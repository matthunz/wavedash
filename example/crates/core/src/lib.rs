use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Reflect, Serialize, Deserialize)]
pub struct Example {
    pub value: i32,
}
