//! Race event management.
//!
//! Handles race creation, discovery, and participation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::storage::Database;

/// Race status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaceStatus {
    /// Waiting for start time
    Scheduled,
    /// 60 second countdown active
    Countdown,
    /// Race running
    InProgress,
    /// Race completed
    Finished,
    /// Race cancelled
    Cancelled,
}

impl RaceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RaceStatus::Scheduled => "scheduled",
            RaceStatus::Countdown => "countdown",
            RaceStatus::InProgress => "in_progress",
            RaceStatus::Finished => "finished",
            RaceStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "scheduled" => Some(RaceStatus::Scheduled),
            "countdown" => Some(RaceStatus::Countdown),
            "in_progress" => Some(RaceStatus::InProgress),
            "finished" => Some(RaceStatus::Finished),
            "cancelled" => Some(RaceStatus::Cancelled),
            _ => None,
        }
    }
}

/// Participant status in a race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantStatus {
    /// Joined but race not started
    Registered,
    /// Currently racing
    Racing,
    /// Completed the race
    Finished,
    /// Did Not Finish (disconnected >60s)
    Dnf,
}

impl ParticipantStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParticipantStatus::Registered => "registered",
            ParticipantStatus::Racing => "racing",
            ParticipantStatus::Finished => "finished",
            ParticipantStatus::Dnf => "dnf",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "registered" => Some(ParticipantStatus::Registered),
            "racing" => Some(ParticipantStatus::Racing),
            "finished" => Some(ParticipantStatus::Finished),
            "dnf" => Some(ParticipantStatus::Dnf),
            _ => None,
        }
    }
}

/// Race event configuration.
#[derive(Debug, Clone)]
pub struct RaceConfig {
    pub name: String,
    pub world_id: String,
    pub route_id: String,
    pub distance_km: f64,
    pub scheduled_start: DateTime<Utc>,
}

/// Race event.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub created_at: DateTime<Utc>,
}

/// Race participation.
#[derive(Debug, Clone)]
pub struct RaceParticipation {
    pub race: RaceEvent,
    pub joined_at: DateTime<Utc>,
    pub position: u32,
}

/// Racer information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacerInfo {
    pub rider_id: Uuid,
    pub rider_name: String,
}

/// Race participant record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceParticipant {
    pub id: Uuid,
    pub race_id: Uuid,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub status: ParticipantStatus,
    pub distance_m: f64,
    pub elapsed_time_ms: u64,
    pub current_position: u32,
    pub current_power: u16,
    pub gap_to_leader_ms: i64,
    pub finish_time_ms: Option<u32>,
    pub finish_position: Option<u16>,
}

/// Active race state.
#[derive(Debug, Clone)]
pub struct ActiveRace {
    pub event: RaceEvent,
    pub state: RaceState,
    pub participants: Vec<RaceParticipant>,
    pub local_participant: RaceParticipant,
}

/// Race state.
#[derive(Debug, Clone)]
pub struct RaceState {
    pub status: RaceStatus,
    pub countdown_seconds: Option<u8>,
    pub elapsed_time_ms: u64,
    pub start_time: Option<DateTime<Utc>>,
}

/// Racing configuration constants.
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

/// Manages virtual race events.
pub struct RaceManager {
    db: Arc<Database>,
}

