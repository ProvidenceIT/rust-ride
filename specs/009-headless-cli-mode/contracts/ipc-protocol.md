# IPC Protocol Contract: Headless/CLI Mode

**Feature**: 009-headless-cli-mode
**Date**: 2025-12-28
**Protocol Version**: 1.0

## Overview

The CLI client communicates with the daemon via Unix domain sockets using a JSON-based request-response protocol.

**Socket Location**:
- System-wide: `/run/rustride/rustride.sock` (requires root)
- User-level: `$XDG_RUNTIME_DIR/rustride/rustride.sock` or `~/.local/run/rustride/rustride.sock`

## Message Framing

All messages use length-prefix framing:

```
┌─────────────────┬─────────────────────────────────┐
│ Length (4 bytes)│ JSON Payload (N bytes)          │
│ Big-endian u32  │ UTF-8 encoded                   │
└─────────────────┴─────────────────────────────────┘
```

## Request Format

All requests follow this structure:

```json
{
  "id": "uuid-v4",
  "command": "CommandName",
  "params": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | UUID v4 for request correlation |
| `command` | `string` | Command name (see Commands below) |
| `params` | `object` | Command-specific parameters |

## Response Format

All responses follow this structure:

```json
{
  "id": "uuid-v4",
  "success": true,
  "result": { ... },
  "error": null
}
```

Or on error:

```json
{
  "id": "uuid-v4",
  "success": false,
  "result": null,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable message"
  }
}
```

## Commands

### Daemon Commands

#### `DaemonStatus`

Get current daemon status.

**Request**:
```json
{
  "id": "...",
  "command": "DaemonStatus",
  "params": {}
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "pid": 12345,
    "status": "Running",
    "uptime_seconds": 3600,
    "started_at": "2025-12-28T10:00:00Z",
    "ble_adapter_available": true,
    "active_session": null,
    "connected_sensors": [...],
    "version": "0.2.0"
  }
}
```

#### `DaemonShutdown`

Request graceful daemon shutdown.

**Request**:
```json
{
  "id": "...",
  "command": "DaemonShutdown",
  "params": {
    "force": false
  }
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "message": "Shutdown initiated"
  }
}
```

### Sensor Commands

#### `SensorsList`

List all discovered and connected sensors.

**Request**:
```json
{
  "id": "...",
  "command": "SensorsList",
  "params": {
    "scan": true,
    "scan_duration_seconds": 5
  }
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "sensors": [
      {
        "id": "AA:BB:CC:DD:EE:FF",
        "name": "Wahoo KICKR",
        "sensor_type": "SmartTrainer",
        "connection_status": "Connected",
        "signal_strength_dbm": -65,
        "battery_percent": null
      },
      {
        "id": "11:22:33:44:55:66",
        "name": "Garmin HRM",
        "sensor_type": "HeartRateMonitor",
        "connection_status": "Discovered",
        "signal_strength_dbm": -70,
        "battery_percent": 85
      }
    ]
  }
}
```

#### `SensorConnect`

Connect to a specific sensor.

**Request**:
```json
{
  "id": "...",
  "command": "SensorConnect",
  "params": {
    "sensor_id": "AA:BB:CC:DD:EE:FF"
  }
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "sensor_id": "AA:BB:CC:DD:EE:FF",
    "connection_status": "Connected"
  }
}
```

#### `SensorDisconnect`

Disconnect from a sensor.

**Request**:
```json
{
  "id": "...",
  "command": "SensorDisconnect",
  "params": {
    "sensor_id": "AA:BB:CC:DD:EE:FF"
  }
}
```

### Ride Commands

#### `RideStart`

Start a free ride session.

**Request**:
```json
{
  "id": "...",
  "command": "RideStart",
  "params": {}
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "session_id": "uuid-v4",
    "started_at": "2025-12-28T10:00:00Z"
  }
}
```

#### `RideStop`

Stop the current ride.

**Request**:
```json
{
  "id": "...",
  "command": "RideStop",
  "params": {
    "discard": false
  }
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "ride_id": 123,
    "duration_seconds": 3600,
    "distance_km": 25.5,
    "average_power": 180
  }
}
```

#### `RideExport`

Export a completed ride.

**Request**:
```json
{
  "id": "...",
  "command": "RideExport",
  "params": {
    "ride_id": 123,
    "format": "fit",
    "output_path": "/path/to/ride.fit"
  }
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "path": "/path/to/ride.fit",
    "size_bytes": 45678
  }
}
```

#### `RideRecover`

Recover an interrupted ride.

**Request**:
```json
{
  "id": "...",
  "command": "RideRecover",
  "params": {
    "ride_id": 123,
    "action": "finalize"
  }
}
```

`action` can be: `"finalize"` (save as complete) or `"discard"` (delete)

### Workout Commands

#### `WorkoutStart`

Start a structured workout.

**Request**:
```json
{
  "id": "...",
  "command": "WorkoutStart",
  "params": {
    "workout_path": "/path/to/workout.zwo"
  }
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "session_id": "uuid-v4",
    "workout_name": "Sweet Spot 2x20",
    "total_duration_seconds": 3600,
    "total_intervals": 8
  }
}
```

#### `WorkoutPause`

Pause the current workout.

**Request**:
```json
{
  "id": "...",
  "command": "WorkoutPause",
  "params": {}
}
```

#### `WorkoutResume`

Resume a paused workout.

**Request**:
```json
{
  "id": "...",
  "command": "WorkoutResume",
  "params": {}
}
```

#### `WorkoutSkip`

Skip to the next interval.

**Request**:
```json
{
  "id": "...",
  "command": "WorkoutSkip",
  "params": {}
}
```

#### `WorkoutStop`

End the workout (same as RideStop but for workout context).

**Request**:
```json
{
  "id": "...",
  "command": "WorkoutStop",
  "params": {
    "discard": false
  }
}
```

### Status Commands

#### `StatusLive`

Get current live metrics (snapshot).

**Request**:
```json
{
  "id": "...",
  "command": "StatusLive",
  "params": {}
}
```

**Response**:
```json
{
  "id": "...",
  "success": true,
  "result": {
    "session_active": true,
    "session_type": "Workout",
    "elapsed_seconds": 600,
    "is_paused": false,
    "metrics": {
      "power_watts": 220,
      "heart_rate_bpm": 145,
      "cadence_rpm": 90,
      "speed_kmh": 32.5,
      "distance_km": 5.4
    },
    "workout": {
      "name": "Sweet Spot 2x20",
      "current_interval": 3,
      "total_intervals": 8,
      "interval_name": "Work",
      "target_power_watts": 230,
      "interval_remaining_seconds": 540
    }
  }
}
```

## Error Codes

| Code | Description |
|------|-------------|
| `NO_SESSION` | No active ride/workout session |
| `SESSION_ACTIVE` | Cannot start, session already in progress |
| `SENSOR_NOT_FOUND` | Sensor ID not found |
| `SENSOR_CONNECTION_FAILED` | Failed to connect to sensor |
| `WORKOUT_NOT_FOUND` | Workout file not found |
| `WORKOUT_PARSE_ERROR` | Failed to parse workout file |
| `EXPORT_FAILED` | Failed to export ride |
| `RIDE_NOT_FOUND` | Ride ID not found |
| `NO_BLE_ADAPTER` | BLE adapter not available |
| `PERMISSION_DENIED` | Operation not permitted |
| `INTERNAL_ERROR` | Unexpected internal error |

## Exit Codes

CLI returns these exit codes for scripting:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Daemon not running |
| 4 | Connection failed |
| 5 | Command rejected (e.g., session active) |
| 6 | Resource not found |
| 7 | Operation timeout |
