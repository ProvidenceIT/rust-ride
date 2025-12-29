//! Storage operations for user achievements.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use uuid::Uuid;

/// User achievement record from the database.
#[derive(Debug, Clone)]
pub struct UserAchievementRecord {
    pub user_id: i64,
    pub achievement_key: String,
    pub earned_at: DateTime<Utc>,
    pub ride_id: Option<Uuid>,
    pub progress_value: Option<f64>,
}

/// Storage operations for user achievements.
pub struct AchievementsStore;

impl AchievementsStore {
    /// Get all earned achievements for a user.
    pub fn get_earned(conn: &Connection, user_id: i64) -> Result<Vec<UserAchievementRecord>> {
        let mut stmt = conn.prepare(
            "SELECT user_id, achievement_key, earned_at, ride_id, progress_value
             FROM user_achievements WHERE user_id = ? ORDER BY earned_at DESC",
        )?;

        let records = stmt
            .query_map(params![user_id], |row| {
                Ok(UserAchievementRecord {
                    user_id: row.get(0)?,
                    achievement_key: row.get(1)?,
                    earned_at: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    ride_id: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    progress_value: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Check if a user has earned a specific achievement.
    pub fn has_earned(conn: &Connection, user_id: i64, achievement_key: &str) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM user_achievements WHERE user_id = ? AND achievement_key = ?",
            params![user_id, achievement_key],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Record a newly earned achievement.
    pub fn record_achievement(
        conn: &Connection,
        user_id: i64,
        achievement_key: &str,
        ride_id: Option<Uuid>,
        progress_value: Option<f64>,
    ) -> Result<()> {
        let now = Utc::now();
        conn.execute(
            "INSERT OR IGNORE INTO user_achievements
             (user_id, achievement_key, earned_at, ride_id, progress_value)
             VALUES (?, ?, ?, ?, ?)",
            params![
                user_id,
                achievement_key,
                now.to_rfc3339(),
                ride_id.map(|id| id.to_string()),
                progress_value
            ],
        )?;
        Ok(())
    }

    /// Get the count of earned achievements for a user.
    pub fn count_earned(conn: &Connection, user_id: i64) -> Result<u32> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM user_achievements WHERE user_id = ?",
            params![user_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    /// Get achievements earned from a specific ride.
    pub fn get_for_ride(conn: &Connection, ride_id: Uuid) -> Result<Vec<UserAchievementRecord>> {
        let mut stmt = conn.prepare(
            "SELECT user_id, achievement_key, earned_at, ride_id, progress_value
             FROM user_achievements WHERE ride_id = ?",
        )?;

        let records = stmt
            .query_map(params![ride_id.to_string()], |row| {
                Ok(UserAchievementRecord {
                    user_id: row.get(0)?,
                    achievement_key: row.get(1)?,
                    earned_at: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    ride_id: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    progress_value: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Delete an achievement (for testing/admin purposes).
    pub fn delete(conn: &Connection, user_id: i64, achievement_key: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM user_achievements WHERE user_id = ? AND achievement_key = ?",
            params![user_id, achievement_key],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE user_achievements (
                user_id INTEGER NOT NULL,
                achievement_key TEXT NOT NULL,
                earned_at TEXT NOT NULL,
                ride_id TEXT,
                progress_value REAL,
                PRIMARY KEY (user_id, achievement_key)
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_record_and_check() {
        let conn = setup_test_db();

        assert!(!AchievementsStore::has_earned(&conn, 1, "first_ride").unwrap());

        AchievementsStore::record_achievement(&conn, 1, "first_ride", None, None).unwrap();

        assert!(AchievementsStore::has_earned(&conn, 1, "first_ride").unwrap());
    }

    #[test]
    fn test_count_earned() {
        let conn = setup_test_db();

        AchievementsStore::record_achievement(&conn, 1, "first_ride", None, None).unwrap();
        AchievementsStore::record_achievement(&conn, 1, "distance_100km", None, Some(100.0))
            .unwrap();

        assert_eq!(AchievementsStore::count_earned(&conn, 1).unwrap(), 2);
    }
}
