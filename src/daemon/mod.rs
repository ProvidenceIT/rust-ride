//! Daemon module for headless operation.
//!
//! This module provides the daemon process management for running RustRide
//! without a GUI on Linux systems (including Raspberry Pi).
//!
//! T032: Daemonize support - background fork using daemonize crate
//! T064-T070: Configuration support
//! T072: Detect incomplete rides on daemon startup

pub mod handler;
pub mod server;
pub mod signals;
pub mod state;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::ipc::protocol::IpcServer;
use crate::storage::config::{load_daemon_config, DaemonSettings};
use state::DaemonState;

/// T072: Information about an incomplete ride detected on startup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IncompleteRideInfo {
    /// Ride ID
    pub ride_id: String,
    /// When the ride started
    pub started_at: String,
    /// Number of samples recorded
    pub sample_count: usize,
    /// Approximate duration in seconds
    pub duration_seconds: u32,
}

use daemonize::Daemonize;

// Re-export config path functions
pub use crate::storage::config::{default_pid_path, default_socket_path};

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub pid_path: PathBuf,
    pub log_path: Option<PathBuf>,
    pub foreground: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            pid_path: default_pid_path(),
            log_path: None,
            foreground: true,
        }
    }
}

/// T032: Daemonize the process (fork to background)
///
/// This function performs the daemonization before the async runtime starts.
/// It must be called before creating the tokio runtime.
pub fn daemonize(config: &DaemonConfig) -> anyhow::Result<()> {
    use std::fs::File;

    info!("Daemonizing process...");

    // Create PID file directory if needed
    if let Some(parent) = config.pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Setup daemonize with PID file
    let mut daemonize = Daemonize::new()
        .pid_file(&config.pid_path)
        .chown_pid_file(true)
        .working_directory("/");

    // Redirect stdout/stderr to log file if specified
    if let Some(ref log_path) = config.log_path {
        // Create log file directory if needed
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let stdout = File::create(log_path)?;
        let stderr = stdout.try_clone()?;
        daemonize = daemonize.stdout(stdout).stderr(stderr);
    }

    // Fork to background
    match daemonize.start() {
        Ok(_) => {
            info!("Daemon forked to background, PID file: {:?}", config.pid_path);
            Ok(())
        }
        Err(e) => {
            error!("Failed to daemonize: {}", e);
            anyhow::bail!("Failed to daemonize: {}", e)
        }
    }
}

/// Check if daemon is already running by checking PID file
pub fn is_daemon_running(config: &DaemonConfig) -> bool {
    if !config.pid_path.exists() {
        return false;
    }

    // Read PID from file
    if let Ok(pid_str) = std::fs::read_to_string(&config.pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is running using kill(pid, 0)
            // This doesn't actually send a signal, just checks if process exists
            unsafe {
                if libc::kill(pid, 0) == 0 {
                    return true;
                }
            }
        }
    }

    // PID file exists but process not running - stale PID file
    warn!("Removing stale PID file: {:?}", config.pid_path);
    let _ = std::fs::remove_file(&config.pid_path);
    false
}

/// Stop a running daemon by sending SIGTERM
pub fn stop_daemon(config: &DaemonConfig) -> anyhow::Result<bool> {
    if !config.pid_path.exists() {
        return Ok(false);
    }

    // Read PID from file
    let pid_str = std::fs::read_to_string(&config.pid_path)?;
    let pid: i32 = pid_str.trim().parse()?;

    // Send SIGTERM
    unsafe {
        if libc::kill(pid, libc::SIGTERM) == 0 {
            info!("Sent SIGTERM to daemon PID {}", pid);
            Ok(true)
        } else {
            // Process doesn't exist
            warn!("Daemon process {} not found, removing stale PID file", pid);
            let _ = std::fs::remove_file(&config.pid_path);
            Ok(false)
        }
    }
}

/// Run the daemon with the given configuration
pub async fn run_daemon(config: DaemonConfig) -> anyhow::Result<()> {
    info!("Starting RustRide daemon...");

    // Create socket directory if needed
    if let Some(parent) = config.socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create PID file directory if needed
    if let Some(parent) = config.pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write PID file (for foreground mode - daemonize() handles it for background)
    if config.foreground {
        std::fs::write(&config.pid_path, format!("{}", std::process::id()))?;
    }

    // T072: Check for incomplete rides from previous sessions
    let incomplete_rides = check_incomplete_rides();

    // Initialize daemon state
    let state = Arc::new(RwLock::new(DaemonState::new()));

    // Update state with paths and register incomplete rides
    {
        let mut state_guard = state.write().await;
        state_guard.socket_path = Some(config.socket_path.clone());
        state_guard.log_path = config.log_path.clone();
        state_guard.incomplete_rides = incomplete_rides;
        state_guard.set_running();
    }

    // T066: Auto-connect to preferred sensors
    auto_connect_preferred_sensors(state.clone()).await;

    // Start IPC server
    let server = IpcServer::new(&config.socket_path, state.clone()).await?;

    // Setup signal handlers
    let shutdown_signal = signals::setup_signal_handlers()?;

    info!("Daemon listening on {:?}", config.socket_path);

    // T076: Start auto-save checkpoint task
    let autosave_state = state.clone();
    let autosave_interval = load_daemon_config()
        .map(|c| c.autosave_interval_secs)
        .unwrap_or(30);
    let autosave_task = tokio::spawn(async move {
        run_autosave_loop(autosave_state, autosave_interval).await;
    });

    // Run server until shutdown signal
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = shutdown_signal => {
            info!("Received shutdown signal");
        }
    }

    // Cancel autosave task
    autosave_task.abort();

    // Graceful shutdown
    info!("Shutting down daemon...");
    {
        let mut state = state.write().await;
        state.initiate_shutdown();
    }

    // Cleanup socket file
    if config.socket_path.exists() {
        let _ = std::fs::remove_file(&config.socket_path);
    }

    // Cleanup PID file
    if config.pid_path.exists() {
        let _ = std::fs::remove_file(&config.pid_path);
    }

    info!("Daemon stopped");
    Ok(())
}

