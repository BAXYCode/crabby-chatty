use crate::error::ChatError;

pub trait Handler<T, R> {
    async fn handle(&mut self, event: T) -> Result<R, ChatError>;
}
