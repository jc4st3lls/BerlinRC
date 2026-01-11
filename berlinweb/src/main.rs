//! berlinweb crate entrypoint.
//!
//! Starts the Tokio runtime and launches the web server defined in the
//! `server` module. Keep this file minimal — most application logic lives
//! in `server`, `config`, and `html`.
//!
/// HTTP server implementation and request handling
mod server;
/// Configuration management and settings
mod config;
/// HTML rendering and page generation
mod html;

/// Entry point for the async Tokio runtime
#[tokio::main]
async fn main() {
    server::run().await;
}
