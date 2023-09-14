//! This module defines the tracer type for exposing telemetry

use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

/// Tracer type.
#[derive(Debug)]
pub struct Tracer<'a> {
    /// The application name.
    name: &'a str,

    /// The trace level.
    env_filter: &'a str,
}

impl<'a> Tracer<'a> {
    /// Creates new tracer.
    pub const fn new(name: &'a str, env_filter: &'a str) -> Self {
        Self { name, env_filter }
    }

    /// Initialize the tracer subscriber.
    pub fn init_subscriber<Sink>(&self, sink: Sink)
    where
        Sink: for<'b> MakeWriter<'b> + Send + Sync + 'static,
    {
        let formatting_layer = BunyanFormattingLayer::new(self.name.into(), sink);
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(self.env_filter));
        let subscriber = Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(formatting_layer);
        LogTracer::init().expect("failed to set logger.");
        set_global_default(subscriber).expect("failed to set subscriber");
    }
}
