//! This module defines the application type and operations.

use axum::Router;
use std::net::TcpListener;

use crate::error::Result;
use crate::routes;
use crate::settings::Settings;
use crate::state::AppState;

/// The application type for starting the server.
#[allow(missing_debug_implementations)]
pub struct Application {
    /// The application listener.
    pub listener: TcpListener,
    /// The application server router for handling requests.
    router: Router,
}

impl Application {
    /// Creates new application.
    pub async fn new(settings: Settings) -> Result<Self> {
        let addr = settings.application.connection_string();
        let listener = TcpListener::bind(addr)?;
        let Settings {
            jwt: jwt_settings,
            odin: odin_settings,
            ..
        } = settings;

        let state = AppState::new(jwt_settings.pub_key, odin_settings.url)?;
        let router = routes::register_handlers(state);

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