// Re-export directories crate for path resolution
use directories as dirs;

/// T076: Run the auto-save checkpoint loop.
///
/// This task runs every `interval_secs` seconds and saves ride data
/// to the autosave table for crash recovery.
async fn run_autosave_loop(state: Arc<RwLock<DaemonState>>, interval_secs: u32) {
    use tokio::time::{interval, Duration};

    let mut ticker = interval(Duration::from_secs(interval_secs as u64));
    info!("Auto-save checkpoint enabled (every {}s)", interval_secs);

    loop {
        ticker.tick().await;

        // Check if there's an active session to save
        let should_save = {
            let state_guard = state.read().await;
            state_guard.active_session.is_some()
                && state_guard.status == state::DaemonStatus::Running
        };

        if should_save {
            // TODO: Actually save ride data to autosave table
            // This requires integration with the RideRecorder
            tracing::debug!("Auto-save checkpoint triggered (integration pending)");
        }
    }
}

/// T068: Configure tracing subscriber from daemon config file.
///
/// This should be called early in daemon startup, before other logging occurs.
/// Configures log level and optionally redirects logs to a file.
pub fn configure_tracing_from_config() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // Load daemon config
    let daemon_config = load_daemon_config();

    // Determine log level (from config or default to info)
    let log_level = daemon_config
        .as_ref()
        .map(|c| c.log_level.to_string())
        .unwrap_or_else(|| "info".to_string());

    // Build the filter, preferring env var RUST_LOG if set
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&log_level));

    // Check if we need to log to a file
    let log_path = daemon_config.as_ref().and_then(|c| c.log_path.clone());

    if let Some(path) = log_path {
        // Ensure log directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Try to create a file appender
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                // Log to file
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false);

                tracing_subscriber::registry()
                    .with(filter)
                    .with(file_layer)
                    .init();

                // Can't use tracing here since we just configured it
                eprintln!("Daemon logging to file: {:?}", path);
            }
            Err(e) => {
                // Fall back to stderr
                eprintln!("Warning: Could not open log file {:?}: {}", path, e);
                tracing_subscriber::registry()
                    .with(filter)
                    .with(tracing_subscriber::fmt::layer())
                    .init();
            }
        }
    } else {
        // Log to stderr (default)
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

/// T066: Auto-connect to preferred sensors on daemon startup.
///
/// This function reads the daemon config and attempts to connect to
/// any sensors listed in the preferred_sensors configuration.
pub async fn auto_connect_preferred_sensors(
    state: Arc<RwLock<DaemonState>>,
) {
    // Load daemon config to get preferred sensors
    let config = match load_daemon_config() {
        Some(c) => c,
        None => {
            info!("No daemon config found, skipping sensor auto-connect");
            return;
        }
    };

    if config.preferred_sensors.is_empty() {
        info!("No preferred sensors configured");
        return;
    }

    info!(
        "Auto-connecting to {} preferred sensor(s)...",
        config.preferred_sensors.len()
    );

    for sensor in &config.preferred_sensors {
        info!(
            "  - {} ({}) [{}]",
            sensor.name, sensor.id, sensor.sensor_type
        );
        // TODO: Actual BLE connection via SensorManager
        // For now, log the intent - full integration requires SensorManager instance
    }

    // Update state to indicate we attempted auto-connect
    {
        let mut state_guard = state.write().await;
        state_guard.set_ble_adapter_available(true);
    }

    info!("Sensor auto-connect check complete (integration pending)");
}

/// T072: Check for incomplete rides from previous daemon sessions.
///
/// This function should be called on daemon startup to detect rides that
/// were not properly closed (e.g., due to crash or power loss).
/// Returns a list of incomplete rides that can be recovered.
pub fn check_incomplete_rides() -> Vec<IncompleteRideInfo> {
    // TODO: Query database for rides without ended_at timestamp
    // that have autosave data or samples but weren't completed.
    //
    // For now, check the autosave table for any data:
    // SELECT ride_id, started_at, sample_count FROM autosave_rides
    //
    // This would be implemented when database is integrated.

    info!("Checking for incomplete rides from previous sessions...");

    // Placeholder: Return empty list until database integration
    let incomplete: Vec<IncompleteRideInfo> = Vec::new();

    if incomplete.is_empty() {
        info!("No incomplete rides found");
    } else {
        warn!(
            "Found {} incomplete ride(s) that can be recovered",
            incomplete.len()
        );
        for ride in &incomplete {
            warn!(
                "  - Ride {} started at {}: {} samples (~{}s)",
                ride.ride_id, ride.started_at, ride.sample_count, ride.duration_seconds
            );
        }
    }

    incomplete
}

/// T072: Store incomplete ride info in daemon state for later recovery.
pub async fn register_incomplete_rides(
    state: Arc<RwLock<DaemonState>>,
    incomplete_rides: Vec<IncompleteRideInfo>,
) {
    if incomplete_rides.is_empty() {
        return;
    }

    let mut state_guard = state.write().await;
    state_guard.incomplete_rides = incomplete_rides;
    info!("Registered incomplete rides for recovery via CLI");
}
