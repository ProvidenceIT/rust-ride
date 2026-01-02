//! Analytics data export and import.
//!
//! Provides JSON and CSV export for analytics data including PDC,
//! training load, CP model, and fitness profile.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::pdc::PdcPoint;
use super::training_load::DailyLoad;

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

/// A single day's training load values for export.
///
/// Includes date, TSS, and all calculated load metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyLoadExport {
    /// The date for this training load entry.
    pub date: NaiveDate,
    /// Training Stress Score for the day.
    pub tss: f32,
    /// Acute Training Load (7-day EWMA).
    pub atl: f32,
    /// Chronic Training Load (42-day EWMA).
    pub ctl: f32,
    /// Training Stress Balance (CTL - ATL).
    pub tsb: f32,
    /// Acute:Chronic Workload Ratio (ATL / CTL), if calculable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acwr: Option<f32>,
}

impl DailyLoadExport {
    /// Create a new daily load export entry.
    pub fn new(date: NaiveDate, tss: f32, atl: f32, ctl: f32, tsb: f32) -> Self {
        Self {
            date,
            tss,
            atl,
            ctl,
            tsb,
            acwr: None,
        }
    }

    /// Create a new daily load export entry with ACWR.
    pub fn with_acwr(date: NaiveDate, tss: f32, atl: f32, ctl: f32, tsb: f32, acwr: f32) -> Self {
        Self {
            date,
            tss,
            atl,
            ctl,
            tsb,
            acwr: Some(acwr),
        }
    }

    /// Create from a DailyLoad with date.
    pub fn from_daily_load(date: NaiveDate, load: DailyLoad) -> Self {
        let acwr = if load.ctl > 0.0 {
            Some(load.atl / load.ctl)
        } else {
            None
        };

        Self {
            date,
            tss: load.tss,
            atl: load.atl,
            ctl: load.ctl,
            tsb: load.tsb,
            acwr,
        }
    }
}

/// Export format for training load history.
///
/// Contains daily training load values sorted chronologically.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainingLoadExport {
    /// All daily load entries, sorted by date ascending.
    pub days: Vec<DailyLoadExport>,
}

impl TrainingLoadExport {
    /// Create a new empty training load export.
    pub fn new() -> Self {
        Self { days: Vec::new() }
    }

    /// Create from a collection of daily load entries.
    pub fn from_days(mut days: Vec<DailyLoadExport>) -> Self {
        days.sort_by_key(|d| d.date);
        Self { days }
    }

    /// Check if the export is empty.
    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    /// Get the number of days.
    pub fn len(&self) -> usize {
        self.days.len()
    }

