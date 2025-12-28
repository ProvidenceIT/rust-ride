# Feature Specification: Headless/CLI Mode

**Feature Branch**: `009-headless-cli-mode`
**Created**: 2025-12-28
**Status**: Draft
**Input**: User description: "Headless/CLI Mode: Daemon without GUI. Scripting support. Raspberry Pi/server deployment."

## Clarifications

### Session 2025-12-28

- Q: Who can issue CLI commands to the daemon? → A: Any local user can control daemon (rely on SSH/system login for access control)
- Q: Which platforms does headless mode support? → A: Linux-only (including ARM64/Raspberry Pi)
- Q: How should the daemon handle crash recovery? → A: Auto-save ride data every 30 seconds; recover partial ride on next daemon start

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run Daemon on Headless Server (Priority: P1)

As a user with a dedicated training station (Raspberry Pi or home server connected to my smart trainer), I want to run RustRide as a background daemon without any graphical interface so that I can connect sensors, execute workouts, and record rides without needing a monitor attached to the device.

**Why this priority**: This is the core value proposition - enabling RustRide to run on headless hardware. Without this, the feature has no foundation. Many users want dedicated, always-on training setups using low-power devices like Raspberry Pi.

**Independent Test**: Can be fully tested by starting the daemon on a headless system, connecting a smart trainer via BLE, starting a free ride, and verifying metrics are recorded - all without any display attached.

**Acceptance Scenarios**:

1. **Given** a headless Linux server with BLE capability, **When** a user starts the RustRide daemon via command line, **Then** the application runs as a background process without requiring a display or window manager.

2. **Given** the daemon is running, **When** a user sends a termination signal (SIGTERM), **Then** the daemon gracefully shuts down, saving any in-progress ride data before exiting.

3. **Given** the daemon is running, **When** the system reboots, **Then** the daemon automatically restarts if configured for auto-start.

---

### User Story 2 - Execute Workouts via Command Line (Priority: P2)

As a user who wants to automate my training, I want to start, control, and monitor workouts using command-line arguments and commands so that I can script my training sessions and integrate RustRide with other tools and schedulers.

**Why this priority**: Once the daemon runs, users need a way to interact with it. CLI commands provide the primary interface for headless operation and enable automation.

**Independent Test**: Can be fully tested by using CLI commands to start a workout file, verify it's running, skip intervals, pause/resume, and end the session - all via terminal commands.

**Acceptance Scenarios**:

1. **Given** the daemon is running with a connected trainer, **When** a user executes `rustride workout start myworkout.zwo`, **Then** the specified workout begins execution with ERG mode controlling the trainer.

2. **Given** a workout is in progress, **When** a user executes `rustride workout pause`, **Then** the workout pauses and the trainer target power drops to recovery level.

3. **Given** a workout is in progress, **When** a user executes `rustride status`, **Then** the current workout state, elapsed time, current interval, and live metrics are displayed.

4. **Given** no workout is running, **When** a user executes `rustride ride start`, **Then** a free ride session begins recording metrics from connected sensors.

---

### User Story 3 - Script Automated Training Sessions (Priority: P3)

As a power user or developer, I want to write scripts that automate complex training scenarios so that I can create scheduled training sessions, implement custom training logic, and integrate with external systems.

**Why this priority**: Scripting extends the CLI functionality to enable advanced automation. This unlocks integration possibilities and custom workflows without modifying the core application.

**Independent Test**: Can be fully tested by writing a shell script that schedules a workout at a specific time, monitors completion, and exports the ride file to a network location.

**Acceptance Scenarios**:

1. **Given** a shell script that calls RustRide CLI commands, **When** the script executes `rustride workout start morning.zwo --wait`, **Then** the command blocks until the workout completes (or is cancelled) and returns an appropriate exit code.

2. **Given** a workout has completed, **When** a script executes `rustride ride export --format fit --output /path/to/ride.fit`, **Then** the most recent ride is exported to the specified path in FIT format.

3. **Given** the daemon is running, **When** a script executes `rustride sensors list --json`, **Then** a JSON array of connected sensors with their current readings is output to stdout.

---

### User Story 4 - Monitor System via Status Commands (Priority: P4)

As an administrator of a multi-station training facility, I want to query the status of RustRide instances remotely so that I can monitor all stations from a central location and ensure everything is functioning correctly.

**Why this priority**: Status monitoring is essential for maintaining headless systems but depends on the core daemon and CLI infrastructure being in place first.

**Independent Test**: Can be fully tested by running status commands and verifying they return accurate, machine-parseable information about the current system state.

**Acceptance Scenarios**:

1. **Given** the daemon is running, **When** a user executes `rustride status --json`, **Then** a JSON object containing daemon status, connected sensors, active ride/workout info, and system health is output.

2. **Given** sensors are connected, **When** a user executes `rustride sensors status`, **Then** a list of all connected sensors with their type, name, signal strength, and battery level is displayed.

3. **Given** the daemon is not running, **When** a user executes any command, **Then** a clear error message indicates the daemon is not running with instructions on how to start it.

---

### User Story 5 - Configure via Configuration Files (Priority: P5)

As a user deploying RustRide on a headless system, I want to configure all settings via configuration files so that I can set up the system once and have it work consistently without manual intervention.

**Why this priority**: Configuration file support is important for deployments but is a lower priority than core daemon and CLI functionality.

**Independent Test**: Can be fully tested by creating a configuration file with sensor preferences and workout defaults, then verifying the daemon uses these settings on startup.

**Acceptance Scenarios**:

