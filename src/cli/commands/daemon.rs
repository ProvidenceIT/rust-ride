//! Daemon control commands (start, stop, status).
//!
//! T034/T035: CLI daemon commands with IPC integration

use clap::Subcommand;
use std::path::Path;
use std::process::Command;

use crate::cli::client::{is_daemon_running, send_command};
use crate::cli::{exit_codes, is_json_output};
use crate::daemon::{default_pid_path, default_socket_path, stop_daemon, DaemonConfig};
use crate::ipc::messages::IpcRequest;

/// Daemon control subcommands
#[derive(Debug, Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon in the background
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop a running daemon
    Stop,
    /// Show daemon status
    Status,
}

/// Execute a daemon subcommand
pub async fn execute(cmd: DaemonCommands) -> i32 {
    match cmd {
        DaemonCommands::Start { foreground } => execute_start(foreground).await,
        DaemonCommands::Stop => execute_stop().await,
        DaemonCommands::Status => execute_status().await,
    }
}

/// Start the daemon
async fn execute_start(foreground: bool) -> i32 {
    let socket_path = default_socket_path();

    // Check if daemon is already running
    if is_daemon_running(&socket_path).await {
        if is_json_output() {
            println!(r#"{{"error": "Daemon is already running"}}"#);
        } else {
            eprintln!("Error: Daemon is already running");
        }
        return exit_codes::COMMAND_REJECTED;
    }

    // Start the daemon by executing rustride with --headless flag
    let exe = std::env::current_exe().unwrap_or_else(|_| "rustride".into());
    // Try to find the main binary (rustride, not rustride-cli)
    let exe_dir = exe.parent().unwrap_or(Path::new("."));
    let main_exe = exe_dir.join("rustride");

    let mut cmd = Command::new(&main_exe);
    cmd.arg("--headless");
    if foreground {
        cmd.arg("--foreground");
    }

    if foreground {
        // Run in foreground - exec the command (replaces this process)
        if is_json_output() {
            println!(r#"{{"status": "starting", "mode": "foreground"}}"#);
        } else {
            println!("Starting daemon in foreground mode...");
        }

        // Use exec to replace this process
        let status = cmd.status();
        match status {
            Ok(s) => s.code().unwrap_or(exit_codes::GENERAL_ERROR),
            Err(e) => {
                if is_json_output() {
                    println!(r#"{{"error": "Failed to start daemon: {}"}}"#, e);
                } else {
                    eprintln!("Failed to start daemon: {}", e);
                }
                exit_codes::GENERAL_ERROR
            }
        }
    } else {
        // Start daemon in background
        match cmd.spawn() {
            Ok(_) => {
                // Wait a moment for daemon to start
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Verify it's running
                if is_daemon_running(&socket_path).await {
                    if is_json_output() {
                        println!(
                            r#"{{"status": "started", "socket": "{}"}}"#,
                            socket_path.display()
                        );
                    } else {
                        println!("Daemon started successfully");
                        println!("Socket: {}", socket_path.display());
                    }
                    exit_codes::SUCCESS
                } else {
                    if is_json_output() {
                        println!(r#"{{"error": "Daemon started but failed to connect"}}"#);
                    } else {
                        eprintln!("Daemon started but failed to connect. Check logs.");
                    }
                    exit_codes::GENERAL_ERROR
                }
            }
            Err(e) => {
                if is_json_output() {
                    println!(r#"{{"error": "Failed to start daemon: {}"}}"#, e);
                } else {
                    eprintln!("Failed to start daemon: {}", e);
                }
                exit_codes::GENERAL_ERROR
            }
        }
    }
}

/// Stop the daemon
async fn execute_stop() -> i32 {
    let socket_path = default_socket_path();
    let config = DaemonConfig::default();

    // First try graceful shutdown via IPC
    if is_daemon_running(&socket_path).await {
        let request = IpcRequest::new("daemon.shutdown");
        match send_command(&socket_path, request).await {
            Ok(response) => {
                if response.success {
                    if is_json_output() {
                        println!(r#"{{"status": "stopped"}}"#);
                    } else {
                        println!("Daemon stopped successfully");
                    }
                    return exit_codes::SUCCESS;
                }
            }
            Err(_) => {
                // Connection closed, daemon might be shutting down
            }
        }
    }

    // Fall back to SIGTERM via PID file
    match stop_daemon(&config) {
        Ok(true) => {
            if is_json_output() {
                println!(r#"{{"status": "stopped", "method": "signal"}}"#);
            } else {
                println!("Daemon stopped via signal");
            }
            exit_codes::SUCCESS
        }
        Ok(false) => {
            if is_json_output() {
                println!(r#"{{"error": "Daemon is not running"}}"#);
            } else {
                eprintln!("Daemon is not running");
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
        Err(e) => {
            if is_json_output() {
                println!(r#"{{"error": "Failed to stop daemon: {}"}}"#, e);
            } else {
                eprintln!("Failed to stop daemon: {}", e);
            }
            exit_codes::GENERAL_ERROR
        }
    }
}

/// Get daemon status
async fn execute_status() -> i32 {
    let socket_path = default_socket_path();

    // Check if daemon is running
    if !is_daemon_running(&socket_path).await {
        if is_json_output() {
            println!(r#"{{"running": false}}"#);
        } else {
            println!("Daemon is not running");
        }
        return exit_codes::DAEMON_NOT_RUNNING;
    }

    // Send status request
    let request = IpcRequest::new("daemon.status");
    match send_command(&socket_path, request).await {
        Ok(response) => {
            if response.success {
                if is_json_output() {
                    if let Some(result) = response.result {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result).unwrap_or_default()
                        );
                    }
                } else {
                    // Human-readable output
                    if let Some(result) = response.result {
                        println!("Daemon Status");
                        println!("=============");
                        if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
                            println!("Status: {}", status);
                        }
                        if let Some(pid) = result.get("pid").and_then(|v| v.as_u64()) {
                            println!("PID: {}", pid);
                        }
                        if let Some(uptime) = result.get("uptime_seconds").and_then(|v| v.as_u64())
                        {
                            let hours = uptime / 3600;
                            let minutes = (uptime % 3600) / 60;
                            let seconds = uptime % 60;
                            println!("Uptime: {}h {}m {}s", hours, minutes, seconds);
                        }
                        if let Some(sensors) =
                            result.get("connected_sensors").and_then(|v| v.as_array())
                        {
                            println!("Connected sensors: {}", sensors.len());
                        }
                        if let Some(session) = result.get("active_session") {
                            if !session.is_null() {
                                println!("Active session: Yes");
                            } else {
                                println!("Active session: None");
                            }
                        }
                    }
                }
                exit_codes::SUCCESS
            } else {
                let error_msg = response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unknown error".into());
                if is_json_output() {
                    println!(r#"{{"error": "{}"}}"#, error_msg);
                } else {
                    eprintln!("Error: {}", error_msg);
                }
                exit_codes::GENERAL_ERROR
            }
        }
        Err(e) => {
            if is_json_output() {
                println!(r#"{{"error": "Failed to connect to daemon: {}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
            }
            exit_codes::CONNECTION_FAILED
        }
    }
}
