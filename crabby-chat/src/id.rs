use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use ferroid::{
    futures::{SnowflakeGeneratorAsyncExt, TokioSleep},
    generator::AtomicSnowflakeGenerator,
    id::SnowflakeTwitterId,
    time::MonotonicClock,
};
#[async_trait]
pub trait GenerateId: Send + Sync + 'static {
    async fn id(&self) -> u64;
}

pub struct IdGenerator {
    generator: Arc<dyn GenerateId>,
}
impl IdGenerator {
    pub fn new(g: impl GenerateId) -> Self {
        Self {
            generator: Arc::new(g),
        }
    }
}
#[async_trait]
impl GenerateId for IdGenerator {
    async fn id(&self) -> u64 {
        self.generator.id().await
    }
}
#[async_trait]
impl GenerateId
    for AtomicSnowflakeGenerator<SnowflakeTwitterId, MonotonicClock>
{
    async fn id(&self) -> u64 {
        self.next_id_async::<TokioSleep>().await.to_raw()
    }
}

#[cfg(test)]
pub struct NoOpIdGeneratorImpl;
#[cfg(test)]
#[async_trait]
impl GenerateId for NoOpIdGeneratorImpl {
    async fn id(&self) -> u64 {
        1u64
    }
}
