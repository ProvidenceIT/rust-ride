# Contract: Achievement System Module

**Module**: `src/world/achievements/`
**Date**: 2025-12-25

## Purpose

Track user accomplishments, award badges, manage collectibles, and provide engagement through goal-based challenges.

## Public API

### Types

```rust
/// Achievement manager tracks progress and awards
pub struct AchievementManager {
    definitions: Vec<Achievement>,
    progress: HashMap<Uuid, AchievementProgress>,
    pending_awards: Vec<AchievementAward>,
}

/// Award notification for UI
pub struct AchievementAward {
    pub achievement: Achievement,
    pub awarded_at: DateTime<Utc>,
    pub is_hidden_reveal: bool,
}

/// Achievement unlock criteria
pub enum AchievementCriteria {
    /// Cumulative distance in meters
    TotalDistance(u64),
    /// Cumulative elevation gain in meters
    TotalElevation(u64),
    /// Number of segments completed
    SegmentsCompleted(u32),
    /// Number of landmarks discovered
    LandmarksDiscovered(u32),
    /// Single ride duration in minutes
    SingleRideDuration(u32),
    /// NPCs passed in single ride
    NpcsPassed(u32),
    /// Collectibles collected
    CollectiblesCollected(u32),
    /// Personal bests achieved
    PersonalBests(u32),
    /// Specific segment completed
    SpecificSegment(Uuid),
    /// Specific landmark discovered
    SpecificLandmark(Uuid),
    /// All landmarks on route discovered
    RouteFullyExplored(Uuid),
}
```

### Functions

```rust
impl AchievementManager {
    /// Create manager and load definitions
    pub fn new(db: &Database) -> Result<Self, DatabaseError>;

    /// Load user's progress from database
    pub fn load_progress(
        &mut self,
        user_id: Uuid,
        db: &Database,
    ) -> Result<(), DatabaseError>;

    /// Get all achievements
    pub fn achievements(&self) -> &[Achievement];

    /// Get achievements by category
    pub fn achievements_by_category(
        &self,
        category: AchievementCategory,
    ) -> Vec<&Achievement>;

    /// Get user's progress on an achievement
    pub fn get_progress(&self, achievement_id: Uuid) -> Option<&AchievementProgress>;

    /// Get all completed achievements
    pub fn completed(&self) -> Vec<&Achievement>;

    /// Get achievements in progress (>0% but not complete)
    pub fn in_progress(&self) -> Vec<(&Achievement, &AchievementProgress)>;

    /// Process ride completion event
    ///
    /// Updates progress for relevant achievements and returns any new awards.
    pub fn on_ride_completed(
        &mut self,
        user_id: Uuid,
        ride_stats: &RideStats,
        db: &mut Database,
    ) -> Result<Vec<AchievementAward>, DatabaseError>;

    /// Process segment completion event
    pub fn on_segment_completed(
        &mut self,
        user_id: Uuid,
        segment: &Segment,
        time: &SegmentTime,
        db: &mut Database,
    ) -> Result<Vec<AchievementAward>, DatabaseError>;

    /// Process landmark discovery event
    pub fn on_landmark_discovered(
        &mut self,
        user_id: Uuid,
        landmark: &Landmark,
        db: &mut Database,
    ) -> Result<Vec<AchievementAward>, DatabaseError>;

    /// Process collectible pickup event
    pub fn on_collectible_picked(
        &mut self,
        user_id: Uuid,
        collectible: &Collectible,
        db: &mut Database,
    ) -> Result<Vec<AchievementAward>, DatabaseError>;

    /// Get pending awards (clears queue after call)
    pub fn take_pending_awards(&mut self) -> Vec<AchievementAward>;

    /// Get total achievement points earned
    pub fn total_points(&self) -> u32;

    /// Get completion percentage
    pub fn completion_percent(&self) -> f32;
}

/// Ride statistics for achievement processing
pub struct RideStats {
    pub distance_meters: f64,
    pub elevation_gain_meters: f64,
    pub duration_seconds: u32,
    pub npcs_passed: u32,
    pub segments_completed: u32,
    pub landmarks_discovered: u32,
    pub collectibles_collected: u32,
    pub personal_bests: u32,
}
```

### Collectible Manager

