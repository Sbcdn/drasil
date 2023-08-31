//! This module defines the application type and operations.

use axum::Router;
use std::net::TcpListener;

use crate::error::Result;

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
    pub async fn new() -> Result<Self> {
        let router = new_router()?;
        let listener = TcpListener::bind("127.0.0.1:4000")?;
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
pub fn new_router() -> Result<Router> {
    let router = Router::new();
    Ok(router)
}
