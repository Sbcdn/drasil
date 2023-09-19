use heimdallr_v2::bootstrap::Application;
use heimdallr_v2::settings::Settings;

/// Test application
pub struct TestApp {
    pub client: reqwest::Client,
    pub address: String,
}

impl TestApp {
    /// Spawns new test application.
    pub async fn spawn() -> Self {
        let client = reqwest::Client::builder()
            .build()
            .expect("failed to create client");

        let settings = {
            let mut settings = Settings::load().expect("failed to load settings");
            settings.application.port = 0;
            settings
        };

        let host = settings.application.host.clone();
        let app = Application::new(settings)
            .await
            .expect("failed to create application");

        let address = app
            .listener
            .local_addr()
            .map(|addr| format!("http://{host}:{}", addr.port()))
            .expect("failed to get local address");

        tokio::spawn(app.listen_and_serve());
        Self { client, address }
    }
}
