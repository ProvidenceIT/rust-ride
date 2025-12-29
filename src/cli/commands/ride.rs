//! Ride control commands (start, stop).
//!
//! T044: CLI ride commands with IPC integration

use clap::Subcommand;

use crate::cli::client::send_command;
use crate::cli::{exit_codes, is_json_output};
use crate::daemon::default_socket_path;
use crate::ipc::messages::IpcRequest;

/// Ride control subcommands
#[derive(Debug, Subcommand)]
pub enum RideCommands {
    /// Start a free ride session
    Start,
    /// Stop the current ride session
    Stop,
}

/// Execute a ride subcommand
pub async fn execute(cmd: RideCommands) -> i32 {
    match cmd {
        RideCommands::Start => execute_start().await,
        RideCommands::Stop => execute_stop().await,
    }
}

/// Start a free ride
async fn execute_start() -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::new("ride.start");
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
                        let session_id = result
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        println!("Started free ride session: {}", session_id);
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
                exit_codes::COMMAND_REJECTED
            }
        }
        Err(e) => {
            if is_json_output() {
                println!(r#"{{"error": "Failed to connect to daemon: {}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
                eprintln!("Is the daemon running? Try: rustride-cli daemon start");
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// Stop the current ride
async fn execute_stop() -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::new("ride.stop");
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
                        let elapsed = result
                            .get("elapsed_seconds")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let hours = elapsed / 3600;
                        let minutes = (elapsed % 3600) / 60;
                        let seconds = elapsed % 60;
                        println!("Ride stopped");
                        println!("Duration: {}h {}m {}s", hours, minutes, seconds);

                        if let Some(metrics) = result.get("metrics") {
                            if let Some(distance) =
                                metrics.get("distance_km").and_then(|v| v.as_f64())
                            {
                                println!("Distance: {:.2} km", distance);
                            }
                            if let Some(calories) = metrics.get("calories").and_then(|v| v.as_u64())
                            {
                                println!("Calories: {}", calories);
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
                exit_codes::COMMAND_REJECTED
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
