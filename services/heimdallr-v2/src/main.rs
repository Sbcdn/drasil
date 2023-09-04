use std::process;

use heimdallr_v2::bootstrap;
use heimdallr_v2::settings::Settings;
use heimdallr_v2::tracer::Tracer;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    Tracer::new("heimdallr", "info").init_subscriber(std::io::stdout);
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(err) => {
            tracing::error!("failed to load configuration {err}");
            process::exit(1);
        }
    };
    let app = bootstrap::Application::new(settings).await?;
    if let Err(err) = app.listen_and_serve().await {
        tracing::error!("failed to application server {err}");
        process::exit(1);
    }

    Ok(())
}
