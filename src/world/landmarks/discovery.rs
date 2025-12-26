//! Landmark discovery and progression system.

use super::{Landmark, LandmarkType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Discovery record for a landmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandmarkDiscovery {
    /// Unique discovery ID
    pub id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Landmark ID
    pub landmark_id: Uuid,
    /// Ride during which discovered
    pub ride_id: Uuid,
    /// When discovered
    pub discovered_at: DateTime<Utc>,
    /// Optional screenshot path
    pub screenshot_path: Option<String>,
}

impl LandmarkDiscovery {
    /// Create new discovery
    pub fn new(user_id: Uuid, landmark_id: Uuid, ride_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            landmark_id,
            ride_id,
            discovered_at: Utc::now(),
            screenshot_path: None,
        }
    }
}

/// Discovery progress for a route/region
#[derive(Debug, Clone, Default)]
pub struct DiscoveryProgress {
    /// Total landmarks in region
    pub total: u32,
    /// Discovered landmarks
    pub discovered: u32,
    /// By type breakdown
    pub by_type: std::collections::HashMap<LandmarkType, (u32, u32)>, // (discovered, total)
}

impl DiscoveryProgress {
    /// Calculate percentage complete
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.discovered as f32 / self.total as f32) * 100.0
        }
    }

    /// Check if fully discovered
    pub fn is_complete(&self) -> bool {
        self.discovered >= self.total
    }
}

/// Discovery tracker maintains user's exploration progress
pub struct DiscoveryTracker {
    /// User ID
    user_id: Uuid,
    /// All discoveries (landmark_id -> discovery)
    discoveries: std::collections::HashMap<Uuid, LandmarkDiscovery>,
    /// Progress by route
    route_progress: std::collections::HashMap<Uuid, DiscoveryProgress>,
}

impl DiscoveryTracker {
    /// Create discovery tracker for user
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            discoveries: std::collections::HashMap::new(),
            route_progress: std::collections::HashMap::new(),
        }
    }

    /// Check if landmark is discovered
    pub fn is_discovered(&self, landmark_id: Uuid) -> bool {
        self.discoveries.contains_key(&landmark_id)
    }

    /// Record a new discovery
    pub fn discover(&mut self, landmark: &Landmark, ride_id: Uuid) -> Option<LandmarkDiscovery> {
        if self.is_discovered(landmark.id) {
            return None;
        }

        let discovery = LandmarkDiscovery::new(self.user_id, landmark.id, ride_id);
        self.discoveries.insert(landmark.id, discovery.clone());

        // Update route progress if applicable
        if let Some(route_id) = landmark.route_id {
            if let Some(progress) = self.route_progress.get_mut(&route_id) {
                progress.discovered += 1;

                if let Some((discovered, _total)) =
                    progress.by_type.get_mut(&landmark.landmark_type)
                {
                    *discovered += 1;
                }
            }
        }

        Some(discovery)
    }

    /// Initialize progress for a route
    #[allow(clippy::field_reassign_with_default)]
    pub fn init_route_progress(&mut self, route_id: Uuid, landmarks: &[Landmark]) {
        let mut progress = DiscoveryProgress::default();
        progress.total = landmarks.len() as u32;

        // Count by type
        for lm in landmarks {
            let entry = progress.by_type.entry(lm.landmark_type).or_insert((0, 0));
            entry.1 += 1;

            if self.is_discovered(lm.id) {
                progress.discovered += 1;
                entry.0 += 1;
            }
        }

        self.route_progress.insert(route_id, progress);
    }

    /// Get route progress
    pub fn route_progress(&self, route_id: Uuid) -> Option<&DiscoveryProgress> {
        self.route_progress.get(&route_id)
    }

    /// Get all discoveries
    pub fn all_discoveries(&self) -> Vec<&LandmarkDiscovery> {
        self.discoveries.values().collect()
    }

    /// Get total discovery count
    pub fn total_discovered(&self) -> usize {
        self.discoveries.len()
    }

    /// Load discoveries from storage (stub)
    pub fn load_from_storage(&mut self, discoveries: Vec<LandmarkDiscovery>) {
        for d in discoveries {
            self.discoveries.insert(d.landmark_id, d);
        }
    }
}

/// Check if a landmark would trigger a special discovery event
pub fn is_notable_discovery(landmark: &Landmark, discovery_count: u32) -> bool {
    // Notable if:
    // - Summit
    // - Historic site
    // - Milestone discovery count (10, 25, 50, 100...)
    matches!(
        landmark.landmark_type,
        LandmarkType::Summit | LandmarkType::Historic
    ) || matches!(discovery_count, 10 | 25 | 50 | 100 | 250 | 500 | 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_progress() {
        let progress = DiscoveryProgress {
            total: 10,
            discovered: 5,
            by_type: std::collections::HashMap::new(),
        };

        assert!((progress.percentage() - 50.0).abs() < 0.1);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_discovery_tracker() {
        let user_id = Uuid::new_v4();
        let mut tracker = DiscoveryTracker::new(user_id);

        let landmark = Landmark::new(
            LandmarkType::Summit,
            "Test Peak".to_string(),
            0.0,
            0.0,
            1000.0,
        );

        // First discovery should succeed
        let discovery = tracker.discover(&landmark, Uuid::new_v4());
        assert!(discovery.is_some());

        // Second discovery should return None (already discovered)
        let duplicate = tracker.discover(&landmark, Uuid::new_v4());
        assert!(duplicate.is_none());

        assert!(tracker.is_discovered(landmark.id));
    }
}
