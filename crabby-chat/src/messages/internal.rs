use serde::{Deserialize, Serialize};

use kameo::prelude::Recipient;
use uuid::Uuid;

pub struct UserConnected(pub Uuid, pub Recipient<UserMessage>);
#[derive(Serialize, Deserialize)]
pub struct UserDisconnected(pub Uuid);
#[derive(Clone, Deserialize, Serialize)]
pub struct UserMessage;
