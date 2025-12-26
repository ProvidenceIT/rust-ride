//! Race results management.
//!
//! Records and retrieves race results.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::events::ParticipantStatus;
use crate::storage::Database;

/// Finish result for a rider.
#[derive(Debug, Clone)]
pub struct FinishResult {
    pub position: u32,
    pub finish_time_ms: u32,
    pub gap_to_winner_ms: i64,
    pub is_personal_best: bool,
}

/// Race results summary.
#[derive(Debug, Clone)]
pub struct RaceResultsSummary {
    pub race: RaceEventSummary,
    pub finishers: Vec<RaceFinisher>,
    pub dnf_count: u32,
    pub total_participants: u32,
}

/// Minimal race event info for results.
#[derive(Debug, Clone)]
pub struct RaceEventSummary {
    pub id: Uuid,
    pub name: String,
    pub world_id: String,
    pub distance_km: f64,
    pub date: DateTime<Utc>,
}

/// Race finisher.
#[derive(Debug, Clone)]
pub struct RaceFinisher {
    pub position: u32,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub finish_time_ms: u32,
    pub gap_to_winner_ms: i64,
}

/// Race history entry.
#[derive(Debug, Clone)]
pub struct RaceHistoryEntry {
    pub race_name: String,
    pub date: DateTime<Utc>,
    pub distance_km: f64,
    pub position: Option<u32>,
    pub total_racers: u32,
    pub finish_time_ms: Option<u32>,
    pub dnf: bool,
}

/// Race results manager.
pub struct RaceResults {
    db: Arc<Database>,
}

impl RaceResults {
    /// Create a new race results manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Record a race finish.
    pub fn record_finish(
        &self,
        race_id: Uuid,
        rider_id: Uuid,
        finish_time_ms: u32,
    ) -> Result<FinishResult, ResultsError> {
        let conn = self.db.connection();

        // Get current position (count of finishers + 1)
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(*) FROM race_participants
                 WHERE race_id = ?1 AND status = 'finished'",
            )
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let finished_count: u32 = stmt
            .query_row([race_id.to_string()], |row| row.get(0))
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let position = finished_count + 1;

        // Get winner's time for gap calculation
        let winner_time: Option<u32> = if position == 1 {
            None
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT finish_time_ms FROM race_participants
                     WHERE race_id = ?1 AND finish_position = 1",
                )
                .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

