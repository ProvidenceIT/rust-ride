//! Sensor management commands (list, connect, disconnect, status, quality, diagnostics).
//!
//! T060: CLI sensors commands with IPC integration
//! T009-6.3: Added quality, reconnect, and diagnostics commands

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
    /// Show connection quality metrics for sensors
    Quality {
        /// Optional sensor ID (shows all if not specified)
        sensor_id: Option<String>,
    },
    /// Force reconnection to a sensor
    Reconnect {
        /// Sensor ID to reconnect
        sensor_id: String,
        /// Reset backoff state before reconnecting
        #[arg(long, short)]
        reset_backoff: bool,
    },
    /// Show detailed sensor diagnostics
    Diagnostics {
        /// Optional sensor ID (shows all if not specified)
        sensor_id: Option<String>,
        /// Include health monitoring info
        #[arg(long)]
        health: bool,
        /// Include connection state machine info
        #[arg(long)]
        state: bool,
        /// Include reconnection backoff info
        #[arg(long)]
        backoff: bool,
        /// Show all diagnostic information
        #[arg(long, short)]
        all: bool,
    },
}

/// Execute a sensors subcommand
pub async fn execute(cmd: SensorsCommands) -> i32 {
    match cmd {
        SensorsCommands::List => execute_list().await,
        SensorsCommands::Connect { sensor_id } => execute_connect(sensor_id).await,
        SensorsCommands::Disconnect { sensor_id } => execute_disconnect(sensor_id).await,
        SensorsCommands::Status => execute_status().await,
        SensorsCommands::Quality { sensor_id } => execute_quality(sensor_id).await,
        SensorsCommands::Reconnect {
            sensor_id,
            reset_backoff,
        } => execute_reconnect(sensor_id, reset_backoff).await,
        SensorsCommands::Diagnostics {
            sensor_id,
            health,
            state,
            backoff,
            all,
        } => execute_diagnostics(sensor_id, health, state, backoff, all).await,
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

/// T009-6.3: Show connection quality metrics for sensors
async fn execute_quality(sensor_id: Option<String>) -> i32 {
    let socket_path = default_socket_path();

    let request = match &sensor_id {
        Some(id) => IpcRequest::with_params(
            "sensor.quality",
            serde_json::json!({
                "sensor_id": id
            }),
        ),
        None => IpcRequest::new("sensors.quality"),
    };

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
                    // Single sensor quality
                    if let Some(quality) = result.get("quality") {
                        print_sensor_quality(quality);
                    }
                    // Multiple sensor qualities
                    else if let Some(qualities) = result.get("sensors").and_then(|v| v.as_array()) {
                        println!("Sensor Connection Quality:");
                        println!("===========================");
                        if qualities.is_empty() {
                            println!("  No sensors with quality monitoring active.");
                        } else {
                            for quality in qualities {
                                print_sensor_quality(quality);
                                println!();
                            }
                        }

                        // Summary
                        if let Some(summary) = result.get("summary") {
                            println!("Summary:");
                            println!("--------");
                            if let Some(poor) = summary.get("poor_count").and_then(|v| v.as_u64()) {
                                if poor > 0 {
                                    println!(
                                        "  ⚠ {} sensor(s) with poor connection quality",
                                        poor
                                    );
                                }
                            }
                            if let Some(degraded) =
                                summary.get("degraded_count").and_then(|v| v.as_u64())
                            {
                                if degraded > 0 {
                                    println!(
                                        "  ⚡ {} sensor(s) with degraded connection quality",
                                        degraded
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

/// Helper to print connection quality in human-readable format
fn print_sensor_quality(quality: &serde_json::Value) {
    let device_id = quality.get("device_id").and_then(|v| v.as_str()).unwrap_or("?");
    let name = quality.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let level = quality.get("level").and_then(|v| v.as_str()).unwrap_or("?");
    let score = quality.get("score").and_then(|v| v.as_u64()).unwrap_or(0);
    let signal_bars = quality.get("signal_bars").and_then(|v| v.as_u64()).unwrap_or(0);

    // Signal bars visualization
    let bars_display = match signal_bars {
        4 => "████",
        3 => "███░",
        2 => "██░░",
        1 => "█░░░",
        _ => "░░░░",
    };

    // Color indicator based on level
    let level_icon = match level {
        "Excellent" => "✓",
        "Good" => "✓",
        "Fair" => "⚡",
        "Poor" => "⚠",
        _ => "?",
    };

    println!("  {} {} [{}] - {} ({}%)", level_icon, name, device_id, level, score);
    println!("    Signal: {} ({} bars)", bars_display, signal_bars);

    // Metrics
    if let Some(metrics) = quality.get("metrics") {
        if let Some(rssi) = metrics.get("rssi_avg").and_then(|v| v.as_i64()) {
            println!("    RSSI: {} dBm", rssi);
        }
        if let Some(rate) = metrics.get("data_rate").and_then(|v| v.as_f64()) {
            println!("    Data Rate: {:.2} pkt/s", rate);
        }
        if let Some(loss) = metrics.get("packet_loss_rate").and_then(|v| v.as_f64()) {
            if loss > 0.0 {
                println!("    Packet Loss: {:.1}%", loss);
            }
        }
        if let Some(latency) = metrics.get("latency_avg_ms").and_then(|v| v.as_u64()) {
            println!("    Latency: {} ms", latency);
        }
    }

    // Component scores
    if let Some(metrics) = quality.get("metrics") {
        let rssi_score = metrics.get("rssi_score").and_then(|v| v.as_u64()).unwrap_or(0);
        let rate_score = metrics.get("data_rate_score").and_then(|v| v.as_u64()).unwrap_or(0);
        let loss_score = metrics.get("packet_loss_score").and_then(|v| v.as_u64()).unwrap_or(0);
        let latency_score = metrics.get("latency_score").and_then(|v| v.as_u64()).unwrap_or(0);

        println!(
            "    Scores: RSSI={}% Rate={}% Loss={}% Latency={}%",
            rssi_score, rate_score, loss_score, latency_score
        );
    }
}

/// T009-6.3: Force reconnection to a sensor
async fn execute_reconnect(sensor_id: String, reset_backoff: bool) -> i32 {
    let socket_path = default_socket_path();

    let request = IpcRequest::with_params(
        "sensor.reconnect",
        serde_json::json!({
            "sensor_id": sensor_id,
            "reset_backoff": reset_backoff
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
                    let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");

                    match status {
                        "reconnecting" => {
                            println!("Reconnecting to sensor: {}", sensor_id);
                            if let Some(delay) = result.get("next_delay_secs").and_then(|v| v.as_f64()) {
                                println!("  Next attempt delay: {:.1}s", delay);
                            }
                            if let Some(attempt) = result.get("attempt").and_then(|v| v.as_u64()) {
                                println!("  Attempt: {}", attempt);
                            }
                        }
                        "connected" => {
                            println!("Sensor {} is already connected", sensor_id);
                        }
                        "disconnecting" => {
                            println!("Disconnecting sensor {} for reconnection...", sensor_id);
                        }
                        _ => {
                            println!("Reconnection initiated for: {}", sensor_id);
                        }
                    }

                    if reset_backoff {
                        println!("  Backoff state reset");
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
                    eprintln!("Failed to reconnect sensor: {}", error_msg);
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

/// T009-6.3: Show detailed sensor diagnostics
async fn execute_diagnostics(
    sensor_id: Option<String>,
    health: bool,
    state: bool,
    backoff: bool,
    all: bool,
) -> i32 {
    let socket_path = default_socket_path();

    // If --all is specified, enable all diagnostic sections
    let include_health = health || all;
    let include_state = state || all;
    let include_backoff = backoff || all;

    // If no flags specified, show all by default
    let (include_health, include_state, include_backoff) = if !health && !state && !backoff && !all
    {
        (true, true, true)
    } else {
        (include_health, include_state, include_backoff)
    };

    let request = match &sensor_id {
        Some(id) => IpcRequest::with_params(
            "sensor.diagnostics",
            serde_json::json!({
                "sensor_id": id,
                "include_health": include_health,
                "include_state": include_state,
                "include_backoff": include_backoff
            }),
        ),
        None => IpcRequest::with_params(
            "sensors.diagnostics",
            serde_json::json!({
                "include_health": include_health,
                "include_state": include_state,
                "include_backoff": include_backoff
            }),
        ),
    };

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
                    // Single sensor diagnostics
                    if let Some(diag) = result.get("diagnostics") {
                        print_sensor_diagnostics(diag, include_health, include_state, include_backoff);
                    }
                    // Multiple sensor diagnostics
                    else if let Some(sensors) = result.get("sensors").and_then(|v| v.as_array()) {
                        println!("Sensor Diagnostics:");
                        println!("===================");
                        if sensors.is_empty() {
                            println!("  No sensors connected.");
                        } else {
                            for sensor in sensors {
                                print_sensor_diagnostics(
                                    sensor,
                                    include_health,
                                    include_state,
                                    include_backoff,
                                );
                                println!();
                            }
                        }

                        // System summary
                        if let Some(summary) = result.get("system") {
                            println!("System Summary:");
                            println!("---------------");
                            if let Some(connected) = summary.get("connected_count").and_then(|v| v.as_u64()) {
                                println!("  Connected sensors: {}", connected);
                            }
                            if let Some(health_monitored) = summary.get("health_monitored_count").and_then(|v| v.as_u64()) {
                                println!("  Health monitored: {}", health_monitored);
                            }
                            if let Some(quality_monitored) = summary.get("quality_monitored_count").and_then(|v| v.as_u64()) {
                                println!("  Quality monitored: {}", quality_monitored);
                            }
                            if let Some(stale) = summary.get("stale_connections").and_then(|v| v.as_u64()) {
                                if stale > 0 {
                                    println!("  ⚠ Stale connections: {}", stale);
                                }
                            }
                            if let Some(exhausted) = summary.get("reconnection_exhausted").and_then(|v| v.as_u64()) {
                                if exhausted > 0 {
                                    println!("  ⚠ Reconnection exhausted: {}", exhausted);
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

/// Helper to print sensor diagnostics in human-readable format
fn print_sensor_diagnostics(
    diag: &serde_json::Value,
    include_health: bool,
    include_state: bool,
    include_backoff: bool,
) {
    let device_id = diag.get("device_id").and_then(|v| v.as_str()).unwrap_or("?");
    let name = diag.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let sensor_type = diag.get("sensor_type").and_then(|v| v.as_str()).unwrap_or("?");
    let protocol = diag.get("protocol").and_then(|v| v.as_str()).unwrap_or("?");
    let connection_status = diag.get("connection_status").and_then(|v| v.as_str()).unwrap_or("?");

    // Header
    println!("┌─ {} ({}) ─────────────────────", name, sensor_type);
    println!("│  ID: {}", device_id);
    println!("│  Protocol: {}", protocol);
    println!("│  Status: {}", connection_status);

    // Health section
    if include_health {
        if let Some(health) = diag.get("health") {
            println!("│");
            println!("│  Health Monitoring:");

            let status = health.get("status").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let status_icon = match status {
                "Healthy" => "✓",
                "Degraded" => "⚡",
                "Stale" => "⚠",
                _ => "?",
            };
            println!("│    Status: {} {}", status_icon, status);

            if let Some(rate) = health.get("data_rate").and_then(|v| v.as_f64()) {
                println!("│    Data Rate: {:.2} pkt/s", rate);
            }
            if let Some(since_data) = health.get("time_since_last_data_secs").and_then(|v| v.as_f64()) {
                if since_data > 0.0 {
                    println!("│    Last Data: {:.1}s ago", since_data);
                }
            }
            if let Some(uptime) = health.get("uptime_secs").and_then(|v| v.as_u64()) {
                let mins = uptime / 60;
                let secs = uptime % 60;
                if mins > 0 {
                    println!("│    Uptime: {}m {}s", mins, secs);
                } else {
                    println!("│    Uptime: {}s", secs);
                }
            }
            if let Some(streak) = health.get("healthy_streak").and_then(|v| v.as_u64()) {
                if streak > 0 {
                    println!("│    Healthy Streak: {} checks", streak);
                }
            }
            if let Some(unhealthy_streak) = health.get("unhealthy_streak").and_then(|v| v.as_u64()) {
                if unhealthy_streak > 0 {
                    println!("│    Unhealthy Streak: {} checks", unhealthy_streak);
                }
            }
        } else {
            println!("│");
            println!("│  Health Monitoring: Not active");
        }
    }

    // Connection state section
    if include_state {
        if let Some(state) = diag.get("connection_state") {
            println!("│");
            println!("│  Connection State Machine:");

            let current_state = state.get("current_state").and_then(|v| v.as_str()).unwrap_or("?");
            println!("│    Current: {}", current_state);

            if let Some(in_state_secs) = state.get("time_in_state_secs").and_then(|v| v.as_f64()) {
                println!("│    Time in State: {:.1}s", in_state_secs);
            }
            if let Some(reconnect_attempts) = state.get("reconnection_attempts").and_then(|v| v.as_u64()) {
                if reconnect_attempts > 0 {
                    println!("│    Reconnection Attempts: {}", reconnect_attempts);
                }
            }
            if let Some(exhausted) = state.get("is_exhausted").and_then(|v| v.as_bool()) {
                if exhausted {
                    println!("│    ⚠ Reconnection Exhausted!");
                }
            }

            // State stats
            if let Some(stats) = state.get("stats") {
                if let Some(connects) = stats.get("total_connects").and_then(|v| v.as_u64()) {
                    if connects > 0 {
                        println!("│    Total Connects: {}", connects);
                    }
                }
                if let Some(disconnects) = stats.get("total_disconnects").and_then(|v| v.as_u64()) {
                    if disconnects > 0 {
                        println!("│    Total Disconnects: {}", disconnects);
                    }
                }
                if let Some(reconnects) = stats.get("total_reconnections").and_then(|v| v.as_u64()) {
                    if reconnects > 0 {
                        println!("│    Total Reconnections: {}", reconnects);
                    }
                }
            }
        } else {
            println!("│");
            println!("│  Connection State: Not tracked");
        }
    }

    // Backoff section
    if include_backoff {
        if let Some(backoff) = diag.get("reconnection_backoff") {
            println!("│");
            println!("│  Reconnection Backoff:");

            let attempt = backoff.get("current_attempt").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("│    Current Attempt: {}", attempt);

            if let Some(next_delay) = backoff.get("next_delay_secs").and_then(|v| v.as_f64()) {
                println!("│    Next Delay: {:.1}s", next_delay);
            }
            if let Some(remaining) = backoff.get("remaining_attempts") {
                if remaining.is_null() {
                    println!("│    Remaining: unlimited");
                } else if let Some(count) = remaining.as_u64() {
                    println!("│    Remaining: {}", count);
                }
            }
            if let Some(exhausted) = backoff.get("is_exhausted").and_then(|v| v.as_bool()) {
                if exhausted {
                    println!("│    ⚠ Max Attempts Exceeded!");
                }
            }

            // Show delay sequence preview
            if let Some(delays) = backoff.get("delay_sequence").and_then(|v| v.as_array()) {
                let delay_strs: Vec<String> = delays
                    .iter()
                    .take(6)
                    .filter_map(|v| v.as_f64())
                    .map(|d| format!("{:.0}s", d))
                    .collect();
                if !delay_strs.is_empty() {
                    println!("│    Delay Pattern: {}", delay_strs.join(" → "));
                }
            }
        } else {
            println!("│");
            println!("│  Reconnection Backoff: Not active");
        }
    }

    println!("└──────────────────────────────────────────");
}
