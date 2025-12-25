# Contract: Ride Persistence Module

**Module**: `src/recording/`, `src/storage/`
**Feature**: 002-3d-world-features
**Status**: Extension of existing modules

## Purpose

Complete ride data persistence with automatic saving during rides and crash recovery on startup. Extends the existing recorder and storage modules.

## Public Interface

### RideRecorder Extensions

```rust
impl RideRecorder {
    /// Save current ride to database
    /// Called when user ends ride normally
    pub fn save_ride(&mut self) -> Result<Uuid, RecorderError>;

    /// Enable auto-save (called when ride starts)
    /// Saves to autosave table every 30 seconds
    pub fn enable_autosave(&mut self);

    /// Clear autosave data (called after successful save or discard)
    pub fn clear_autosave(&self) -> Result<(), RecorderError>;

    /// Check if crash recovery data exists
    pub fn has_recovery_data(&self) -> bool;

    /// Load recovery data and return recoverable ride
    pub fn recover_ride(&self) -> Result<RecoverableRide, RecorderError>;

    /// Discard recovery data without saving
    pub fn discard_recovery(&self) -> Result<(), RecorderError>;
}

pub struct RecoverableRide {
    pub ride_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub last_sample_at: DateTime<Utc>,
    pub samples: Vec<RideSample>,
    pub workout_id: Option<Uuid>,
    pub workout_elapsed: Option<u32>,
}
```

### Database Extensions

```rust
impl Database {
    /// Save completed ride with all samples and metrics
    pub fn save_ride(&self, ride: &Ride, samples: &[RideSample]) -> Result<(), DatabaseError>;

    /// Get all rides ordered by date descending
    pub fn get_all_rides(&self) -> Result<Vec<RideSummary>, DatabaseError>;

    /// Get ride with all samples for detail view
    pub fn get_ride_detail(&self, ride_id: Uuid) -> Result<Option<RideDetail>, DatabaseError>;

    /// Delete ride and all associated samples
    pub fn delete_ride(&self, ride_id: Uuid) -> Result<(), DatabaseError>;

    /// Autosave operations
    pub fn upsert_autosave(&self, data: &AutosaveData) -> Result<(), DatabaseError>;
    pub fn get_autosave(&self) -> Result<Option<AutosaveData>, DatabaseError>;
    pub fn clear_autosave(&self) -> Result<(), DatabaseError>;
}
```

### Data Types

```rust
pub struct RideSummary {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: u32,
    pub distance_meters: f32,
    pub avg_power: Option<u16>,
    pub avg_heart_rate: Option<u8>,
    pub avg_cadence: Option<u8>,
    pub tss: Option<f32>,
}

pub struct RideDetail {
    pub summary: RideSummary,
    pub samples: Vec<RideSample>,
    pub normalized_power: Option<u16>,
    pub intensity_factor: Option<f32>,
    pub max_power: Option<u16>,
    pub max_heart_rate: Option<u8>,
    pub max_speed: Option<f32>,
    pub power_zones: [u32; 7],      // seconds in each zone
    pub hr_zones: [u32; 5],         // seconds in each zone
}

pub struct AutosaveData {
    pub ride_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub last_sample_at: DateTime<Utc>,
    pub samples_json: String,
    pub workout_id: Option<Uuid>,
    pub workout_elapsed: Option<u32>,
    pub updated_at: DateTime<Utc>,
}
```

## Database Schema

```sql
-- Autosave table (single row)
CREATE TABLE IF NOT EXISTS autosave (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    ride_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    last_sample_at TEXT NOT NULL,
    samples_json TEXT NOT NULL,
    workout_id TEXT,
    workout_elapsed INTEGER,
    updated_at TEXT NOT NULL
);
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RecorderError {
    #[error("No active ride")]
    NoActiveRide,

    #[error("Ride already in progress")]
    RideInProgress,

    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("No recovery data available")]
    NoRecoveryData,
}
```

## Autosave Flow

```
Ride Start
    │
    ▼
Enable Autosave ──────────────────────────┐
    │                                      │
    ▼                                      │
Recording Loop                             │
    │                                      │
    ├──── Every 30s ───► Upsert Autosave ─┘
    │
    ▼
Ride End
    │
    ├── Save Ride ──► Clear Autosave
    │
    └── (or crash)
           │
           ▼
        Next Launch
           │
           ▼
        Check Autosave
           │
           ├── Found ──► Show Recovery Dialog
           │                  │
           │                  ├── Recover ──► Load + Save + Clear
           │                  │
           │                  └── Discard ──► Clear
           │
           └── Not Found ──► Normal Startup
```

## Implementation Notes

1. **Autosave interval**: 30 seconds (configurable)
2. **Sample serialization**: JSON array for flexibility
3. **Recovery prompt**: Modal dialog on startup, cannot be dismissed without choice
4. **Metrics calculation**: NP, TSS, IF calculated on save, not during recording
5. **Max speed**: Calculate from samples during save for TCX export

## Testing Requirements

- Unit tests for save/load operations
- Integration tests for crash recovery simulation
- Test with large sample counts (2+ hour rides = 7200+ samples)
