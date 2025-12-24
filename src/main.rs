//! RustRide - Indoor Cycling Training Application
//!
//! Main entry point for the application.

use eframe::egui;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod app;

fn main() -> eframe::Result<()> {
    // T024: Configure tracing subscriber
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting RustRide v{}", env!("CARGO_PKG_VERSION"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("RustRide"),
        ..Default::default()
    };

    eframe::run_native(
        "RustRide",
        options,
        Box::new(|cc| Ok(Box::new(app::RustRideApp::new(cc)))),
    )
}
