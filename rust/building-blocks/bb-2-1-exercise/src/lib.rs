use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Move {
    pub x: String,
    pub y: u32,
}
