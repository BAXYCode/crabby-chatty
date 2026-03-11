use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(JsonSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
#[schemars(inline)]
pub enum Destination {
    Individual { id: Uuid },
    Group { id: Uuid },
}
