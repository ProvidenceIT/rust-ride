//! Sensor management commands (list, connect, disconnect, status).
//!
//! T060: CLI sensors commands with IPC integration

use clap::Subcommand;

use crate::cli::client::send_command;
use crate::cli::{exit_codes, is_json_output};
use crate::daemon::default_socket_path;
use crate::ipc::messages::IpcRequest;

/// Sensor management subcommands
#[derive(Debug, Subcommand)]
pub enum SensorsCommands {
    /// List available sensors (discovered and connected)
    List,
    /// Connect to a sensor
    Connect {
        /// Sensor ID (BLE address or ANT+ device ID)
        sensor_id: String,
    },
    /// Disconnect from a sensor
    Disconnect {
        /// Sensor ID to disconnect
        sensor_id: String,
    },
    /// Show sensor status
    Status,
}

/// Execute a sensors subcommand
pub async fn execute(cmd: SensorsCommands) -> i32 {
    match cmd {
        SensorsCommands::List => execute_list().await,
        SensorsCommands::Connect { sensor_id } => execute_connect(sensor_id).await,
        SensorsCommands::Disconnect { sensor_id } => execute_disconnect(sensor_id).await,
        SensorsCommands::Status => execute_status().await,
    }
}

/// T057: List sensors
async fn execute_list() -> i32 {
    let socket_path = default_socket_path();
    let request = IpcRequest::new("sensors.list");

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
                    // Connected sensors
                    if let Some(connected) = result.get("connected").and_then(|v| v.as_array()) {
                        println!("Connected Sensors ({}):", connected.len());
                        println!("------------------------");
                        if connected.is_empty() {
                            println!("  (none)");
                        } else {
                            for sensor in connected {
                                print_sensor(sensor);
                            }
                        }
                    }

                    println!();

                    // Discovered sensors
                    if let Some(discovered) = result.get("discovered").and_then(|v| v.as_array()) {
                        println!("Discovered Sensors ({}):", discovered.len());
                        println!("--------------------------");
                        if discovered.is_empty() {
                            println!("  (none - try scanning first)");
                        } else {
                            for sensor in discovered {
                                print_sensor(sensor);
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

/// Helper to print a sensor in human-readable format
fn print_sensor(sensor: &serde_json::Value) {
    let id = sensor.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let name = sensor
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let sensor_type = sensor
        .get("sensor_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let status = sensor
        .get("connection_status")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    print!("  {} ({}) - {} [{}]", name, sensor_type, id, status);

    // Signal strength
    if let Some(rssi) = sensor.get("signal_strength_dbm").and_then(|v| v.as_i64()) {
        print!(" Signal: {}dBm", rssi);
    }

    // Battery
    if let Some(battery) = sensor.get("battery_percent").and_then(|v| v.as_u64()) {
        print!(" Battery: {}%", battery);
    }

    println!();
}

/// T058: Connect to a sensor
async fn execute_connect(sensor_id: String) -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::with_params(
        "sensor.connect",
        serde_json::json!({
            "sensor_id": sensor_id
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
                    println!("Connected to sensor: {}", sensor_id);
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
                    eprintln!("Failed to connect to sensor: {}", error_msg);
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

/// T059: Disconnect from a sensor
async fn execute_disconnect(sensor_id: String) -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::with_params(
        "sensor.disconnect",
        serde_json::json!({
            "sensor_id": sensor_id
        }),
    );

    match send_command(&socket_path, request).await {
        Ok(response) => {
            if response.success {
                if is_json_output() {
                    println!(
                        r#"{{"status": "disconnected", "sensor_id": "{}"}}"#,
                        sensor_id
                    );
                } else {
                    println!("Disconnected from sensor: {}", sensor_id);
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
                    eprintln!("Failed to disconnect: {}", error_msg);
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

/// Show sensor status
async fn execute_status() -> i32 {
    // Reuse list command for status
    execute_list().await
}
