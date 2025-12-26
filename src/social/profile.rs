//! Rider profile management.
//!
//! Provides profile creation, updates, and stats aggregation.

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use super::types::RiderProfile;
use crate::storage::{Database, SocialStore};

/// Profile manager for rider profiles.
pub struct ProfileManager {
    db: Arc<Database>,
}

impl ProfileManager {
    /// Create a new profile manager.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get or create a rider profile.
    pub fn get_or_create_profile(&self, user_id: &Uuid) -> Result<RiderProfile, ProfileError> {
        let store = SocialStore::new(self.db.connection());
        let rider = store
            .get_or_create_rider(user_id)
            .map_err(|e| ProfileError::DatabaseError(e.to_string()))?;

        Ok(RiderProfile {
            id: rider.id,
            display_name: rider.display_name,
            avatar_id: rider.avatar_id,
            bio: rider.bio,
            ftp: rider.ftp,
            total_distance_km: rider.total_distance_km,
            total_time_hours: rider.total_time_hours,
            total_rides: 0,
            sharing_enabled: rider.sharing_enabled,
            created_at: rider.created_at,
            updated_at: rider.updated_at,
        })
    }

    /// Update a rider profile.
    pub fn update_profile(&self, profile: &RiderProfile) -> Result<(), ProfileError> {
        let store = SocialStore::new(self.db.connection());
        let rider = crate::storage::Rider {
            id: profile.id,
            display_name: profile.display_name.clone(),
            avatar_id: profile.avatar_id.clone(),
            bio: profile.bio.clone(),
            ftp: profile.ftp,
            total_distance_km: profile.total_distance_km,
            total_time_hours: profile.total_time_hours,
            sharing_enabled: profile.sharing_enabled,
            created_at: profile.created_at,
            updated_at: Utc::now(),
        };

        store
            .update_rider(&rider)
            .map_err(|e| ProfileError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update rider stats after a ride.
    pub fn update_stats_after_ride(
        &self,
        user_id: &Uuid,
        distance_km: f64,
        duration_hours: f64,
    ) -> Result<(), ProfileError> {
        let profile = self.get_or_create_profile(user_id)?;

        let updated = RiderProfile {
            total_distance_km: profile.total_distance_km + distance_km,
            total_time_hours: profile.total_time_hours + duration_hours,
            updated_at: Utc::now(),
            ..profile
        };

        self.update_profile(&updated)
    }
}

/// Profile errors.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Profile not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid display name")]
    InvalidDisplayName,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
