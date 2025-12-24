# Contract: Workout Engine

**Module**: `src/workouts/`
**Responsibility**: Workout parsing, execution, and ERG target calculation

## Public Interface

### WorkoutEngine

State machine for executing structured workouts.

```rust
pub struct WorkoutEngine {
    // Internal state
}

impl WorkoutEngine {
    /// Create a new workout engine
    pub fn new(user_ftp: u16) -> Self;

    /// Load a workout for execution
    pub fn load(&mut self, workout: Workout) -> Result<(), WorkoutError>;

    /// Start the workout
    pub fn start(&mut self) -> Result<(), WorkoutError>;

    /// Pause the workout (freezes timer, clears ERG target)
    pub fn pause(&mut self) -> Result<(), WorkoutError>;

    /// Resume a paused workout
    pub fn resume(&mut self) -> Result<(), WorkoutError>;

    /// Stop the workout (can't be resumed)
    pub fn stop(&mut self) -> Result<WorkoutSummary, WorkoutError>;

    /// Skip to the next segment
    pub fn skip_segment(&mut self) -> Result<(), WorkoutError>;

    /// Extend current segment by additional time
    pub fn extend_segment(&mut self, additional: Duration) -> Result<(), WorkoutError>;

    /// Manually adjust target power offset (+/- watts)
    pub fn adjust_power(&mut self, offset: i16);

    /// Get current workout state
    pub fn state(&self) -> &WorkoutState;

    /// Tick the workout forward (call every frame or on timer)
    /// Returns the current ERG target power
    pub fn tick(&mut self, now: Instant) -> Option<u16>;

    /// Update user's FTP (affects % FTP calculations)
    pub fn set_ftp(&mut self, ftp: u16);
}
```

### WorkoutState

Current execution state.

```rust
pub struct WorkoutState {
    /// Current execution status
    pub status: WorkoutStatus,

    /// Loaded workout (if any)
    pub workout: Option<Workout>,

    /// Index of current segment
    pub current_segment_index: usize,

    /// Current segment details
    pub current_segment: Option<SegmentProgress>,

    /// Time elapsed in workout
    pub elapsed_time: Duration,

    /// Time remaining in workout
    pub remaining_time: Duration,

    /// Current ERG target power (absolute watts)
    pub target_power: Option<u16>,

    /// User-applied power offset
    pub power_offset: i16,

    /// Next segment preview
    pub next_segment: Option<SegmentPreview>,
}

pub enum WorkoutStatus {
    Empty,      // No workout loaded
    Ready,      // Workout loaded, not started
    Running,    // Active execution
    Paused,     // Temporarily stopped
    Completed,  // Finished normally
    Stopped,    // Stopped early by user
}

pub struct SegmentProgress {
    /// Segment definition
    pub segment: WorkoutSegment,

    /// Time elapsed in this segment
    pub elapsed: Duration,

    /// Time remaining in this segment
    pub remaining: Duration,

    /// Progress percentage (0.0 - 1.0)
    pub progress: f32,

    /// For intervals: current repeat number
    pub current_repeat: Option<u8>,

    /// For intervals: total repeats
    pub total_repeats: Option<u8>,

    /// For intervals: on or off portion
    pub interval_phase: Option<IntervalPhase>,
}

pub enum IntervalPhase {
    On,
    Off,
}

pub struct SegmentPreview {
    pub segment_type: SegmentType,
    pub duration: Duration,
    pub target_power_start: u16,
    pub target_power_end: u16,
    pub cadence_target: Option<CadenceTarget>,
}

pub struct WorkoutSummary {
    /// Whether workout was completed
    pub completed: bool,

    /// Total time spent
    pub total_time: Duration,

    /// Segments completed
    pub segments_completed: usize,

    /// Total segments
    pub total_segments: usize,
}
```

### Workout Parsing

```rust
/// Parse a .zwo (Zwift) workout file
pub fn parse_zwo(content: &str) -> Result<Workout, WorkoutParseError>;

/// Parse a .mrc/.erg (TrainerRoad) workout file
pub fn parse_mrc(content: &str) -> Result<Workout, WorkoutParseError>;

/// Serialize workout to native JSON format
pub fn to_json(workout: &Workout) -> Result<String, WorkoutParseError>;

/// Parse workout from native JSON format
pub fn from_json(json: &str) -> Result<Workout, WorkoutParseError>;

#[derive(Debug, thiserror::Error)]
pub enum WorkoutParseError {
    #[error("Invalid XML: {0}")]
    InvalidXml(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid power value: {0}")]
    InvalidPower(String),

    #[error("Invalid duration: {0}")]
    InvalidDuration(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum WorkoutError {
    #[error("No workout loaded")]
    NoWorkoutLoaded,

    #[error("Workout already running")]
    AlreadyRunning,

    #[error("Workout not running")]
    NotRunning,

    #[error("Workout completed")]
    Completed,

    #[error("Cannot skip: already on last segment")]
    CannotSkip,

    #[error("Invalid FTP value: {0}")]
    InvalidFtp(u16),
}
```

## Power Calculation Rules

### Steady State Segments
```
target_watts = power_target.to_absolute(ftp) + power_offset
```

### Ramp Segments
```
progress = elapsed_time / duration
start_watts = start_power.to_absolute(ftp)
end_watts = end_power.to_absolute(ftp)
target_watts = start_watts + (end_watts - start_watts) * progress + power_offset
```

### Interval Segments
```
if on_phase:
    target_watts = on_power.to_absolute(ftp) + power_offset
else:
    target_watts = off_power.to_absolute(ftp) + power_offset
```

### FreeRide Segments
```
target_watts = None  // ERG mode disabled
```

### Power Target Conversion
```rust
impl PowerTarget {
    fn to_absolute(&self, ftp: u16) -> u16 {
        match self {
            PowerTarget::Absolute(watts) => *watts,
            PowerTarget::PercentFtp(percent) => (ftp as f32 * (*percent as f32 / 100.0)) as u16,
            PowerTarget::Range { .. } => unreachable!(), // Ranges are for ramps
        }
    }
}
```

## Behavioral Requirements

1. **Timing**: Use monotonic clock (Instant) for elapsed time calculations
2. **Ramp smoothing**: Configurable ramp rate for transitions (default 3 seconds)
3. **Pause behavior**: On pause, clear ERG target; on resume, restore with ramp
4. **Segment extension**: Only affects current segment, doesn't shift subsequent segments
5. **Power offset**: Applied after all calculations, clamped to prevent negative values
6. **Text events**: Emit text events when segment has `text_event` field

## State Diagram

```
         load()
[Empty] ───────► [Ready]
                    │
                    │ start()
                    ▼
                [Running] ◄───────┐
                    │             │
          pause()   │   resume()  │
                    ▼             │
                [Paused] ─────────┘
                    │
                    │ stop()
                    ▼
                [Stopped]

[Running] ─────────────────────────►  [Completed]
          (all segments finished)
```
