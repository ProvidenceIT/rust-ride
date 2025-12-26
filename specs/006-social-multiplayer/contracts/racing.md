# Contract: Racing Module

**Module**: `src/racing/`
**Purpose**: Virtual race event management, synchronized starts, and results

## Public API

### Race Manager

```rust
/// Manages virtual race events
pub struct RaceManager {
    // Internal state
}

impl RaceManager {
    pub fn new(
        db: Arc<Database>,
        discovery: Arc<DiscoveryService>,
        session_manager: Arc<SessionManager>,
    ) -> Self;

    /// Create a new race event
    pub fn create_race(&self, config: RaceConfig) -> Result<RaceEvent, RacingError>;

    /// Get upcoming races on LAN
    pub fn discover_races(&self) -> Result<Vec<RaceEvent>, RacingError>;

    /// Join a race
    pub fn join_race(&self, race_id: Uuid) -> Result<RaceParticipation, RacingError>;

    /// Leave a race (before it starts)
    pub fn leave_race(&self, race_id: Uuid) -> Result<(), RacingError>;

    /// Get current race status
    pub fn current_race(&self) -> Option<ActiveRace>;

    /// Cancel a race (organizer only)
    pub fn cancel_race(&self, race_id: Uuid) -> Result<(), RacingError>;

    /// Subscribe to race events
    pub fn subscribe(&self) -> Receiver<RaceUpdate>;
}

pub struct RaceConfig {
    pub name: String,
    pub world_id: String,
    pub route_id: String,
    pub distance_km: f64,
    pub scheduled_start: DateTime<Utc>,
}

pub struct RaceEvent {
    pub id: Uuid,
    pub name: String,
    pub world_id: String,
    pub route_id: String,
    pub distance_km: f64,
    pub scheduled_start: DateTime<Utc>,
    pub status: RaceStatus,
    pub organizer: RacerInfo,
    pub participant_count: u32,
}

pub struct RaceParticipation {
    pub race: RaceEvent,
    pub joined_at: DateTime<Utc>,
    pub position: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RaceStatus {
    Scheduled,
    Countdown,
    InProgress,
    Finished,
    Cancelled,
}
```

### Active Race

```rust
/// Represents an active race in progress
pub struct ActiveRace {
    pub event: RaceEvent,
    pub state: RaceState,
    pub participants: Vec<RaceParticipant>,
    pub local_participant: RaceParticipant,
}

pub struct RaceState {
    pub status: RaceStatus,
    pub countdown_seconds: Option<u8>,
    pub elapsed_time_ms: u64,
    pub start_time: Option<DateTime<Utc>>,
}

pub struct RaceParticipant {
    pub rider_id: Uuid,
    pub rider_name: String,
    pub status: ParticipantStatus,
    pub distance_m: f64,
    pub elapsed_time_ms: u64,
    pub current_position: u32,
    pub current_power: u16,
    pub gap_to_leader_ms: i64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ParticipantStatus {
    Waiting,
    Racing,
    Finished,
    DNF,
}

pub struct RacerInfo {
    pub rider_id: Uuid,
    pub rider_name: String,
}
```

### Countdown Synchronizer

```rust
/// Handles synchronized race starts
pub struct CountdownSync {
    // Internal clock sync state
}

impl CountdownSync {
    pub fn new(session: &Session) -> Self;

    /// Start countdown synchronization
    pub fn start_countdown(&self, start_time: DateTime<Utc>) -> Result<(), RacingError>;

    /// Get synchronized countdown value
    pub fn current_countdown(&self) -> Option<u8>;

    /// Check if race should start
    pub fn should_start(&self) -> bool;

    /// Get clock offset with peers
    pub fn clock_offset_ms(&self) -> i64;

    /// Subscribe to countdown events
    pub fn subscribe(&self) -> Receiver<CountdownEvent>;
}

pub enum CountdownEvent {
    CountdownStarted { seconds: u8 },
    CountdownTick { seconds: u8 },
    RaceStart,
    SyncError { message: String },
}
```

