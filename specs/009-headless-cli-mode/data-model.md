# Data Model: Headless/CLI Mode

**Feature**: 009-headless-cli-mode
**Date**: 2025-12-28

## Entities

### DaemonState

Represents the current state of the running daemon.

| Field | Type | Description |
|-------|------|-------------|
| `pid` | `u32` | Process ID of the daemon |
| `started_at` | `DateTime<Utc>` | When the daemon started |
| `uptime_seconds` | `u64` | Seconds since daemon start |
| `status` | `DaemonStatus` | Current daemon status |
| `active_session` | `Option<SessionInfo>` | Current ride/workout if active |
| `connected_sensors` | `Vec<SensorInfo>` | List of connected sensors |
| `ble_adapter_available` | `bool` | Whether BLE adapter is present |
| `config_path` | `PathBuf` | Path to active config file |
| `socket_path` | `PathBuf` | Path to IPC socket |
| `log_path` | `PathBuf` | Path to log file |

**State Transitions**:
```
Starting → Running → ShuttingDown → Stopped
              ↓
          Degraded (no BLE adapter)
```

### DaemonStatus

Enum representing daemon lifecycle states.

| Variant | Description |
|---------|-------------|
| `Starting` | Daemon is initializing |
| `Running` | Daemon is operational |
| `Degraded` | Running but with limited functionality (e.g., no BLE) |
| `ShuttingDown` | Graceful shutdown in progress |
| `Stopped` | Daemon has stopped |

### SessionInfo

Information about an active ride or workout session.

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | `Uuid` | Unique session identifier |
| `session_type` | `SessionType` | FreeRide or Workout |
| `started_at` | `DateTime<Utc>` | Session start time |
| `elapsed_seconds` | `u64` | Seconds since session start |
| `workout_info` | `Option<WorkoutInfo>` | Workout details if applicable |
| `current_metrics` | `LiveMetrics` | Current sensor readings |
| `is_paused` | `bool` | Whether session is paused |

### SessionType

Enum for session types.

| Variant | Description |
|---------|-------------|
| `FreeRide` | Unstructured riding |
| `Workout { path: PathBuf }` | Structured workout from file |

### WorkoutInfo

Details about a running workout.

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Workout name |
| `file_path` | `PathBuf` | Path to workout file |
| `total_duration_seconds` | `u64` | Total workout duration |
| `current_interval_index` | `usize` | Current interval (0-based) |
| `total_intervals` | `usize` | Total number of intervals |
| `current_interval_name` | `String` | Name of current interval |
| `interval_elapsed_seconds` | `u64` | Seconds into current interval |
| `interval_remaining_seconds` | `u64` | Seconds remaining in interval |
| `target_power_watts` | `u16` | Current ERG target |
| `target_power_percent_ftp` | `f32` | Target as % of FTP |

### LiveMetrics

Real-time sensor readings.

| Field | Type | Description |
|-------|------|-------------|
| `power_watts` | `Option<u16>` | Current power (W) |
| `heart_rate_bpm` | `Option<u8>` | Current heart rate (bpm) |
| `cadence_rpm` | `Option<u8>` | Current cadence (rpm) |
| `speed_kmh` | `Option<f32>` | Current speed (km/h) |
| `distance_km` | `f32` | Total distance (km) |
| `calories` | `u32` | Estimated calories burned |
| `normalized_power` | `Option<u16>` | Rolling NP (W) |
| `average_power` | `Option<u16>` | Average power (W) |
| `average_heart_rate` | `Option<u8>` | Average HR (bpm) |

### SensorInfo

Information about a connected or discovered sensor.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique sensor identifier (BLE address) |
| `name` | `String` | Sensor display name |
| `sensor_type` | `SensorType` | Type of sensor |
| `connection_status` | `ConnectionStatus` | Current connection state |
| `signal_strength_dbm` | `Option<i8>` | RSSI signal strength |
| `battery_percent` | `Option<u8>` | Battery level if available |
| `last_seen` | `DateTime<Utc>` | Last data received |

### SensorType

Enum for sensor categories.

| Variant | Description |
|---------|-------------|
| `SmartTrainer` | FTMS-compatible smart trainer |
| `PowerMeter` | Cycling power meter |
| `HeartRateMonitor` | Heart rate strap/sensor |
| `CadenceSensor` | Cadence-only sensor |
| `SpeedSensor` | Speed-only sensor |
| `SpeedCadenceSensor` | Combined speed/cadence |

### ConnectionStatus

Enum for sensor connection states.

| Variant | Description |
|---------|-------------|
| `Discovered` | Sensor found but not connected |
| `Connecting` | Connection in progress |
| `Connected` | Actively connected |
| `Reconnecting` | Lost connection, attempting reconnect |
| `Disconnected` | Intentionally disconnected |
| `Failed { reason: String }` | Connection failed |

### RecoveryInfo

Information about a recoverable interrupted session.

| Field | Type | Description |
|-------|------|-------------|
| `ride_id` | `i64` | Database ID of incomplete ride |
| `started_at` | `DateTime<Utc>` | When ride started |
| `last_sample_at` | `DateTime<Utc>` | Last recorded sample time |
| `duration_seconds` | `u64` | Duration before interruption |
| `sample_count` | `usize` | Number of samples recorded |
| `session_type` | `SessionType` | Was it free ride or workout |

## Relationships

```
DaemonState
    ├── has one: active SessionInfo (optional)
    │       ├── has one: WorkoutInfo (optional, if workout)
    │       └── has one: LiveMetrics
    ├── has many: SensorInfo (connected sensors)
    └── may have: RecoveryInfo (on startup if crash detected)

SensorInfo
    └── provides data to: LiveMetrics
```

## Validation Rules

1. **DaemonState**:
   - Only one daemon instance per host (enforced via PID file and socket)
   - `active_session` must be `None` when `status` is `Starting` or `ShuttingDown`

2. **SessionInfo**:
   - `session_id` must be unique (UUID v4)
   - `elapsed_seconds` must increase monotonically
   - If `session_type` is `Workout`, `workout_info` must be `Some`

3. **SensorInfo**:
   - `id` must be valid BLE address format
   - `battery_percent` range: 0-100

4. **WorkoutInfo**:
   - `current_interval_index` must be < `total_intervals`
   - `target_power_watts` must be > 0

## Persistence

### Existing Tables (reused)

- `rides`: Stores completed and in-progress rides
- `ride_samples`: 1-second sample data
- `sensors`: Known sensors with preferences
- `workouts`: Parsed workout metadata

### New Fields

Add to `rides` table:
```sql
ALTER TABLE rides ADD COLUMN is_headless BOOLEAN DEFAULT FALSE;
ALTER TABLE rides ADD COLUMN recovery_attempted BOOLEAN DEFAULT FALSE;
```

### Runtime State (not persisted)

- `DaemonState`: In-memory, reconstructed on daemon start
- `LiveMetrics`: In-memory, updated every second
- `ConnectionStatus`: In-memory, reflects current BLE state
