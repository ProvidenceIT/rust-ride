//! Analytics data export and import.
//!
//! Provides JSON and CSV export for analytics data including PDC,
//! training load, CP model, and fitness profile.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::pdc::PdcPoint;

/// A single point on the power duration curve for export.
///
/// Includes the optional timestamp when this best power was achieved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PdcPointExport {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Maximum average power at this duration (watts).
    pub power_watts: u16,
    /// When this best power was achieved (if tracked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub achieved_at: Option<DateTime<Utc>>,
}

impl PdcPointExport {
    /// Create a new PDC point export without timestamp.
    pub fn new(duration_secs: u32, power_watts: u16) -> Self {
        Self {
            duration_secs,
            power_watts,
            achieved_at: None,
        }
    }

    /// Create a new PDC point export with timestamp.
    pub fn with_timestamp(duration_secs: u32, power_watts: u16, achieved_at: DateTime<Utc>) -> Self {
        Self {
            duration_secs,
            power_watts,
            achieved_at: Some(achieved_at),
        }
    }
}

impl From<PdcPoint> for PdcPointExport {
    fn from(point: PdcPoint) -> Self {
        Self {
            duration_secs: point.duration_secs,
            power_watts: point.power_watts,
            achieved_at: None,
        }
    }
}

/// Export format for Power Duration Curve data.
///
/// Contains all PDC points sorted by duration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdcExport {
    /// All PDC points, sorted by duration ascending.
    pub points: Vec<PdcPointExport>,
}

impl PdcExport {
    /// Create a new empty PDC export.
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Create from a collection of points.
    pub fn from_points(mut points: Vec<PdcPointExport>) -> Self {
        points.sort_by_key(|p| p.duration_secs);
        Self { points }
    }

    /// Check if the export is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get the number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }
}

/// Export format for analytics data.
///
/// Contains all analytics data for a user with metadata for portability
/// and version compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsExport {
    /// Timestamp when the export was created.
    pub exported_at: DateTime<Utc>,
    /// Version of the export format for compatibility.
    pub export_version: String,
    /// User ID for portability across systems.
    pub user_id: String,
    /// Power Duration Curve data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdc: Option<PdcExport>,
}

impl AnalyticsExport {
    /// Current export format version.
    pub const CURRENT_VERSION: &'static str = "1.0";

