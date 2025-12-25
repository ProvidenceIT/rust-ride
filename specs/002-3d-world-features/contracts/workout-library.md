# Contract: Workout Library Module

**Module**: `src/workouts/`, `src/ui/screens/workout_library.rs`
**Feature**: 002-3d-world-features
**Status**: Extension of existing module

## Purpose

Complete the workout library with file browser import functionality using native file dialogs. The existing parsers (.zwo, .mrc) and workout engine are already implemented; this adds the import flow and library management UI.

## Public Interface

### WorkoutLibrary

```rust
pub struct WorkoutLibrary {
    workouts: Vec<WorkoutSummary>,
}

impl WorkoutLibrary {
    /// Create new library, loading from database
    pub fn new(db: &Database) -> Result<Self, WorkoutError>;

    /// Import workout from file path
    /// Parses file, validates, saves to database
    pub fn import_file(&mut self, path: &Path, db: &Database) -> Result<WorkoutSummary, WorkoutError>;

    /// Get all workouts for library display
    pub fn get_all(&self) -> &[WorkoutSummary];

    /// Get full workout definition for execution
    pub fn get_workout(&self, id: Uuid, db: &Database) -> Result<Workout, WorkoutError>;

    /// Delete workout from library
    pub fn delete(&mut self, id: Uuid, db: &Database) -> Result<(), WorkoutError>;

    /// Refresh library from database
    pub fn refresh(&mut self, db: &Database) -> Result<(), WorkoutError>;
}
```

### WorkoutSummary

```rust
pub struct WorkoutSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub duration_seconds: u32,
    pub workout_type: WorkoutType,
    pub source_file: Option<String>,
    pub tss_estimate: Option<f32>,
    pub interval_count: u16,
    pub preview_data: Vec<IntervalPreview>,  // For visual preview
}

pub struct IntervalPreview {
    pub start_percent: f32,      // 0.0 - 1.0 position
    pub end_percent: f32,
    pub intensity: f32,          // % of FTP for display height
    pub interval_type: IntervalType,
}

pub enum WorkoutType {
    Endurance,
    Tempo,
    Threshold,
    VO2Max,
    Anaerobic,
    Mixed,
}
```

### File Import Dialog

```rust
/// Opens native file dialog for workout import
/// Returns selected file path or None if cancelled
pub async fn pick_workout_file() -> Option<PathBuf> {
    use rfd::AsyncFileDialog;

    let file = AsyncFileDialog::new()
        .add_filter("Workout Files", &["zwo", "mrc"])
        .add_filter("Zwift Workouts", &["zwo"])
        .add_filter("MRC Workouts", &["mrc"])
        .set_title("Import Workout")
        .pick_file()
        .await?;

    Some(file.path().to_path_buf())
}
```

### Database Operations

```rust
impl Database {
    /// Save imported workout
    pub fn save_workout(&self, workout: &Workout) -> Result<(), DatabaseError>;

    /// Get all workout summaries
    pub fn get_workout_summaries(&self) -> Result<Vec<WorkoutSummary>, DatabaseError>;

    /// Get full workout by ID
    pub fn get_workout(&self, id: Uuid) -> Result<Option<Workout>, DatabaseError>;

    /// Delete workout
    pub fn delete_workout(&self, id: Uuid) -> Result<(), DatabaseError>;
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum WorkoutError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Workout not found: {0}")]
    WorkoutNotFound(Uuid),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## Import Flow

```
User clicks "Import"
    │
    ▼
Native File Dialog Opens
    │
    ├── Cancel ──► No action
    │
    └── Select file
           │
           ▼
        Detect format (.zwo/.mrc)
           │
           ▼
        Parse workout file
           │
           ├── Error ──► Show error toast
           │
           └── Success
                  │
                  ▼
               Save to database
                  │
                  ▼
               Add to library list
                  │
                  ▼
               Show success toast
```

## UI Components

### Library View
- Grid/list of workout cards
- Each card shows: name, duration, type icon, interval preview
- Import button in header
- Search/filter functionality (future)

### Workout Card
```
┌────────────────────────────────┐
│ Sweet Spot Base 1             │
│ ░░░░░▓▓▓▓▓░░░▓▓▓▓▓░░░░        │  ← Interval preview bars
│ 1:00:00 | Threshold | 12 int  │
└────────────────────────────────┘
```

### Import Button
- Primary action button in library header
- Opens native file picker
- Shows loading state during import
- Toast notification on success/failure

## Dependencies

- `rfd` - Native file dialogs (new dependency)
- Existing: `src/workouts/parser_zwo.rs`, `src/workouts/parser_mrc.rs`

## Implementation Notes

1. **Async file dialog**: Use `rfd::AsyncFileDialog` to avoid blocking UI
2. **File validation**: Check file extension and content before parsing
3. **Duplicate detection**: Warn if workout with same name exists (allow overwrite)
4. **Preview generation**: Calculate preview bars during import, store with summary
5. **TSS estimation**: Calculate from workout intervals and user FTP

## Testing Requirements

- Unit tests for import flow with fixture files
- Test both .zwo and .mrc formats
- Test error cases (corrupt file, unsupported format, permission denied)
- Manual testing of native file dialog on each platform
