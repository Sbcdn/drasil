use heimdallr_v2::bootstrap;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let app = bootstrap::Application::new().await?;
    app.listen_and_serve().await?;
    Ok(())
}