impl RaceManager {
    /// Create a new race manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new race event.
    pub fn create_race(
        &self,
        config: RaceConfig,
        organizer: RacerInfo,
    ) -> Result<RaceEvent, RacingError> {
        let now = Utc::now();

        // Validate scheduled start is in the future
        if config.scheduled_start <= now {
            return Err(RacingError::InvalidScheduledTime);
        }

        let id = Uuid::new_v4();
        let event = RaceEvent {
            id,
            name: config.name,
            world_id: config.world_id,
            route_id: config.route_id,
            distance_km: config.distance_km,
            scheduled_start: config.scheduled_start,
            status: RaceStatus::Scheduled,
            organizer,
            participant_count: 1, // Organizer is automatically registered
            created_at: now,
        };

        // Save to database
        let conn = self.db.connection();
        conn.execute(
            "INSERT INTO race_events (id, name, world_id, route_id, distance_km, scheduled_start, status, organizer_rider_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                event.id.to_string(),
                event.name,
                event.world_id,
                event.route_id,
                event.distance_km,
                event.scheduled_start.to_rfc3339(),
                event.status.as_str(),
                event.organizer.rider_id.to_string(),
                event.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        // Register organizer as participant
        let participant_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO race_participants (id, race_id, rider_id, status, joined_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                participant_id.to_string(),
                event.id.to_string(),
                event.organizer.rider_id.to_string(),
                ParticipantStatus::Registered.as_str(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        Ok(event)
    }

    /// Get upcoming races.
    pub fn get_upcoming_races(&self) -> Result<Vec<RaceEvent>, RacingError> {
        let conn = self.db.connection();
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn
            .prepare(
                "SELECT r.id, r.name, r.world_id, r.route_id, r.distance_km, r.scheduled_start,
                        r.status, r.organizer_rider_id, r.created_at,
                        (SELECT COUNT(*) FROM race_participants WHERE race_id = r.id) as participant_count,
                        rd.display_name
                 FROM race_events r
                 LEFT JOIN riders rd ON r.organizer_rider_id = rd.id
                 WHERE r.scheduled_start > ?1 AND r.status IN ('scheduled', 'countdown')
                 ORDER BY r.scheduled_start",
            )
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([now], |row| {
                let id_str: String = row.get(0)?;
                let status_str: String = row.get(6)?;
                let organizer_id_str: String = row.get(7)?;
                let created_str: String = row.get(8)?;
                let scheduled_str: String = row.get(5)?;
                let organizer_name: Option<String> = row.get(10)?;

                Ok((
                    id_str,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    scheduled_str,
                    status_str,
                    organizer_id_str,
                    created_str,
                    row.get::<_, u32>(9)?,
                    organizer_name,
                ))
            })
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        let mut events = Vec::new();
        for row in rows {
            let (id_str, name, world_id, route_id, distance_km, scheduled_str, status_str, organizer_id_str, created_str, count, organizer_name) =
                row.map_err(|e| RacingError::DatabaseError(e.to_string()))?;

            events.push(RaceEvent {
                id: Uuid::parse_str(&id_str).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                name,
                world_id,
                route_id,
                distance_km,
                scheduled_start: DateTime::parse_from_rfc3339(&scheduled_str)
                    .map_err(|e| RacingError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
                status: RaceStatus::from_str(&status_str).unwrap_or(RaceStatus::Scheduled),
                organizer: RacerInfo {
                    rider_id: Uuid::parse_str(&organizer_id_str)
                        .map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                    rider_name: organizer_name.unwrap_or_else(|| "Unknown".to_string()),
                },
                participant_count: count,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| RacingError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        Ok(events)
    }

    /// Join a race.
    pub fn join_race(&self, race_id: Uuid, rider_id: Uuid) -> Result<RaceParticipation, RacingError> {
        let conn = self.db.connection();

        // Check if race exists and is joinable
        let race = self.get_race(race_id)?;
        if race.status != RaceStatus::Scheduled {
            return Err(RacingError::RaceAlreadyStarted);
        }

        if race.participant_count >= constants::MAX_RACE_PARTICIPANTS as u32 {
            return Err(RacingError::RaceFull);
        }

        // Check if already registered
        let mut stmt = conn
            .prepare("SELECT id FROM race_participants WHERE race_id = ?1 AND rider_id = ?2")
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        let exists = stmt
            .exists(rusqlite::params![race_id.to_string(), rider_id.to_string()])
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        if exists {
            return Err(RacingError::AlreadyRegistered);
        }

        // Register participant
        let now = Utc::now();
        let participant_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO race_participants (id, race_id, rider_id, status, joined_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                participant_id.to_string(),
                race_id.to_string(),
                rider_id.to_string(),
                ParticipantStatus::Registered.as_str(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        let position = race.participant_count + 1;
        Ok(RaceParticipation {
            race,
            joined_at: now,
            position,
        })
    }

    /// Leave a race.
    pub fn leave_race(&self, race_id: Uuid, rider_id: Uuid) -> Result<(), RacingError> {
        let race = self.get_race(race_id)?;

        if race.status != RaceStatus::Scheduled {
            return Err(RacingError::RaceAlreadyStarted);
        }

        // Can't leave if you're the organizer
        if race.organizer.rider_id == rider_id {
            return Err(RacingError::NotRegistered); // Use existing error
        }

        let conn = self.db.connection();
        let rows = conn
            .execute(
                "DELETE FROM race_participants WHERE race_id = ?1 AND rider_id = ?2",
                rusqlite::params![race_id.to_string(), rider_id.to_string()],
            )
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        if rows == 0 {
            return Err(RacingError::NotRegistered);
        }

        Ok(())
    }

    /// Get a race by ID.
    fn get_race(&self, race_id: Uuid) -> Result<RaceEvent, RacingError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT r.id, r.name, r.world_id, r.route_id, r.distance_km, r.scheduled_start,
                        r.status, r.organizer_rider_id, r.created_at,
                        (SELECT COUNT(*) FROM race_participants WHERE race_id = r.id) as participant_count,
                        rd.display_name
                 FROM race_events r
                 LEFT JOIN riders rd ON r.organizer_rider_id = rd.id
                 WHERE r.id = ?1",
            )
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        let mut rows = stmt
            .query([race_id.to_string()])
            .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| RacingError::DatabaseError(e.to_string()))? {
            let id_str: String = row.get(0).map_err(|e| RacingError::DatabaseError(e.to_string()))?;
            let status_str: String = row.get(6).map_err(|e| RacingError::DatabaseError(e.to_string()))?;
            let organizer_id_str: String = row.get(7).map_err(|e| RacingError::DatabaseError(e.to_string()))?;
            let created_str: String = row.get(8).map_err(|e| RacingError::DatabaseError(e.to_string()))?;
            let scheduled_str: String = row.get(5).map_err(|e| RacingError::DatabaseError(e.to_string()))?;
            let organizer_name: Option<String> = row.get(10).map_err(|e| RacingError::DatabaseError(e.to_string()))?;

            Ok(RaceEvent {
                id: Uuid::parse_str(&id_str).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                name: row.get(1).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                world_id: row.get(2).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                route_id: row.get(3).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                distance_km: row.get(4).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                scheduled_start: DateTime::parse_from_rfc3339(&scheduled_str)
                    .map_err(|e| RacingError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
                status: RaceStatus::from_str(&status_str).unwrap_or(RaceStatus::Scheduled),
                organizer: RacerInfo {
                    rider_id: Uuid::parse_str(&organizer_id_str)
                        .map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                    rider_name: organizer_name.unwrap_or_else(|| "Unknown".to_string()),
                },
                participant_count: row.get(9).map_err(|e| RacingError::DatabaseError(e.to_string()))?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| RacingError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            })
        } else {
            Err(RacingError::RaceNotFound(race_id))
        }
    }

    /// Cancel a race (organizer only).
    pub fn cancel_race(&self, race_id: Uuid, rider_id: Uuid) -> Result<(), RacingError> {
        let race = self.get_race(race_id)?;

        if race.organizer.rider_id != rider_id {
            return Err(RacingError::NotOrganizer);
        }

        if race.status == RaceStatus::InProgress {
            return Err(RacingError::RaceAlreadyStarted);
        }

        let conn = self.db.connection();
        conn.execute(
            "UPDATE race_events SET status = ?2 WHERE id = ?1",
            rusqlite::params![race_id.to_string(), RaceStatus::Cancelled.as_str()],
        )
        .map_err(|e| RacingError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// Racing errors.
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
    NetworkError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
