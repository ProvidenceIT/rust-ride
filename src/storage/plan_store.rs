//! Storage operations for training plan assignments.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use uuid::Uuid;

/// Plan assignment record from the database.
#[derive(Debug, Clone)]
pub struct PlanAssignmentRecord {
    pub user_id: i64,
    pub plan_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub current_week: u8,
    pub completed_workouts: u32,
    pub skipped_workouts: u32,
    pub status: String,
    pub available_days: u8,
}

/// Storage operations for plan assignments.
pub struct PlanStore;

impl PlanStore {
    /// Get the current plan assignment for a user.
    pub fn get_current(conn: &Connection, user_id: i64) -> Result<Option<PlanAssignmentRecord>> {
        let result = conn.query_row(
            "SELECT user_id, plan_id, started_at, current_week, completed_workouts,
                    skipped_workouts, status, available_days
             FROM plan_assignments WHERE user_id = ?",
            params![user_id],
            |row| {
                Ok(PlanAssignmentRecord {
                    user_id: row.get(0)?,
                    plan_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    started_at: row.get::<_, String>(2)?.parse().unwrap_or_else(|_| Utc::now()),
                    current_week: row.get::<_, i64>(3)? as u8,
                    completed_workouts: row.get::<_, i64>(4)? as u32,
                    skipped_workouts: row.get::<_, i64>(5)? as u32,
                    status: row.get(6)?,
                    available_days: row.get::<_, i64>(7)? as u8,
                })
            },
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Assign a plan to a user (replaces any existing assignment).
    pub fn assign(
        conn: &Connection,
        user_id: i64,
        plan_id: Uuid,
        available_days: u8,
    ) -> Result<()> {
        let now = Utc::now();
        conn.execute(
            "INSERT INTO plan_assignments
             (user_id, plan_id, started_at, current_week, completed_workouts,
              skipped_workouts, status, available_days)
             VALUES (?, ?, ?, 1, 0, 0, 'active', ?)
             ON CONFLICT(user_id) DO UPDATE SET
                plan_id = excluded.plan_id,
                started_at = excluded.started_at,
                current_week = 1,
                completed_workouts = 0,
                skipped_workouts = 0,
                status = 'active',
                available_days = excluded.available_days",
            params![user_id, plan_id.to_string(), now.to_rfc3339(), available_days as i64],
        )?;
        Ok(())
    }

    /// Update plan progress.
    pub fn update_progress(
        conn: &Connection,
        user_id: i64,
        current_week: u8,
        completed_workouts: u32,
        skipped_workouts: u32,
    ) -> Result<()> {
        conn.execute(
            "UPDATE plan_assignments SET
                current_week = ?,
                completed_workouts = ?,
                skipped_workouts = ?
             WHERE user_id = ?",
            params![
                current_week as i64,
                completed_workouts as i64,
                skipped_workouts as i64,
                user_id
            ],
        )?;
        Ok(())
    }

    /// Update plan status.
    pub fn update_status(conn: &Connection, user_id: i64, status: &str) -> Result<()> {
        conn.execute(
            "UPDATE plan_assignments SET status = ? WHERE user_id = ?",
            params![status, user_id],
        )?;
        Ok(())
    }

    /// Record a completed workout.
    pub fn record_workout_completed(conn: &Connection, user_id: i64) -> Result<()> {
        conn.execute(
            "UPDATE plan_assignments SET completed_workouts = completed_workouts + 1
             WHERE user_id = ?",
            params![user_id],
        )?;
        Ok(())
    }

    /// Record a skipped workout.
    pub fn record_workout_skipped(conn: &Connection, user_id: i64) -> Result<()> {
        conn.execute(
            "UPDATE plan_assignments SET skipped_workouts = skipped_workouts + 1
             WHERE user_id = ?",
            params![user_id],
        )?;
        Ok(())
    }

    /// Advance to the next week.
    pub fn advance_week(conn: &Connection, user_id: i64) -> Result<()> {
        conn.execute(
            "UPDATE plan_assignments SET current_week = current_week + 1
             WHERE user_id = ?",
            params![user_id],
        )?;
        Ok(())
    }

    /// Remove plan assignment.
    pub fn remove(conn: &Connection, user_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM plan_assignments WHERE user_id = ?",
            params![user_id],
        )?;
        Ok(())
    }

    /// Check if user has an active plan.
    pub fn has_active_plan(conn: &Connection, user_id: i64) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM plan_assignments WHERE user_id = ? AND status = 'active'",
            params![user_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE plan_assignments (
                user_id INTEGER PRIMARY KEY,
                plan_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                current_week INTEGER NOT NULL DEFAULT 1,
                completed_workouts INTEGER NOT NULL DEFAULT 0,
                skipped_workouts INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'active',
                available_days INTEGER NOT NULL DEFAULT 127
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn test_assign_and_get() {
        let conn = setup_test_db();
        let plan_id = Uuid::new_v4();

        PlanStore::assign(&conn, 1, plan_id, 0b1111111).unwrap();
        let assignment = PlanStore::get_current(&conn, 1).unwrap();

        assert!(assignment.is_some());
        let a = assignment.unwrap();
        assert_eq!(a.plan_id, plan_id);
        assert_eq!(a.current_week, 1);
        assert_eq!(a.status, "active");
    }

    #[test]
    fn test_progress_tracking() {
        let conn = setup_test_db();
        let plan_id = Uuid::new_v4();

        PlanStore::assign(&conn, 1, plan_id, 0b1111111).unwrap();
        PlanStore::record_workout_completed(&conn, 1).unwrap();
        PlanStore::record_workout_completed(&conn, 1).unwrap();
        PlanStore::record_workout_skipped(&conn, 1).unwrap();

        let assignment = PlanStore::get_current(&conn, 1).unwrap().unwrap();
        assert_eq!(assignment.completed_workouts, 2);
        assert_eq!(assignment.skipped_workouts, 1);
    }

    #[test]
    fn test_status_update() {
        let conn = setup_test_db();
        let plan_id = Uuid::new_v4();

        PlanStore::assign(&conn, 1, plan_id, 0b1111111).unwrap();
        PlanStore::update_status(&conn, 1, "paused").unwrap();

        let assignment = PlanStore::get_current(&conn, 1).unwrap().unwrap();
        assert_eq!(assignment.status, "paused");
    }
}
