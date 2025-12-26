//! Activity feed for social sharing.
//!
//! Displays recent activities from LAN peers.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::types::ActivitySummary;
use crate::storage::Database;

/// Activity feed item.
#[derive(Debug, Clone)]
pub struct FeedItem {
    pub activity: ActivitySummary,
    pub is_local: bool,
}

/// Activity feed manager.
pub struct ActivityFeed {
    db: Arc<Database>,
    /// Cached peer activities (from LAN broadcasts).
    peer_activities: Vec<ActivitySummary>,
}

impl ActivityFeed {
    /// Create a new activity feed.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            peer_activities: Vec::new(),
        }
    }

    /// Get recent feed items.
    pub fn get_feed(&self, limit: usize) -> Result<Vec<FeedItem>, FeedError> {
        let mut items = Vec::new();

        // Get local activities from database
        let local = self.get_local_activities(limit)?;
        for activity in local {
            items.push(FeedItem {
                activity,
                is_local: true,
            });
        }

        // Add peer activities
        for activity in &self.peer_activities {
            items.push(FeedItem {
                activity: activity.clone(),
                is_local: false,
            });
        }

        // Sort by recorded_at descending
        items.sort_by(|a, b| b.activity.recorded_at.cmp(&a.activity.recorded_at));

        // Limit
        items.truncate(limit);

        Ok(items)
    }

    /// Get local activities.
    fn get_local_activities(&self, limit: usize) -> Result<Vec<ActivitySummary>, FeedError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, ride_id, rider_id, rider_name, distance_km, duration_minutes,
                        avg_power_watts, elevation_gain_m, world_id, recorded_at, shared
                 FROM activity_summaries
                 WHERE shared = 1
                 ORDER BY recorded_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| FeedError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, u32>(5)?,
                    row.get::<_, Option<u16>>(6)?,
                    row.get::<_, f64>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, bool>(10)?,
                ))
            })
            .map_err(|e| FeedError::DatabaseError(e.to_string()))?;

        let mut activities = Vec::new();
        for row in rows {
            let (id, ride_id, rider_id, rider_name, distance_km, duration_minutes, avg_power, elevation, world_id, recorded_at, shared) =
                row.map_err(|e| FeedError::DatabaseError(e.to_string()))?;

            activities.push(ActivitySummary {
                id: Uuid::parse_str(&id).map_err(|e| FeedError::DatabaseError(e.to_string()))?,
                ride_id: ride_id
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                rider_id: Uuid::parse_str(&rider_id).map_err(|e| FeedError::DatabaseError(e.to_string()))?,
                rider_name,
                distance_km,
                duration_minutes,
                avg_power_watts: avg_power,
                elevation_gain_m: elevation,
                world_id,
                recorded_at: DateTime::parse_from_rfc3339(&recorded_at)
                    .map_err(|e| FeedError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
                shared,
            });
        }

        Ok(activities)
    }

    /// Add a peer activity from LAN broadcast.
    pub fn add_peer_activity(&mut self, activity: ActivitySummary) {
        // Remove old activities from same rider
        self.peer_activities
            .retain(|a| a.rider_id != activity.rider_id || a.id != activity.id);

        // Add new activity
        self.peer_activities.push(activity);

        // Keep only recent 50 peer activities
        if self.peer_activities.len() > 50 {
            self.peer_activities.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
            self.peer_activities.truncate(50);
        }
    }

    /// Clear peer activities.
    pub fn clear_peer_activities(&mut self) {
        self.peer_activities.clear();
    }

    /// Save a local activity.
    pub fn save_activity(&self, activity: &ActivitySummary) -> Result<(), FeedError> {
        let conn = self.db.connection();
        conn.execute(
            "INSERT INTO activity_summaries (id, ride_id, rider_id, rider_name, distance_km, duration_minutes, avg_power_watts, elevation_gain_m, world_id, recorded_at, shared)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                activity.id.to_string(),
                activity.ride_id.map(|id| id.to_string()),
                activity.rider_id.to_string(),
                activity.rider_name,
                activity.distance_km,
                activity.duration_minutes,
                activity.avg_power_watts,
                activity.elevation_gain_m,
                activity.world_id,
                activity.recorded_at.to_rfc3339(),
                activity.shared,
            ],
        )
        .map_err(|e| FeedError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// Feed errors.
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("Database error: {0}")]
    DatabaseError(String),
}
