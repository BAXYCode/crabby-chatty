use eyre::Result;

///This trait is used to convert incoming messages from inbound types to a domain specific type
pub trait Decode<I> {
    type Output;
    type Error;
    fn decode(item: I) -> Result<Self::Output>;
}
