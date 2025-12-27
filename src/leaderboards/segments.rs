//! Segment definitions for leaderboards.
//!
//! Manages segment definitions and entry/exit detection.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::storage::Database;

/// Segment category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentCategory {
    /// Primarily uphill
    Climb,
    /// Flat or downhill, short
    Sprint,
    /// General segment
    Mixed,
}

impl SegmentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SegmentCategory::Climb => "climb",
            SegmentCategory::Sprint => "sprint",
            SegmentCategory::Mixed => "mixed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "climb" => Some(SegmentCategory::Climb),
            "sprint" => Some(SegmentCategory::Sprint),
            "mixed" => Some(SegmentCategory::Mixed),
            _ => None,
        }
    }
}

/// Segment definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: Uuid,
    pub world_id: String,
    pub name: String,
    pub start_distance_m: f64,
    pub end_distance_m: f64,
    pub category: SegmentCategory,
    pub elevation_gain_m: f64,
}

impl Segment {
    /// Get segment length in meters.
    pub fn length_m(&self) -> f64 {
        self.end_distance_m - self.start_distance_m
    }

    /// Check if distance is within segment bounds.
    pub fn contains(&self, distance_m: f64) -> bool {
        distance_m >= self.start_distance_m && distance_m <= self.end_distance_m
    }
}

/// Segment definition for initialization.
#[derive(Debug, Clone)]
pub struct SegmentDefinition {
    pub name: String,
    pub start_distance_m: f64,
    pub end_distance_m: f64,
    pub category: SegmentCategory,
    pub elevation_gain_m: f64,
}

/// Active segment being tracked.
#[derive(Debug, Clone)]
pub struct ActiveSegment {
    pub segment: Segment,
    pub entry_time: Instant,
    pub entry_distance_m: f64,
    /// Running sum of power samples.
    pub power_sum: u64,
    /// Running sum of heart rate samples.
    pub hr_sum: u32,
    /// Number of samples collected.
    pub sample_count: u32,
}

impl ActiveSegment {
    /// Create a new active segment.
    pub fn new(segment: Segment, entry_distance_m: f64) -> Self {
        Self {
            segment,
            entry_time: Instant::now(),
            entry_distance_m,
            power_sum: 0,
            hr_sum: 0,
            sample_count: 0,
        }
    }

    /// Record a metric sample.
    pub fn record_sample(&mut self, power_watts: u16, heart_rate_bpm: Option<u8>) {
        self.power_sum += power_watts as u64;
        if let Some(hr) = heart_rate_bpm {
            self.hr_sum += hr as u32;
        }
        self.sample_count += 1;
    }

    /// Get average power.
    pub fn avg_power_watts(&self) -> Option<u16> {
        if self.sample_count > 0 {
            Some((self.power_sum / self.sample_count as u64) as u16)
        } else {
            None
        }
    }

    /// Get average heart rate.
    pub fn avg_hr_bpm(&self) -> Option<u8> {
        if self.sample_count > 0 && self.hr_sum > 0 {
            Some((self.hr_sum / self.sample_count) as u8)
        } else {
            None
        }
    }
}

/// Segment completion data.
#[derive(Debug, Clone)]
pub struct SegmentCompletion {
    pub segment: Segment,
    pub elapsed_time_ms: u32,
    pub avg_power_watts: Option<u16>,
    pub avg_hr_bpm: Option<u8>,
}

/// Manages segment definitions.
pub struct SegmentManager {
    db: Arc<Database>,
}

impl SegmentManager {
    /// Create a new segment manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get all segments for a world.
    pub fn segments_for_world(&self, world_id: &str) -> Result<Vec<Segment>, SegmentError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, world_id, name, start_distance_m, end_distance_m, category, elevation_gain_m
                 FROM social_segments WHERE world_id = ?1 ORDER BY start_distance_m",
            )
            .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([world_id], |row| {
                let id_str: String = row.get(0)?;
                let category_str: String = row.get(5)?;
                Ok((
                    id_str,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    category_str,
                    row.get(6)?,
                ))
            })
            .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;

        let mut segments = Vec::new();
        for row in rows {
            let (id_str, world_id, name, start, end, category_str, elevation): (
                String,
                String,
                String,
                f64,
                f64,
                String,
                f64,
            ) = row.map_err(|e| SegmentError::DatabaseError(e.to_string()))?;

            segments.push(Segment {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
                world_id,
                name,
                start_distance_m: start,
                end_distance_m: end,
                category: SegmentCategory::from_str(&category_str)
                    .unwrap_or(SegmentCategory::Mixed),
                elevation_gain_m: elevation,
            });
        }

        Ok(segments)
    }

    /// Get segment by ID.
    pub fn get_segment(&self, segment_id: Uuid) -> Result<Segment, SegmentError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, world_id, name, start_distance_m, end_distance_m, category, elevation_gain_m
                 FROM social_segments WHERE id = ?1",
            )
            .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;

        let mut rows = stmt
            .query([segment_id.to_string()])
            .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| SegmentError::DatabaseError(e.to_string()))?
        {
            let id_str: String = row
                .get(0)
                .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;
            let category_str: String = row
                .get(5)
                .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;

            Ok(Segment {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
                world_id: row
                    .get(1)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
                name: row
                    .get(2)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
                start_distance_m: row
                    .get(3)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
                end_distance_m: row
                    .get(4)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
                category: SegmentCategory::from_str(&category_str)
                    .unwrap_or(SegmentCategory::Mixed),
                elevation_gain_m: row
                    .get(6)
                    .map_err(|e| SegmentError::DatabaseError(e.to_string()))?,
            })
        } else {
            Err(SegmentError::SegmentNotFound(segment_id))
        }
    }

    /// Initialize segments for a world.
    pub fn initialize_segments(
        &self,
        world_id: &str,
        segments: Vec<SegmentDefinition>,
    ) -> Result<(), SegmentError> {
        let conn = self.db.connection();

        for def in segments {
            let id = Uuid::new_v4();
            conn.execute(
                "INSERT OR IGNORE INTO social_segments (id, world_id, name, start_distance_m, end_distance_m, category, elevation_gain_m)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    id.to_string(),
                    world_id,
                    def.name,
                    def.start_distance_m,
                    def.end_distance_m,
                    def.category.as_str(),
                    def.elevation_gain_m,
                ],
            )
            .map_err(|e| SegmentError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Check if rider is entering a segment.
    pub fn check_segment_entry(
        &self,
        world_id: &str,
        distance_m: f64,
        previous_distance_m: f64,
    ) -> Option<Segment> {
        let segments = self.segments_for_world(world_id).ok()?;

        for segment in segments {
            // Check if we crossed the start line (going forward)
            if previous_distance_m < segment.start_distance_m
                && distance_m >= segment.start_distance_m
            {
                return Some(segment);
            }
        }

        None
    }

    /// Check if rider has exited a segment.
    pub fn check_segment_exit(
        &self,
        active: &ActiveSegment,
        distance_m: f64,
    ) -> Option<SegmentCompletion> {
        if distance_m >= active.segment.end_distance_m {
            let elapsed = active.entry_time.elapsed();
            return Some(SegmentCompletion {
                segment: active.segment.clone(),
                elapsed_time_ms: elapsed.as_millis() as u32,
                avg_power_watts: active.avg_power_watts(),
                avg_hr_bpm: active.avg_hr_bpm(),
            });
        }
        None
    }
}

/// Segment-related errors.
#[derive(Debug, thiserror::Error)]
pub enum SegmentError {
    #[error("Segment not found: {0}")]
    SegmentNotFound(Uuid),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
