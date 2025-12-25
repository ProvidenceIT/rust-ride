//! Training goal management.
//!
//! T017: Create GoalManager for CRUD operations
//! T018: Implement priority management

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::types::{GoalStatus, GoalType, MetricType, TargetMetric, TrainingGoal};

/// Manager for training goals.
pub struct GoalManager<'a> {
    conn: &'a Connection,
}

impl<'a> GoalManager<'a> {
    /// Create a new goal manager with a database connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new training goal.
    pub fn create(&self, goal: &TrainingGoal) -> Result<(), GoalError> {
        // Validate event goals have target dates
        if goal.is_event_goal() && goal.target_date.is_none() {
            return Err(GoalError::ValidationError(
                "Event goals require a target date".to_string(),
            ));
        }

        // Ensure priority is unique - shift existing priorities if needed
        self.make_room_for_priority(goal.user_id, goal.priority)?;

        self.conn.execute(
            "INSERT INTO training_goals
             (id, user_id, goal_type, title, description, target_date,
              target_metric_type, target_metric_value, target_metric_current,
              priority, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                goal.id.to_string(),
                goal.user_id.to_string(),
                serde_json::to_string(&goal.goal_type)?,
                goal.title,
                goal.description,
                goal.target_date.map(|d| d.to_string()),
                goal.target_metric.as_ref().map(|m| format!("{:?}", m.metric_type)),
                goal.target_metric.as_ref().map(|m| m.target_value),
                goal.target_metric.as_ref().and_then(|m| m.current_value),
                goal.priority,
                format!("{:?}", goal.status),
                goal.created_at.to_rfc3339(),
                goal.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get a goal by ID.
    pub fn get(&self, id: Uuid) -> Result<Option<TrainingGoal>, GoalError> {
        self.conn
            .query_row(
                "SELECT id, user_id, goal_type, title, description, target_date,
                        target_metric_type, target_metric_value, target_metric_current,
                        priority, status, created_at, updated_at
                 FROM training_goals WHERE id = ?1",
                params![id.to_string()],
                parse_goal_row,
            )
            .optional()
            .map_err(GoalError::from)
    }

    /// Get all goals for a user.
    pub fn get_for_user(&self, user_id: Uuid) -> Result<Vec<TrainingGoal>, GoalError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, goal_type, title, description, target_date,
                    target_metric_type, target_metric_value, target_metric_current,
                    priority, status, created_at, updated_at
             FROM training_goals
             WHERE user_id = ?1
             ORDER BY priority ASC",
        )?;

        let rows = stmt.query_map(params![user_id.to_string()], parse_goal_row)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(GoalError::from)
    }

    /// Get active goals for a user.
    pub fn get_active(&self, user_id: Uuid) -> Result<Vec<TrainingGoal>, GoalError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, goal_type, title, description, target_date,
                    target_metric_type, target_metric_value, target_metric_current,
                    priority, status, created_at, updated_at
             FROM training_goals
             WHERE user_id = ?1 AND status = 'Active'
             ORDER BY priority ASC",
        )?;

