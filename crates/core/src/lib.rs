use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Log(String),
    GetResource { type_path: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Empty,
    Resource(Value),
}

#[derive(Debug, Deserialize, Serialize, Reflect, Resource)]
pub struct ExampleResource {
    pub value: i32,
}
