//! Storage operations for user rewards.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};

/// User reward record from the database.
#[derive(Debug, Clone)]
pub struct UserRewardRecord {
    pub user_id: i64,
    pub reward_type: String,
    pub reward_id: String,
    pub unlocked_at: DateTime<Utc>,
    pub unlocked_at_level: u32,
}

/// Storage operations for user rewards.
pub struct RewardsStore;

impl RewardsStore {
    /// Get all unlocked rewards for a user.
    pub fn get_unlocked(conn: &Connection, user_id: i64) -> Result<Vec<UserRewardRecord>> {
        let mut stmt = conn.prepare(
            "SELECT user_id, reward_type, reward_id, unlocked_at, unlocked_at_level
             FROM user_rewards WHERE user_id = ? ORDER BY unlocked_at DESC",
        )?;

        let records = stmt
            .query_map(params![user_id], |row| {
                Ok(UserRewardRecord {
                    user_id: row.get(0)?,
                    reward_type: row.get(1)?,
                    reward_id: row.get(2)?,
                    unlocked_at: row.get::<_, String>(3)?.parse().unwrap_or_else(|_| Utc::now()),
                    unlocked_at_level: row.get::<_, i64>(4)? as u32,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Check if a user has unlocked a specific reward.
    pub fn has_unlocked(
        conn: &Connection,
        user_id: i64,
        reward_type: &str,
        reward_id: &str,
    ) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM user_rewards
             WHERE user_id = ? AND reward_type = ? AND reward_id = ?",
            params![user_id, reward_type, reward_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Record a newly unlocked reward.
    pub fn record_unlock(
        conn: &Connection,
        user_id: i64,
        reward_type: &str,
        reward_id: &str,
        level: u32,
    ) -> Result<()> {
        let now = Utc::now();
        conn.execute(
            "INSERT OR IGNORE INTO user_rewards
             (user_id, reward_type, reward_id, unlocked_at, unlocked_at_level)
             VALUES (?, ?, ?, ?, ?)",
            params![user_id, reward_type, reward_id, now.to_rfc3339(), level as i64],
        )?;
        Ok(())
    }

    /// Get rewards unlocked at a specific level.
    pub fn get_for_level(conn: &Connection, user_id: i64, level: u32) -> Result<Vec<UserRewardRecord>> {
        let mut stmt = conn.prepare(
            "SELECT user_id, reward_type, reward_id, unlocked_at, unlocked_at_level
             FROM user_rewards WHERE user_id = ? AND unlocked_at_level = ?",
        )?;

        let records = stmt
            .query_map(params![user_id, level as i64], |row| {
                Ok(UserRewardRecord {
                    user_id: row.get(0)?,
                    reward_type: row.get(1)?,
                    reward_id: row.get(2)?,
                    unlocked_at: row.get::<_, String>(3)?.parse().unwrap_or_else(|_| Utc::now()),
                    unlocked_at_level: row.get::<_, i64>(4)? as u32,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Get rewards by type for a user.
    pub fn get_by_type(
        conn: &Connection,
        user_id: i64,
        reward_type: &str,
    ) -> Result<Vec<UserRewardRecord>> {
        let mut stmt = conn.prepare(
            "SELECT user_id, reward_type, reward_id, unlocked_at, unlocked_at_level
             FROM user_rewards WHERE user_id = ? AND reward_type = ?",
        )?;

        let records = stmt
            .query_map(params![user_id, reward_type], |row| {
                Ok(UserRewardRecord {
                    user_id: row.get(0)?,
                    reward_type: row.get(1)?,
                    reward_id: row.get(2)?,
                    unlocked_at: row.get::<_, String>(3)?.parse().unwrap_or_else(|_| Utc::now()),
                    unlocked_at_level: row.get::<_, i64>(4)? as u32,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    /// Count total unlocked rewards for a user.
    pub fn count_unlocked(conn: &Connection, user_id: i64) -> Result<u32> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM user_rewards WHERE user_id = ?",
            params![user_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE user_rewards (
                user_id INTEGER NOT NULL,
                reward_type TEXT NOT NULL,
                reward_id TEXT NOT NULL,
                unlocked_at TEXT NOT NULL,
                unlocked_at_level INTEGER NOT NULL,
                PRIMARY KEY (user_id, reward_type, reward_id)
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn test_record_and_check() {
        let conn = setup_test_db();

        assert!(!RewardsStore::has_unlocked(&conn, 1, "JerseyColor", "blue").unwrap());

        RewardsStore::record_unlock(&conn, 1, "JerseyColor", "blue", 5).unwrap();

        assert!(RewardsStore::has_unlocked(&conn, 1, "JerseyColor", "blue").unwrap());
    }

    #[test]
    fn test_get_for_level() {
        let conn = setup_test_db();

        RewardsStore::record_unlock(&conn, 1, "JerseyColor", "blue", 5).unwrap();
        RewardsStore::record_unlock(&conn, 1, "AccentColor", "red", 5).unwrap();
        RewardsStore::record_unlock(&conn, 1, "BikeFrame", "carbon", 10).unwrap();

        let level_5_rewards = RewardsStore::get_for_level(&conn, 1, 5).unwrap();
        assert_eq!(level_5_rewards.len(), 2);

        let level_10_rewards = RewardsStore::get_for_level(&conn, 1, 10).unwrap();
        assert_eq!(level_10_rewards.len(), 1);
    }
}