    /// Create a new analytics export with metadata.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            exported_at: Utc::now(),
            export_version: Self::CURRENT_VERSION.to_string(),
            user_id: user_id.into(),
            pdc: None,
        }
    }

    /// Set the PDC data for export.
    pub fn with_pdc(mut self, pdc: PdcExport) -> Self {
        self.pdc = Some(pdc);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_export_new() {
        let user_id = "test-user-123";
        let export = AnalyticsExport::new(user_id);

        assert_eq!(export.user_id, user_id);
        assert_eq!(export.export_version, AnalyticsExport::CURRENT_VERSION);
        // exported_at should be recent (within the last second)
        let now = Utc::now();
        let diff = now - export.exported_at;
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_analytics_export_serialize_deserialize() {
        let export = AnalyticsExport::new("user-456");

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&export).expect("should serialize");

        // Deserialize back
        let deserialized: AnalyticsExport =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.user_id, export.user_id);
        assert_eq!(deserialized.export_version, export.export_version);
        assert_eq!(deserialized.exported_at, export.exported_at);
    }

    #[test]
    fn test_analytics_export_contains_required_fields() {
        let export = AnalyticsExport::new("test-user");
        let json = serde_json::to_string(&export).expect("should serialize");

        // Verify JSON contains required fields
        assert!(json.contains("exported_at"));
        assert!(json.contains("export_version"));
        assert!(json.contains("user_id"));
    }

    #[test]
    fn test_pdc_point_export_new() {
        let point = PdcPointExport::new(60, 350);

        assert_eq!(point.duration_secs, 60);
        assert_eq!(point.power_watts, 350);
        assert!(point.achieved_at.is_none());
    }

    #[test]
    fn test_pdc_point_export_with_timestamp() {
        let timestamp = Utc::now();
        let point = PdcPointExport::with_timestamp(300, 280, timestamp);

        assert_eq!(point.duration_secs, 300);
        assert_eq!(point.power_watts, 280);
        assert_eq!(point.achieved_at, Some(timestamp));
    }

    #[test]
    fn test_pdc_point_export_from_pdc_point() {
        let pdc_point = PdcPoint {
            duration_secs: 180,
            power_watts: 320,
        };
        let export_point: PdcPointExport = pdc_point.into();

        assert_eq!(export_point.duration_secs, 180);
        assert_eq!(export_point.power_watts, 320);
        assert!(export_point.achieved_at.is_none());
    }

    #[test]
    fn test_pdc_point_export_serialize_without_timestamp() {
        let point = PdcPointExport::new(60, 350);
        let json = serde_json::to_string(&point).expect("should serialize");

        // Should not include achieved_at when None
        assert!(!json.contains("achieved_at"));
        assert!(json.contains("duration_secs"));
        assert!(json.contains("power_watts"));
    }

    #[test]
    fn test_pdc_point_export_serialize_with_timestamp() {
        let timestamp = Utc::now();
        let point = PdcPointExport::with_timestamp(60, 350, timestamp);
        let json = serde_json::to_string(&point).expect("should serialize");

        // Should include achieved_at when Some
        assert!(json.contains("achieved_at"));
        assert!(json.contains("duration_secs"));
        assert!(json.contains("power_watts"));
    }

    #[test]
    fn test_pdc_export_new() {
        let pdc = PdcExport::new();

        assert!(pdc.is_empty());
        assert_eq!(pdc.len(), 0);
    }

    #[test]
    fn test_pdc_export_from_points() {
        // Create points out of order
        let points = vec![
            PdcPointExport::new(300, 280),
            PdcPointExport::new(60, 350),
            PdcPointExport::new(180, 310),
        ];

        let pdc = PdcExport::from_points(points);

        // Should be sorted by duration
        assert_eq!(pdc.len(), 3);
        assert!(!pdc.is_empty());
        assert_eq!(pdc.points[0].duration_secs, 60);
        assert_eq!(pdc.points[1].duration_secs, 180);
        assert_eq!(pdc.points[2].duration_secs, 300);
    }

    #[test]
    fn test_pdc_export_serialize_deserialize() {
        let points = vec![
            PdcPointExport::new(60, 350),
            PdcPointExport::new(300, 280),
        ];
        let pdc = PdcExport::from_points(points);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&pdc).expect("should serialize");

        // Deserialize back
        let deserialized: PdcExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized.points[0].duration_secs, 60);
        assert_eq!(deserialized.points[0].power_watts, 350);
        assert_eq!(deserialized.points[1].duration_secs, 300);
        assert_eq!(deserialized.points[1].power_watts, 280);
    }

    #[test]
    fn test_analytics_export_with_pdc() {
        let points = vec![
            PdcPointExport::new(60, 350),
            PdcPointExport::new(300, 280),
        ];
        let pdc = PdcExport::from_points(points);

        let export = AnalyticsExport::new("test-user").with_pdc(pdc);

        assert!(export.pdc.is_some());
        let pdc_export = export.pdc.as_ref().unwrap();
        assert_eq!(pdc_export.len(), 2);
    }

    #[test]
    fn test_analytics_export_without_pdc_skips_field() {
        let export = AnalyticsExport::new("test-user");
        let json = serde_json::to_string(&export).expect("should serialize");

        // pdc should not be in JSON when None
        assert!(!json.contains("\"pdc\""));
    }

    #[test]
    fn test_analytics_export_with_pdc_includes_field() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let export = AnalyticsExport::new("test-user").with_pdc(pdc);
        let json = serde_json::to_string(&export).expect("should serialize");

        // pdc should be in JSON when Some
        assert!(json.contains("\"pdc\""));
        assert!(json.contains("\"points\""));
    }

    #[test]
    fn test_analytics_export_with_pdc_roundtrip() {
        let timestamp = Utc::now();
        let points = vec![
            PdcPointExport::new(60, 350),
            PdcPointExport::with_timestamp(300, 280, timestamp),
        ];
        let pdc = PdcExport::from_points(points);
        let export = AnalyticsExport::new("test-user").with_pdc(pdc);

        // Serialize and deserialize
        let json = serde_json::to_string_pretty(&export).expect("should serialize");
        let deserialized: AnalyticsExport =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.user_id, export.user_id);
        assert!(deserialized.pdc.is_some());

        let pdc = deserialized.pdc.unwrap();
        assert_eq!(pdc.len(), 2);
        assert!(pdc.points[0].achieved_at.is_none());
        assert!(pdc.points[1].achieved_at.is_some());
    }
}
