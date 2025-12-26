# Contract: Social Module

**Module**: `src/social/`
**Purpose**: Rider profiles, clubs, badges, challenges, and activity feed

## Public API

### Profile Manager

```rust
/// Manages the local rider's profile
pub struct ProfileManager {
    // Internal database handle
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(db: Arc<Database>) -> Self;

    /// Get the current rider profile
    pub fn current(&self) -> Result<RiderProfile, SocialError>;

    /// Update profile fields
    pub fn update(&self, updates: ProfileUpdate) -> Result<RiderProfile, SocialError>;

    /// Get available avatars
    pub fn available_avatars(&self) -> Vec<AvatarInfo>;

    /// Update aggregate stats after a ride
    pub fn record_ride(&self, distance_km: f64, duration_hours: f64) -> Result<(), SocialError>;
}

pub struct RiderProfile {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_id: Option<String>,
    pub bio: Option<String>,
    pub ftp: Option<u16>,
    pub total_distance_km: f64,
    pub total_time_hours: f64,
    pub sharing_enabled: bool,
}

pub struct ProfileUpdate {
    pub display_name: Option<String>,
    pub avatar_id: Option<String>,
    pub bio: Option<String>,
    pub ftp: Option<u16>,
    pub sharing_enabled: Option<bool>,
}

pub struct AvatarInfo {
    pub id: String,
    pub name: String,
    pub preview_path: String,
}
```

### Club Manager

```rust
/// Manages clubs and memberships
pub struct ClubManager {
    // Internal database handle
}

impl ClubManager {
    pub fn new(db: Arc<Database>) -> Self;

    /// Create a new club (current rider becomes admin)
    pub fn create(&self, name: &str, description: Option<&str>) -> Result<Club, SocialError>;

    /// Join a club by code
    pub fn join(&self, join_code: &str) -> Result<Club, SocialError>;

    /// Leave a club
    pub fn leave(&self, club_id: Uuid) -> Result<(), SocialError>;

    /// Get clubs the current rider belongs to
    pub fn my_clubs(&self) -> Result<Vec<Club>, SocialError>;

    /// Get club details including roster
    pub fn get_club(&self, club_id: Uuid) -> Result<ClubDetails, SocialError>;

    /// Update club aggregate stats
    pub fn update_stats(&self, club_id: Uuid) -> Result<(), SocialError>;
}

pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub join_code: String,
    pub member_count: u32,
    pub is_admin: bool,
}

pub struct ClubDetails {
    pub club: Club,
    pub members: Vec<ClubMember>,
    pub total_distance_km: f64,
    pub total_time_hours: f64,
}

pub struct ClubMember {
    pub rider_id: Uuid,
    pub display_name: String,
    pub joined_at: DateTime<Utc>,
    pub contribution_km: f64,
}
```

### Badge System

```rust
/// Manages achievement badges
pub struct BadgeSystem {
    // Internal database handle
}

impl BadgeSystem {
    pub fn new(db: Arc<Database>) -> Self;

    /// Get all available badges with unlock status
    pub fn all_badges(&self) -> Result<Vec<BadgeStatus>, SocialError>;

    /// Get only earned badges
    pub fn earned_badges(&self) -> Result<Vec<EarnedBadge>, SocialError>;

    /// Check for new badges after activity
    /// Returns newly earned badges
    pub fn check_progress(&self, context: &BadgeContext) -> Result<Vec<EarnedBadge>, SocialError>;

    /// Initialize badge definitions (called on first run)
    pub fn initialize_badges(&self) -> Result<(), SocialError>;
}

pub struct BadgeStatus {
    pub badge: Badge,
    pub earned: bool,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub progress: Option<f64>,  // 0.0 - 1.0 for trackable badges
}

pub struct Badge {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: BadgeCategory,
}

pub struct EarnedBadge {
    pub badge: Badge,
    pub unlocked_at: DateTime<Utc>,
}

pub struct BadgeContext {
    pub total_distance_km: f64,
    pub current_ftp: u16,
    pub previous_ftp: Option<u16>,
    pub consecutive_days: u32,
    pub workouts_completed: u32,
}

#[derive(Clone, Copy)]
pub enum BadgeCategory {
    Distance,
    FTP,
    Consistency,
    Special,
}
```

