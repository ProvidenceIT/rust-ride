//! Rides management commands (list, export, recover).
//!
//! T052: CLI rides commands with IPC integration
//! T053: Export with --format and --output flags
//! T073: Recover subcommand for incomplete rides

use clap::Subcommand;
use std::path::PathBuf;

use crate::cli::client::send_command;
use crate::cli::{exit_codes, is_json_output};
use crate::daemon::default_socket_path;
use crate::ipc::messages::IpcRequest;

/// Export format options
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ExportFormat {
    /// FIT format (Garmin)
    Fit,
    /// TCX format (XML-based)
    Tcx,
    /// CSV format
    Csv,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Fit => write!(f, "fit"),
            ExportFormat::Tcx => write!(f, "tcx"),
            ExportFormat::Csv => write!(f, "csv"),
        }
    }
}

/// Rides management subcommands
#[derive(Debug, Subcommand)]
pub enum RidesCommands {
    /// List recorded rides
    List {
        /// Maximum number of rides to show
        #[arg(long, short, default_value = "10")]
        limit: usize,
    },
    /// Export a ride to file
    Export {
        /// Ride ID to export
        ride_id: String,

        /// T053: Output format
        #[arg(long, short, value_enum, default_value = "fit")]
        format: ExportFormat,

        /// T053: Output file path (default: auto-generated)
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// T073: List incomplete rides that can be recovered
    Incomplete,
    /// T073: Recover an incomplete ride
    Recover {
        /// Ride ID to recover
        ride_id: String,
    },
}

/// Execute a rides subcommand
pub async fn execute(cmd: RidesCommands) -> i32 {
    match cmd {
        RidesCommands::List { limit } => execute_list(limit).await,
        RidesCommands::Export {
            ride_id,
            format,
            output,
        } => execute_export(ride_id, format, output).await,
        RidesCommands::Incomplete => execute_incomplete().await,
        RidesCommands::Recover { ride_id } => execute_recover(ride_id).await,
    }
}

/// List rides
async fn execute_list(limit: usize) -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::with_params(
        "rides.list",
        serde_json::json!({
            "limit": limit
        }),
    );

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
                    if let Some(result) = &response.result {
                        if let Some(rides) = result.get("rides").and_then(|v| v.as_array()) {
                            if rides.is_empty() {
                                println!("No rides recorded yet.");
                            } else {
                                println!("Recorded Rides");
                                println!("==============");
                                for ride in rides {
                                    let id = ride.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                    let date =
                                        ride.get("date").and_then(|v| v.as_str()).unwrap_or("?");
                                    let duration = ride
                                        .get("duration_seconds")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    let distance = ride
                                        .get("distance_km")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);

                                    let hours = duration / 3600;
                                    let minutes = (duration % 3600) / 60;

                                    println!(
                                        "{} | {} | {}h {}m | {:.1} km",
                                        id, date, hours, minutes, distance
                                    );
                                }
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
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// T051/T053: Export a ride
async fn execute_export(ride_id: String, format: ExportFormat, output: Option<PathBuf>) -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::with_params(
        "ride.export",
        serde_json::json!({
            "ride_id": ride_id,
            "format": format.to_string(),
            "output_path": output.as_ref().map(|p| p.display().to_string()),
        }),
    );

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
                    if let Some(result) = &response.result {
                        let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                        println!("Exported ride to: {}", path);
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
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// T073: List incomplete rides that can be recovered
async fn execute_incomplete() -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::new("rides.incomplete");

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
                } else if let Some(result) = &response.result {
                    if let Some(rides) = result.get("incomplete_rides").and_then(|v| v.as_array()) {
                        if rides.is_empty() {
                            println!("No incomplete rides found.");
                        } else {
                            println!("Incomplete Rides (recoverable)");
                            println!("==============================");
                            for ride in rides {
                                let id = ride.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                let date = ride.get("date").and_then(|v| v.as_str()).unwrap_or("?");
                                let samples = ride
                                    .get("sample_count")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let duration_secs = ride
                                    .get("duration_seconds")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let minutes = duration_secs / 60;

                                println!(
                                    "{} | {} | {} samples | ~{} min",
                                    id, date, samples, minutes
                                );
                            }
                            println!();
                            println!(
                                "Use 'rustride-cli rides recover <ride_id>' to recover a ride."
                            );
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
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// T073: Recover an incomplete ride
async fn execute_recover(ride_id: String) -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::with_params(
        "ride.recover",
        serde_json::json!({
            "ride_id": ride_id,
        }),
    );

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
                } else if let Some(result) = &response.result {
                    let recovered_id = result
                        .get("ride_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let status = result
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    println!("Ride {} recovery: {}", recovered_id, status);
                    if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
                        println!("{}", msg);
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
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}
