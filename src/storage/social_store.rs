//! Social data storage operations.
//!
//! Provides persistence for:
//! - Rider profiles
//! - Clubs and memberships
//! - Badges
//! - Group rides
//! - Chat messages
//! - Activity summaries

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::storage::database::DatabaseError;

/// Rider profile for social features.
#[derive(Debug, Clone)]
pub struct Rider {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_id: Option<String>,
    pub bio: Option<String>,
    pub ftp: Option<u16>,
    pub total_distance_km: f64,
    pub total_time_hours: f64,
    pub sharing_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Club for organizing riders.
#[derive(Debug, Clone)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub join_code: String,
    pub admin_rider_id: Uuid,
    pub total_distance_km: f64,
    pub total_time_hours: f64,
    pub created_at: DateTime<Utc>,
}

/// Club membership record.
#[derive(Debug, Clone)]
pub struct ClubMembership {
    pub id: Uuid,
    pub club_id: Uuid,
    pub rider_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

/// Group ride session.
#[derive(Debug, Clone)]
pub struct GroupRideRecord {
    pub id: Uuid,
    pub host_rider_id: Uuid,
    pub name: Option<String>,
    pub world_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub max_participants: u8,
}

/// Group ride participant.
#[derive(Debug, Clone)]
pub struct GroupRideParticipant {
    pub id: Uuid,
    pub group_ride_id: Uuid,
    pub rider_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

/// Chat message.
#[derive(Debug, Clone)]
pub struct ChatMessageRecord {
    pub id: Uuid,
    pub group_ride_id: Uuid,
    pub sender_rider_id: Uuid,
    pub message_text: String,
    pub sent_at: DateTime<Utc>,
}

/// Activity summary for sharing.
#[derive(Debug, Clone)]
pub struct ActivitySummary {
    pub id: Uuid,
    pub ride_id: Option<Uuid>,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub distance_km: f64,
    pub duration_minutes: u32,
    pub avg_power_watts: Option<u16>,
    pub elevation_gain_m: f64,
    pub world_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
    pub shared: bool,
}

/// Social store for persisting social and multiplayer data.
pub struct SocialStore<'a> {
    conn: &'a Connection,
}

impl<'a> SocialStore<'a> {
    /// Create a new social store with the given connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ========== Rider Operations ==========

    /// Get or create the local rider profile.
    pub fn get_or_create_rider(&self, user_id: &Uuid) -> Result<Rider, DatabaseError> {
        // Try to get existing rider
        if let Some(rider) = self.get_rider(user_id)? {
            return Ok(rider);
        }

        // Create new rider with default values
        let now = Utc::now();
        let rider = Rider {
            id: *user_id,
            display_name: format!("Rider{}", &user_id.to_string()[..4]),
            avatar_id: None,
            bio: None,
            ftp: None,
            total_distance_km: 0.0,
            total_time_hours: 0.0,
            sharing_enabled: true,
            created_at: now,
            updated_at: now,
        };

        self.insert_rider(&rider)?;
        Ok(rider)
    }