### Challenge Manager

```rust
/// Manages training challenges
pub struct ChallengeManager {
    // Internal database handle
}

impl ChallengeManager {
    pub fn new(db: Arc<Database>) -> Self;

    /// Create a new challenge
    pub fn create(&self, config: ChallengeConfig) -> Result<Challenge, SocialError>;

    /// Get active challenges for current rider
    pub fn active_challenges(&self) -> Result<Vec<ChallengeWithProgress>, SocialError>;

    /// Get completed challenges
    pub fn completed_challenges(&self) -> Result<Vec<ChallengeWithProgress>, SocialError>;

    /// Update progress after a ride
    pub fn update_progress(&self, ride_data: &RideData) -> Result<Vec<ChallengeCompleted>, SocialError>;

    /// Export challenge to TOML
    pub fn export(&self, challenge_id: Uuid) -> Result<String, SocialError>;

    /// Import challenge from TOML
    pub fn import(&self, toml_content: &str) -> Result<Challenge, SocialError>;

    /// Join an imported challenge
    pub fn join(&self, challenge_id: Uuid) -> Result<(), SocialError>;
}

pub struct ChallengeConfig {
    pub name: String,
    pub description: Option<String>,
    pub goal_type: GoalType,
    pub goal_value: f64,
    pub duration_days: u16,
    pub start_date: NaiveDate,
}

pub struct Challenge {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub goal_type: GoalType,
    pub goal_value: f64,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub created_by: String,
}

pub struct ChallengeWithProgress {
    pub challenge: Challenge,
    pub current_value: f64,
    pub progress_percent: f64,
    pub completed: bool,
    pub days_remaining: i32,
}

pub struct ChallengeCompleted {
    pub challenge: Challenge,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum GoalType {
    TotalDistanceKm,
    TotalTimeHours,
    TotalTss,
    WorkoutCount,
    WorkoutTypeCount(String),
}
```

### Activity Feed

```rust
/// Manages activity feed and sharing
pub struct ActivityFeed {
    // Internal state
}

impl ActivityFeed {
    pub fn new(db: Arc<Database>, discovery: Arc<DiscoveryService>) -> Self;

    /// Get local activities
    pub fn local_activities(&self, limit: usize) -> Result<Vec<ActivitySummary>, SocialError>;

    /// Get activities from LAN peers
    pub fn peer_activities(&self, limit: usize) -> Result<Vec<ActivitySummary>, SocialError>;

    /// Record a new activity (after ride completion)
    pub fn record(&self, ride: &CompletedRide) -> Result<ActivitySummary, SocialError>;

    /// Toggle sharing for an activity
    pub fn set_shared(&self, activity_id: Uuid, shared: bool) -> Result<(), SocialError>;

    /// Refresh activities from peers
    pub fn refresh_peers(&self) -> Result<(), SocialError>;
}

pub struct ActivitySummary {
    pub id: Uuid,
    pub rider: ActivityRider,
    pub distance_km: f64,
    pub duration_minutes: u32,
    pub avg_power_watts: Option<u16>,
    pub elevation_gain_m: f64,
    pub world_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
    pub is_local: bool,
}

pub struct ActivityRider {
    pub id: Uuid,
    pub name: String,
    pub avatar_id: Option<String>,
}
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SocialError {
    #[error("Profile not found")]
    ProfileNotFound,

    #[error("Club not found")]
    ClubNotFound,

    #[error("Invalid join code")]
    InvalidJoinCode,

    #[error("Already a member")]
    AlreadyMember,

    #[error("Not a member")]
    NotMember,

    #[error("Challenge not found")]
    ChallengeNotFound,

    #[error("Challenge expired")]
    ChallengeExpired,

    #[error("Invalid challenge format: {0}")]
    InvalidChallengeFormat(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}
```