### Race Results

```rust
/// Manages race results
pub struct RaceResults {
    // Internal database handle
}

impl RaceResults {
    pub fn new(db: Arc<Database>) -> Self;

    /// Record a race finish
    pub fn record_finish(&self, race_id: Uuid, rider_id: Uuid, finish_time_ms: u32) -> Result<FinishResult, RacingError>;

    /// Mark participant as DNF
    pub fn record_dnf(&self, race_id: Uuid, rider_id: Uuid) -> Result<(), RacingError>;

    /// Get final results for a race
    pub fn get_results(&self, race_id: Uuid) -> Result<RaceResultsSummary, RacingError>;

    /// Get rider's race history
    pub fn rider_history(&self, rider_id: Uuid) -> Result<Vec<RaceHistoryEntry>, RacingError>;
}

pub struct FinishResult {
    pub position: u32,
    pub finish_time_ms: u32,
    pub gap_to_winner_ms: i64,
    pub is_personal_best: bool,
}

pub struct RaceResultsSummary {
    pub race: RaceEvent,
    pub finishers: Vec<RaceFinisher>,
    pub dnf_count: u32,
    pub total_participants: u32,
}

pub struct RaceFinisher {
    pub position: u32,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub finish_time_ms: u32,
    pub gap_to_winner_ms: i64,
}

pub struct RaceHistoryEntry {
    pub race_name: String,
    pub date: DateTime<Utc>,
    pub distance_km: f64,
    pub position: Option<u32>,
    pub total_racers: u32,
    pub finish_time_ms: Option<u32>,
    pub dnf: bool,
}
```

### Race Update Events

```rust
/// Real-time race updates
pub enum RaceUpdate {
    /// Race state changed
    StateChanged {
        race_id: Uuid,
        new_status: RaceStatus,
    },

    /// Countdown tick
    CountdownTick {
        race_id: Uuid,
        seconds: u8,
    },

    /// Participant position update
    PositionUpdate {
        race_id: Uuid,
        positions: Vec<PositionInfo>,
    },

    /// Participant finished
    ParticipantFinished {
        race_id: Uuid,
        rider_id: Uuid,
        position: u32,
        time_ms: u32,
    },

    /// Participant disconnected
    ParticipantDisconnected {
        race_id: Uuid,
        rider_id: Uuid,
        grace_period_seconds: u8,
    },

    /// Participant marked DNF
    ParticipantDNF {
        race_id: Uuid,
        rider_id: Uuid,
    },

    /// Race completed
    RaceCompleted {
        race_id: Uuid,
        results: RaceResultsSummary,
    },
}

pub struct PositionInfo {
    pub rider_id: Uuid,
    pub position: u32,
    pub distance_m: f64,
    pub gap_ms: i64,
}
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RacingError {
    #[error("Race not found: {0}")]
    RaceNotFound(Uuid),

    #[error("Race already started")]
    RaceAlreadyStarted,

    #[error("Race cancelled")]
    RaceCancelled,

    #[error("Not race organizer")]
    NotOrganizer,

    #[error("Already registered")]
    AlreadyRegistered,

    #[error("Not registered")]
    NotRegistered,

    #[error("Race is full")]
    RaceFull,

    #[error("Invalid scheduled time")]
    InvalidScheduledTime,

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] NetworkError),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
```

## Constants

```rust
/// Racing configuration constants
pub mod constants {
    /// Maximum participants per race
    pub const MAX_RACE_PARTICIPANTS: usize = 10;

    /// Countdown duration in seconds
    pub const COUNTDOWN_SECONDS: u8 = 60;

    /// Grace period for disconnection (seconds)
    pub const DISCONNECT_GRACE_PERIOD: u8 = 60;

    /// Position update frequency (Hz)
    pub const POSITION_UPDATE_RATE: u8 = 10;

    /// Maximum acceptable clock sync error (ms)
    pub const MAX_CLOCK_SYNC_ERROR: i64 = 500;
}
```