            stmt.query_row([race_id.to_string()], |row| row.get(0)).ok()
        };

        let gap_to_winner = winner_time.map_or(0, |wt| finish_time_ms as i64 - wt as i64);

        // Update participant record
        conn.execute(
            "UPDATE race_participants SET status = ?2, finish_time_ms = ?3, finish_position = ?4
             WHERE race_id = ?1 AND rider_id = ?5",
            rusqlite::params![
                race_id.to_string(),
                ParticipantStatus::Finished.as_str(),
                finish_time_ms,
                position,
                rider_id.to_string(),
            ],
        )
        .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        // Check for personal best
        let is_pb = self.is_personal_best(race_id, rider_id, finish_time_ms)?;

        Ok(FinishResult {
            position,
            finish_time_ms,
            gap_to_winner_ms: gap_to_winner,
            is_personal_best: is_pb,
        })
    }

    /// Mark participant as DNF.
    pub fn record_dnf(&self, race_id: Uuid, rider_id: Uuid) -> Result<(), ResultsError> {
        let conn = self.db.connection();

        conn.execute(
            "UPDATE race_participants SET status = ?2, disconnected_at = ?3
             WHERE race_id = ?1 AND rider_id = ?4",
            rusqlite::params![
                race_id.to_string(),
                ParticipantStatus::Dnf.as_str(),
                Utc::now().to_rfc3339(),
                rider_id.to_string(),
            ],
        )
        .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get final results for a race.
    pub fn get_results(&self, race_id: Uuid) -> Result<RaceResultsSummary, ResultsError> {
        let conn = self.db.connection();

        // Get race info
        let mut stmt = conn
            .prepare("SELECT id, name, world_id, distance_km, scheduled_start FROM race_events WHERE id = ?1")
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let (id_str, name, world_id, distance_km, date_str): (String, String, String, f64, String) = stmt
            .query_row([race_id.to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(|_| ResultsError::RaceNotFound(race_id))?;

        let race = RaceEventSummary {
            id: Uuid::parse_str(&id_str).map_err(|e| ResultsError::DatabaseError(e.to_string()))?,
            name,
            world_id,
            distance_km,
            date: DateTime::parse_from_rfc3339(&date_str)
                .map_err(|e| ResultsError::DatabaseError(e.to_string()))?
                .with_timezone(&Utc),
        };

        // Get finishers
        let mut stmt = conn
            .prepare(
                "SELECT p.rider_id, r.display_name, p.finish_time_ms, p.finish_position
                 FROM race_participants p
                 LEFT JOIN riders r ON p.rider_id = r.id
                 WHERE p.race_id = ?1 AND p.status = 'finished'
                 ORDER BY p.finish_position",
            )
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([race_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, u32>(3)?,
                ))
            })
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let mut finishers = Vec::new();
        let mut winner_time: Option<u32> = None;

        for row in rows {
            let (rider_id_str, rider_name, finish_time_ms, position) =
                row.map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

            if position == 1 {
                winner_time = Some(finish_time_ms);
            }

            let gap = winner_time.map_or(0, |wt| finish_time_ms as i64 - wt as i64);

            finishers.push(RaceFinisher {
                position,
                rider_id: Uuid::parse_str(&rider_id_str)
                    .map_err(|e| ResultsError::DatabaseError(e.to_string()))?,
                rider_name: rider_name.unwrap_or_else(|| "Unknown".to_string()),
                finish_time_ms,
                gap_to_winner_ms: gap,
            });
        }

        // Get DNF count
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM race_participants WHERE race_id = ?1 AND status = 'dnf'")
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let dnf_count: u32 = stmt
            .query_row([race_id.to_string()], |row| row.get(0))
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        // Get total participants
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM race_participants WHERE race_id = ?1")
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let total: u32 = stmt
            .query_row([race_id.to_string()], |row| row.get(0))
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        Ok(RaceResultsSummary {
            race,
            finishers,
            dnf_count,
            total_participants: total,
        })
    }

    /// Get rider's race history.
    pub fn rider_history(&self, rider_id: Uuid) -> Result<Vec<RaceHistoryEntry>, ResultsError> {
        let conn = self.db.connection();

        let mut stmt = conn
            .prepare(
                "SELECT r.name, r.scheduled_start, r.distance_km, p.finish_position, p.finish_time_ms, p.status,
                        (SELECT COUNT(*) FROM race_participants WHERE race_id = r.id) as total
                 FROM race_events r
                 JOIN race_participants p ON r.id = p.race_id
                 WHERE p.rider_id = ?1 AND r.status = 'finished'
                 ORDER BY r.scheduled_start DESC",
            )
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([rider_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, Option<u32>>(3)?,
                    row.get::<_, Option<u32>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, u32>(6)?,
                ))
            })
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let mut history = Vec::new();
        for row in rows {
            let (name, date_str, distance_km, position, finish_time, status_str, total) =
                row.map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

            let dnf = ParticipantStatus::from_str(&status_str) == Some(ParticipantStatus::Dnf);

            history.push(RaceHistoryEntry {
                race_name: name,
                date: DateTime::parse_from_rfc3339(&date_str)
                    .map_err(|e| ResultsError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
                distance_km,
                position,
                total_racers: total,
                finish_time_ms: finish_time,
                dnf,
            });
        }

        Ok(history)
    }

    fn is_personal_best(
        &self,
        race_id: Uuid,
        rider_id: Uuid,
        finish_time_ms: u32,
    ) -> Result<bool, ResultsError> {
        let conn = self.db.connection();

        // Get the race distance
        let mut stmt = conn
            .prepare("SELECT distance_km FROM race_events WHERE id = ?1")
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let distance_km: f64 = stmt
            .query_row([race_id.to_string()], |row| row.get(0))
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        // Check previous best for similar distance
        let distance_tolerance = distance_km * 0.1; // 10% tolerance

        let mut stmt = conn
            .prepare(
                "SELECT MIN(p.finish_time_ms) FROM race_participants p
                 JOIN race_events r ON p.race_id = r.id
                 WHERE p.rider_id = ?1 AND p.status = 'finished'
                 AND r.distance_km BETWEEN ?2 AND ?3
                 AND r.id != ?4",
            )
            .map_err(|e| ResultsError::DatabaseError(e.to_string()))?;

        let prev_best: Option<u32> = stmt
            .query_row(
                rusqlite::params![
                    rider_id.to_string(),
                    distance_km - distance_tolerance,
                    distance_km + distance_tolerance,
                    race_id.to_string(),
                ],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        Ok(prev_best.map_or(true, |pb| finish_time_ms < pb))
    }
}

/// Results errors.
#[derive(Debug, thiserror::Error)]
pub enum ResultsError {
    #[error("Race not found: {0}")]
    RaceNotFound(Uuid),

    #[error("Participant not found")]
    ParticipantNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
