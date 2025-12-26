//! Route import subsystem for GPS file parsing and elevation data fetching.
//!
//! This module provides functionality to import routes from GPX, FIT, and TCX files,
//! convert GPS coordinates to 3D world coordinates, and generate terrain data.

pub mod elevation;
pub mod fit;
pub mod gpx;
pub mod tcx;

use chrono::{DateTime, Utc};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

/// An imported route with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedRoute {
    /// Unique identifier
    pub id: Uuid,
    /// Route name
    pub name: String,
    /// Total distance in meters
    pub distance_meters: f64,
    /// Total elevation gain in meters
    pub elevation_gain_meters: f32,
    /// Source file format
    pub source_format: String,
    /// When imported
    pub imported_at: DateTime<Utc>,
}

impl ImportedRoute {
    /// Create a new imported route
    pub fn new(
        name: String,
        distance_meters: f64,
        elevation_gain_meters: f32,
        source_format: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            distance_meters,
            elevation_gain_meters,
            source_format,
            imported_at: Utc::now(),
        }
    }
}

/// A waypoint on an imported route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteWaypoint {
    /// Sequence index
    pub index: u32,
    /// GPS latitude
    pub latitude: f64,
    /// GPS longitude
    pub longitude: f64,
    /// Elevation in meters
    pub elevation_meters: f32,
    /// Distance from route start in meters
    pub distance_meters: f64,
    /// Gradient at this point (percent)
    pub gradient_percent: f32,
    /// 3D world position
    pub position: Vec3,
}

/// Errors that can occur during route import
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("File not found: {0}")]
    FileNotFound(std::path::PathBuf),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("File too large: {size_mb:.1}MB exceeds maximum {max_mb:.1}MB")]
    TooLarge { size_mb: f64, max_mb: f64 },

    #[error("Route too long: {distance_km:.1}km exceeds maximum {max_km:.1}km")]
    RouteTooLong { distance_km: f64, max_km: f64 },

    #[error("Elevation fetch failed: {0}")]
    ElevationFetchFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Warning during import (non-fatal issues)
#[derive(Debug, Clone)]
pub enum ImportWarning {
    MissingElevation { count: u32 },
    ElevationFetched { count: u32 },
    PointsSimplified { original: u32, final_count: u32 },
    InvalidTimestamps { count: u32 },
}

/// Result of route import operation
#[derive(Debug)]
pub struct ImportResult {
    pub route: ImportedRoute,
    pub waypoints: Vec<RouteWaypoint>,
    pub warnings: Vec<ImportWarning>,
}

/// Import configuration options
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Name override (uses filename if None)
    pub name: Option<String>,
    /// Whether to fetch missing elevation
    pub fetch_elevation: bool,
    /// Maximum points to keep (simplifies if exceeded)
    pub max_points: u32,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            name: None,
            fetch_elevation: true,
            max_points: 10_000,
        }
    }
}

/// Progress update during import
#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub stage: ImportStage,
    pub percent: f32,
    pub message: String,
}

/// Import stage for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStage {
    Parsing,
    FetchingElevation,
    GeneratingTerrain,
    Saving,
}

/// Route preview without full import
#[derive(Debug, Clone)]
pub struct RoutePreview {
    pub name: String,
    pub point_count: u32,
    pub distance_km: f64,
    pub elevation_gain_m: f64,
    pub has_elevation: bool,
    pub bounds: GeoBounds,
}

/// Geographic bounding box
#[derive(Debug, Clone, Default)]
pub struct GeoBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

/// Raw GPS point from file parsing
#[derive(Debug, Clone)]
pub struct GpsPoint {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: Option<f32>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Detect file format from extension
pub fn detect_format(path: &Path) -> Result<FileFormat, ImportError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| ImportError::InvalidFormat("No file extension".to_string()))?;

    match ext.as_str() {
        "gpx" => Ok(FileFormat::Gpx),
        "fit" => Ok(FileFormat::Fit),
        "tcx" => Ok(FileFormat::Tcx),
        _ => Err(ImportError::InvalidFormat(format!(
            "Unsupported format: {}",
            ext
        ))),
    }
}

/// Supported file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Gpx,
    Fit,
    Tcx,
}

/// Convert GPS coordinates to local 3D world coordinates using Web Mercator projection
pub fn gps_to_world_coords(lat: f64, lon: f64, origin_lat: f64, origin_lon: f64) -> (f32, f32) {
    // Web Mercator projection
    const EARTH_RADIUS: f64 = 6_378_137.0; // meters

    let x = EARTH_RADIUS * (lon - origin_lon).to_radians();
    let y = EARTH_RADIUS
        * ((lat.to_radians() / 2.0 + std::f64::consts::FRAC_PI_4)
            .tan()
            .ln()
            - (origin_lat.to_radians() / 2.0 + std::f64::consts::FRAC_PI_4)
                .tan()
                .ln());

    (x as f32, y as f32)
}

/// Calculate gradient between two points
pub fn calculate_gradient(elevation_change: f32, horizontal_distance: f64) -> f32 {
    if horizontal_distance < 0.1 {
        return 0.0;
    }
    (elevation_change as f64 / horizontal_distance * 100.0) as f32
}

/// Calculate horizontal distance between two GPS points (Haversine formula)
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6_371_000.0; // meters

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS * c
}

/// Simplify route using Ramer-Douglas-Peucker algorithm
pub fn simplify_route(points: &[GpsPoint], epsilon: f64) -> Vec<GpsPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    // Find the point with the maximum distance from line between first and last
    let mut max_distance = 0.0;
    let mut max_index = 0;

    let start = &points[0];
    let end = &points[points.len() - 1];

    for (i, point) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let distance = perpendicular_distance(point, start, end);
        if distance > max_distance {
            max_distance = distance;
            max_index = i;
        }
    }

    // If max distance is greater than epsilon, recursively simplify
    if max_distance > epsilon {
        let mut left = simplify_route(&points[..=max_index], epsilon);
        let right = simplify_route(&points[max_index..], epsilon);

        // Remove duplicate point at junction
        left.pop();
        left.extend(right);
        left
    } else {
        // Return just the endpoints
        vec![start.clone(), end.clone()]
    }
}

/// Calculate perpendicular distance from point to line
fn perpendicular_distance(point: &GpsPoint, line_start: &GpsPoint, line_end: &GpsPoint) -> f64 {
    let dx = line_end.longitude - line_start.longitude;
    let dy = line_end.latitude - line_start.latitude;

    if dx.abs() < 1e-10 && dy.abs() < 1e-10 {
        return haversine_distance(
            point.latitude,
            point.longitude,
            line_start.latitude,
            line_start.longitude,
        );
    }

    let t = ((point.longitude - line_start.longitude) * dx
        + (point.latitude - line_start.latitude) * dy)
        / (dx * dx + dy * dy);
    let t = t.clamp(0.0, 1.0);

    let nearest_lon = line_start.longitude + t * dx;
    let nearest_lat = line_start.latitude + t * dy;

    haversine_distance(point.latitude, point.longitude, nearest_lat, nearest_lon)
}
