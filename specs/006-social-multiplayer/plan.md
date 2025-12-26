# Implementation Plan: Social & Multiplayer

**Branch**: `006-social-multiplayer` | **Date**: 2025-12-26 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/006-social-multiplayer/spec.md`

## Summary

Implement social and multiplayer features for RustRide enabling LAN-based group rides, local leaderboards, community workout repository, training challenges, virtual racing, activity feeds, workout ratings, club management, achievement badges, LAN chat, ride comparison, and rider profiles. All features are designed for self-hosted, offline-first architecture using mDNS discovery and UDP synchronization for peer-to-peer communication.

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**:
- egui/eframe (GUI)
- tokio (async runtime)
- mdns-sd 0.13 (mDNS discovery)
- UDP sockets via tokio (metric sync)
- rusqlite (local storage)
- serde/serde_json (serialization)
- crossbeam (channels)

**Storage**: SQLite via rusqlite (extends existing database)
**Testing**: cargo test (unit + integration)
**Target Platform**: Windows, macOS, Linux (desktop)
**Project Type**: Single Rust application with library
**Performance Goals**:
- mDNS discovery <5 seconds
- UDP metric sync <100ms latency
- Support 10 concurrent participants
- 60 fps UI maintained during group rides

**Constraints**:
- Offline-first (no internet required for core features)
- LAN-only networking (no cloud infrastructure)
- Auto-save every 30 seconds for crash recovery
- Trust all LAN peers automatically

**Scale/Scope**:
- 10 concurrent group ride participants
- 50-100 pre-curated workouts
- Local SQLite storage for all data

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| Library-First | PASS | New modules (social/, networking/) will be self-contained libraries |
| Test-First | PASS | Unit tests for networking protocol, integration tests for discovery |
| Simplicity | PASS | UDP for sync, mDNS for discovery - minimal complexity |
| Offline-Capable | PASS | Core requirement of the feature |

No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/006-social-multiplayer/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── app.rs               # Application state (extend with social state)
├── lib.rs               # Library exports (add social modules)
├── main.rs              # Entry point
│
├── social/              # NEW: Social features module
│   ├── mod.rs           # Module exports
│   ├── types.rs         # Shared types (Rider, Club, Badge, etc.)
│   ├── profile.rs       # Rider profile management
│   ├── clubs.rs         # Club management
│   ├── badges.rs        # Achievement badge system
│   ├── challenges.rs    # Training challenges
│   └── feed.rs          # Activity feed
│
├── networking/          # NEW: LAN networking module
│   ├── mod.rs           # Module exports
│   ├── discovery.rs     # mDNS service discovery
│   ├── protocol.rs      # UDP message protocol
│   ├── session.rs       # Group ride session management
│   ├── sync.rs          # Real-time metric synchronization
│   └── chat.rs          # Group ride chat
│
├── leaderboards/        # NEW: Segment leaderboards
│   ├── mod.rs           # Module exports
│   ├── segments.rs      # Segment definitions
│   ├── efforts.rs       # Segment effort tracking
│   ├── rankings.rs      # Leaderboard calculations
│   └── export.rs        # CSV/JSON export
│
├── racing/              # NEW: Virtual racing
│   ├── mod.rs           # Module exports
│   ├── events.rs        # Race event management
│   ├── countdown.rs     # Synchronized countdown
│   └── results.rs       # Race results
│
├── workouts/            # EXTEND: Add repository features
│   ├── repository.rs    # NEW: Community workout repository
│   └── ratings.rs       # NEW: Workout ratings/reviews
│
├── storage/             # EXTEND: Add social tables
│   ├── social_store.rs  # NEW: Social data persistence
│   └── schema.rs        # EXTEND: Add social schema
│
├── ui/screens/          # EXTEND: Add social screens
│   ├── group_ride.rs    # NEW: Group ride view
│   ├── leaderboard.rs   # NEW: Leaderboard view
│   ├── clubs.rs         # NEW: Club management
│   ├── challenges.rs    # NEW: Challenge view
│   ├── activity_feed.rs # NEW: Activity feed
│   ├── race_lobby.rs    # NEW: Race lobby/results
│   ├── rider_profile.rs # NEW: Profile editor
│   └── ride_compare.rs  # NEW: Ride comparison charts
│
└── ui/widgets/          # EXTEND: Add social widgets
    ├── participant_list.rs  # NEW: Group ride participants
    ├── chat_panel.rs        # NEW: Chat message panel
    └── badge_display.rs     # NEW: Achievement badges

tests/
├── unit/
│   ├── networking/      # Protocol, discovery tests
│   ├── social/          # Profile, clubs, badges tests
│   └── leaderboards/    # Ranking algorithm tests
└── integration/
    ├── group_ride_test.rs   # Multi-instance testing
    └── discovery_test.rs    # mDNS discovery testing
```

**Structure Decision**: Extends existing single-project structure with new top-level modules (social/, networking/, leaderboards/, racing/) following the established pattern. Each module is self-contained and testable independently.

## Complexity Tracking

No violations requiring justification - implementation follows existing patterns.
