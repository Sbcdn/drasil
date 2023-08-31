use heimdallr_v2::{bootstrap, settings::Settings};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::load()?;
    let app = bootstrap::Application::new(settings).await?;
    app.listen_and_serve().await?;
    Ok(())
}
