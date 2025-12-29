//! Storage operations for power profiles.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use uuid::Uuid;

/// Power profile record from the database.
#[derive(Debug, Clone)]
pub struct PowerProfileRecord {
    pub id: i64,
    pub user_id: i64,
    pub profile_type: String,
    pub recorded_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Power profile point record from the database.
#[derive(Debug, Clone)]
pub struct PowerProfilePointRecord {
    pub profile_id: i64,
    pub duration_secs: u32,
    pub power_watts: u16,
    pub achieved_at: DateTime<Utc>,
    pub ride_id: Option<Uuid>,
}

/// Storage operations for power profiles.
pub struct PowerProfileStore;

impl PowerProfileStore {
    /// Get the current profile for a user and type.
    pub fn get_current(
        conn: &Connection,
        user_id: i64,
        profile_type: &str,
    ) -> Result<Option<PowerProfileRecord>> {
        let result = conn.query_row(
            "SELECT id, user_id, profile_type, recorded_at, is_current
             FROM power_profiles
             WHERE user_id = ? AND profile_type = ? AND is_current = 1",
            params![user_id, profile_type],
            |row| {
                Ok(PowerProfileRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    profile_type: row.get(2)?,
                    recorded_at: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    is_current: row.get::<_, i64>(4)? != 0,
                })
            },
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Create a new profile and mark it as current.
    pub fn create_profile(conn: &Connection, user_id: i64, profile_type: &str) -> Result<i64> {
        let now = Utc::now();

        // Mark existing current profile as not current
        conn.execute(
            "UPDATE power_profiles SET is_current = 0
             WHERE user_id = ? AND profile_type = ? AND is_current = 1",
            params![user_id, profile_type],
        )?;

        // Create new profile
        conn.execute(
            "INSERT INTO power_profiles (user_id, profile_type, recorded_at, is_current)
             VALUES (?, ?, ?, 1)",
            params![user_id, profile_type, now.to_rfc3339()],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get points for a profile.
    pub fn get_points(conn: &Connection, profile_id: i64) -> Result<Vec<PowerProfilePointRecord>> {
        let mut stmt = conn.prepare(
            "SELECT profile_id, duration_secs, power_watts, achieved_at, ride_id
             FROM power_profile_points WHERE profile_id = ? ORDER BY duration_secs",
        )?;

        let records = stmt
            .query_map(params![profile_id], |row| {
                Ok(PowerProfilePointRecord {
                    profile_id: row.get(0)?,
                    duration_secs: row.get::<_, i64>(1)? as u32,
                    power_watts: row.get::<_, i64>(2)? as u16,
                    achieved_at: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    ride_id: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Insert or update a point in a profile.
    pub fn upsert_point(
        conn: &Connection,
        profile_id: i64,
        duration_secs: u32,
        power_watts: u16,
        achieved_at: DateTime<Utc>,
        ride_id: Option<Uuid>,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO power_profile_points
             (profile_id, duration_secs, power_watts, achieved_at, ride_id)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(profile_id, duration_secs) DO UPDATE SET
                power_watts = excluded.power_watts,
                achieved_at = excluded.achieved_at,
                ride_id = excluded.ride_id",
            params![
                profile_id,
                duration_secs as i64,
                power_watts as i64,
                achieved_at.to_rfc3339(),
                ride_id.map(|id| id.to_string())
            ],
        )?;
        Ok(())
    }

    /// Get all profiles for a user (for history).
    pub fn get_all_profiles(
        conn: &Connection,
        user_id: i64,
        profile_type: &str,
    ) -> Result<Vec<PowerProfileRecord>> {
        let mut stmt = conn.prepare(
            "SELECT id, user_id, profile_type, recorded_at, is_current
             FROM power_profiles
             WHERE user_id = ? AND profile_type = ?
             ORDER BY recorded_at DESC",
        )?;

        let records = stmt
            .query_map(params![user_id, profile_type], |row| {
                Ok(PowerProfileRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    profile_type: row.get(2)?,
                    recorded_at: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    is_current: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Delete old profiles, keeping the N most recent.
    pub fn cleanup_old_profiles(
        conn: &Connection,
        user_id: i64,
        profile_type: &str,
        keep_count: usize,
    ) -> Result<()> {
        // Get IDs to delete
        let mut stmt = conn.prepare(
            "SELECT id FROM power_profiles
             WHERE user_id = ? AND profile_type = ?
             ORDER BY recorded_at DESC",
        )?;

        let ids: Vec<i64> = stmt
            .query_map(params![user_id, profile_type], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;

        // Delete old profiles (cascade deletes points)
        for id in ids.iter().skip(keep_count) {
            conn.execute("DELETE FROM power_profiles WHERE id = ?", params![id])?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE power_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                profile_type TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                is_current INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE power_profile_points (
                profile_id INTEGER NOT NULL,
                duration_secs INTEGER NOT NULL,
                power_watts INTEGER NOT NULL,
                achieved_at TEXT NOT NULL,
                ride_id TEXT,
                PRIMARY KEY (profile_id, duration_secs),
                FOREIGN KEY (profile_id) REFERENCES power_profiles(id) ON DELETE CASCADE
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_create_and_get_profile() {
        let conn = setup_test_db();

        let profile_id = PowerProfileStore::create_profile(&conn, 1, "current").unwrap();
        let profile = PowerProfileStore::get_current(&conn, 1, "current").unwrap();

        assert!(profile.is_some());
        assert_eq!(profile.unwrap().id, profile_id);
    }

    #[test]
    fn test_upsert_points() {
        let conn = setup_test_db();

        let profile_id = PowerProfileStore::create_profile(&conn, 1, "current").unwrap();
        let now = Utc::now();

        PowerProfileStore::upsert_point(&conn, profile_id, 300, 250, now, None).unwrap();
        PowerProfileStore::upsert_point(&conn, profile_id, 300, 260, now, None).unwrap(); // Update

        let points = PowerProfileStore::get_points(&conn, profile_id).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].power_watts, 260);
    }
}
