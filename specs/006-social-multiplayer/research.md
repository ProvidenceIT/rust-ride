# Research: Social & Multiplayer

**Feature Branch**: `006-social-multiplayer`
**Date**: 2025-12-26

## mDNS Service Discovery

### Decision: Use `mdns-sd` crate

### Rationale
- Pure Rust implementation with minimal dependencies (aligns with project simplicity)
- Actively maintained (0.13.11 released July 2025)
- Supports both client (querier) and server (responder) roles
- Uses thread-based daemon with flume channels - compatible with both sync and async code
- No external system dependencies (unlike `zeroconf` which requires Bonjour SDK on Windows or Avahi on Linux)
- Works offline without platform-specific services

### Alternatives Considered

| Crate | Pros | Cons | Why Rejected |
|-------|------|------|--------------|
| `zeroconf` | Cross-platform wrapper, idiomatic API | Requires Bonjour SDK (Windows) or Avahi (Linux) | External dependencies conflict with self-hosted philosophy |
| `libmdns` | Pure Rust, responder only | No querier support, less actively maintained | Missing discovery (querier) functionality |
| `simple-mdns` | Sync and async versions | Less comprehensive than mdns-sd | mdns-sd has better documentation and more recent releases |
| `astro-dnssd` | Friendly API | Requires Bonjour/Avahi | Same external dependency issue as zeroconf |

### Implementation Notes
```toml
# Cargo.toml addition
mdns-sd = "0.13"
```

Service registration pattern:
- Service type: `_rustride._udp.local.`
- Service name: `{rider_display_name}`
- Port: Dynamic (application-assigned)
- TXT records: `version`, `session_id` (for group rides)

## UDP Synchronization Protocol

### Decision: Use tokio UDP sockets with custom lightweight protocol

### Rationale
- tokio already in project dependencies (consistent async runtime)
- Custom protocol provides exactly what's needed without overhead
- LAN-only use case doesn't require complex reliability (packets rarely lost)
- 10 concurrent participants is low enough that simple broadcast works
- No need for full game networking library (overkill for cycling metrics)

### Alternatives Considered

| Approach | Pros | Cons | Why Rejected |
|----------|------|------|--------------|
| `laminar` | Reliable UDP, sequencing, game-focused | Adds complexity, designed for action games | Overkill - cycling metrics don't need rollback/prediction |
| `GGRS` | Rollback networking | Designed for fighting games | Wrong use case - no rollback needed |
| Raw UDP | Maximum control | Need to build everything | Tokio UDP provides good abstractions |
| TCP | Reliable, ordered | Higher latency, connection overhead | Latency matters for real-time metrics display |

### Protocol Design

**Message Types** (serde + bincode for efficient binary serialization):
```rust
enum SocialMessage {
    // Discovery & Session
    SessionAnnounce { session_id: Uuid, host_name: String, ride_type: RideType },
    SessionJoin { session_id: Uuid, rider_id: Uuid },
    SessionLeave { session_id: Uuid, rider_id: Uuid },

    // Real-time Metrics (high frequency, unreliable OK)
    MetricUpdate { rider_id: Uuid, power: u16, cadence: u8, hr: u8, position: Vec3 },

    // Chat (reliable via ACK)
    ChatMessage { msg_id: Uuid, sender_id: Uuid, text: String },
    ChatAck { msg_id: Uuid },

    // Racing
    RaceCountdown { race_id: Uuid, seconds_remaining: u8 },
    RacePosition { race_id: Uuid, rider_id: Uuid, distance: f32, time_ms: u32 },
    RaceFinish { race_id: Uuid, rider_id: Uuid, final_time_ms: u32 },

    // Heartbeat
    Ping { timestamp_ms: u64 },
    Pong { timestamp_ms: u64 },
}
```

**Broadcast Strategy**:
- Metrics: Multicast to 239.255.42.42:7878 (configurable)
- Session management: Unicast to known peers
- Heartbeat interval: 1 second
- Disconnect detection: 5 missed heartbeats (5 seconds)

**Serialization**:
```toml
# Cargo.toml addition
bincode = "1.3"
```
- bincode for UDP messages (compact binary)
- serde_json for file exports (human-readable)

### Performance Considerations
- Buffer size: 1500 bytes (MTU safe)
- No fragmentation needed (metrics messages ~50 bytes)
- Rate limiting: Max 20 metric updates/second per rider
- Total bandwidth: ~10 riders × 20 updates × 50 bytes = ~10 KB/s (negligible)

## Activity Feed & Sharing

### Decision: JSON files + mDNS discovery

### Rationale
- JSON is human-readable and debuggable
- File-based sharing works offline
- mDNS TXT records can advertise feed availability
- No central server needed

### Implementation
- Each rider exports activities to `~/.rustride/shared/activities/`
- mDNS service advertises path to shared directory
- Peers can browse via mDNS and fetch via simple HTTP (using existing `reqwest`) or direct file access on LAN shares

## Workout Repository

### Decision: Bundled TOML files + optional GitHub raw file fetch

### Rationale
- TOML already used for configuration
- Workouts can be version-controlled
- GitHub raw URLs for updates (no API needed)
- Falls back to bundled workouts when offline

### Implementation
- Bundled: `assets/workouts/*.toml` (compiled into binary or installed alongside)
- User library: `~/.rustride/workouts/`
- GitHub sync: Fetch `https://raw.githubusercontent.com/.../workouts/index.json`

## Time Synchronization for Racing

### Decision: NTP-style offset calculation between peers

### Rationale
- LAN latency is low and stable
- Simple ping/pong timestamp exchange sufficient
- No external time server needed (offline-first)

### Implementation
```rust
// Calculate clock offset
let ping_sent = local_time();
send(Ping { timestamp_ms: ping_sent });
// ... receive Pong with remote timestamp
let ping_received = local_time();
let rtt = ping_received - ping_sent;
let offset = remote_time - (ping_sent + rtt/2);
```

Target: <500ms sync accuracy (well within tolerance for cycling racing)

## Dependencies Summary

```toml
# New dependencies for Social & Multiplayer feature
mdns-sd = "0.13"        # mDNS discovery
bincode = "1.3"         # Compact UDP message serialization
# Note: tokio, serde, serde_json, uuid, reqwest already in project
```

## Sources

- [mdns-sd crate](https://crates.io/crates/mdns-sd)
- [mdns-sd documentation](https://docs.rs/crate/mdns-sd/latest)
- [zeroconf crate](https://crates.io/crates/zeroconf)
- [Laminar UDP protocol](https://github.com/TimonPost/laminar)
- [Are We Game Yet - Networking](https://arewegameyet.rs/ecosystem/networking/)
- [Building UDP Server with Tokio](https://medium.com/@ekfqlwcjswl/building-your-first-udp-server-in-rust-with-tokio-79e35bdc6219)
- [Writing Highly Efficient UDP Server in Rust](https://idndx.com/writing-highly-efficient-udp-server-in-rust/)
