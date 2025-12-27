//! Club management.
//!
//! Provides club creation, membership, and aggregate stats.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::storage::Database;

/// Club information.
#[derive(Debug, Clone)]
pub struct ClubInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub join_code: String,
    pub admin_rider_id: Uuid,
    pub member_count: u32,
    pub total_distance_km: f64,
    pub total_time_hours: f64,
    pub created_at: DateTime<Utc>,
}

/// Club member.
#[derive(Debug, Clone)]
pub struct ClubMember {
    pub rider_id: Uuid,
    pub display_name: String,
    pub avatar_id: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub distance_km: f64,
    pub time_hours: f64,
}

/// Club manager.
pub struct ClubManager {
    db: Arc<Database>,
}

impl ClubManager {
    /// Create a new club manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new club.
    pub fn create_club(
        &self,
        name: String,
        description: Option<String>,
        admin_rider_id: Uuid,
    ) -> Result<ClubInfo, ClubError> {
        let id = Uuid::new_v4();
        let join_code = generate_join_code();
        let now = Utc::now();

        let conn = self.db.connection();
        conn.execute(
            "INSERT INTO clubs (id, name, description, join_code, admin_rider_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                id.to_string(),
                name,
                description,
                join_code,
                admin_rider_id.to_string(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        // Auto-add admin as member
        let membership_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO club_memberships (id, club_id, rider_id, joined_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                membership_id.to_string(),
                id.to_string(),
                admin_rider_id.to_string(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        Ok(ClubInfo {
            id,
            name,
            description,
            join_code,
            admin_rider_id,
            member_count: 1,
            total_distance_km: 0.0,
            total_time_hours: 0.0,
            created_at: now,
        })
    }

    /// Join a club by code.
    pub fn join_club(&self, join_code: &str, rider_id: Uuid) -> Result<ClubInfo, ClubError> {
        let conn = self.db.connection();

        // Find club by code
        let mut stmt = conn
            .prepare("SELECT id FROM clubs WHERE join_code = ?1")
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        let club_id_str: String = stmt
            .query_row([join_code], |row| row.get(0))
            .map_err(|_| ClubError::InvalidJoinCode)?;

        let club_id =
            Uuid::parse_str(&club_id_str).map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        // Check if already member
        let mut check_stmt = conn
            .prepare(
                "SELECT id FROM club_memberships WHERE club_id = ?1 AND rider_id = ?2 AND left_at IS NULL",
            )
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        if check_stmt
            .exists(rusqlite::params![club_id.to_string(), rider_id.to_string()])
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?
        {
            return Err(ClubError::AlreadyMember);
        }

        // Add membership
        let membership_id = Uuid::new_v4();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO club_memberships (id, club_id, rider_id, joined_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                membership_id.to_string(),
                club_id.to_string(),
                rider_id.to_string(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        self.get_club(club_id)
    }

    /// Leave a club.
    pub fn leave_club(&self, club_id: Uuid, rider_id: Uuid) -> Result<(), ClubError> {
        let conn = self.db.connection();
        let now = Utc::now();

        let rows = conn
            .execute(
                "UPDATE club_memberships SET left_at = ?3 WHERE club_id = ?1 AND rider_id = ?2 AND left_at IS NULL",
                rusqlite::params![club_id.to_string(), rider_id.to_string(), now.to_rfc3339()],
            )
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        if rows == 0 {
            return Err(ClubError::NotMember);
        }

        Ok(())
    }

    /// Get club information.
    pub fn get_club(&self, club_id: Uuid) -> Result<ClubInfo, ClubError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.name, c.description, c.join_code, c.admin_rider_id,
                        c.total_distance_km, c.total_time_hours, c.created_at,
                        (SELECT COUNT(*) FROM club_memberships WHERE club_id = c.id AND left_at IS NULL) as member_count
                 FROM clubs c WHERE c.id = ?1",
            )
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        let mut rows = stmt
            .query([club_id.to_string()])
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?
        {
            let id_str: String = row
                .get(0)
                .map_err(|e| ClubError::DatabaseError(e.to_string()))?;
            let admin_str: String = row
                .get(4)
                .map_err(|e| ClubError::DatabaseError(e.to_string()))?;
            let created_str: String = row
                .get(7)
                .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

            Ok(ClubInfo {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                name: row
                    .get(1)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                description: row
                    .get(2)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                join_code: row
                    .get(3)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                admin_rider_id: Uuid::parse_str(&admin_str)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                total_distance_km: row
                    .get(5)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                total_time_hours: row
                    .get(6)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                member_count: row
                    .get(8)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| ClubError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            })
        } else {
            Err(ClubError::NotFound(club_id))
        }
    }

    /// Get clubs a rider belongs to.
    pub fn get_rider_clubs(&self, rider_id: Uuid) -> Result<Vec<ClubInfo>, ClubError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT c.id FROM clubs c
                 JOIN club_memberships m ON c.id = m.club_id
                 WHERE m.rider_id = ?1 AND m.left_at IS NULL",
            )
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([rider_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(|e| ClubError::DatabaseError(e.to_string()))?;

        let mut clubs = Vec::new();
        for row in rows {
            let id_str = row.map_err(|e| ClubError::DatabaseError(e.to_string()))?;
            let club_id =
                Uuid::parse_str(&id_str).map_err(|e| ClubError::DatabaseError(e.to_string()))?;
            clubs.push(self.get_club(club_id)?);
        }

        Ok(clubs)
    }
}

/// Generate a random 8-character join code.
fn generate_join_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let mut state = seed;
    let mut code = String::with_capacity(8);

    for _ in 0..8 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state % CHARSET.len() as u64) as usize;
        code.push(CHARSET[idx] as char);
    }

    code
}

/// Club errors.
#[derive(Debug, thiserror::Error)]
pub enum ClubError {
    #[error("Club not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid join code")]
    InvalidJoinCode,

    #[error("Already a member")]
    AlreadyMember,

    #[error("Not a member")]
    NotMember,

    #[error("Not club admin")]
    NotAdmin,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