    /// Get a rider by ID.
    pub fn get_rider(&self, rider_id: &Uuid) -> Result<Option<Rider>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, display_name, avatar_id, bio, ftp, total_distance_km,
                        total_time_hours, sharing_enabled, created_at, updated_at
                 FROM riders WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query(params![rider_id.to_string()])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| DatabaseError::QueryFailed(e.to_string()))? {
            Ok(Some(self.row_to_rider(row)?))
        } else {
            Ok(None)
        }
    }

    /// Insert a new rider.
    pub fn insert_rider(&self, rider: &Rider) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO riders (id, display_name, avatar_id, bio, ftp, total_distance_km,
                                    total_time_hours, sharing_enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    rider.id.to_string(),
                    rider.display_name,
                    rider.avatar_id,
                    rider.bio,
                    rider.ftp,
                    rider.total_distance_km,
                    rider.total_time_hours,
                    rider.sharing_enabled,
                    rider.created_at.to_rfc3339(),
                    rider.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Update a rider profile.
    pub fn update_rider(&self, rider: &Rider) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "UPDATE riders SET display_name = ?2, avatar_id = ?3, bio = ?4, ftp = ?5,
                                   total_distance_km = ?6, total_time_hours = ?7,
                                   sharing_enabled = ?8, updated_at = ?9
                 WHERE id = ?1",
                params![
                    rider.id.to_string(),
                    rider.display_name,
                    rider.avatar_id,
                    rider.bio,
                    rider.ftp,
                    rider.total_distance_km,
                    rider.total_time_hours,
                    rider.sharing_enabled,
                    Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    fn row_to_rider(&self, row: &rusqlite::Row<'_>) -> Result<Rider, DatabaseError> {
        let id_str: String = row.get(0).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        let created_str: String =
            row.get(8).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        let updated_str: String =
            row.get(9).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(Rider {
            id: Uuid::parse_str(&id_str).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            display_name: row.get(1).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            avatar_id: row.get(2).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            bio: row.get(3).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            ftp: row.get(4).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            total_distance_km: row.get(5).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            total_time_hours: row.get(6).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            sharing_enabled: row.get(7).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?
                .with_timezone(&Utc),
        })
    }

    // ========== Group Ride Operations ==========

    /// Insert a new group ride.
    pub fn insert_group_ride(&self, ride: &GroupRideRecord) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO group_rides (id, host_rider_id, name, world_id, started_at, ended_at, max_participants)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    ride.id.to_string(),
                    ride.host_rider_id.to_string(),
                    ride.name,
                    ride.world_id,
                    ride.started_at.to_rfc3339(),
                    ride.ended_at.map(|dt| dt.to_rfc3339()),
                    ride.max_participants,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Update group ride (e.g., set ended_at).
    pub fn update_group_ride(&self, ride: &GroupRideRecord) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "UPDATE group_rides SET name = ?2, ended_at = ?3 WHERE id = ?1",
                params![
                    ride.id.to_string(),
                    ride.name,
                    ride.ended_at.map(|dt| dt.to_rfc3339()),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get a group ride by ID.
    pub fn get_group_ride(&self, ride_id: &Uuid) -> Result<Option<GroupRideRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, host_rider_id, name, world_id, started_at, ended_at, max_participants
                 FROM group_rides WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query(params![ride_id.to_string()])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| DatabaseError::QueryFailed(e.to_string()))? {
            Ok(Some(self.row_to_group_ride(row)?))
        } else {
            Ok(None)
        }
    }

    fn row_to_group_ride(&self, row: &rusqlite::Row<'_>) -> Result<GroupRideRecord, DatabaseError> {
        let id_str: String = row.get(0).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        let host_str: String = row.get(1).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        let started_str: String =
            row.get(4).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        let ended_str: Option<String> =
            row.get(5).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(GroupRideRecord {
            id: Uuid::parse_str(&id_str).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            host_rider_id: Uuid::parse_str(&host_str)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            name: row.get(2).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            world_id: row.get(3).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            started_at: DateTime::parse_from_rfc3339(&started_str)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?
                .with_timezone(&Utc),
            ended_at: ended_str
                .map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                })
                .flatten(),
            max_participants: row.get(6).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
        })
    }

    // ========== Group Ride Participant Operations ==========

    /// Add a participant to a group ride.
    pub fn add_group_ride_participant(
        &self,
        participant: &GroupRideParticipant,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO group_ride_participants (id, group_ride_id, rider_id, joined_at, left_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    participant.id.to_string(),
                    participant.group_ride_id.to_string(),
                    participant.rider_id.to_string(),
                    participant.joined_at.to_rfc3339(),
                    participant.left_at.map(|dt| dt.to_rfc3339()),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Mark participant as left.
    pub fn update_participant_left(
        &self,
        participant_id: &Uuid,
        left_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "UPDATE group_ride_participants SET left_at = ?2 WHERE id = ?1",
                params![participant_id.to_string(), left_at.to_rfc3339(),],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get participants for a group ride.
    pub fn get_group_ride_participants(
        &self,
        ride_id: &Uuid,
    ) -> Result<Vec<GroupRideParticipant>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, group_ride_id, rider_id, joined_at, left_at
                 FROM group_ride_participants WHERE group_ride_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![ride_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let ride_str: String = row.get(1)?;
                let rider_str: String = row.get(2)?;
                let joined_str: String = row.get(3)?;
                let left_str: Option<String> = row.get(4)?;

                Ok((id_str, ride_str, rider_str, joined_str, left_str))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut participants = Vec::new();
        for row in rows {
            let (id_str, ride_str, rider_str, joined_str, left_str) =
                row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            participants.push(GroupRideParticipant {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
                group_ride_id: Uuid::parse_str(&ride_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
                rider_id: Uuid::parse_str(&rider_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
                joined_at: DateTime::parse_from_rfc3339(&joined_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?
                    .with_timezone(&Utc),
                left_at: left_str
                    .map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    })
                    .flatten(),
            });
        }

        Ok(participants)
    }

    // ========== Chat Message Operations ==========

    /// Insert a chat message.
    pub fn insert_chat_message(&self, message: &ChatMessageRecord) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO chat_messages (id, group_ride_id, sender_rider_id, message_text, sent_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    message.id.to_string(),
                    message.group_ride_id.to_string(),
                    message.sender_rider_id.to_string(),
                    message.message_text,
                    message.sent_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get chat messages for a group ride.
    pub fn get_chat_messages(
        &self,
        ride_id: &Uuid,
        limit: usize,
    ) -> Result<Vec<ChatMessageRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, group_ride_id, sender_rider_id, message_text, sent_at
                 FROM chat_messages WHERE group_ride_id = ?1 ORDER BY sent_at DESC LIMIT ?2",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![ride_id.to_string(), limit], |row| {
                let id_str: String = row.get(0)?;
                let ride_str: String = row.get(1)?;
                let sender_str: String = row.get(2)?;
                let text: String = row.get(3)?;
                let sent_str: String = row.get(4)?;

                Ok((id_str, ride_str, sender_str, text, sent_str))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut messages = Vec::new();
        for row in rows {
            let (id_str, ride_str, sender_str, text, sent_str) =
                row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            messages.push(ChatMessageRecord {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
                group_ride_id: Uuid::parse_str(&ride_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
                sender_rider_id: Uuid::parse_str(&sender_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
                message_text: text,
                sent_at: DateTime::parse_from_rfc3339(&sent_str)
                    .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    // ========== Activity Summary Operations ==========

    /// Insert an activity summary.
    pub fn insert_activity_summary(&self, summary: &ActivitySummary) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO activity_summaries (id, ride_id, rider_id, rider_name, distance_km,
                                                 duration_minutes, avg_power_watts, elevation_gain_m,
                                                 world_id, recorded_at, shared)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    summary.id.to_string(),
                    summary.ride_id.map(|id| id.to_string()),
                    summary.rider_id.to_string(),
                    summary.rider_name,
                    summary.distance_km,
                    summary.duration_minutes,
                    summary.avg_power_watts,
                    summary.elevation_gain_m,
                    summary.world_id,
                    summary.recorded_at.to_rfc3339(),
                    summary.shared,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get recent activity summaries for a rider.
    pub fn get_rider_activities(
        &self,
        rider_id: &Uuid,
        limit: usize,
    ) -> Result<Vec<ActivitySummary>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ride_id, rider_id, rider_name, distance_km, duration_minutes,
                        avg_power_watts, elevation_gain_m, world_id, recorded_at, shared
                 FROM activity_summaries WHERE rider_id = ?1 ORDER BY recorded_at DESC LIMIT ?2",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![rider_id.to_string(), limit], |row| {
                Ok(ActivitySummaryRow {
                    id: row.get(0)?,
                    ride_id: row.get(1)?,
                    rider_id: row.get(2)?,
                    rider_name: row.get(3)?,
                    distance_km: row.get(4)?,
                    duration_minutes: row.get(5)?,
                    avg_power_watts: row.get(6)?,
                    elevation_gain_m: row.get(7)?,
                    world_id: row.get(8)?,
                    recorded_at: row.get(9)?,
                    shared: row.get(10)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut summaries = Vec::new();
        for row in rows {
            let r = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            summaries.push(self.row_data_to_activity_summary(r)?);
        }

        Ok(summaries)
    }

    fn row_data_to_activity_summary(
        &self,
        r: ActivitySummaryRow,
    ) -> Result<ActivitySummary, DatabaseError> {
        Ok(ActivitySummary {
            id: Uuid::parse_str(&r.id).map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            ride_id: r
                .ride_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            rider_id: Uuid::parse_str(&r.rider_id)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?,
            rider_name: r.rider_name,
            distance_km: r.distance_km,
            duration_minutes: r.duration_minutes,
            avg_power_watts: r.avg_power_watts,
            elevation_gain_m: r.elevation_gain_m,
            world_id: r.world_id,
            recorded_at: DateTime::parse_from_rfc3339(&r.recorded_at)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?
                .with_timezone(&Utc),
            shared: r.shared,
        })
    }
}

/// Helper struct for row data.
struct ActivitySummaryRow {
    id: String,
    ride_id: Option<String>,
    rider_id: String,
    rider_name: String,
    distance_km: f64,
    duration_minutes: u32,
    avg_power_watts: Option<u16>,
    elevation_gain_m: f64,
    world_id: Option<String>,
    recorded_at: String,
    shared: bool,
}
