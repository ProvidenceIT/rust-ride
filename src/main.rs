//! RustRide - Indoor Cycling Training Application
//!
//! Main entry point for the application.
//!
//! T115: Enable accesskit for screen reader support
//! T029: Add --headless flag for daemon mode (Linux only)

use clap::Parser;
use eframe::egui;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod app;

/// RustRide - Indoor Cycling Training Application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in headless daemon mode (Linux only)
    #[arg(long)]
    headless: bool,

    /// Run daemon in foreground (don't daemonize)
    #[arg(long)]
    foreground: bool,
}

fn main() {
    let args = Args::parse();

    // T024: Configure tracing subscriber
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting RustRide v{}", env!("CARGO_PKG_VERSION"));

    // T029: Handle --headless flag for daemon mode
    if args.headless {
        #[cfg(target_os = "linux")]
        {
            run_headless_daemon(args.foreground);
        }

        #[cfg(not(target_os = "linux"))]
        {
            eprintln!("Error: --headless mode is only supported on Linux.");
            std::process::exit(1);
        }
    } else {
        // Normal GUI mode
        run_gui().unwrap_or_else(|e| {
            eprintln!("Error running GUI: {}", e);
            std::process::exit(1);
        });
    }
}

/// Run the GUI application
fn run_gui() -> eframe::Result<()> {
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

/// Run the headless daemon (Linux only)
#[cfg(target_os = "linux")]
fn run_headless_daemon(foreground: bool) {
    use rustride::daemon::{
        configure_tracing_from_config, daemonize, is_daemon_running, run_daemon, DaemonConfig,
    };

    let config = DaemonConfig {
        foreground,
        ..Default::default()
    };

    // T032: Check if daemon is already running
    if is_daemon_running(&config) {
        eprintln!("Error: Daemon is already running. Use --foreground or stop the existing daemon first.");
        std::process::exit(1);
    }

    // T032: Daemonize if not running in foreground
    if !foreground {
        if let Err(e) = daemonize(&config) {
            eprintln!("Failed to daemonize: {}", e);
            std::process::exit(1);
        }
    }

    // T068: Configure tracing from daemon config file
    // This must be called after daemonize (in the child process)
    configure_tracing_from_config();

    // Create tokio runtime for async daemon
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        if let Err(e) = run_daemon(config).await {
            eprintln!("Daemon error: {}", e);
            std::process::exit(1);
        }
    });
}
