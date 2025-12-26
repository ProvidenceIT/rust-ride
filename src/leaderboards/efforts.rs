//! Effort tracking for segments.
//!
//! Tracks active segment efforts and records completions.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::segments::{ActiveSegment, Segment, SegmentCompletion, SegmentManager};
use crate::storage::Database;

/// Recorded segment effort.
#[derive(Debug, Clone)]
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

/// Segment event during a ride.
#[derive(Debug, Clone)]
pub enum SegmentEvent {
    /// Entered a segment.
    Entered(Segment),
    /// Exited a segment.
    Exited(SegmentCompletion),
    /// Achieved a personal best.
    PersonalBest(RecordedEffort),
}

/// Current ride state for effort tracking.
#[derive(Debug, Clone)]
pub struct RideState {
    pub distance_m: f64,
    pub power_watts: u16,
    pub heart_rate_bpm: Option<u8>,
    pub elapsed_time_ms: u64,
}

/// Tracks segment efforts during a ride.
pub struct EffortTracker {
    db: Arc<Database>,
    segment_manager: Arc<SegmentManager>,
    world_id: Option<String>,
    active_segment: Option<ActiveSegment>,
    previous_distance_m: f64,
    rider_id: Uuid,
}

impl EffortTracker {
    /// Create a new effort tracker.
    pub fn new(db: Arc<Database>, segment_manager: Arc<SegmentManager>, rider_id: Uuid) -> Self {
        Self {
            db,
            segment_manager,
            world_id: None,
            active_segment: None,
            previous_distance_m: 0.0,
            rider_id,
        }
    }

    /// Start tracking for a ride.
    pub fn start_ride(&mut self, world_id: &str) {
        self.world_id = Some(world_id.to_string());
        self.active_segment = None;
        self.previous_distance_m = 0.0;
    }

    /// Update with current ride state.
    pub fn update(&mut self, state: &RideState) -> Option<SegmentEvent> {
        let world_id = self.world_id.as_ref()?;

        // Check for segment entry
        if self.active_segment.is_none() {
            if let Some(segment) = self.segment_manager.check_segment_entry(
                world_id,
                state.distance_m,
                self.previous_distance_m,
            ) {
                self.active_segment = Some(ActiveSegment::new(segment.clone(), state.distance_m));
                self.previous_distance_m = state.distance_m;
                return Some(SegmentEvent::Entered(segment));
            }
        }

        // Check for segment exit
        if let Some(ref mut active) = self.active_segment {
            // Record sample
            active.record_sample(state.power_watts, state.heart_rate_bpm);

            // Check if exited
            if let Some(completion) =
                self.segment_manager.check_segment_exit(active, state.distance_m)
            {
                self.active_segment = None;
                self.previous_distance_m = state.distance_m;
                return Some(SegmentEvent::Exited(completion));
            }
        }

        self.previous_distance_m = state.distance_m;
        None
    }

    /// End tracking and finalize any pending efforts.
    pub fn end_ride(&mut self) -> Vec<RecordedEffort> {
        self.active_segment = None;
        self.world_id = None;
        Vec::new()
    }

    /// Record a completed effort.
    pub fn record_effort(
        &self,
        completion: SegmentCompletion,
        ride_id: Uuid,
    ) -> Result<RecordedEffort, EffortError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let conn = self.db.connection();

        // Insert effort
        conn.execute(
            "INSERT INTO social_segment_efforts (id, segment_id, rider_id, ride_id, elapsed_time_ms, avg_power_watts, avg_hr_bpm, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id.to_string(),
                completion.segment.id.to_string(),
                self.rider_id.to_string(),
                ride_id.to_string(),
                completion.elapsed_time_ms,
                completion.avg_power_watts,
                completion.avg_hr_bpm,
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| EffortError::DatabaseError(e.to_string()))?;

        // Get rank
        let rank = self.get_effort_rank(&completion.segment.id, completion.elapsed_time_ms)?;

        // Check if personal best
        let is_pb = self.is_personal_best(&completion.segment.id, completion.elapsed_time_ms)?;

        Ok(RecordedEffort {
            id,
            segment: completion.segment,
            elapsed_time_ms: completion.elapsed_time_ms,
            avg_power_watts: completion.avg_power_watts,
            avg_hr_bpm: completion.avg_hr_bpm,
            recorded_at: now,
            rank,
            is_personal_best: is_pb,
        })
    }

    /// Get rank for an effort time.
    fn get_effort_rank(&self, segment_id: &Uuid, elapsed_time_ms: u32) -> Result<u32, EffortError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(*) FROM social_segment_efforts
                 WHERE segment_id = ?1 AND elapsed_time_ms < ?2",
            )
            .map_err(|e| EffortError::DatabaseError(e.to_string()))?;

        let count: u32 = stmt
            .query_row(
                rusqlite::params![segment_id.to_string(), elapsed_time_ms],
                |row| row.get(0),
            )
            .map_err(|e| EffortError::DatabaseError(e.to_string()))?;

        Ok(count + 1)
    }

    /// Check if this is a personal best.
    fn is_personal_best(&self, segment_id: &Uuid, elapsed_time_ms: u32) -> Result<bool, EffortError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT MIN(elapsed_time_ms) FROM social_segment_efforts
                 WHERE segment_id = ?1 AND rider_id = ?2",
            )
            .map_err(|e| EffortError::DatabaseError(e.to_string()))?;

        let min_time: Option<u32> = stmt
            .query_row(
                rusqlite::params![segment_id.to_string(), self.rider_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| EffortError::DatabaseError(e.to_string()))?;

        Ok(min_time.map_or(true, |min| elapsed_time_ms <= min))
    }
}

/// Effort tracking errors.
#[derive(Debug, thiserror::Error)]
pub enum EffortError {
    #[error("No active segment")]
    NoActiveSegment,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
