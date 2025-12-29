//! Storage operations for user XP and level data.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};

/// User XP record from the database.
#[derive(Debug, Clone)]
pub struct UserXpRecord {
    pub user_id: i64,
    pub total_xp: u64,
    pub current_level: u32,
    pub updated_at: DateTime<Utc>,
}

/// Storage operations for user XP.
pub struct XpStore;

impl XpStore {
    /// Get XP record for a user, creating default if not exists.
    pub fn get_or_create(conn: &Connection, user_id: i64) -> Result<UserXpRecord> {
        // Try to get existing record
        let result = conn.query_row(
            "SELECT user_id, total_xp, current_level, updated_at FROM user_xp WHERE user_id = ?",
            params![user_id],
            |row| {
                Ok(UserXpRecord {
                    user_id: row.get(0)?,
                    total_xp: row.get::<_, i64>(1)? as u64,
                    current_level: row.get::<_, i64>(2)? as u32,
                    updated_at: row.get::<_, String>(3)?.parse().unwrap_or_else(|_| Utc::now()),
                })
            },
        );

        match result {
            Ok(record) => Ok(record),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Create default record
                let now = Utc::now();
                conn.execute(
                    "INSERT INTO user_xp (user_id, total_xp, current_level, updated_at) VALUES (?, 0, 1, ?)",
                    params![user_id, now.to_rfc3339()],
                )?;
                Ok(UserXpRecord {
                    user_id,
                    total_xp: 0,
                    current_level: 1,
                    updated_at: now,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Update XP and level for a user.
    pub fn update(
        conn: &Connection,
        user_id: i64,
        total_xp: u64,
        current_level: u32,
    ) -> Result<()> {
        let now = Utc::now();
        conn.execute(
            "INSERT INTO user_xp (user_id, total_xp, current_level, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                total_xp = excluded.total_xp,
                current_level = excluded.current_level,
                updated_at = excluded.updated_at",
            params![user_id, total_xp as i64, current_level as i64, now.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Add XP to a user's total.
    pub fn add_xp(conn: &Connection, user_id: i64, xp_amount: u32) -> Result<UserXpRecord> {
        let record = Self::get_or_create(conn, user_id)?;
        let new_total = record.total_xp + xp_amount as u64;

        // Calculate new level using the XP curve
        let new_level = crate::career::level_from_xp(new_total);

        Self::update(conn, user_id, new_total, new_level)?;

        Ok(UserXpRecord {
            user_id,
            total_xp: new_total,
            current_level: new_level,
            updated_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE user_xp (
                user_id INTEGER PRIMARY KEY,
                total_xp INTEGER NOT NULL DEFAULT 0,
                current_level INTEGER NOT NULL DEFAULT 1,
                updated_at TEXT NOT NULL
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn test_get_or_create() {
        let conn = setup_test_db();
        let record = XpStore::get_or_create(&conn, 1).unwrap();
        assert_eq!(record.user_id, 1);
        assert_eq!(record.total_xp, 0);
        assert_eq!(record.current_level, 1);
    }

    #[test]
    fn test_update() {
        let conn = setup_test_db();
        XpStore::update(&conn, 1, 5000, 5).unwrap();
        let record = XpStore::get_or_create(&conn, 1).unwrap();
        assert_eq!(record.total_xp, 5000);
        assert_eq!(record.current_level, 5);
    }
}
