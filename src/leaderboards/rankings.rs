//! Leaderboard rankings service.
//!
//! Provides leaderboard queries and personal bests.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::segments::{Segment, SegmentCategory};
use crate::storage::Database;

/// Leaderboard filter options.
#[derive(Debug, Clone, Default)]
pub struct LeaderboardFilter {
    pub limit: usize,
    pub time_range: Option<TimeRange>,
    pub rider_filter: Option<RiderFilter>,
}

/// Time range filter.
#[derive(Debug, Clone, Copy)]
pub enum TimeRange {
    AllTime,
    ThisWeek,
    ThisMonth,
    ThisYear,
}

/// Rider filter.
#[derive(Debug, Clone)]
pub enum RiderFilter {
    All,
    Local,
    Club(Uuid),
}

/// Leaderboard entry.
#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub elapsed_time_ms: u32,
    pub avg_power_watts: Option<u16>,
    pub recorded_at: DateTime<Utc>,
    pub is_current_rider: bool,
}

/// Leaderboard for a segment.
#[derive(Debug, Clone)]
pub struct Leaderboard {
    pub segment: Segment,
    pub entries: Vec<LeaderboardEntry>,
    pub total_efforts: u32,
    pub rider_entry: Option<LeaderboardEntry>,
}

/// Personal best record.
#[derive(Debug, Clone)]
pub struct PersonalBest {
    pub segment: Segment,
    pub elapsed_time_ms: u32,
    pub recorded_at: DateTime<Utc>,
    pub rank: u32,
}

/// Rank information.
#[derive(Debug, Clone)]
pub struct RankInfo {
    pub rank: u32,
    pub total_riders: u32,
    pub best_time_ms: u32,
    pub percentile: f64,
}

/// Leaderboard service.
pub struct LeaderboardService {
    db: Arc<Database>,
}

