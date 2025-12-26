# Contract: Workouts Extensions

**Module**: `src/workouts/` (extensions to existing module)
**Purpose**: Community workout repository and ratings/reviews

## Public API

### Workout Repository

```rust
/// Manages the community workout repository
pub struct WorkoutRepository {
    // Internal state
}

impl WorkoutRepository {
    pub fn new(db: Arc<Database>) -> Self;

    /// Get all repository workouts (bundled + synced)
    pub fn all_workouts(&self) -> Result<Vec<RepositoryWorkout>, WorkoutError>;

    /// Search and filter workouts
    pub fn search(&self, query: WorkoutQuery) -> Result<Vec<RepositoryWorkout>, WorkoutError>;

    /// Get workout details
    pub fn get_workout(&self, workout_id: &str) -> Result<RepositoryWorkout, WorkoutError>;

    /// Add workout to personal library
    pub fn add_to_library(&self, workout_id: &str) -> Result<(), WorkoutError>;

    /// Remove from personal library
    pub fn remove_from_library(&self, workout_id: &str) -> Result<(), WorkoutError>;

    /// Check for updates from GitHub (optional)
    pub async fn sync_from_github(&self) -> Result<SyncResult, WorkoutError>;

    /// Get bundled workout count
    pub fn bundled_count(&self) -> usize;
}

pub struct RepositoryWorkout {
    pub id: String,
    pub name: String,
    pub description: String,
    pub duration_minutes: u16,
    pub difficulty: Difficulty,
    pub focus: TrainingFocus,
    pub author: Option<String>,
    pub avg_rating: Option<f32>,
    pub rating_count: u32,
    pub in_library: bool,
    pub bundled: bool,
}

pub struct WorkoutQuery {
    pub text_search: Option<String>,
    pub difficulty: Option<Difficulty>,
    pub focus: Option<TrainingFocus>,
    pub min_duration: Option<u16>,
    pub max_duration: Option<u16>,
    pub min_rating: Option<f32>,
    pub in_library_only: bool,
    pub sort_by: WorkoutSort,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Clone, Copy)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Clone, Copy)]
pub enum TrainingFocus {
    Endurance,
    Tempo,
    Threshold,
    VO2Max,
    Sprint,
    Recovery,
    Mixed,
}

#[derive(Clone, Copy)]
pub enum WorkoutSort {
    NameAsc,
    NameDesc,
    DurationAsc,
    DurationDesc,
    RatingDesc,
    RecentlyAdded,
}

pub struct SyncResult {
    pub new_workouts: u32,
    pub updated_workouts: u32,
    pub removed_workouts: u32,
    pub last_sync: DateTime<Utc>,
}
```

### Workout Ratings

```rust
/// Manages workout ratings and reviews
pub struct WorkoutRatings {
    // Internal database handle
}

impl WorkoutRatings {
    pub fn new(db: Arc<Database>) -> Self;

    /// Rate a workout
    pub fn rate(&self, workout_id: &str, rating: u8, review: Option<&str>) -> Result<(), WorkoutError>;

    /// Update an existing rating
    pub fn update_rating(&self, workout_id: &str, rating: u8, review: Option<&str>) -> Result<(), WorkoutError>;

    /// Delete a rating
    pub fn delete_rating(&self, workout_id: &str) -> Result<(), WorkoutError>;

    /// Get current rider's rating for a workout
    pub fn my_rating(&self, workout_id: &str) -> Result<Option<WorkoutRating>, WorkoutError>;

    /// Get all ratings for a workout
    pub fn get_ratings(&self, workout_id: &str) -> Result<WorkoutRatingsSummary, WorkoutError>;

    /// Get recent reviews
    pub fn recent_reviews(&self, limit: usize) -> Result<Vec<WorkoutReview>, WorkoutError>;
}

pub struct WorkoutRating {
    pub workout_id: String,
    pub rating: u8,
    pub review_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct WorkoutRatingsSummary {
    pub workout_id: String,
    pub avg_rating: f32,
    pub rating_count: u32,
    pub rating_distribution: [u32; 5],  // Count per star (1-5)
    pub reviews: Vec<WorkoutReview>,
}

pub struct WorkoutReview {
    pub rider_name: String,
    pub rating: u8,
    pub review_text: String,
    pub created_at: DateTime<Utc>,
}
```

## Bundled Workouts

The repository includes 50-100 pre-curated workouts in TOML format:

```toml
# Example: workouts/threshold/sweet_spot_60.toml
[workout]
id = "threshold_sweet_spot_60"
name = "Sweet Spot 60"
description = "One hour of sweet spot intervals for building threshold power"
duration_minutes = 60
difficulty = "intermediate"
focus = "threshold"
author = "RustRide Team"

[[intervals]]
type = "warmup"
duration_seconds = 600
power_low = 0.50
power_high = 0.65

[[intervals]]
type = "interval"
duration_seconds = 1200
power_target = 0.90
repeat = 3
rest_seconds = 300
rest_power = 0.55

[[intervals]]
type = "cooldown"
duration_seconds = 300
power_low = 0.50
power_high = 0.40
```

### Bundled Workout Categories

| Focus | Count | Description |
|-------|-------|-------------|
| Endurance | 15 | Zone 2 base building |
| Tempo | 10 | Sweet spot and tempo work |
| Threshold | 15 | FTP development |
| VO2Max | 10 | High intensity intervals |
| Sprint | 5 | Neuromuscular power |
| Recovery | 5 | Active recovery |
| Mixed | 15 | Multi-focus workouts |

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum WorkoutError {
    #[error("Workout not found: {0}")]
    WorkoutNotFound(String),

    #[error("Already in library")]
    AlreadyInLibrary,

    #[error("Not in library")]
    NotInLibrary,

    #[error("Invalid rating: must be 1-5")]
    InvalidRating,

    #[error("Already rated")]
    AlreadyRated,

    #[error("Sync failed: {0}")]
    SyncFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
```
