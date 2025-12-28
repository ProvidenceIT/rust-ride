# Research: Headless/CLI Mode

**Feature**: 009-headless-cli-mode
**Date**: 2025-12-28
**Status**: Complete

## Research Topics

### 1. IPC Mechanism for Daemon-CLI Communication

**Decision**: Unix domain sockets with JSON-framed messages

**Rationale**:
- Unix sockets are the standard IPC mechanism for daemon-client communication on Linux
- Native support in tokio via `tokio::net::UnixStream` and `UnixListener`
- No additional dependencies required beyond tokio (already in use)
- Provides bidirectional, reliable, ordered communication
- Socket file location at `/run/rustride/rustride.sock` or `~/.local/run/rustride.sock`
- JSON framing allows easy debugging (human-readable) and extensibility

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| D-Bus | Heavyweight, requires libdbus, complex API for simple request-response |
| Named pipes (FIFO) | Unidirectional, would need two pipes, no connection semantics |
| TCP localhost | Unnecessary network overhead, port management issues |
| Shared memory | Complex synchronization, overkill for command frequency |
| gRPC | Heavy dependency (tonic + prost), protobuf compilation step |

### 2. CLI Framework

**Decision**: clap v4 with derive macros

**Rationale**:
- Industry standard for Rust CLI applications
- Derive macros provide type-safe argument parsing with minimal boilerplate
- Built-in help generation, shell completions, and error messages
- Subcommand support for `rustride workout start`, `rustride sensors list`, etc.
- Already widely used in the Rust ecosystem, well-documented

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| structopt | Deprecated, merged into clap 3+ |
| argh | Simpler but less feature-rich, smaller ecosystem |
| pico-args | Too minimal, no subcommand support |

### 3. Daemon Process Management

**Decision**: Use `daemonize` crate for double-fork daemonization, with systemd integration

**Rationale**:
- `daemonize` crate provides standard Unix double-fork pattern
- Handles PID file creation, working directory change, stdio redirection
- Systemd unit file for production deployments (Type=notify or Type=simple)
- PID file at `/run/rustride/rustride.pid` or `~/.local/run/rustride.pid`
- Support both `rustride --headless` (foreground) and `rustride daemon start` (background)

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| nix crate manual fork | More control but more code, `daemonize` handles edge cases |
| systemd-only | Requires systemd, less portable to minimal Linux systems |
| No daemonization | Poor UX, requires manual backgrounding with `&` |

### 4. Signal Handling

**Decision**: Use `signal-hook` crate with tokio integration

**Rationale**:
- `signal-hook` is the standard Rust crate for Unix signal handling
- `signal-hook-tokio` provides async signal streams for tokio runtime
- Handle SIGTERM and SIGINT for graceful shutdown
- Handle SIGHUP for configuration reload (optional)
- Integrates cleanly with tokio's event loop

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| tokio::signal | Only basic signal support, less flexible |
| ctrlc crate | Only handles Ctrl+C, not full signal set |
| nix crate signals | Lower level, more boilerplate |

### 5. JSON Message Framing

**Decision**: Length-prefixed JSON messages

**Rationale**:
- Simple protocol: 4-byte big-endian length prefix + JSON payload
- Allows streaming multiple messages over single connection
- Easy to debug (JSON is human-readable)
- serde_json already in dependencies
- Extensible: new fields can be added without breaking protocol

**Message Format**:
```
[4 bytes: message length (big-endian u32)]
[N bytes: JSON payload]
```

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| Newline-delimited JSON | Risk of embedded newlines in data |
| MessagePack | Less human-readable, additional dependency |
| Protocol Buffers | Schema compilation step, heavier |
| Raw JSON with delimiter | Delimiter escaping issues |

### 6. Auto-Save and Crash Recovery

**Decision**: SQLite WAL mode with 30-second checkpoint intervals

**Rationale**:
- SQLite WAL (Write-Ahead Logging) mode provides crash safety
- Existing rusqlite dependency supports WAL mode
- 30-second periodic flush aligns with spec requirement
- On startup, check for incomplete rides (no end_time) and offer recovery
- Recovery state stored in `ride_samples` table with `is_recovered` flag

**Implementation**:
1. Enable WAL mode: `PRAGMA journal_mode=WAL;`
2. Periodic checkpoint: `PRAGMA wal_checkpoint(PASSIVE);` every 30 seconds
3. On startup: `SELECT * FROM rides WHERE end_time IS NULL` to find incomplete rides
4. CLI prompt: `rustride ride recover` or auto-recover on daemon start

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| File-based auto-save | Duplicate data, sync complexity |
| In-memory with periodic dump | Risk of data loss between dumps |
| Append-only log | Additional file management |

### 7. ARM64 / Raspberry Pi Considerations

**Decision**: Cross-compile with `cross` tool, software BLE via BlueZ

**Rationale**:
- `cross` tool provides Docker-based cross-compilation for aarch64-unknown-linux-gnu
- btleplug uses BlueZ D-Bus API on Linux (no native code changes needed)
- Existing dependencies (rusqlite bundled, tokio) work on ARM64
- Test on Raspberry Pi OS (Debian-based) as reference platform
- Release builds with `--release` for performance on limited hardware

**Build Command**:
```bash
cross build --target aarch64-unknown-linux-gnu --release
```

**Alternatives Considered**:
| Alternative | Why Rejected |
|-------------|--------------|
| Native ARM64 build | Requires Pi hardware, slower builds |
| QEMU emulation | Slow, complex setup |
| Different BLE stack | btleplug already works, no need to change |

## Dependency Summary

New dependencies to add to `Cargo.toml`:

```toml
# CLI Framework
clap = { version = "4", features = ["derive"] }

# Daemonization (Linux only)
[target.'cfg(target_os = "linux")'.dependencies]
daemonize = "0.5"

# Signal Handling
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }
```

## Resolved Clarifications

All technical clarifications from Technical Context have been resolved:
- IPC: Unix domain sockets with JSON framing
- CLI: clap v4 with derive
- Daemonization: `daemonize` crate
- Signals: `signal-hook` with tokio
- Crash recovery: SQLite WAL mode

No NEEDS CLARIFICATION items remain.
