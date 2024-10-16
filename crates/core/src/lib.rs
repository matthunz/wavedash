use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Log(String),
    GetResource { type_path: String },
    SetResource { type_path: String, value: Value },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Empty,
    Resource(Value),
}
