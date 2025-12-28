# Implementation Plan: Headless/CLI Mode

**Branch**: `009-headless-cli-mode` | **Date**: 2025-12-28 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/009-headless-cli-mode/spec.md`

## Summary

Enable RustRide to run as a headless daemon on Linux systems (including Raspberry Pi), controlled entirely via CLI commands. This allows users to deploy dedicated training stations without displays, automate workouts via scripts, and integrate with external systems. The daemon maintains BLE sensor connections, executes workouts, and records rides, communicating with CLI clients via Unix domain sockets.

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**:
- clap (CLI argument parsing)
- tokio (async runtime, already in use)
- Unix domain sockets via tokio (IPC)
- serde_json (IPC message serialization)
- daemonize or nix (process daemonization)
- signal-hook (Unix signal handling)

**Storage**: SQLite via rusqlite (existing), TOML config (existing)
**Testing**: cargo test, integration tests with mock daemon
**Target Platform**: Linux x86_64 and ARM64 (Raspberry Pi 4/5)
**Project Type**: Single project (extends existing crate with `--headless` mode)
**Performance Goals**: CLI commands respond <1s for status, <5s for operations
**Constraints**: <100MB memory for daemon, 30-second auto-save intervals
**Scale/Scope**: Single daemon instance per host, 8+ hour continuous operation

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Note**: Project constitution is a template (not yet ratified). Proceeding with standard Rust best practices:
- [x] Single project extension (no new crates required)
- [x] Uses existing dependencies where possible (tokio, serde, rusqlite)
- [x] Follows existing code patterns (crossbeam channels, Arc<Mutex<T>>)
- [x] Test coverage for new functionality

## Project Structure

### Documentation (this feature)

```text
specs/009-headless-cli-mode/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── ipc-protocol.md  # IPC message contracts
└── tasks.md             # Phase 2 output (from /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Updated: add --headless flag dispatch
├── daemon/              # NEW: Daemon process management
│   ├── mod.rs           # Daemon module exports
│   ├── server.rs        # IPC server (Unix socket listener)
│   ├── handler.rs       # Command handler (routes IPC messages)
│   ├── state.rs         # Daemon state machine
│   └── signals.rs       # Unix signal handling (SIGTERM, SIGINT)
├── cli/                 # NEW: CLI client commands
│   ├── mod.rs           # CLI module exports
│   ├── main.rs          # CLI entry point (separate binary)
│   ├── commands/        # Command implementations
│   │   ├── daemon.rs    # start, stop, status
│   │   ├── ride.rs      # start, stop, export
│   │   ├── workout.rs   # start, pause, resume, skip, stop
│   │   └── sensors.rs   # list, connect, disconnect, status
│   └── client.rs        # IPC client (connects to daemon socket)
├── ipc/                 # NEW: IPC protocol definitions
│   ├── mod.rs           # IPC module exports
│   ├── messages.rs      # Request/Response message types
│   └── protocol.rs      # Serialization, framing
├── sensors/             # EXISTING: BLE sensor management (reused)
├── workouts/            # EXISTING: Workout execution (reused)
├── recording/           # EXISTING: Ride recording (reused)
├── metrics/             # EXISTING: Metrics calculations (reused)
└── storage/             # EXISTING: SQLite, config (reused)

tests/
├── integration/
│   └── daemon/          # NEW: Daemon integration tests
│       ├── startup.rs   # Daemon start/stop tests
│       ├── workout.rs   # Workout execution via CLI
│       └── recovery.rs  # Crash recovery tests
└── unit/
    └── cli/             # NEW: CLI command parsing tests
```

**Structure Decision**: Extends existing crate with new `daemon/`, `cli/`, and `ipc/` modules. The CLI will be a separate binary (`rustride-cli`) that communicates with the daemon via Unix sockets. This avoids duplicating the core logic and reuses existing sensor, workout, recording, and metrics modules.

## Complexity Tracking

No constitution violations to justify. The implementation follows the existing project patterns.

