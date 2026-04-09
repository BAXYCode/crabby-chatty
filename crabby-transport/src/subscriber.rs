use std::{marker::PhantomData, pin::Pin, task::Poll};

use async_trait::async_trait;
use bytes::Bytes;
use eyre::Result;
use futures_util::Stream;

use crate::{channel::Channel, codec::Codec};
#[async_trait]
pub trait Subscriber<C: Channel>: Send + Sync + 'static {
    //Here we use an associated typed stream to convert incoming messages
    // to the expected domain format
    type Stream: Stream + Unpin;
    type Message;

    async fn subscribe(
        &self,
        topic: impl Channel + Send + 'static,
    ) -> Result<Self::Stream>;
}

pub struct ChannelStream<C, S>
where
    S: Stream + Unpin,
    C: Channel + Unpin,
{
    stream: S,
    _channel: PhantomData<C>,
}
impl<C, S> ChannelStream<C, S>
where
    S: Stream + Unpin,
    C: Channel + Unpin,
{
    pub fn new(stream: S) -> Self {
        ChannelStream {
            stream,
            _channel: PhantomData,
        }
    }
}
///The ChannelStream is a stream of domain specific messages. Those
/// messages are what the Channel implementation is generic over. The
/// underlying stream that this struct is generic over is atransport
/// specific stream which often has protocol specific information
/// associated to it, so we strip that information to only yield the
/// domain message when the message is succesfully decoded.
/// The responsibility of stripping that transport specific
/// information is left to whoever wants to use this struct.
impl<C, S> Stream for ChannelStream<C, S>
where
    //the Bytes here are the encoded
    // payload bytes
    S: Stream<Item = Result<Bytes>> + Unpin,
    C: Channel + Unpin,
{
    type Item = Result<C::Message>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let channel_stream = self.get_mut();
        match Pin::new(&mut channel_stream.stream).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(encoded))) => {
                let decoded = C::Codec::decode(encoded.as_ref());
                if let Ok(msg) = decoded {
                    Poll::Ready(Some(Ok(msg)))
                } else {
                    Poll::Ready(None)
                }
            }
            std::task::Poll::Ready(None) => todo!(),
            std::task::Poll::Pending => todo!(),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
        }
    }
}
