use serde::{Serialize, de::DeserializeOwned};

use crate::codec::Codec;

///The channel trait represent a communication channel between two
///entities. This sets a boundary on what the type being used as a
///message is while allowing to keep the actual communication
///transport abstract.
///
///The `Codec` associated type controls how the message is to be
///serialized and deserialized into the concrete type
pub trait Channel {
    type Message: Serialize + DeserializeOwned + 'static;
    type Codec: Codec<Self::Message>;
    fn channel_name() -> &'static str;
    fn subject(&self) -> String;
}
