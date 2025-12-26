# Contract: Route Import Module

**Module**: `src/world/import/`
**Date**: 2025-12-25

## Purpose

Parse GPS route files (GPX, FIT, TCX), fetch missing elevation data, convert to internal route representation, and persist to database.

## Public API

### Types

```rust
/// Result of route import operation
pub struct ImportResult {
    pub route: ImportedRoute,
    pub waypoints: Vec<RouteWaypoint>,
    pub warnings: Vec<ImportWarning>,
}

/// Warning during import (non-fatal issues)
pub enum ImportWarning {
    MissingElevation { count: u32 },
    ElevationFetched { count: u32 },
    PointsSimplified { original: u32, final_count: u32 },
    InvalidTimestamps { count: u32 },
}

/// Import error (fatal)
pub enum ImportError {
    FileNotFound(PathBuf),
    InvalidFormat(String),
    ParseError(String),
    TooLarge { size_mb: f64, max_mb: f64 },
    RouteTooLong { distance_km: f64, max_km: f64 },
    ElevationFetchFailed(String),
    DatabaseError(String),
}
```

### Functions

```rust
/// Import a route from file
///
/// Detects format from extension (.gpx, .fit, .tcx) and parses accordingly.
/// Fetches missing elevation data if network available.
/// Simplifies route if too many points (>10,000).
///
/// # Arguments
/// * `path` - Path to route file
/// * `user_id` - Owner of imported route
/// * `options` - Import configuration options
///
/// # Returns
/// * `Ok(ImportResult)` - Successfully imported route with any warnings
/// * `Err(ImportError)` - Fatal error during import
pub async fn import_route(
    path: &Path,
    user_id: Uuid,
    options: ImportOptions,
) -> Result<ImportResult, ImportError>;

/// Import options
pub struct ImportOptions {
    /// Name override (uses filename if None)
    pub name: Option<String>,
    /// Whether to fetch missing elevation
    pub fetch_elevation: bool,
    /// Maximum points to keep (simplifies if exceeded)
    pub max_points: u32,
    /// Progress callback
    pub progress: Option<Box<dyn Fn(ImportProgress) + Send>>,
}

/// Progress update during import
pub struct ImportProgress {
    pub stage: ImportStage,
    pub percent: f32,
    pub message: String,
}

pub enum ImportStage {
    Parsing,
    FetchingElevation,
    GeneratingTerrain,
    Saving,
}

/// Preview a route without full import
///
/// Parses file and returns summary without saving to database.
pub async fn preview_route(path: &Path) -> Result<RoutePreview, ImportError>;

pub struct RoutePreview {
    pub name: String,
    pub point_count: u32,
    pub distance_km: f64,
    pub elevation_gain_m: f64,
    pub has_elevation: bool,
    pub bounds: GeoBounds,
}

pub struct GeoBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}
```

## Internal Modules

### `gpx.rs`

```rust
/// Parse GPX file to waypoints
pub fn parse_gpx(content: &[u8]) -> Result<Vec<GpsPoint>, ParseError>;
```

### `fit.rs`

```rust
/// Parse FIT file to waypoints
pub fn parse_fit(content: &[u8]) -> Result<Vec<GpsPoint>, ParseError>;
```

### `tcx.rs`

```rust
/// Parse TCX file to waypoints
pub fn parse_tcx(content: &[u8]) -> Result<Vec<GpsPoint>, ParseError>;
```

### `elevation.rs`

```rust
/// Fetch elevation for points missing it
pub async fn fetch_elevation(
    points: &mut [GpsPoint],
) -> Result<u32, ElevationError>;
```

## Dependencies

- `gpx` crate - GPX parsing
- `fitparser` crate - FIT parsing
- `quick-xml` crate - TCX parsing (existing)
- `reqwest` crate - HTTP client for elevation API (existing)

## Error Handling

- File read errors → `ImportError::FileNotFound`
- Format detection failure → `ImportError::InvalidFormat`
- Parse errors → `ImportError::ParseError` with details
- File >50MB → `ImportError::TooLarge`
- Route >500km → `ImportError::RouteTooLong`
- Network failure → Warn but continue without elevation
- Database errors → `ImportError::DatabaseError`

## Performance Requirements

- Parse 50MB GPX file in <10s
- Elevation fetch for 1000 points in <5s (batched)
- Route simplification in <1s
- Total import time <30s for typical routes
