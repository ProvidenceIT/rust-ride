//! RustRide - Indoor Cycling Training Application
//!
//! Main entry point for the application.
//!
//! T115: Enable accesskit for screen reader support

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

    // T115: Enable accesskit for screen reader support (NVDA, VoiceOver, Orca)
    // AccessKit is enabled by default in eframe 0.33+ when using egui's built-in accessibility
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("RustRide"),
        // T115: AccessKit is automatically enabled in eframe 0.33+
        // Screen readers can now access the UI through platform accessibility APIs
        ..Default::default()
    };

    eframe::run_native(
        "RustRide",
        options,
        Box::new(|cc| Ok(Box::new(app::RustRideApp::new(cc)))),
    )
}
