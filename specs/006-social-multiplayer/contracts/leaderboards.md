# Contract: Leaderboards Module

**Module**: `src/leaderboards/`
**Purpose**: Segment definitions, effort tracking, and rankings

## Public API

### Segment Manager

```rust
/// Manages segment definitions
pub struct SegmentManager {
    // Internal database handle
}

impl SegmentManager {
    pub fn new(db: Arc<Database>) -> Self;

    /// Get all segments for a world
    pub fn segments_for_world(&self, world_id: &str) -> Result<Vec<Segment>, LeaderboardError>;

    /// Get segment by ID
    pub fn get_segment(&self, segment_id: Uuid) -> Result<Segment, LeaderboardError>;

    /// Initialize segments for a world (called when world is loaded)
    pub fn initialize_segments(&self, world_id: &str, segments: Vec<SegmentDefinition>) -> Result<(), LeaderboardError>;

    /// Check if rider is currently on a segment
    pub fn check_segment_entry(&self, world_id: &str, distance_m: f64) -> Option<ActiveSegment>;

    /// Check if rider has exited a segment
    pub fn check_segment_exit(&self, active: &ActiveSegment, distance_m: f64) -> Option<SegmentCompletion>;
}

pub struct Segment {
    pub id: Uuid,
    pub world_id: String,
    pub name: String,
    pub start_distance_m: f64,
    pub end_distance_m: f64,
    pub category: SegmentCategory,
    pub elevation_gain_m: f64,
    pub length_m: f64,
}

pub struct SegmentDefinition {
    pub name: String,
    pub start_distance_m: f64,
    pub end_distance_m: f64,
    pub category: SegmentCategory,
    pub elevation_gain_m: f64,
}

pub struct ActiveSegment {
    pub segment: Segment,
    pub entry_time: Instant,
    pub entry_distance_m: f64,
}

pub struct SegmentCompletion {
    pub segment: Segment,
    pub elapsed_time_ms: u32,
    pub avg_power_watts: Option<u16>,
    pub avg_hr_bpm: Option<u8>,
}

#[derive(Clone, Copy)]
pub enum SegmentCategory {
    Climb,
    Sprint,
    Mixed,
}
```

### Effort Tracker

```rust
/// Tracks and records segment efforts
pub struct EffortTracker {
    // Internal state
}

impl EffortTracker {
    pub fn new(db: Arc<Database>, segment_manager: Arc<SegmentManager>) -> Self;

    /// Start tracking for a ride
    pub fn start_ride(&mut self, world_id: &str);

    /// Update with current ride state (call every second)
    pub fn update(&mut self, state: &RideState) -> Option<SegmentEvent>;

    /// End tracking and finalize any pending efforts
    pub fn end_ride(&mut self) -> Vec<RecordedEffort>;

    /// Record a completed effort
    pub fn record_effort(&self, completion: SegmentCompletion, ride_id: Uuid) -> Result<RecordedEffort, LeaderboardError>;
}

pub struct RideState {
    pub distance_m: f64,
    pub power_watts: u16,
    pub heart_rate_bpm: u8,
    pub elapsed_time_ms: u64,
}

pub enum SegmentEvent {
    Entered(Segment),
    Exited(SegmentCompletion),
    PersonalBest(RecordedEffort),
}

pub struct RecordedEffort {
    pub id: Uuid,
    pub segment: Segment,
    pub elapsed_time_ms: u32,
    pub avg_power_watts: Option<u16>,
    pub avg_hr_bpm: Option<u8>,
    pub recorded_at: DateTime<Utc>,
    pub rank: u32,
    pub is_personal_best: bool,
}
```

### Leaderboard Service

```rust
/// Provides leaderboard rankings
pub struct LeaderboardService {
    // Internal database handle
}

impl LeaderboardService {
    pub fn new(db: Arc<Database>) -> Self;

    /// Get leaderboard for a segment
    pub fn get_leaderboard(&self, segment_id: Uuid, filter: LeaderboardFilter) -> Result<Leaderboard, LeaderboardError>;

    /// Get personal bests for a rider
    pub fn personal_bests(&self, rider_id: Uuid) -> Result<Vec<PersonalBest>, LeaderboardError>;

    /// Get rider's rank on a segment
    pub fn rider_rank(&self, segment_id: Uuid, rider_id: Uuid) -> Result<Option<RankInfo>, LeaderboardError>;
}

pub struct LeaderboardFilter {
    pub limit: usize,
    pub time_range: Option<TimeRange>,
    pub rider_filter: Option<RiderFilter>,
}

pub enum TimeRange {
    AllTime,
    ThisWeek,
    ThisMonth,
    ThisYear,
}

pub enum RiderFilter {
    All,
    Local,      // Only local rider
    Club(Uuid), // Only club members
}

pub struct Leaderboard {
    pub segment: Segment,
    pub entries: Vec<LeaderboardEntry>,
    pub total_efforts: u32,
    pub rider_entry: Option<LeaderboardEntry>,
}

pub struct LeaderboardEntry {
    pub rank: u32,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub elapsed_time_ms: u32,
    pub avg_power_watts: Option<u16>,
    pub recorded_at: DateTime<Utc>,
    pub is_current_rider: bool,
}

pub struct PersonalBest {
    pub segment: Segment,
    pub elapsed_time_ms: u32,
    pub recorded_at: DateTime<Utc>,
    pub rank: u32,
}

pub struct RankInfo {
    pub rank: u32,
    pub total_riders: u32,
    pub best_time_ms: u32,
    pub percentile: f64,
}
```

### Export/Import

```rust
/// Leaderboard data export and import
pub struct LeaderboardExporter {
    // Internal database handle
}

impl LeaderboardExporter {
    pub fn new(db: Arc<Database>) -> Self;

    /// Export leaderboard to JSON
    pub fn export_json(&self, segment_id: Uuid) -> Result<String, LeaderboardError>;

    /// Export leaderboard to CSV
    pub fn export_csv(&self, segment_id: Uuid) -> Result<String, LeaderboardError>;

    /// Import efforts from JSON
    /// Returns (imported_count, duplicate_count, conflict_names)
    pub fn import_json(&self, json_content: &str) -> Result<ImportResult, LeaderboardError>;
}

pub struct ImportResult {
    pub imported_count: u32,
    pub duplicate_count: u32,
    pub name_conflicts: Vec<NameConflict>,
}

pub struct NameConflict {
    pub imported_name: String,
    pub existing_rider_id: Option<Uuid>,
    pub effort_count: u32,
}

/// Resolution for name conflicts
pub enum ConflictResolution {
    MergeWithExisting(Uuid),
    CreateNew,
    Skip,
}
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum LeaderboardError {
    #[error("Segment not found: {0}")]
    SegmentNotFound(Uuid),

    #[error("No efforts recorded")]
    NoEfforts,

    #[error("Invalid export format")]
    InvalidExportFormat,

    #[error("Import failed: {0}")]
    ImportFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
```
