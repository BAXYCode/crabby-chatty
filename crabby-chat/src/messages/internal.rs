use crabby_specs::ws::outgoing::CrabbyWsFromServer;
use serde::{Deserialize, Serialize};

use kameo::prelude::Recipient;
use uuid::Uuid;

pub struct UserConnected(pub Uuid, pub Recipient<CrabbyWsFromServer>);
#[derive(Serialize, Deserialize)]
pub struct UserDisconnected(pub Uuid);
#[derive(Clone)]
pub struct UserMessage;