1. **Given** a configuration file specifying preferred sensors by ID, **When** the daemon starts, **Then** it automatically connects to those sensors without user intervention.

2. **Given** a configuration file with default FTP and HR zones, **When** a workout is started, **Then** the zones from the configuration are used for intensity calculations.

3. **Given** an invalid configuration file, **When** the daemon starts, **Then** it logs specific validation errors and uses safe defaults for invalid values.

---

### Edge Cases

- What happens when the daemon loses BLE connection to all sensors mid-workout?
  - The daemon should continue recording with gaps in data, attempt automatic reconnection, and log connection events.

- How does the system handle simultaneous commands (e.g., two scripts trying to start workouts)?
  - Commands should be queued or rejected with a clear error if conflicting.

- What happens if the system runs out of disk space during ride recording?
  - The daemon should detect low disk conditions, warn via logs/status, and gracefully handle write failures.

- How does the daemon behave when started with no BLE adapter present?
  - It should start in a degraded mode, clearly indicating no sensors can be connected, and allow status queries.

- What happens if a long-running workout script is interrupted (Ctrl+C, kill)?
  - In-progress ride data should be preserved, and partial workout data should be recoverable.

- What happens if the daemon crashes unexpectedly (power loss, OOM kill, segfault)?
  - Ride data auto-saved every 30 seconds is preserved; on next daemon start, user is prompted to recover the partial ride (maximum 30 seconds of data loss).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST run as a background daemon process without requiring any graphical display, window manager, or X11/Wayland session.

- **FR-002**: System MUST provide a command-line interface (CLI) for all user interactions including starting/stopping rides, executing workouts, querying status, and managing sensors.

- **FR-003**: System MUST support starting workouts from workout files (.zwo, .mrc formats) via CLI command.

- **FR-004**: System MUST support free ride mode initiation and control via CLI commands.

- **FR-005**: System MUST provide real-time status output including current power, heart rate, cadence, workout progress, and elapsed time.

- **FR-006**: System MUST support output in both human-readable and machine-parseable (JSON) formats for all status and query commands.

- **FR-007**: System MUST maintain BLE sensor connections and manage automatic reconnection when running in daemon mode.

- **FR-008**: System MUST support graceful shutdown with ride data preservation when receiving termination signals (SIGTERM, SIGINT).

- **FR-009**: System MUST write logs to a configurable location for debugging and monitoring purposes.

- **FR-010**: System MUST support configuration via TOML configuration files for all settings that would otherwise require GUI interaction.

- **FR-011**: System MUST provide commands to list, connect, and disconnect sensors by name or ID.

- **FR-012**: System MUST provide commands to export completed rides in standard formats (FIT, TCX, CSV).

- **FR-013**: System MUST return meaningful exit codes from CLI commands to enable scripting and automation (0 for success, non-zero for various error conditions).

- **FR-014**: System MUST provide a `--wait` or blocking mode for workout commands to enable scripts to wait for workout completion.

- **FR-015**: System MUST support running on ARM64 architecture (Raspberry Pi 4/5) with standard Linux distributions.

- **FR-016**: Headless/daemon mode is Linux-only; Windows and macOS are explicitly out of scope for this feature.

- **FR-017**: System MUST auto-save ride data every 30 seconds during active sessions to enable crash recovery.

- **FR-018**: System MUST detect and recover partial ride data from previous daemon crashes on startup, prompting user to resume or finalize the interrupted ride.

### Key Entities

- **Daemon Process**: The background service that maintains sensor connections, executes workouts, and records ride data. Communicates with CLI commands via IPC.

- **CLI Command**: Individual commands that interact with the daemon to control operation, query status, or perform actions. Each command has arguments, options, and return codes.

- **Configuration**: TOML-based settings including sensor preferences, user profile (FTP, zones), logging configuration, and default behaviors.

- **Session**: An active ride or workout instance managed by the daemon, encompassing sensor data streams, workout state, and recording buffers.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can start and complete a full workout on a headless Raspberry Pi with no display connected, controlling the entire session via SSH terminal commands.

- **SC-002**: A scripted training session (workout start, monitoring, export) executes successfully with no manual intervention required after initial setup.

- **SC-003**: The daemon maintains stable operation for 8+ hours continuously, handling multiple workout sessions and sensor reconnections without memory leaks or crashes.

- **SC-004**: CLI commands respond within 1 second for status queries and 5 seconds for operations like sensor connection.

- **SC-005**: All recorded ride data from headless sessions matches the quality and completeness of GUI-recorded rides (same data points, export formats, and metrics calculations).

- **SC-006**: System administrators can monitor the status of headless RustRide instances remotely using standard command-line tools (SSH + CLI commands).

- **SC-007**: Configuration changes can be made by editing a single configuration file without requiring application restarts for most settings.

## Assumptions

- BLE adapter is available and compatible with btleplug on the target headless system.
- Users have basic familiarity with command-line interfaces and SSH for remote access.
- Target headless systems run Linux (Raspberry Pi OS, Ubuntu, Debian, or similar).
- CLI access control relies on system-level security (SSH login, Unix permissions); any authenticated local user can issue daemon commands.
- The system has sufficient storage for ride recordings (minimum ~100MB free recommended).
- Network connectivity is available for remote access (SSH) but not required for core functionality.
- Systemd or similar init system is available for daemon management (auto-start configuration).

## Dependencies

- Depends on core RustRide functionality from 001-indoor-cycling-app (sensor management, workout execution, ride recording).
- Configuration system leverages existing TOML-based configuration infrastructure.
- Ride export functionality uses existing recording/export modules.