        let rows = stmt.query_map(params![user_id.to_string()], parse_goal_row)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(GoalError::from)
    }

    /// Update a goal.
    pub fn update(&self, goal: &TrainingGoal) -> Result<(), GoalError> {
        let now = Utc::now();

        self.conn.execute(
            "UPDATE training_goals SET
             goal_type = ?1, title = ?2, description = ?3, target_date = ?4,
             target_metric_type = ?5, target_metric_value = ?6, target_metric_current = ?7,
             priority = ?8, status = ?9, updated_at = ?10
             WHERE id = ?11",
            params![
                serde_json::to_string(&goal.goal_type)?,
                goal.title,
                goal.description,
                goal.target_date.map(|d| d.to_string()),
                goal.target_metric.as_ref().map(|m| format!("{:?}", m.metric_type)),
                goal.target_metric.as_ref().map(|m| m.target_value),
                goal.target_metric.as_ref().and_then(|m| m.current_value),
                goal.priority,
                format!("{:?}", goal.status),
                now.to_rfc3339(),
                goal.id.to_string(),
            ],
        )?;

        Ok(())
    }

    /// Delete a goal.
    pub fn delete(&self, id: Uuid) -> Result<bool, GoalError> {
        let deleted = self.conn.execute(
            "DELETE FROM training_goals WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(deleted > 0)
    }

    /// Update goal status.
    pub fn update_status(&self, id: Uuid, status: GoalStatus) -> Result<(), GoalError> {
        let now = Utc::now();

        self.conn.execute(
            "UPDATE training_goals SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![format!("{:?}", status), now.to_rfc3339(), id.to_string()],
        )?;

        Ok(())
    }

    /// Update current value for a goal's target metric.
    pub fn update_progress(&self, id: Uuid, current_value: f32) -> Result<(), GoalError> {
        let now = Utc::now();

        self.conn.execute(
            "UPDATE training_goals SET target_metric_current = ?1, updated_at = ?2 WHERE id = ?3",
            params![current_value, now.to_rfc3339(), id.to_string()],
        )?;

        Ok(())
    }

    /// Change goal priority.
    ///
    /// This also adjusts other goals' priorities to maintain uniqueness.
    pub fn change_priority(
        &self,
        id: Uuid,
        new_priority: u8,
    ) -> Result<(), GoalError> {
        // Get the goal to find user_id and current priority
        let goal = self.get(id)?.ok_or_else(|| {
            GoalError::NotFound(id)
        })?;

        if goal.priority == new_priority {
            return Ok(());
        }

        // Make room for the new priority
        self.make_room_for_priority(goal.user_id, new_priority)?;

        // Update the goal's priority
        let now = Utc::now();
        self.conn.execute(
            "UPDATE training_goals SET priority = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_priority, now.to_rfc3339(), id.to_string()],
        )?;

        // Compact priorities to remove gaps
        self.compact_priorities(goal.user_id)?;

        Ok(())
    }

    /// Make room for a priority by shifting existing goals.
    fn make_room_for_priority(&self, user_id: Uuid, priority: u8) -> Result<(), GoalError> {
        // Check if priority is already taken
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM training_goals WHERE user_id = ?1 AND priority = ?2 AND status = 'Active')",
            params![user_id.to_string(), priority],
            |row| row.get(0),
        )?;

        if exists {
            // Shift all goals with priority >= new priority down by 1
            self.conn.execute(
                "UPDATE training_goals
                 SET priority = priority + 1
                 WHERE user_id = ?1 AND priority >= ?2 AND status = 'Active'",
                params![user_id.to_string(), priority],
            )?;
        }

        Ok(())
    }

    /// Compact priorities to remove gaps after deletions.
    fn compact_priorities(&self, user_id: Uuid) -> Result<(), GoalError> {
        let goals = self.get_active(user_id)?;
        let now = Utc::now();

        for (index, goal) in goals.iter().enumerate() {
            let expected_priority = (index + 1) as u8;
            if goal.priority != expected_priority {
                self.conn.execute(
                    "UPDATE training_goals SET priority = ?1, updated_at = ?2 WHERE id = ?3",
                    params![expected_priority, now.to_rfc3339(), goal.id.to_string()],
                )?;
            }
        }

        Ok(())
    }

    /// Get goals with upcoming target dates.
    pub fn get_upcoming_events(
        &self,
        user_id: Uuid,
        within_days: i64,
    ) -> Result<Vec<TrainingGoal>, GoalError> {
        let today = Utc::now().date_naive();
        let cutoff = today + chrono::Duration::days(within_days);

        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, goal_type, title, description, target_date,
                    target_metric_type, target_metric_value, target_metric_current,
                    priority, status, created_at, updated_at
             FROM training_goals
             WHERE user_id = ?1 AND status = 'Active'
               AND target_date IS NOT NULL
               AND target_date >= ?2 AND target_date <= ?3
             ORDER BY target_date ASC",
        )?;

        let rows = stmt.query_map(
            params![
                user_id.to_string(),
                today.to_string(),
                cutoff.to_string()
            ],
            parse_goal_row,
        )?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(GoalError::from)
    }
}

