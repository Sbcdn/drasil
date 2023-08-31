//! This module defines the application type and operations.

use axum::Router;
use secrecy::Secret;
use std::net::TcpListener;

use crate::error::Result;
use crate::settings::Settings;
use crate::state::AppState;

/// The application type for starting the server.
#[derive(Debug)]
pub struct Application {
    /// The application port number.
    listener: TcpListener,
    /// The application router for handling requests.
    router: Router,
}

impl Application {
    /// Creates new application.
    pub async fn new(settings: Settings) -> Result<Self> {
        let Settings {
            application: app_settings,
            jwt: jwt_settings,
        } = settings;
        let addr = format!("{}:{}", app_settings.host, app_settings.port);
        let listener = TcpListener::bind(addr)?;

        let router = new_router(jwt_settings.pub_key)?;

        Ok(Application { listener, router })
    }

    /// Listen and serve requests.
    pub async fn listen_and_serve(self) -> Result<()> {
        let Application { listener, router } = self;
        axum_server::from_tcp(listener)
            .serve(router.into_make_service())
            .await?;
        Ok(())
    }
}

/// Create and configure new router.
pub fn new_router(jwt_pub_key: Secret<String>) -> Result<Router> {
    let app_state = AppState::new(jwt_pub_key)?;
    let router = Router::new().with_state(app_state);
    Ok(router)
}
