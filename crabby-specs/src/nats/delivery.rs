use crabby_transport::codec::Codec;
use eyre::Result;
use futures_util::{Stream, StreamExt};
use uuid::Uuid;

use crate::ws::outgoing::CrabbyWsFromServer;

/// Adapter produced by the subscriber stream. Pairs the target
/// user_id (extracted from the NATS subject `user.{uuid}.delivery`)
/// with the decoded domain message.
#[derive(Debug, Clone)]
pub struct PayloadWithDestination {
    pub user_id: Uuid,
    pub message: CrabbyWsFromServer,
}

/// Subscribes to `user.*.delivery` and returns a stream of
/// [`PayloadWithDestination`] items, parsing the user_id from the
/// NATS subject and decoding the payload via the channel's codec.
pub async fn user_delivery_stream(
    client: async_nats::Client,
) -> Result<impl Stream<Item = PayloadWithDestination> + Send> {
    let sub = client.subscribe("user.*.delivery").await?;
    Ok(sub.filter_map(|msg| {
        async move {
            if msg.status.is_some() {
                return None;
            }

            let user_id = parse_user_id_from_subject(msg.subject.as_str())?;

            let decoded: CrabbyWsFromServer =
                <crabby_transport::codec::JsonCodec as Codec<
                    CrabbyWsFromServer,
                >>::decode(&msg.payload)
                .ok()?;

            Some(PayloadWithDestination {
                user_id,
                message: decoded,
            })
        }
    }))
}

/// Extracts the UUID segment from a `user.{uuid}.delivery` subject.
fn parse_user_id_from_subject(subject: &str) -> Option<Uuid> {
    let mut parts = subject.split('.');
    // skip "user"
    parts.next()?;
    let id_str = parts.next()?;
    Uuid::parse_str(id_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_subject() {
        let id = Uuid::nil();
        let subject = format!("user.{}.delivery", id);
        assert_eq!(parse_user_id_from_subject(&subject), Some(id));
    }

    #[test]
    fn parse_real_uuid_subject() {
        let id = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440000);
        let subject = format!("user.{}.delivery", id);
        assert_eq!(parse_user_id_from_subject(&subject), Some(id));
    }

    #[test]
    fn parse_invalid_uuid_returns_none() {
        assert_eq!(
            parse_user_id_from_subject("user.not-a-uuid.delivery"),
            None
        );
    }

    #[test]
    fn parse_missing_segments_returns_none() {
        assert_eq!(parse_user_id_from_subject("user"), None);
        assert_eq!(parse_user_id_from_subject(""), None);
    }

    #[test]
    fn parse_extra_segments_still_works() {
        let id = Uuid::nil();
        let subject = format!("user.{}.delivery.extra", id);
        // Only reads the second segment, so this still parses
        assert_eq!(parse_user_id_from_subject(&subject), Some(id));
    }
}
