# Contract: Segment & Leaderboard Module

**Module**: `src/world/segments/`
**Date**: 2025-12-25

## Purpose

Define route segments, track user times during rides, maintain leaderboards, and provide real-time segment progress feedback.

## Public API

### Types

```rust
/// Segment manager handles timing and leaderboards
pub struct SegmentManager {
    segments: Vec<Segment>,
    active_segment: Option<ActiveSegmentState>,
    times_cache: HashMap<Uuid, Vec<SegmentTime>>,
}

/// State during active segment effort
pub struct ActiveSegmentState {
    pub segment_id: Uuid,
    pub start_time: Instant,
    pub start_distance: f64,
    pub power_samples: Vec<u16>,
    pub hr_samples: Vec<u8>,
}

/// Real-time segment progress for HUD
pub struct SegmentProgress {
    pub segment_name: String,
    pub elapsed_seconds: f32,
    pub distance_remaining_meters: f32,
    pub percent_complete: f32,
    pub current_pace_vs_pb: Option<f32>, // +/- seconds vs personal best
    pub projected_time_seconds: f32,
}

/// Leaderboard entry for display
pub struct LeaderboardEntry {
    pub rank: u32,
    pub user_name: String,
    pub time_seconds: f64,
    pub avg_power: Option<u16>,
    pub date: DateTime<Utc>,
    pub is_current_user: bool,
    pub is_personal_best: bool,
}

/// Leaderboard filter options
pub struct LeaderboardFilter {
    pub time_range: TimeRange,
    pub category: Option<LeaderboardCategory>,
}

pub enum TimeRange {
    AllTime,
    ThisMonth,
    ThisWeek,
    Today,
}

pub enum LeaderboardCategory {
    Overall,
    WeightClass(WeightClass),
    AgeGroup(AgeGroup),
}
```

### Functions

```rust
impl SegmentManager {
    /// Create manager with segments from route
    pub fn new(route_id: Uuid, db: &Database) -> Result<Self, DatabaseError>;

    /// Load segments for a route
    pub fn load_segments(&mut self, db: &Database) -> Result<(), DatabaseError>;

    /// Get all segments for current route
    pub fn segments(&self) -> &[Segment];

    /// Update based on user position
    ///
    /// Returns segment events (start, complete, etc.)
    pub fn update(
        &mut self,
        user_distance: f64,
        current_power: Option<u16>,
        current_hr: Option<u8>,
    ) -> Vec<SegmentEvent>;

    /// Get current active segment progress (if any)
    pub fn active_progress(&self) -> Option<SegmentProgress>;

    /// Complete current segment and save time
    pub fn complete_segment(
        &mut self,
        user_id: Uuid,
        ride_id: Uuid,
        ftp: u16,
        db: &mut Database,
    ) -> Result<SegmentCompletion, DatabaseError>;

    /// Get leaderboard for a segment
    pub fn get_leaderboard(
        &self,
        segment_id: Uuid,
        filter: LeaderboardFilter,
        limit: u32,
        db: &Database,
    ) -> Result<Vec<LeaderboardEntry>, DatabaseError>;

    /// Get user's personal best for segment
    pub fn get_personal_best(
        &self,
        segment_id: Uuid,
        user_id: Uuid,
        db: &Database,
    ) -> Result<Option<SegmentTime>, DatabaseError>;

    /// Get user's rank on segment
    pub fn get_user_rank(
        &self,
        segment_id: Uuid,
        user_id: Uuid,
        filter: LeaderboardFilter,
        db: &Database,
    ) -> Result<Option<u32>, DatabaseError>;

    /// Reset for new ride
    pub fn reset(&mut self);
}

/// Events emitted during segment tracking
pub enum SegmentEvent {
    /// Entered a segment
    SegmentStarted {
        segment: Segment,
        personal_best: Option<f64>,
    },
    /// Progress update (every second)
    SegmentProgress(SegmentProgress),
    /// Completed a segment
    SegmentCompleted(SegmentCompletion),
}

pub struct SegmentCompletion {
    pub segment_id: Uuid,
    pub time_seconds: f64,
    pub avg_power: Option<u16>,
    pub avg_hr: Option<u8>,
    pub is_personal_best: bool,
    pub rank: Option<u32>,
    pub improvement_seconds: Option<f64>,
}
```

### Database Operations

```rust
/// Segment CRUD operations
impl Database {
    /// Insert a segment
    pub fn insert_segment(&self, segment: &Segment) -> Result<(), DatabaseError>;

    /// Get segments for a route
    pub fn get_segments(&self, route_id: &Uuid) -> Result<Vec<Segment>, DatabaseError>;

    /// Delete a segment
    pub fn delete_segment(&self, id: &Uuid) -> Result<(), DatabaseError>;

    /// Insert a segment time
    pub fn insert_segment_time(&self, time: &SegmentTime) -> Result<(), DatabaseError>;

    /// Get segment times with filtering
    pub fn get_segment_times(
        &self,
        segment_id: &Uuid,
        filter: &LeaderboardFilter,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SegmentTime>, DatabaseError>;

    /// Get user's best time on segment
    pub fn get_best_segment_time(
        &self,
        segment_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<SegmentTime>, DatabaseError>;

    /// Update personal best flag
    pub fn update_personal_best(
        &self,
        segment_id: &Uuid,
        user_id: &Uuid,
        new_best_id: &Uuid,
    ) -> Result<(), DatabaseError>;

    /// Count times on segment (for ranking)
    pub fn count_segment_times(
        &self,
        segment_id: &Uuid,
        filter: &LeaderboardFilter,
    ) -> Result<u32, DatabaseError>;
}
```

## Segment Detection Logic

```
Enter Segment:
- User distance >= segment.start_distance - 1m
- AND user distance <= segment.start_distance + 5m
- Start timing

During Segment:
- Record power/HR samples every second
- Calculate running average
- Compare to personal best pace

Exit Segment:
- User distance >= segment.end_distance
- Stop timing
- Calculate final time, averages
- Check for personal best
- Save to database
```

## Leaderboard Queries

```sql
-- Get leaderboard (all-time, top 100)
SELECT st.*, u.name as user_name
FROM segment_times st
JOIN users u ON st.user_id = u.id
WHERE st.segment_id = ?
ORDER BY st.time_seconds ASC
LIMIT 100;

-- Get user rank
SELECT COUNT(*) + 1 as rank
FROM segment_times
WHERE segment_id = ?
AND time_seconds < (
    SELECT MIN(time_seconds)
    FROM segment_times
    WHERE segment_id = ? AND user_id = ?
);

-- Get monthly leaderboard
SELECT st.*, u.name as user_name
FROM segment_times st
JOIN users u ON st.user_id = u.id
WHERE st.segment_id = ?
AND st.recorded_at >= datetime('now', '-1 month')
ORDER BY st.time_seconds ASC
LIMIT 100;
```

## Performance Requirements

- Segment detection: <0.1ms per update
- Leaderboard query: <2s for 10,000 entries
- Personal best lookup: <100ms
- Time recording: <50ms

## Real-Time HUD Display

During active segment:
- Segment name
- Elapsed time (updating)
- Distance remaining
- Pace vs PB (+/- seconds, color coded)
- Projected finish time

On completion:
- Final time
- Rank (if available)
- PB indicator with celebration animation
