//! Training challenge management.
//!
//! Handles challenge creation, progress tracking, and completion.

use chrono::{DateTime, NaiveDate, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::types::{Challenge, ChallengeProgress, GoalType};
use crate::storage::Database;

/// Challenge manager.
pub struct ChallengeManager {
    db: Arc<Database>,
}

impl ChallengeManager {
    /// Create a new challenge manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new challenge.
    pub fn create_challenge(
        &self,
        name: String,
        description: Option<String>,
        goal_type: GoalType,
        goal_value: f64,
        duration_days: u16,
        start_date: NaiveDate,
        created_by: Uuid,
    ) -> Result<Challenge, ChallengeError> {
        let id = Uuid::new_v4();
        let end_date = start_date + chrono::Duration::days(duration_days as i64);
        let now = Utc::now();

        let conn = self.db.connection();
        conn.execute(
            "INSERT INTO challenges (id, name, description, goal_type, goal_value, duration_days, start_date, end_date, created_by_rider_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id.to_string(),
                name,
                description,
                goal_type.as_str(),
                goal_value,
                duration_days,
                start_date.to_string(),
                end_date.to_string(),
                created_by.to_string(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        Ok(Challenge {
            id,
            name,
            description,
            goal_type,
            goal_value,
            duration_days,
            start_date,
            end_date,
            created_by_rider_id: Some(created_by),
            created_at: now,
        })
    }

    /// Join a challenge.
    pub fn join_challenge(
        &self,
        challenge_id: Uuid,
        rider_id: Uuid,
    ) -> Result<ChallengeProgress, ChallengeError> {
        let conn = self.db.connection();

        // Check if already joined
        let mut check_stmt = conn
            .prepare("SELECT id FROM challenge_progress WHERE challenge_id = ?1 AND rider_id = ?2")
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        if check_stmt
            .exists(rusqlite::params![
                challenge_id.to_string(),
                rider_id.to_string()
            ])
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?
        {
            return Err(ChallengeError::AlreadyJoined);
        }

        let id = Uuid::new_v4();
        let now = Utc::now();

        conn.execute(
            "INSERT INTO challenge_progress (id, challenge_id, rider_id, current_value, completed, last_updated)
             VALUES (?1, ?2, ?3, 0, 0, ?4)",
            rusqlite::params![
                id.to_string(),
                challenge_id.to_string(),
                rider_id.to_string(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        Ok(ChallengeProgress {
            challenge_id,
            rider_id,
            current_value: 0.0,
            completed: false,
            completed_at: None,
            last_updated: now,
        })
    }

    /// Update challenge progress.
    pub fn update_progress(
        &self,
        challenge_id: Uuid,
        rider_id: Uuid,
        value_delta: f64,
    ) -> Result<ChallengeProgress, ChallengeError> {
        let conn = self.db.connection();
        let now = Utc::now();

        // Get challenge goal
        let challenge = self.get_challenge(challenge_id)?;

        // Get current progress
        let current = self.get_progress(challenge_id, rider_id)?;

        if current.completed {
            return Ok(current);
        }

        let new_value = current.current_value + value_delta;
        let completed = new_value >= challenge.goal_value;
        let completed_at = if completed { Some(now) } else { None };

        conn.execute(
            "UPDATE challenge_progress SET current_value = ?3, completed = ?4, completed_at = ?5, last_updated = ?6
             WHERE challenge_id = ?1 AND rider_id = ?2",
            rusqlite::params![
                challenge_id.to_string(),
                rider_id.to_string(),
                new_value,
                completed,
                completed_at.map(|dt| dt.to_rfc3339()),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        Ok(ChallengeProgress {
            challenge_id,
            rider_id,
            current_value: new_value,
            completed,
            completed_at,
            last_updated: now,
        })
    }

    /// Get challenge progress for a rider.
    pub fn get_progress(
        &self,
        challenge_id: Uuid,
        rider_id: Uuid,
    ) -> Result<ChallengeProgress, ChallengeError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT current_value, completed, completed_at, last_updated
                 FROM challenge_progress WHERE challenge_id = ?1 AND rider_id = ?2",
            )
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        let mut rows = stmt
            .query(rusqlite::params![
                challenge_id.to_string(),
                rider_id.to_string()
            ])
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?
        {
            let completed_str: Option<String> = row
                .get(2)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let last_updated_str: String = row
                .get(3)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

            Ok(ChallengeProgress {
                challenge_id,
                rider_id,
                current_value: row
                    .get(0)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                completed: row
                    .get(1)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                completed_at: completed_str
                    .map(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .flatten()
                    .map(|dt| dt.with_timezone(&Utc)),
                last_updated: DateTime::parse_from_rfc3339(&last_updated_str)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            })
        } else {
            Err(ChallengeError::NotJoined)
        }
    }

    /// Get a challenge by ID.
    pub fn get_challenge(&self, challenge_id: Uuid) -> Result<Challenge, ChallengeError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, goal_type, goal_value, duration_days, start_date, end_date, created_by_rider_id, created_at
                 FROM challenges WHERE id = ?1",
            )
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        let mut rows = stmt
            .query([challenge_id.to_string()])
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?
        {
            let id_str: String = row
                .get(0)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let goal_type_str: String = row
                .get(3)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let start_str: String = row
                .get(6)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let end_str: String = row
                .get(7)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let created_by_str: Option<String> = row
                .get(8)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let created_str: String = row
                .get(9)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

            Ok(Challenge {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                name: row
                    .get(1)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                description: row
                    .get(2)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                goal_type: GoalType::from_str(&goal_type_str).unwrap_or(GoalType::TotalDistanceKm),
                goal_value: row
                    .get(4)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                duration_days: row
                    .get(5)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                start_date: NaiveDate::parse_from_str(&start_str, "%Y-%m-%d")
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                end_date: NaiveDate::parse_from_str(&end_str, "%Y-%m-%d")
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?,
                created_by_rider_id: created_by_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            })
        } else {
            Err(ChallengeError::NotFound(challenge_id))
        }
    }

    /// Get active challenges for a rider.
    pub fn get_active_challenges(
        &self,
        rider_id: Uuid,
    ) -> Result<Vec<(Challenge, ChallengeProgress)>, ChallengeError> {
        let conn = self.db.connection();
        let today = Utc::now().date_naive().to_string();

        let mut stmt = conn
            .prepare(
                "SELECT c.id FROM challenges c
                 JOIN challenge_progress cp ON c.id = cp.challenge_id
                 WHERE cp.rider_id = ?1 AND cp.completed = 0 AND c.end_date >= ?2
                 ORDER BY c.end_date",
            )
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![rider_id.to_string(), today], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let id_str = row.map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let challenge_id = Uuid::parse_str(&id_str)
                .map_err(|e| ChallengeError::DatabaseError(e.to_string()))?;
            let challenge = self.get_challenge(challenge_id)?;
            let progress = self.get_progress(challenge_id, rider_id)?;
            results.push((challenge, progress));
        }

        Ok(results)
    }
}

/// Challenge errors.
#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Challenge not found: {0}")]
    NotFound(Uuid),

    #[error("Already joined this challenge")]
    AlreadyJoined,

    #[error("Not joined this challenge")]
    NotJoined,

    #[error("Challenge has ended")]
    ChallengeEnded,

    #[error("Invalid goal value")]
    InvalidGoalValue,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