impl LeaderboardService {
    /// Create a new leaderboard service.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get leaderboard for a segment.
    pub fn get_leaderboard(
        &self,
        segment_id: Uuid,
        filter: LeaderboardFilter,
        current_rider_id: Uuid,
    ) -> Result<Leaderboard, LeaderboardError> {
        let conn = self.db.connection();

        // Get segment
        let segment = self.get_segment(segment_id)?;

        // Get total efforts
        let total_efforts = self.get_total_efforts(&segment_id)?;

        // Build query based on filter
        let limit = if filter.limit > 0 { filter.limit } else { 10 };

        let mut stmt = conn
            .prepare(
                "SELECT e.rider_id, r.display_name, MIN(e.elapsed_time_ms) as best_time, e.avg_power_watts, e.recorded_at
                 FROM social_segment_efforts e
                 LEFT JOIN riders r ON e.rider_id = r.id
                 WHERE e.segment_id = ?1
                 GROUP BY e.rider_id
                 ORDER BY best_time ASC
                 LIMIT ?2",
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![segment_id.to_string(), limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, Option<u16>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let mut entries = Vec::new();
        let mut rank = 0u32;
        let mut rider_entry = None;

        for row in rows {
            rank += 1;
            let (rider_id_str, rider_name, elapsed_time_ms, avg_power, recorded_at_str) =
                row.map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

            let rider_id = Uuid::parse_str(&rider_id_str)
                .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;
            let is_current = rider_id == current_rider_id;

            let entry = LeaderboardEntry {
                rank,
                rider_id,
                rider_name: rider_name.unwrap_or_else(|| "Unknown".to_string()),
                elapsed_time_ms,
                avg_power_watts: avg_power,
                recorded_at: DateTime::parse_from_rfc3339(&recorded_at_str)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
                is_current_rider: is_current,
            };

            if is_current {
                rider_entry = Some(entry.clone());
            }

            entries.push(entry);
        }

        Ok(Leaderboard {
            segment,
            entries,
            total_efforts,
            rider_entry,
        })
    }

    /// Get personal bests for a rider.
    pub fn personal_bests(&self, rider_id: Uuid) -> Result<Vec<PersonalBest>, LeaderboardError> {
        let conn = self.db.connection();

        let mut stmt = conn
            .prepare(
                "SELECT e.segment_id, MIN(e.elapsed_time_ms) as best_time, e.recorded_at
                 FROM social_segment_efforts e
                 WHERE e.rider_id = ?1
                 GROUP BY e.segment_id",
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([rider_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let mut pbs = Vec::new();
        for row in rows {
            let (segment_id_str, elapsed_time_ms, recorded_at_str) =
                row.map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

            let segment_id = Uuid::parse_str(&segment_id_str)
                .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

            let segment = self.get_segment(segment_id)?;
            let rank = self.get_rank(segment_id, elapsed_time_ms)?;

            pbs.push(PersonalBest {
                segment,
                elapsed_time_ms,
                recorded_at: DateTime::parse_from_rfc3339(&recorded_at_str)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
                rank,
            });
        }

        Ok(pbs)
    }

    /// Get rider's rank on a segment.
    pub fn rider_rank(
        &self,
        segment_id: Uuid,
        rider_id: Uuid,
    ) -> Result<Option<RankInfo>, LeaderboardError> {
        let conn = self.db.connection();

        // Get rider's best time
        let mut stmt = conn
            .prepare(
                "SELECT MIN(elapsed_time_ms) FROM social_segment_efforts
                 WHERE segment_id = ?1 AND rider_id = ?2",
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let best_time: Option<u32> = stmt
            .query_row(
                rusqlite::params![segment_id.to_string(), rider_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let best_time = match best_time {
            Some(t) => t,
            None => return Ok(None),
        };

        let rank = self.get_rank(segment_id, best_time)?;
        let total_riders = self.get_unique_riders(&segment_id)?;
        let percentile = if total_riders > 0 {
            100.0 * (1.0 - (rank as f64 / total_riders as f64))
        } else {
            0.0
        };

        Ok(Some(RankInfo {
            rank,
            total_riders,
            best_time_ms: best_time,
            percentile,
        }))
    }

    fn get_segment(&self, segment_id: Uuid) -> Result<Segment, LeaderboardError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, world_id, name, start_distance_m, end_distance_m, category, elevation_gain_m
                 FROM social_segments WHERE id = ?1",
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let mut rows = stmt
            .query([segment_id.to_string()])
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?
        {
            let id_str: String = row
                .get(0)
                .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;
            let category_str: String = row
                .get(5)
                .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

            Ok(Segment {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?,
                world_id: row
                    .get(1)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?,
                name: row
                    .get(2)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?,
                start_distance_m: row
                    .get(3)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?,
                end_distance_m: row
                    .get(4)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?,
                category: SegmentCategory::from_str(&category_str)
                    .unwrap_or(SegmentCategory::Mixed),
                elevation_gain_m: row
                    .get(6)
                    .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?,
            })
        } else {
            Err(LeaderboardError::SegmentNotFound(segment_id))
        }
    }

    fn get_total_efforts(&self, segment_id: &Uuid) -> Result<u32, LeaderboardError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM social_segment_efforts WHERE segment_id = ?1")
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        stmt.query_row([segment_id.to_string()], |row| row.get(0))
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))
    }

    fn get_unique_riders(&self, segment_id: &Uuid) -> Result<u32, LeaderboardError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare("SELECT COUNT(DISTINCT rider_id) FROM social_segment_efforts WHERE segment_id = ?1")
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        stmt.query_row([segment_id.to_string()], |row| row.get(0))
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))
    }

    fn get_rank(&self, segment_id: Uuid, elapsed_time_ms: u32) -> Result<u32, LeaderboardError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(DISTINCT rider_id) FROM social_segment_efforts
                 WHERE segment_id = ?1 AND elapsed_time_ms < ?2",
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        let count: u32 = stmt
            .query_row(
                rusqlite::params![segment_id.to_string(), elapsed_time_ms],
                |row| row.get(0),
            )
            .map_err(|e| LeaderboardError::DatabaseError(e.to_string()))?;

        Ok(count + 1)
    }
}

/// Leaderboard errors.
#[derive(Debug, thiserror::Error)]
pub enum LeaderboardError {
    #[error("Segment not found: {0}")]
    SegmentNotFound(Uuid),

    #[error("No efforts recorded")]
    NoEfforts,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