```rust
/// Manages in-world collectibles
pub struct CollectibleManager {
    collectibles: Vec<Collectible>,
    collected: HashSet<Uuid>,
}

impl CollectibleManager {
    /// Create manager for route
    pub fn new(route_id: Uuid, user_id: Uuid, db: &Database) -> Result<Self, DatabaseError>;

    /// Get collectibles on route
    pub fn collectibles(&self) -> &[Collectible];

    /// Get uncollected collectibles
    pub fn uncollected(&self) -> Vec<&Collectible>;

    /// Check for collectible pickup at distance
    ///
    /// Returns collected item if user is within pickup range.
    pub fn check_pickup(
        &mut self,
        user_distance: f64,
        user_lateral_offset: f32,
    ) -> Option<&Collectible>;

    /// Record pickup to database
    pub fn record_pickup(
        &mut self,
        collectible_id: Uuid,
        user_id: Uuid,
        ride_id: Uuid,
        db: &mut Database,
    ) -> Result<(), DatabaseError>;

    /// Get render data for uncollected items
    pub fn render_data(&self) -> Vec<CollectibleRenderData>;
}

pub struct CollectibleRenderData {
    pub position: Vec3,
    pub collectible_type: CollectibleType,
    pub rotation: f32,
    pub scale: f32,
}
```

### Database Operations

```rust
impl Database {
    /// Get all achievement definitions
    pub fn get_achievements(&self) -> Result<Vec<Achievement>, DatabaseError>;

    /// Get user's achievement progress
    pub fn get_achievement_progress(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<AchievementProgress>, DatabaseError>;

    /// Update achievement progress
    pub fn update_achievement_progress(
        &self,
        progress: &AchievementProgress,
    ) -> Result<(), DatabaseError>;

    /// Get collectibles for route
    pub fn get_collectibles(
        &self,
        route_id: &Uuid,
    ) -> Result<Vec<Collectible>, DatabaseError>;

    /// Get user's collected items
    pub fn get_collected(
        &self,
        user_id: &Uuid,
        route_id: &Uuid,
    ) -> Result<Vec<Uuid>, DatabaseError>;

    /// Record collectible pickup
    pub fn insert_collectible_pickup(
        &self,
        pickup: &CollectiblePickup,
    ) -> Result<(), DatabaseError>;
}
```

## Achievement Definitions (Initial Set)

### Distance Achievements
| Name | Target | Rarity | Points |
|------|--------|--------|--------|
| First Kilometer | 1 km | Common | 10 |
| Century Rider | 100 km | Uncommon | 50 |
| Metric Century | 1,000 km | Rare | 100 |
| Tour Veteran | 10,000 km | Epic | 500 |

### Climbing Achievements
| Name | Target | Rarity | Points |
|------|--------|--------|--------|
| Hill Hunter | 1,000 m | Common | 20 |
| Mountain Goat | 10,000 m | Uncommon | 100 |
| Everesting | 8,849 m single | Epic | 1000 |
| King of the Mountains | 100,000 m | Legendary | 2000 |

### Exploration Achievements
| Name | Target | Rarity | Points |
|------|--------|--------|--------|
| Explorer | 10 landmarks | Common | 30 |
| Cartographer | 50 landmarks | Uncommon | 150 |
| World Traveler | 200 landmarks | Rare | 500 |

### Segment Achievements
| Name | Target | Rarity | Points |
|------|--------|--------|--------|
| Segment Hunter | 10 segments | Common | 25 |
| Personal Best | 5 PBs | Uncommon | 50 |
| Leaderboard Legend | Top 10 any segment | Rare | 200 |

## Collectible Pickup Logic

```
Pickup Range:
- Distance tolerance: ±3 meters along route
- Lateral tolerance: ±2 meters from center

Collectible Positioning:
- Stars: On road, visible
- Hidden items: Off-road, requires exploration
- Jersey unlocks: At landmarks

Respawn: Collectibles do not respawn once collected
```

## Performance Requirements

- Achievement check: <1ms per event
- Progress query: <50ms
- Award notification: <100ms
- Collectible pickup detection: <0.1ms per update

## UI Integration

- Achievement gallery screen showing all achievements
- Progress bars for in-progress achievements
- Award notification popup (toast style)
- Points counter in profile
- Collectible pickup animation and sound