    /// Get the date range of the export.
    pub fn date_range(&self) -> Option<(NaiveDate, NaiveDate)> {
        if self.days.is_empty() {
            None
        } else {
            Some((self.days[0].date, self.days[self.days.len() - 1].date))
        }
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
    /// Training load history data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_load: Option<TrainingLoadExport>,
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
            training_load: None,
        }
    }

    /// Set the PDC data for export.
    pub fn with_pdc(mut self, pdc: PdcExport) -> Self {
        self.pdc = Some(pdc);
        self
    }

    /// Set the training load data for export.
    pub fn with_training_load(mut self, training_load: TrainingLoadExport) -> Self {
        self.training_load = Some(training_load);
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

    // ============ TrainingLoadExport Tests ============

    #[test]
    fn test_daily_load_export_new() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = DailyLoadExport::new(date, 100.0, 75.0, 80.0, 5.0);

        assert_eq!(entry.date, date);
        assert_eq!(entry.tss, 100.0);
        assert_eq!(entry.atl, 75.0);
        assert_eq!(entry.ctl, 80.0);
        assert_eq!(entry.tsb, 5.0);
        assert!(entry.acwr.is_none());
    }

    #[test]
    fn test_daily_load_export_with_acwr() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = DailyLoadExport::with_acwr(date, 100.0, 75.0, 80.0, 5.0, 0.9375);

        assert_eq!(entry.date, date);
        assert_eq!(entry.tss, 100.0);
        assert_eq!(entry.atl, 75.0);
        assert_eq!(entry.ctl, 80.0);
        assert_eq!(entry.tsb, 5.0);
        assert_eq!(entry.acwr, Some(0.9375));
    }

    #[test]
    fn test_daily_load_export_from_daily_load() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let load = DailyLoad {
            tss: 100.0,
            atl: 75.0,
            ctl: 80.0,
            tsb: 5.0,
        };

        let export = DailyLoadExport::from_daily_load(date, load);

        assert_eq!(export.date, date);
        assert_eq!(export.tss, 100.0);
        assert_eq!(export.atl, 75.0);
        assert_eq!(export.ctl, 80.0);
        assert_eq!(export.tsb, 5.0);
        // ACWR = 75.0 / 80.0 = 0.9375
        assert!((export.acwr.unwrap() - 0.9375).abs() < 0.001);
    }

    #[test]
    fn test_daily_load_export_from_daily_load_zero_ctl() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let load = DailyLoad {
            tss: 100.0,
            atl: 25.0,
            ctl: 0.0,
            tsb: -25.0,
        };

        let export = DailyLoadExport::from_daily_load(date, load);

        // ACWR should be None when CTL is 0 (division by zero)
        assert!(export.acwr.is_none());
    }

    #[test]
    fn test_daily_load_export_serialize_without_acwr() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = DailyLoadExport::new(date, 100.0, 75.0, 80.0, 5.0);
        let json = serde_json::to_string(&entry).expect("should serialize");

        // Should not include acwr when None
        assert!(!json.contains("acwr"));
        assert!(json.contains("date"));
        assert!(json.contains("tss"));
        assert!(json.contains("atl"));
        assert!(json.contains("ctl"));
        assert!(json.contains("tsb"));
    }

    #[test]
    fn test_daily_load_export_serialize_with_acwr() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = DailyLoadExport::with_acwr(date, 100.0, 75.0, 80.0, 5.0, 0.9375);
        let json = serde_json::to_string(&entry).expect("should serialize");

        // Should include acwr when Some
        assert!(json.contains("acwr"));
    }

    #[test]
    fn test_training_load_export_new() {
        let training_load = TrainingLoadExport::new();

        assert!(training_load.is_empty());
        assert_eq!(training_load.len(), 0);
        assert!(training_load.date_range().is_none());
    }

    #[test]
    fn test_training_load_export_from_days() {
        // Create days out of order
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                120.0,
                85.0,
                82.0,
                -3.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                80.0,
                78.0,
                81.0,
                3.0,
            ),
        ];

        let training_load = TrainingLoadExport::from_days(days);

        // Should be sorted by date
        assert_eq!(training_load.len(), 3);
        assert!(!training_load.is_empty());
        assert_eq!(
            training_load.days[0].date,
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
        assert_eq!(
            training_load.days[1].date,
            NaiveDate::from_ymd_opt(2024, 6, 16).unwrap()
        );
        assert_eq!(
            training_load.days[2].date,
            NaiveDate::from_ymd_opt(2024, 6, 17).unwrap()
        );
    }

    #[test]
    fn test_training_load_export_date_range() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(),
                120.0,
                85.0,
                82.0,
                -3.0,
            ),
        ];

        let training_load = TrainingLoadExport::from_days(days);
        let range = training_load.date_range().unwrap();

        assert_eq!(range.0, NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
        assert_eq!(range.1, NaiveDate::from_ymd_opt(2024, 6, 20).unwrap());
    }

    #[test]
    fn test_training_load_export_serialize_deserialize() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                80.0,
                78.0,
                81.0,
                3.0,
                0.963,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&training_load).expect("should serialize");

        // Deserialize back
        let deserialized: TrainingLoadExport =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.len(), 2);
        assert_eq!(
            deserialized.days[0].date,
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
        assert_eq!(deserialized.days[0].tss, 100.0);
        assert!(deserialized.days[0].acwr.is_none());
        assert_eq!(
            deserialized.days[1].date,
            NaiveDate::from_ymd_opt(2024, 6, 16).unwrap()
        );
        assert!(deserialized.days[1].acwr.is_some());
    }

    #[test]
    fn test_analytics_export_with_training_load() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        let export = AnalyticsExport::new("test-user").with_training_load(training_load);

        assert!(export.training_load.is_some());
        let tl_export = export.training_load.as_ref().unwrap();
        assert_eq!(tl_export.len(), 1);
    }

    #[test]
    fn test_analytics_export_without_training_load_skips_field() {
        let export = AnalyticsExport::new("test-user");
        let json = serde_json::to_string(&export).expect("should serialize");

        // training_load should not be in JSON when None
        assert!(!json.contains("\"training_load\""));
    }

    #[test]
    fn test_analytics_export_with_training_load_includes_field() {
        let days = vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.0,
            75.0,
            80.0,
            5.0,
        )];
        let training_load = TrainingLoadExport::from_days(days);
        let export = AnalyticsExport::new("test-user").with_training_load(training_load);
        let json = serde_json::to_string(&export).expect("should serialize");

        // training_load should be in JSON when Some
        assert!(json.contains("\"training_load\""));
        assert!(json.contains("\"days\""));
    }

    #[test]
    fn test_analytics_export_with_training_load_roundtrip() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                80.0,
                78.0,
                81.0,
                3.0,
                0.963,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);
        let export = AnalyticsExport::new("test-user").with_training_load(training_load);

        // Serialize and deserialize
        let json = serde_json::to_string_pretty(&export).expect("should serialize");
        let deserialized: AnalyticsExport =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.user_id, export.user_id);
        assert!(deserialized.training_load.is_some());

        let tl = deserialized.training_load.unwrap();
        assert_eq!(tl.len(), 2);
        assert!(tl.days[0].acwr.is_none());
        assert!(tl.days[1].acwr.is_some());
    }

    #[test]
    fn test_analytics_export_with_pdc_and_training_load() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let training_load = TrainingLoadExport::from_days(vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.0,
            75.0,
            80.0,
            5.0,
        )]);

        let export = AnalyticsExport::new("test-user")
            .with_pdc(pdc)
            .with_training_load(training_load);

        assert!(export.pdc.is_some());
        assert!(export.training_load.is_some());

        let json = serde_json::to_string(&export).expect("should serialize");
        assert!(json.contains("\"pdc\""));
        assert!(json.contains("\"training_load\""));
    }
}
