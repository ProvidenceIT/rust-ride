//! Badge and achievement management.
//!
//! Tracks badge definitions, criteria checking, and badge unlocking.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::types::{Badge, BadgeCategory, CriteriaType, EarnedBadge};
use crate::storage::Database;

/// Badge manager.
pub struct BadgeManager {
    db: Arc<Database>,
}

impl BadgeManager {
    /// Create a new badge manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Initialize default badges if not present.
    pub fn initialize_badges(&self) -> Result<(), BadgeError> {
        let badges = super::types::default_badges();
        let conn = self.db.connection();

        for badge in badges {
            conn.execute(
                "INSERT OR IGNORE INTO badges (id, name, description, icon, category, criteria_type, criteria_value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    badge.id,
                    badge.name,
                    badge.description,
                    badge.icon,
                    badge.category.as_str(),
                    badge.criteria_type.as_str(),
                    badge.criteria_value,
                ],
            )
            .map_err(|e| BadgeError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Get all available badges.
    pub fn get_all_badges(&self) -> Result<Vec<Badge>, BadgeError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare("SELECT id, name, description, icon, category, criteria_type, criteria_value FROM badges")
            .map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let category_str: String = row.get(4)?;
                let criteria_str: String = row.get(5)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    category_str,
                    criteria_str,
                    row.get::<_, f64>(6)?,
                ))
            })
            .map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

        let mut badges = Vec::new();
        for row in rows {
            let (id, name, description, icon, category_str, criteria_str, criteria_value) =
                row.map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

            badges.push(Badge {
                id,
                name,
                description,
                icon,
                category: BadgeCategory::from_str(&category_str).unwrap_or(BadgeCategory::Special),
                criteria_type: CriteriaType::from_str(&criteria_str)
                    .unwrap_or(CriteriaType::WorkoutsCompleted),
                criteria_value,
                earned: false,
                progress: 0.0,
                target: criteria_value,
            });
        }

        Ok(badges)
    }

    /// Get badges earned by a rider.
    pub fn get_earned_badges(&self, rider_id: Uuid) -> Result<Vec<EarnedBadge>, BadgeError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT b.id, b.name, b.description, b.icon, b.category, b.criteria_type, b.criteria_value, rb.unlocked_at
                 FROM badges b
                 JOIN rider_badges rb ON b.id = rb.badge_id
                 WHERE rb.rider_id = ?1
                 ORDER BY rb.unlocked_at DESC",
            )
            .map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([rider_id.to_string()], |row| {
                let category_str: String = row.get(4)?;
                let criteria_str: String = row.get(5)?;
                let unlocked_str: String = row.get(7)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    category_str,
                    criteria_str,
                    row.get::<_, f64>(6)?,
                    unlocked_str,
                ))
            })
            .map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

        let mut earned = Vec::new();
        for row in rows {
            let (
                id,
                name,
                description,
                icon,
                category_str,
                criteria_str,
                criteria_value,
                unlocked_str,
            ) = row.map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

            earned.push(EarnedBadge {
                badge: Badge {
                    id,
                    name,
                    description,
                    icon,
                    category: BadgeCategory::from_str(&category_str)
                        .unwrap_or(BadgeCategory::Special),
                    criteria_type: CriteriaType::from_str(&criteria_str)
                        .unwrap_or(CriteriaType::WorkoutsCompleted),
                    criteria_value,
                    earned: true,
                    progress: criteria_value,
                    target: criteria_value,
                },
                unlocked_at: DateTime::parse_from_rfc3339(&unlocked_str)
                    .map_err(|e| BadgeError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        Ok(earned)
    }

    /// Check and unlock badges for a rider.
    pub fn check_and_unlock_badges(
        &self,
        rider_id: Uuid,
        stats: &RiderStats,
    ) -> Result<Vec<EarnedBadge>, BadgeError> {
        let all_badges = self.get_all_badges()?;
        let earned_ids: Vec<String> = self
            .get_earned_badges(rider_id)?
            .iter()
            .map(|e| e.badge.id.clone())
            .collect();

        let mut newly_earned = Vec::new();

        for badge in all_badges {
            if earned_ids.contains(&badge.id) {
                continue;
            }

            let meets_criteria = match badge.criteria_type {
                CriteriaType::TotalDistanceKm => stats.total_distance_km >= badge.criteria_value,
                CriteriaType::FtpIncrease => stats.ftp_increase >= badge.criteria_value as u16,
                CriteriaType::ConsecutiveDays => {
                    stats.consecutive_days >= badge.criteria_value as u32
                }
                CriteriaType::WorkoutsCompleted => {
                    stats.workouts_completed >= badge.criteria_value as u32
                }
            };

            if meets_criteria {
                let earned = self.unlock_badge(rider_id, &badge)?;
                newly_earned.push(earned);
            }
        }

        Ok(newly_earned)
    }

    /// Unlock a badge for a rider.
    fn unlock_badge(&self, rider_id: Uuid, badge: &Badge) -> Result<EarnedBadge, BadgeError> {
        let conn = self.db.connection();
        let now = Utc::now();
        let id = Uuid::new_v4();

        conn.execute(
            "INSERT INTO rider_badges (id, rider_id, badge_id, unlocked_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                id.to_string(),
                rider_id.to_string(),
                badge.id,
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| BadgeError::DatabaseError(e.to_string()))?;

        Ok(EarnedBadge {
            badge: badge.clone(),
            unlocked_at: now,
        })
    }
}

/// Rider stats for badge checking.
#[derive(Debug, Clone, Default)]
pub struct RiderStats {
    pub total_distance_km: f64,
    pub ftp_increase: u16,
    pub consecutive_days: u32,
    pub workouts_completed: u32,
}

/// Badge errors.
#[derive(Debug, thiserror::Error)]
pub enum BadgeError {
    #[error("Badge not found: {0}")]
    NotFound(String),

    #[error("Already earned")]
    AlreadyEarned,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
