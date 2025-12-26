# Quickstart: Social & Multiplayer

**Feature Branch**: `006-social-multiplayer`
**Date**: 2025-12-26

## Prerequisites

- Rust 1.75+ installed
- RustRide base application builds successfully
- Two devices on the same LAN (for testing multiplayer)

## Setup

### 1. Add Dependencies

Update `Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...

# NEW: Social & Multiplayer
mdns-sd = "0.13"    # mDNS service discovery
bincode = "1.3"     # Compact binary serialization for UDP
```

### 2. Create Module Structure

```bash
# Create new module directories
mkdir -p src/social
mkdir -p src/networking
mkdir -p src/leaderboards
mkdir -p src/racing

# Create module files
touch src/social/mod.rs
touch src/social/types.rs
touch src/social/profile.rs
touch src/social/clubs.rs
touch src/social/badges.rs
touch src/social/challenges.rs
touch src/social/feed.rs

touch src/networking/mod.rs
touch src/networking/discovery.rs
touch src/networking/protocol.rs
touch src/networking/session.rs
touch src/networking/sync.rs
touch src/networking/chat.rs

touch src/leaderboards/mod.rs
touch src/leaderboards/segments.rs
touch src/leaderboards/efforts.rs
touch src/leaderboards/rankings.rs
touch src/leaderboards/export.rs

touch src/racing/mod.rs
touch src/racing/events.rs
touch src/racing/countdown.rs
touch src/racing/results.rs

# Extend existing modules
touch src/workouts/repository.rs
touch src/workouts/ratings.rs
touch src/storage/social_store.rs
```

### 3. Initialize Database Schema

The schema will be auto-migrated on first run. See `data-model.md` for full schema.

Key tables:
- `riders` - User profiles
- `clubs`, `club_memberships` - Club management
- `badges`, `rider_badges` - Achievement system
- `segments`, `segment_efforts` - Leaderboards
- `challenges`, `challenge_progress` - Training challenges
- `race_events`, `race_participants` - Virtual racing
- `group_rides`, `chat_messages` - Group ride sessions
- `workout_ratings` - Workout reviews

### 4. Bundle Default Content

Create workout repository:
```bash
mkdir -p assets/workouts/{endurance,tempo,threshold,vo2max,sprint,recovery,mixed}
```

Each workout file: `assets/workouts/{focus}/{workout_id}.toml`

## Key Implementation Order

### Phase 1: Foundation (P1 Core)

1. **Rider Profile** (`src/social/profile.rs`)
   - Create/update profile
   - Avatar selection
   - Stats aggregation

2. **mDNS Discovery** (`src/networking/discovery.rs`)
   - Service registration
   - Peer discovery
   - Event subscription

3. **UDP Protocol** (`src/networking/protocol.rs`)
   - Message types
   - Serialization
   - Socket management

4. **Session Management** (`src/networking/session.rs`)
   - Host/join sessions
   - Participant tracking
   - Heartbeat/disconnect

5. **Metric Sync** (`src/networking/sync.rs`)
   - Broadcast local metrics
   - Receive peer metrics
   - Position updates

### Phase 2: Leaderboards (P2)

1. **Segment Manager** (`src/leaderboards/segments.rs`)
   - Load from world data
   - Entry/exit detection

2. **Effort Tracker** (`src/leaderboards/efforts.rs`)
   - Active segment tracking
   - Effort recording

3. **Rankings** (`src/leaderboards/rankings.rs`)
   - Leaderboard queries
   - Personal bests

4. **Export** (`src/leaderboards/export.rs`)
   - JSON/CSV export
   - Import with name matching

### Phase 3: Workouts & Challenges (P3-P4)

1. **Repository** (`src/workouts/repository.rs`)
   - Load bundled workouts
   - Search/filter
   - GitHub sync (optional)

2. **Ratings** (`src/workouts/ratings.rs`)
   - Rate/review
   - Aggregate ratings

3. **Challenges** (`src/social/challenges.rs`)
   - Create/join
   - Progress tracking
   - Export/import

### Phase 4: Racing (P5)

1. **Race Events** (`src/racing/events.rs`)
   - Create/discover races
   - Registration

2. **Countdown Sync** (`src/racing/countdown.rs`)
   - Clock synchronization
   - Synchronized start

3. **Results** (`src/racing/results.rs`)
   - Finish recording
   - DNF handling
   - Result display

### Phase 5: Social Features (P6-P12)

1. Activity Feed
2. Workout Ratings UI
3. Club Management
4. Achievement Badges
5. Group Chat
6. Ride Comparison
7. Profile UI

## Testing Strategy

### Unit Tests

```rust
// Test protocol serialization
#[test]
fn test_metric_update_serialization() {
    let msg = ProtocolMessage::MetricUpdate {
        rider_id: Uuid::new_v4(),
        metrics: RiderMetrics { power_watts: 250, ... },
        sequence: 1,
    };
    let bytes = bincode::serialize(&msg).unwrap();
    let decoded: ProtocolMessage = bincode::deserialize(&bytes).unwrap();
    assert_eq!(msg, decoded);
}

// Test leaderboard ranking
#[test]
fn test_effort_ranking() {
    // Add efforts, verify ranking order
}

// Test badge criteria
#[test]
fn test_distance_badge_unlock() {
    // Verify badge unlocks at threshold
}
```

### Integration Tests

```rust
// Test mDNS discovery (requires two processes)
#[test]
fn test_peer_discovery() {
    // Start discovery on two instances
    // Verify mutual discovery
}

// Test group ride session
#[test]
fn test_session_join_leave() {
    // Host creates session
    // Peer joins
    // Verify participant list
    // Peer leaves
}
```

### Manual Testing Checklist

- [ ] Two devices discover each other on LAN
- [ ] Group ride shows real-time metrics from peer
- [ ] Chat messages appear on both devices
- [ ] Segment effort recorded and appears on leaderboard
- [ ] Challenge progress updates after ride
- [ ] Race countdown starts simultaneously on all devices
- [ ] Activity feed shows peer's completed rides

## Configuration

### Network Settings

`~/.rustride/config.toml`:
```toml
[network]
service_type = "_rustride._udp.local."
multicast_addr = "239.255.42.42:7878"
heartbeat_interval_ms = 1000
disconnect_timeout_ms = 5000
metric_rate_hz = 20
```

### Profile Defaults

First-run creates profile with:
- `display_name`: "Rider" + random suffix
- `avatar_id`: random from available
- `sharing_enabled`: true

## Common Issues

### mDNS not discovering peers

1. Check firewall allows UDP 5353 (mDNS) and 7878 (app)
2. Verify devices are on same subnet
3. Check antivirus isn't blocking multicast

### High latency in group rides

1. Reduce `metric_rate_hz` to 10
2. Check for network congestion
3. Verify not on VPN

### Database migration errors

1. Delete `~/.rustride/rustride.db` (loses data)
2. Check schema matches `data-model.md`

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| mDNS discovery | <5 seconds | Time from launch to peer visible |
| Metric latency | <100ms | Round-trip time for metric update |
| UI frame rate | 60 fps | During 10-person group ride |
| Memory overhead | <50 MB | Additional for social features |
| Network bandwidth | <20 KB/s | Per participant in group ride |

## Next Steps

After completing implementation:

1. Run `/speckit.tasks` to generate detailed task breakdown
2. Implement features in priority order (P1 → P12)
3. Write tests alongside implementation
4. Test with multiple devices on LAN
5. Profile performance with 10 participants
