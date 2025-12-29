//! Storage operations for gradient settings.

use rusqlite::{params, Connection, Result};

/// Gradient settings record from the database.
#[derive(Debug, Clone)]
pub struct GradientSettingsRecord {
    pub user_id: i64,
    pub difficulty_percent: u8,
    pub max_gradient: f32,
    pub min_gradient: f32,
    pub smoothing_secs: u8,
    pub rolling_resistance: f32,
}

impl Default for GradientSettingsRecord {
    fn default() -> Self {
        Self {
            user_id: 1,
            difficulty_percent: 100,
            max_gradient: 15.0,
            min_gradient: -15.0,
            smoothing_secs: 3,
            rolling_resistance: 0.004,
        }
    }
}

/// Storage operations for gradient settings.
pub struct GradientStore;

impl GradientStore {
    /// Get gradient settings for a user, returning defaults if not set.
    pub fn get_or_default(conn: &Connection, user_id: i64) -> Result<GradientSettingsRecord> {
        let result = conn.query_row(
            "SELECT user_id, difficulty_percent, max_gradient, min_gradient,
                    smoothing_secs, rolling_resistance
             FROM gradient_settings WHERE user_id = ?",
            params![user_id],
            |row| {
                Ok(GradientSettingsRecord {
                    user_id: row.get(0)?,
                    difficulty_percent: row.get::<_, i64>(1)? as u8,
                    max_gradient: row.get(2)?,
                    min_gradient: row.get(3)?,
                    smoothing_secs: row.get::<_, i64>(4)? as u8,
                    rolling_resistance: row.get(5)?,
                })
            },
        );

        match result {
            Ok(record) => Ok(record),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(GradientSettingsRecord {
                user_id,
                ..Default::default()
            }),
            Err(e) => Err(e),
        }
    }

    /// Save gradient settings for a user.
    pub fn save(conn: &Connection, settings: &GradientSettingsRecord) -> Result<()> {
        conn.execute(
            "INSERT INTO gradient_settings
             (user_id, difficulty_percent, max_gradient, min_gradient,
              smoothing_secs, rolling_resistance)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                difficulty_percent = excluded.difficulty_percent,
                max_gradient = excluded.max_gradient,
                min_gradient = excluded.min_gradient,
                smoothing_secs = excluded.smoothing_secs,
                rolling_resistance = excluded.rolling_resistance",
            params![
                settings.user_id,
                settings.difficulty_percent as i64,
                settings.max_gradient,
                settings.min_gradient,
                settings.smoothing_secs as i64,
                settings.rolling_resistance
            ],
        )?;
        Ok(())
    }

    /// Update just the difficulty setting.
    pub fn set_difficulty(conn: &Connection, user_id: i64, difficulty_percent: u8) -> Result<()> {
        let mut settings = Self::get_or_default(conn, user_id)?;
        settings.difficulty_percent = difficulty_percent;
        Self::save(conn, &settings)
    }

    /// Update gradient limits.
    pub fn set_gradient_limits(
        conn: &Connection,
        user_id: i64,
        max_gradient: f32,
        min_gradient: f32,
    ) -> Result<()> {
        let mut settings = Self::get_or_default(conn, user_id)?;
        settings.max_gradient = max_gradient;
        settings.min_gradient = min_gradient;
        Self::save(conn, &settings)
    }

    /// Reset settings to defaults.
    pub fn reset_to_defaults(conn: &Connection, user_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM gradient_settings WHERE user_id = ?",
            params![user_id],
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
            "CREATE TABLE gradient_settings (
                user_id INTEGER PRIMARY KEY,
                difficulty_percent INTEGER NOT NULL DEFAULT 100,
                max_gradient REAL NOT NULL DEFAULT 15.0,
                min_gradient REAL NOT NULL DEFAULT -15.0,
                smoothing_secs INTEGER NOT NULL DEFAULT 3,
                rolling_resistance REAL NOT NULL DEFAULT 0.004
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_get_default() {
        let conn = setup_test_db();
        let settings = GradientStore::get_or_default(&conn, 1).unwrap();

        assert_eq!(settings.difficulty_percent, 100);
        assert_eq!(settings.max_gradient, 15.0);
        assert_eq!(settings.min_gradient, -15.0);
    }

    #[test]
    fn test_save_and_get() {
        let conn = setup_test_db();

        let settings = GradientSettingsRecord {
            user_id: 1,
            difficulty_percent: 50,
            max_gradient: 10.0,
            min_gradient: -10.0,
            smoothing_secs: 5,
            rolling_resistance: 0.005,
        };

        GradientStore::save(&conn, &settings).unwrap();
        let loaded = GradientStore::get_or_default(&conn, 1).unwrap();

        assert_eq!(loaded.difficulty_percent, 50);
        assert_eq!(loaded.max_gradient, 10.0);
    }

    #[test]
    fn test_set_difficulty() {
        let conn = setup_test_db();

        GradientStore::set_difficulty(&conn, 1, 75).unwrap();
        let settings = GradientStore::get_or_default(&conn, 1).unwrap();

        assert_eq!(settings.difficulty_percent, 75);
    }
}
