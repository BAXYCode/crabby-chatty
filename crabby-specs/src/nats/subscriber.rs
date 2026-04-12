use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use crabby_transport::{
    channel::Channel,
    subscriber::{ChannelStream, Subscriber},
};
use eyre::Result;
use futures_util::{Stream, StreamExt};

use crate::nats::{
    channel::{FanoutMessageDelivery, GroupChangeEvent, UserMessageDelivery},
    error::NatsAdapterError,
};

pub struct NatsCoreSubscriber {
    inner: async_nats::Client,
}
#[async_trait]
impl Subscriber<UserMessageDelivery> for NatsCoreSubscriber {
    type Stream = ChannelStream<
        UserMessageDelivery,
        //This Bytes object represents the Payload
        Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
    >;
    type Message = <UserMessageDelivery as Channel>::Message;
    async fn subscribe(
        &self,
        topic: impl Channel + Send + 'static,
    ) -> Result<Self::Stream> {
        let subject = topic.subject();
        let sub = self.inner.subscribe(subject).await?;
        let stream = into_payload_byte_stream(sub, topic);
        Ok(ChannelStream::new(Box::pin(stream)))
    }
}

pub type FanoutStream = ChannelStream<
    FanoutMessageDelivery,
    Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
>;
#[async_trait]
impl Subscriber<FanoutMessageDelivery> for NatsCoreSubscriber {
    type Stream = FanoutStream;
    type Message = <FanoutMessageDelivery as Channel>::Message;
    async fn subscribe(
        &self,
        topic: impl Channel + Send + 'static,
    ) -> Result<Self::Stream> {
        let subject = topic.subject();
        let sub = self.inner.subscribe(subject).await?;
        let stream = sub.map(|msg| Ok(msg.payload));
        Ok(ChannelStream::new(Box::pin(stream)))
    }
}

pub type GroupEventStream = ChannelStream<
    GroupChangeEvent,
    Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
>;
#[async_trait]
impl Subscriber<GroupChangeEvent> for NatsCoreSubscriber {
    type Stream = GroupEventStream;
    type Message = <GroupChangeEvent as Channel>::Message;
    async fn subscribe(
        &self,
        topic: impl Channel + Send + 'static,
    ) -> Result<Self::Stream> {
        let subject = topic.subject();
        let sub = self.inner.subscribe(subject).await?;
        let stream = sub.map(|msg| Ok(msg.payload));
        Ok(ChannelStream::new(Box::pin(stream)))
    }
}
impl NatsCoreSubscriber {
    pub fn new(client: async_nats::Client) -> Self {
        Self { inner: client }
    }
}
///This is a transport specific stripping method to transform some
/// stream into a stream of domain specific Channel associated
/// messages. Currently, a similar function needs to be made for any
/// concrete instance of a different transport protocol.
///
///TODO:Make this function into a concrete struct that implements
/// stream
fn into_payload_byte_stream(
    sub: async_nats::Subscriber,
    channel: impl Channel,
) -> impl Stream<Item = Result<Bytes>> + Send {
    let subject = channel.subject();
    sub.filter_map(move |msg| {
        let subject = subject.clone();
        async move {
            if let Some(status) = msg.status {
                return Some(Err(NatsAdapterError::ProtocolError {
                    status,
                    description: msg.description,
                }
                .into()));
            }

            if msg.subject.as_str() != subject {
                return Some(Err(NatsAdapterError::SubjectMismatch {
                    expected: subject,
                    got: msg.subject.to_string(),
                }
                .into()));
            }

            Some(Ok(msg.payload))
        }
    })
}
