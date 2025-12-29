//! Workout control commands (start, pause, resume, skip, stop).
//!
//! T045: CLI workout commands with IPC integration

use clap::Subcommand;
use std::path::PathBuf;

use crate::cli::client::send_command;
use crate::cli::{exit_codes, is_json_output};
use crate::daemon::default_socket_path;
use crate::ipc::messages::IpcRequest;

/// Workout control subcommands
#[derive(Debug, Subcommand)]
pub enum WorkoutCommands {
    /// Start a structured workout
    Start {
        /// Path to workout file (.zwo or .mrc)
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// T049: Wait for workout to complete before returning
        #[arg(long)]
        wait: bool,
    },
    /// Pause the current workout
    Pause,
    /// Resume a paused workout
    Resume,
    /// Skip to the next interval
    Skip,
    /// Stop the current workout
    Stop,
    /// Show live workout status
    Status,
}

/// Execute a workout subcommand
pub async fn execute(cmd: WorkoutCommands) -> i32 {
    match cmd {
        WorkoutCommands::Start { path, wait } => execute_start(path, wait).await,
        WorkoutCommands::Pause => execute_pause().await,
        WorkoutCommands::Resume => execute_resume().await,
        WorkoutCommands::Skip => execute_skip().await,
        WorkoutCommands::Stop => execute_stop().await,
        WorkoutCommands::Status => execute_status().await,
    }
}

/// Start a workout
/// T049: Added --wait flag support
async fn execute_start(path: PathBuf, wait: bool) -> i32 {
    let socket_path = default_socket_path();

    // Validate path exists
    if !path.exists() {
        if is_json_output() {
            println!(
                r#"{{"error": "Workout file not found: {}"}}"#,
                path.display()
            );
        } else {
            eprintln!("Error: Workout file not found: {}", path.display());
        }
        return exit_codes::RESOURCE_NOT_FOUND;
    }

    let request = IpcRequest::with_params(
        "workout.start",
        serde_json::json!({
            "path": path.display().to_string()
        }),
    );

    match send_command(&socket_path, request).await {
        Ok(response) => {
            if response.success {
                if is_json_output() {
                    if let Some(result) = response.result.clone() {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result).unwrap_or_default()
                        );
                    }
                } else if let Some(result) = &response.result {
                    let name = result
                        .get("workout_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let session_id = result
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    println!("Started workout: {}", name);
                    println!("Session ID: {}", session_id);
                }

                // T049/T050: Wait for workout completion if --wait flag is set
                if wait {
                    if !is_json_output() {
                        println!("Waiting for workout to complete...");
                    }
                    return wait_for_session_completion(&socket_path).await;
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

/// T050: Wait for session completion by polling status
async fn wait_for_session_completion(socket_path: &std::path::Path) -> i32 {
    loop {
        // Poll every 2 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let request = IpcRequest::new("status.live");
        match send_command(socket_path, request).await {
            Ok(response) => {
                if !response.success {
                    // No active session means workout completed
                    if is_json_output() {
                        println!(r#"{{"status": "completed"}}"#);
                    } else {
                        println!("Workout completed!");
                    }
                    return exit_codes::SUCCESS;
                }
                // Session still active, continue waiting
            }
            Err(_) => {
                // Connection lost, daemon may have stopped
                if is_json_output() {
                    println!(r#"{{"error": "Lost connection to daemon"}}"#);
                } else {
                    eprintln!("Lost connection to daemon");
                }
                return exit_codes::CONNECTION_FAILED;
            }
        }
    }
}

/// Pause workout
async fn execute_pause() -> i32 {
    let socket_path = default_socket_path();
    let request = IpcRequest::new("workout.pause");

    match send_command(&socket_path, request).await {
        Ok(response) => {
            if response.success {
                if is_json_output() {
                    println!(r#"{{"status": "paused"}}"#);
                } else {
                    println!("Workout paused");
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
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// Resume workout
async fn execute_resume() -> i32 {
    let socket_path = default_socket_path();
    let request = IpcRequest::new("workout.resume");

    match send_command(&socket_path, request).await {
        Ok(response) => {
            if response.success {
                if is_json_output() {
                    println!(r#"{{"status": "resumed"}}"#);
                } else {
                    println!("Workout resumed");
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
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// Skip interval
async fn execute_skip() -> i32 {
    let socket_path = default_socket_path();
    let request = IpcRequest::new("workout.skip");

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
                    let index = result
                        .get("current_interval_index")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let total = result
                        .get("total_intervals")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    println!("Skipped to interval {} of {}", index + 1, total);
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
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// Stop workout
async fn execute_stop() -> i32 {
    let socket_path = default_socket_path();
    let request = IpcRequest::new("workout.stop");

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
                    let elapsed = result
                        .get("elapsed_seconds")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let hours = elapsed / 3600;
                    let minutes = (elapsed % 3600) / 60;
                    let seconds = elapsed % 60;
                    println!("Workout stopped");
                    println!("Duration: {}h {}m {}s", hours, minutes, seconds);
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
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}

/// Show live workout status
async fn execute_status() -> i32 {
    let socket_path = default_socket_path();
    let request = IpcRequest::new("status.live");

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
                    println!("Live Status");
                    println!("===========");

                    let session_type = result
                        .get("session_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    println!("Session: {}", session_type);

                    let elapsed = result
                        .get("elapsed_seconds")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let hours = elapsed / 3600;
                    let minutes = (elapsed % 3600) / 60;
                    let seconds = elapsed % 60;
                    println!("Elapsed: {}h {}m {}s", hours, minutes, seconds);

                    let is_paused = result
                        .get("is_paused")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if is_paused {
                        println!("Status: PAUSED");
                    } else {
                        println!("Status: RUNNING");
                    }

                    if let Some(metrics) = result.get("metrics") {
                        println!("\nMetrics:");
                        if let Some(power) = metrics.get("power_watts").and_then(|v| v.as_u64()) {
                            println!("  Power: {}W", power);
                        }
                        if let Some(hr) = metrics.get("heart_rate_bpm").and_then(|v| v.as_u64()) {
                            println!("  Heart Rate: {} bpm", hr);
                        }
                        if let Some(cadence) = metrics.get("cadence_rpm").and_then(|v| v.as_u64()) {
                            println!("  Cadence: {} rpm", cadence);
                        }
                        if let Some(speed) = metrics.get("speed_kmh").and_then(|v| v.as_f64()) {
                            println!("  Speed: {:.1} km/h", speed);
                        }
                    }

                    if let Some(workout_info) = result.get("workout_info") {
                        if !workout_info.is_null() {
                            println!("\nWorkout:");
                            if let Some(name) = workout_info.get("name").and_then(|v| v.as_str()) {
                                println!("  Name: {}", name);
                            }
                            if let Some(interval_name) = workout_info
                                .get("current_interval_name")
                                .and_then(|v| v.as_str())
                            {
                                println!("  Current Interval: {}", interval_name);
                            }
                            if let Some(target) = workout_info
                                .get("target_power_watts")
                                .and_then(|v| v.as_u64())
                            {
                                println!("  Target Power: {}W", target);
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
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Failed to connect to daemon: {}", e);
            }
            exit_codes::DAEMON_NOT_RUNNING
        }
    }
}