/// Parse a database row into a TrainingGoal.
fn parse_goal_row(row: &rusqlite::Row) -> rusqlite::Result<TrainingGoal> {
    let id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let goal_type_json: String = row.get(2)?;
    let target_date_str: Option<String> = row.get(5)?;
    let metric_type_str: Option<String> = row.get(6)?;
    let metric_value: Option<f32> = row.get(7)?;
    let metric_current: Option<f32> = row.get(8)?;
    let status_str: String = row.get(10)?;
    let created_at_str: String = row.get(11)?;
    let updated_at_str: String = row.get(12)?;

    let goal_type: GoalType = serde_json::from_str(&goal_type_json)
        .unwrap_or(GoalType::GetFaster);

    let target_date = target_date_str.and_then(|s| {
        NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
    });

    let target_metric = match (metric_type_str, metric_value) {
        (Some(type_str), Some(value)) => {
            let metric_type = match type_str.as_str() {
                "Ctl" => MetricType::Ctl,
                "Ftp" => MetricType::Ftp,
                "Vo2max" => MetricType::Vo2max,
                "Weight" => MetricType::Weight,
                _ => MetricType::Ftp,
            };
            Some(TargetMetric {
                metric_type,
                target_value: value,
                current_value: metric_current,
            })
        }
        _ => None,
    };

    let status = match status_str.as_str() {
        "Active" => GoalStatus::Active,
        "Completed" => GoalStatus::Completed,
        "Abandoned" => GoalStatus::Abandoned,
        "OnHold" => GoalStatus::OnHold,
        _ => GoalStatus::Active,
    };

    Ok(TrainingGoal {
        id: Uuid::parse_str(&id_str).unwrap_or_default(),
        user_id: Uuid::parse_str(&user_id_str).unwrap_or_default(),
        goal_type,
        title: row.get(3)?,
        description: row.get(4)?,
        target_date,
        target_metric,
        priority: row.get(9)?,
        status,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

/// Goal management errors.
#[derive(Debug, thiserror::Error)]
pub enum GoalError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Goal not found: {0}")]
    NotFound(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE training_goals (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                goal_type TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                target_date TEXT,
                target_metric_type TEXT,
                target_metric_value REAL,
                target_metric_current REAL,
                priority INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL DEFAULT 'Active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        (file, conn)
    }

    #[test]
    fn test_create_and_get_goal() {
        let (_file, conn) = setup_test_db();
        let manager = GoalManager::new(&conn);
        let user_id = Uuid::new_v4();

        let goal = TrainingGoal::new(
            user_id,
            GoalType::ImproveVo2max,
            "Boost VO2max".to_string(),
        );

        manager.create(&goal).unwrap();

        let retrieved = manager.get(goal.id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, "Boost VO2max");
        assert!(matches!(retrieved.goal_type, GoalType::ImproveVo2max));
    }

    #[test]
    fn test_priority_management() {
        let (_file, conn) = setup_test_db();
        let manager = GoalManager::new(&conn);
        let user_id = Uuid::new_v4();

        // Create three goals
        let mut goal1 = TrainingGoal::new(user_id, GoalType::GetFaster, "Goal 1".to_string());
        goal1.priority = 1;
        manager.create(&goal1).unwrap();

        let mut goal2 = TrainingGoal::new(user_id, GoalType::ImproveEndurance, "Goal 2".to_string());
        goal2.priority = 2;
        manager.create(&goal2).unwrap();

        let mut goal3 = TrainingGoal::new(user_id, GoalType::LoseWeight, "Goal 3".to_string());
        goal3.priority = 3;
        manager.create(&goal3).unwrap();

        // Get all goals
        let goals = manager.get_active(user_id).unwrap();
        assert_eq!(goals.len(), 3);
        assert_eq!(goals[0].priority, 1);
        assert_eq!(goals[1].priority, 2);
        assert_eq!(goals[2].priority, 3);
    }

    #[test]
    fn test_update_status() {
        let (_file, conn) = setup_test_db();
        let manager = GoalManager::new(&conn);
        let user_id = Uuid::new_v4();

        let goal = TrainingGoal::new(user_id, GoalType::GetFaster, "Test".to_string());
        manager.create(&goal).unwrap();

        assert!(manager.get(goal.id).unwrap().unwrap().status.is_active());

        manager.update_status(goal.id, GoalStatus::Completed).unwrap();

        let updated = manager.get(goal.id).unwrap().unwrap();
        assert_eq!(updated.status, GoalStatus::Completed);
    }

    #[test]
    fn test_event_goal_validation() {
        let (_file, conn) = setup_test_db();
        let manager = GoalManager::new(&conn);
        let user_id = Uuid::new_v4();

        // Event goal without target date should fail
        let goal = TrainingGoal::new(user_id, GoalType::CenturyRide, "Century".to_string());
        let result = manager.create(&goal);
        assert!(matches!(result, Err(GoalError::ValidationError(_))));
    }
}
