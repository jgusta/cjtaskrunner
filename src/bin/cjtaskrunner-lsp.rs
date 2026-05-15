#[tokio::main]
async fn main() {
    cjtaskrunner::lsp::run_stdio().await;
}
