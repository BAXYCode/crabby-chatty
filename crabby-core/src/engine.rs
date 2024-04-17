pub trait Engine {
    #[allow(async_fn_in_trait)]
    async fn run() {}
}
