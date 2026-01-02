//! Analytics data export and import.
//!
//! Provides JSON and CSV export for analytics data including PDC,
//! training load, CP model, and fitness profile.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::critical_power::CpModel;
use super::pdc::PdcPoint;
use super::rider_type::{PowerProfile, RiderType};
use super::training_load::DailyLoad;
use super::vo2max::{FitnessLevel, Vo2maxMethod, Vo2maxResult};
use crate::storage::analytics_store::AnalyticsStore;
use crate::storage::Database;

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

/// Export format for Critical Power model data.
///
/// Contains the CP model parameters (CP, W') and fit quality metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpModelExport {
    /// Critical Power in watts.
    pub cp_watts: u16,
    /// W' (anaerobic work capacity) in joules.
    pub w_prime_joules: u32,
    /// Model fit quality (R² value, 0.0-1.0).
    pub r_squared: f32,
    /// When this model was calculated (if tracked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculated_at: Option<DateTime<Utc>>,
}

impl CpModelExport {
    /// Create a new CP model export.
    pub fn new(cp_watts: u16, w_prime_joules: u32, r_squared: f32) -> Self {
        Self {
            cp_watts,
            w_prime_joules,
            r_squared,
            calculated_at: None,
        }
    }

    /// Create a new CP model export with timestamp.
    pub fn with_timestamp(
        cp_watts: u16,
        w_prime_joules: u32,
        r_squared: f32,
        calculated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            cp_watts,
            w_prime_joules,
            r_squared,
            calculated_at: Some(calculated_at),
        }
    }

    /// Check if the model fit is considered good (R² >= 0.9).
    pub fn is_good_fit(&self) -> bool {
        self.r_squared >= 0.9
    }
}

impl From<CpModel> for CpModelExport {
    fn from(model: CpModel) -> Self {
        Self {
            cp_watts: model.cp,
            w_prime_joules: model.w_prime,
            r_squared: model.r_squared,
            calculated_at: None,
        }
    }
}

/// Export format for VO2max estimation data.
///
/// Contains the estimated VO2max value, fitness classification, and estimation method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vo2maxExport {
    /// Estimated VO2max in ml/kg/min.
    pub vo2max: f32,
    /// Fitness classification (human-readable string).
    pub classification: String,
    /// Method used for estimation (human-readable string).
    pub method: String,
    /// When this estimate was calculated (if tracked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculated_at: Option<DateTime<Utc>>,
}

impl Vo2maxExport {
    /// Create a new VO2max export.
    pub fn new(vo2max: f32, classification: &str, method: &str) -> Self {
        Self {
            vo2max,
            classification: classification.to_string(),
            method: method.to_string(),
            calculated_at: None,
        }
    }

    /// Create a new VO2max export with timestamp.
    pub fn with_timestamp(
        vo2max: f32,
        classification: &str,
        method: &str,
        calculated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            vo2max,
            classification: classification.to_string(),
            method: method.to_string(),
            calculated_at: Some(calculated_at),
        }
    }

    /// Convert FitnessLevel to human-readable string.
    pub fn fitness_level_to_string(level: FitnessLevel) -> &'static str {
        match level {
            FitnessLevel::Untrained => "Untrained",
            FitnessLevel::Recreational => "Recreational",
            FitnessLevel::Trained => "Trained",
            FitnessLevel::WellTrained => "Well-Trained",
            FitnessLevel::Elite => "Elite",
            FitnessLevel::WorldClass => "World-Class",
        }
    }

    /// Convert Vo2maxMethod to human-readable string.
    pub fn method_to_string(method: Vo2maxMethod) -> &'static str {
        match method {
            Vo2maxMethod::FiveMinutePower => "5-minute power (Hawley-Noakes)",
            Vo2maxMethod::FtpBased => "FTP-based estimation",
            Vo2maxMethod::CriticalPowerBased => "Critical Power-based",
        }
    }
}

impl From<Vo2maxResult> for Vo2maxExport {
    fn from(result: Vo2maxResult) -> Self {
        Self {
            vo2max: result.vo2max,
            classification: Self::fitness_level_to_string(result.classification).to_string(),
            method: Self::method_to_string(result.method).to_string(),
            calculated_at: None,
        }
    }
}

/// Export format for power profile percentages.
///
/// Contains power values at key durations as percentages of FTP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerProfileExport {
    /// 5-second power as % of FTP (neuromuscular capacity).
    pub neuromuscular_pct: f32,
    /// 1-minute power as % of FTP (anaerobic capacity).
    pub anaerobic_pct: f32,
    /// 5-minute power as % of FTP (VO2max capacity).
    pub vo2max_pct: f32,
    /// FTP reference (always 100%).
    pub threshold_pct: f32,
}

impl PowerProfileExport {
    /// Create a new power profile export.
    pub fn new(neuromuscular_pct: f32, anaerobic_pct: f32, vo2max_pct: f32) -> Self {
        Self {
            neuromuscular_pct,
            anaerobic_pct,
            vo2max_pct,
            threshold_pct: 100.0,
        }
    }
}

impl From<PowerProfile> for PowerProfileExport {
    fn from(profile: PowerProfile) -> Self {
        Self {
            neuromuscular_pct: profile.neuromuscular,
            anaerobic_pct: profile.anaerobic,
            vo2max_pct: profile.vo2max,
            threshold_pct: profile.threshold,
        }
    }
}

/// Export format for fitness profile data.
///
/// Contains VO2max, FTP, rider type classification, and power profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitnessProfileExport {
    /// Estimated VO2max data (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vo2max: Option<Vo2maxExport>,
    /// Functional Threshold Power in watts (if known).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ftp_watts: Option<u16>,
    /// Rider type classification (human-readable string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rider_type: Option<String>,
    /// Power profile percentages at key durations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_profile: Option<PowerProfileExport>,
    /// When this profile was last updated (if tracked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl FitnessProfileExport {
    /// Create a new empty fitness profile export.
    pub fn new() -> Self {
        Self {
            vo2max: None,
            ftp_watts: None,
            rider_type: None,
            power_profile: None,
            updated_at: None,
        }
    }

    /// Set the VO2max data.
    pub fn with_vo2max(mut self, vo2max: Vo2maxExport) -> Self {
        self.vo2max = Some(vo2max);
        self
    }

    /// Set the FTP value.
    pub fn with_ftp(mut self, ftp_watts: u16) -> Self {
        self.ftp_watts = Some(ftp_watts);
        self
    }

    /// Set the rider type.
    pub fn with_rider_type(mut self, rider_type: RiderType) -> Self {
        self.rider_type = Some(Self::rider_type_to_string(rider_type).to_string());
        self
    }

    /// Set the rider type from a string.
    pub fn with_rider_type_string(mut self, rider_type: impl Into<String>) -> Self {
        self.rider_type = Some(rider_type.into());
        self
    }

    /// Set the power profile.
    pub fn with_power_profile(mut self, power_profile: PowerProfileExport) -> Self {
        self.power_profile = Some(power_profile);
        self
    }

    /// Set the update timestamp.
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Convert RiderType enum to human-readable string.
    pub fn rider_type_to_string(rider_type: RiderType) -> &'static str {
        match rider_type {
            RiderType::Sprinter => "Sprinter",
            RiderType::Pursuiter => "Pursuiter",
            RiderType::TimeTrialist => "Time Trialist",
            RiderType::AllRounder => "All-Rounder",
            RiderType::Unknown => "Unknown",
        }
    }

    /// Check if the profile has any data.
    pub fn has_data(&self) -> bool {
        self.vo2max.is_some()
            || self.ftp_watts.is_some()
            || self.rider_type.is_some()
            || self.power_profile.is_some()
    }
}

impl Default for FitnessProfileExport {
    fn default() -> Self {
        Self::new()
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
    /// Critical Power model data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cp_model: Option<CpModelExport>,
    /// Fitness profile data (VO2max, FTP, rider type, power profile).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fitness_profile: Option<FitnessProfileExport>,
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
            cp_model: None,
            fitness_profile: None,
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

    /// Set the CP model data for export.
    pub fn with_cp_model(mut self, cp_model: CpModelExport) -> Self {
        self.cp_model = Some(cp_model);
        self
    }

    /// Set the fitness profile data for export.
    pub fn with_fitness_profile(mut self, fitness_profile: FitnessProfileExport) -> Self {
        self.fitness_profile = Some(fitness_profile);
        self
    }

    /// Serialize the analytics export to pretty-printed JSON.
    ///
    /// Returns the full analytics export as a formatted JSON string with
    /// indentation for human readability.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the pretty-printed JSON string on success,
    /// or an [`ExportError::SerializationFailed`] if serialization fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let export = AnalyticsExport::new("user-123")
    ///     .with_pdc(pdc_data)
    ///     .with_training_load(load_data);
    ///
    /// let json = export.export_json()?;
    /// std::fs::write("analytics.json", json)?;
    /// ```
    pub fn export_json(&self) -> Result<String, ExportError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| ExportError::SerializationFailed(e.to_string()))
    }
}

/// Analytics data exporter.
///
/// Provides methods for exporting analytics data to JSON and CSV formats.
/// Follows the same pattern as [`crate::leaderboards::export::LeaderboardExporter`].
pub struct AnalyticsExporter {
    db: Arc<Database>,
}

impl AnalyticsExporter {
    /// Create a new analytics exporter.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Build a full analytics export for a user.
    ///
    /// Retrieves all available analytics data for the user including:
    /// - Power Duration Curve (PDC)
    /// - Training load history (last 365 days)
    /// - Current CP model
    /// - Fitness profile (VO2max, FTP, rider type, power profile)
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID to export analytics for
    ///
    /// # Returns
    ///
    /// Returns an [`AnalyticsExport`] containing all available analytics data,
    /// or an error if the export fails.
    pub fn build_export(&self, user_id: Uuid) -> Result<AnalyticsExport, ExportError> {
        let conn = self.db.connection();
        let store = AnalyticsStore::new(conn);

        let mut export = AnalyticsExport::new(user_id.to_string());

        // Load PDC data
        let pdc = store
            .load_pdc(&user_id)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        if !pdc.is_empty() {
            let pdc_points: Vec<PdcPointExport> = pdc
                .points()
                .map(|p| PdcPointExport::from(p.clone()))
                .collect();
            export = export.with_pdc(PdcExport::from_points(pdc_points));
        }

        // Load training load history (last 365 days)
        let end_date = Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(365);

        let load_history = store
            .load_training_load_history(&user_id, start_date, end_date)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        if !load_history.is_empty() {
            let daily_loads: Vec<DailyLoadExport> = load_history
                .into_iter()
                .map(|(date, load)| DailyLoadExport::from_daily_load(date, load))
                .collect();
            export = export.with_training_load(TrainingLoadExport::from_days(daily_loads));
        }

        // Load current CP model
        if let Some(cp_model) = store
            .load_current_cp_model(&user_id)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?
        {
            export = export.with_cp_model(CpModelExport::from(cp_model));
        }

        // Build fitness profile from available data
        let mut fitness_profile = FitnessProfileExport::new();

        // Load FTP
        if let Some(ftp) = store
            .load_accepted_ftp(&user_id)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?
        {
            fitness_profile = fitness_profile.with_ftp(ftp);
        }

        // Load VO2max
        if let Some(vo2max) = store
            .load_current_vo2max(&user_id)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?
        {
            fitness_profile = fitness_profile.with_vo2max(Vo2maxExport::from(vo2max));
        }

        // Load rider profile (type and power profile)
        if let Some((rider_type, power_profile)) = store
            .load_rider_profile(&user_id)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?
        {
            fitness_profile = fitness_profile
                .with_rider_type(rider_type)
                .with_power_profile(PowerProfileExport::from(power_profile));
        }

        // Only add fitness profile if it has any data
        if fitness_profile.has_data() {
            export = export.with_fitness_profile(fitness_profile);
        }

        Ok(export)
    }

    /// Export all analytics data for a user to pretty-printed JSON.
    ///
    /// This is a convenience method that builds the export and serializes it
    /// to JSON in a single call. Equivalent to:
    /// ```ignore
    /// exporter.build_export(user_id)?.export_json()
    /// ```
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID to export analytics for
    ///
    /// # Returns
    ///
    /// Returns a pretty-printed JSON string containing all available analytics
    /// data for the user, or an error if building or serializing fails.
    pub fn export_json(&self, user_id: Uuid) -> Result<String, ExportError> {
        let export = self.build_export(user_id)?;
        export.export_json()
    }
}

/// Errors that can occur during analytics export operations.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// User was not found in the database.
    #[error("User not found: {0}")]
    UserNotFound(Uuid),

    /// Serialization of export data failed.
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    /// A database error occurred during export.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Not enough data available to perform the export.
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
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

    // ============ CpModelExport Tests ============

    #[test]
    fn test_cp_model_export_new() {
        let cp_export = CpModelExport::new(250, 20000, 0.98);

        assert_eq!(cp_export.cp_watts, 250);
        assert_eq!(cp_export.w_prime_joules, 20000);
        assert_eq!(cp_export.r_squared, 0.98);
        assert!(cp_export.calculated_at.is_none());
    }

    #[test]
    fn test_cp_model_export_with_timestamp() {
        let timestamp = Utc::now();
        let cp_export = CpModelExport::with_timestamp(280, 18000, 0.95, timestamp);

        assert_eq!(cp_export.cp_watts, 280);
        assert_eq!(cp_export.w_prime_joules, 18000);
        assert_eq!(cp_export.r_squared, 0.95);
        assert_eq!(cp_export.calculated_at, Some(timestamp));
    }

    #[test]
    fn test_cp_model_export_from_cp_model() {
        let cp_model = CpModel {
            cp: 260,
            w_prime: 22000,
            r_squared: 0.97,
        };
        let export: CpModelExport = cp_model.into();

        assert_eq!(export.cp_watts, 260);
        assert_eq!(export.w_prime_joules, 22000);
        assert_eq!(export.r_squared, 0.97);
        assert!(export.calculated_at.is_none());
    }

    #[test]
    fn test_cp_model_export_is_good_fit() {
        let good_fit = CpModelExport::new(250, 20000, 0.95);
        let exactly_good = CpModelExport::new(250, 20000, 0.9);
        let poor_fit = CpModelExport::new(250, 20000, 0.85);

        assert!(good_fit.is_good_fit());
        assert!(exactly_good.is_good_fit());
        assert!(!poor_fit.is_good_fit());
    }

    #[test]
    fn test_cp_model_export_serialize_without_timestamp() {
        let cp_export = CpModelExport::new(250, 20000, 0.98);
        let json = serde_json::to_string(&cp_export).expect("should serialize");

        // Should not include calculated_at when None
        assert!(!json.contains("calculated_at"));
        assert!(json.contains("cp_watts"));
        assert!(json.contains("w_prime_joules"));
        assert!(json.contains("r_squared"));
    }

    #[test]
    fn test_cp_model_export_serialize_with_timestamp() {
        let timestamp = Utc::now();
        let cp_export = CpModelExport::with_timestamp(250, 20000, 0.98, timestamp);
        let json = serde_json::to_string(&cp_export).expect("should serialize");

        // Should include calculated_at when Some
        assert!(json.contains("calculated_at"));
        assert!(json.contains("cp_watts"));
        assert!(json.contains("w_prime_joules"));
        assert!(json.contains("r_squared"));
    }

    #[test]
    fn test_cp_model_export_serialize_deserialize() {
        let cp_export = CpModelExport::new(250, 20000, 0.98);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&cp_export).expect("should serialize");

        // Deserialize back
        let deserialized: CpModelExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.cp_watts, 250);
        assert_eq!(deserialized.w_prime_joules, 20000);
        assert!((deserialized.r_squared - 0.98).abs() < 0.001);
        assert!(deserialized.calculated_at.is_none());
    }

    #[test]
    fn test_cp_model_export_roundtrip_with_timestamp() {
        let timestamp = Utc::now();
        let cp_export = CpModelExport::with_timestamp(275, 19500, 0.96, timestamp);

        // Serialize and deserialize
        let json = serde_json::to_string_pretty(&cp_export).expect("should serialize");
        let deserialized: CpModelExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.cp_watts, cp_export.cp_watts);
        assert_eq!(deserialized.w_prime_joules, cp_export.w_prime_joules);
        assert_eq!(deserialized.r_squared, cp_export.r_squared);
        assert_eq!(deserialized.calculated_at, Some(timestamp));
    }

    #[test]
    fn test_analytics_export_with_cp_model() {
        let cp_model = CpModelExport::new(250, 20000, 0.98);

        let export = AnalyticsExport::new("test-user").with_cp_model(cp_model);

        assert!(export.cp_model.is_some());
        let cp_export = export.cp_model.as_ref().unwrap();
        assert_eq!(cp_export.cp_watts, 250);
        assert_eq!(cp_export.w_prime_joules, 20000);
    }

    #[test]
    fn test_analytics_export_without_cp_model_skips_field() {
        let export = AnalyticsExport::new("test-user");
        let json = serde_json::to_string(&export).expect("should serialize");

        // cp_model should not be in JSON when None
        assert!(!json.contains("\"cp_model\""));
    }

    #[test]
    fn test_analytics_export_with_cp_model_includes_field() {
        let cp_model = CpModelExport::new(250, 20000, 0.98);
        let export = AnalyticsExport::new("test-user").with_cp_model(cp_model);
        let json = serde_json::to_string(&export).expect("should serialize");

        // cp_model should be in JSON when Some
        assert!(json.contains("\"cp_model\""));
        assert!(json.contains("\"cp_watts\""));
        assert!(json.contains("\"w_prime_joules\""));
        assert!(json.contains("\"r_squared\""));
    }

    #[test]
    fn test_analytics_export_with_cp_model_roundtrip() {
        let timestamp = Utc::now();
        let cp_model = CpModelExport::with_timestamp(260, 21000, 0.97, timestamp);
        let export = AnalyticsExport::new("test-user").with_cp_model(cp_model);

        // Serialize and deserialize
        let json = serde_json::to_string_pretty(&export).expect("should serialize");
        let deserialized: AnalyticsExport =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.user_id, export.user_id);
        assert!(deserialized.cp_model.is_some());

        let cp = deserialized.cp_model.unwrap();
        assert_eq!(cp.cp_watts, 260);
        assert_eq!(cp.w_prime_joules, 21000);
        assert!(cp.calculated_at.is_some());
    }

    #[test]
    fn test_analytics_export_with_all_data_types() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let training_load = TrainingLoadExport::from_days(vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.0,
            75.0,
            80.0,
            5.0,
        )]);
        let cp_model = CpModelExport::new(250, 20000, 0.98);

        let export = AnalyticsExport::new("test-user")
            .with_pdc(pdc)
            .with_training_load(training_load)
            .with_cp_model(cp_model);

        assert!(export.pdc.is_some());
        assert!(export.training_load.is_some());
        assert!(export.cp_model.is_some());

        let json = serde_json::to_string(&export).expect("should serialize");
        assert!(json.contains("\"pdc\""));
        assert!(json.contains("\"training_load\""));
        assert!(json.contains("\"cp_model\""));
    }

    #[test]
    fn test_cp_model_export_equality() {
        let model1 = CpModelExport::new(250, 20000, 0.98);
        let model2 = CpModelExport::new(250, 20000, 0.98);
        let model3 = CpModelExport::new(260, 20000, 0.98);

        assert_eq!(model1, model2);
        assert_ne!(model1, model3);
    }

    // ============ Vo2maxExport Tests ============

    #[test]
    fn test_vo2max_export_new() {
        let vo2max = Vo2maxExport::new(55.0, "Well-Trained", "FTP-based estimation");

        assert_eq!(vo2max.vo2max, 55.0);
        assert_eq!(vo2max.classification, "Well-Trained");
        assert_eq!(vo2max.method, "FTP-based estimation");
        assert!(vo2max.calculated_at.is_none());
    }

    #[test]
    fn test_vo2max_export_with_timestamp() {
        let timestamp = Utc::now();
        let vo2max = Vo2maxExport::with_timestamp(
            60.0,
            "Elite",
            "5-minute power (Hawley-Noakes)",
            timestamp,
        );

        assert_eq!(vo2max.vo2max, 60.0);
        assert_eq!(vo2max.classification, "Elite");
        assert_eq!(vo2max.calculated_at, Some(timestamp));
    }

    #[test]
    fn test_vo2max_export_from_result() {
        use super::super::vo2max::{FitnessLevel, Vo2maxMethod, Vo2maxResult};

        let result = Vo2maxResult {
            vo2max: 52.0,
            classification: FitnessLevel::Trained,
            method: Vo2maxMethod::FtpBased,
        };
        let export: Vo2maxExport = result.into();

        assert_eq!(export.vo2max, 52.0);
        assert_eq!(export.classification, "Trained");
        assert_eq!(export.method, "FTP-based estimation");
    }

    #[test]
    fn test_vo2max_export_serialize_without_timestamp() {
        let vo2max = Vo2maxExport::new(55.0, "Well-Trained", "FTP-based estimation");
        let json = serde_json::to_string(&vo2max).expect("should serialize");

        assert!(!json.contains("calculated_at"));
        assert!(json.contains("vo2max"));
        assert!(json.contains("classification"));
        assert!(json.contains("method"));
    }

    #[test]
    fn test_vo2max_export_serialize_deserialize() {
        let vo2max = Vo2maxExport::new(55.0, "Well-Trained", "FTP-based estimation");

        let json = serde_json::to_string_pretty(&vo2max).expect("should serialize");
        let deserialized: Vo2maxExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.vo2max, 55.0);
        assert_eq!(deserialized.classification, "Well-Trained");
        assert_eq!(deserialized.method, "FTP-based estimation");
    }

    #[test]
    fn test_fitness_level_to_string() {
        use super::super::vo2max::FitnessLevel;

        assert_eq!(Vo2maxExport::fitness_level_to_string(FitnessLevel::Untrained), "Untrained");
        assert_eq!(Vo2maxExport::fitness_level_to_string(FitnessLevel::Recreational), "Recreational");
        assert_eq!(Vo2maxExport::fitness_level_to_string(FitnessLevel::Trained), "Trained");
        assert_eq!(Vo2maxExport::fitness_level_to_string(FitnessLevel::WellTrained), "Well-Trained");
        assert_eq!(Vo2maxExport::fitness_level_to_string(FitnessLevel::Elite), "Elite");
        assert_eq!(Vo2maxExport::fitness_level_to_string(FitnessLevel::WorldClass), "World-Class");
    }

    #[test]
    fn test_vo2max_method_to_string() {
        use super::super::vo2max::Vo2maxMethod;

        assert_eq!(Vo2maxExport::method_to_string(Vo2maxMethod::FiveMinutePower), "5-minute power (Hawley-Noakes)");
        assert_eq!(Vo2maxExport::method_to_string(Vo2maxMethod::FtpBased), "FTP-based estimation");
        assert_eq!(Vo2maxExport::method_to_string(Vo2maxMethod::CriticalPowerBased), "Critical Power-based");
    }

    // ============ PowerProfileExport Tests ============

    #[test]
    fn test_power_profile_export_new() {
        let profile = PowerProfileExport::new(185.0, 130.0, 95.0);

        assert_eq!(profile.neuromuscular_pct, 185.0);
        assert_eq!(profile.anaerobic_pct, 130.0);
        assert_eq!(profile.vo2max_pct, 95.0);
        assert_eq!(profile.threshold_pct, 100.0);
    }

    #[test]
    fn test_power_profile_export_from_power_profile() {
        let profile = PowerProfile {
            neuromuscular: 175.0,
            anaerobic: 125.0,
            vo2max: 90.0,
            threshold: 100.0,
        };
        let export: PowerProfileExport = profile.into();

        assert_eq!(export.neuromuscular_pct, 175.0);
        assert_eq!(export.anaerobic_pct, 125.0);
        assert_eq!(export.vo2max_pct, 90.0);
        assert_eq!(export.threshold_pct, 100.0);
    }

    #[test]
    fn test_power_profile_export_serialize_deserialize() {
        let profile = PowerProfileExport::new(185.0, 130.0, 95.0);

        let json = serde_json::to_string_pretty(&profile).expect("should serialize");
        let deserialized: PowerProfileExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.neuromuscular_pct, 185.0);
        assert_eq!(deserialized.anaerobic_pct, 130.0);
        assert_eq!(deserialized.vo2max_pct, 95.0);
        assert_eq!(deserialized.threshold_pct, 100.0);
    }

    // ============ FitnessProfileExport Tests ============

    #[test]
    fn test_fitness_profile_export_new() {
        let profile = FitnessProfileExport::new();

        assert!(profile.vo2max.is_none());
        assert!(profile.ftp_watts.is_none());
        assert!(profile.rider_type.is_none());
        assert!(profile.power_profile.is_none());
        assert!(profile.updated_at.is_none());
        assert!(!profile.has_data());
    }

    #[test]
    fn test_fitness_profile_export_default() {
        let profile = FitnessProfileExport::default();
        assert!(!profile.has_data());
    }

    #[test]
    fn test_fitness_profile_export_with_ftp() {
        let profile = FitnessProfileExport::new().with_ftp(280);

        assert_eq!(profile.ftp_watts, Some(280));
        assert!(profile.has_data());
    }

    #[test]
    fn test_fitness_profile_export_with_rider_type() {
        let profile = FitnessProfileExport::new().with_rider_type(RiderType::Sprinter);

        assert_eq!(profile.rider_type, Some("Sprinter".to_string()));
        assert!(profile.has_data());
    }

    #[test]
    fn test_fitness_profile_export_with_rider_type_string() {
        let profile = FitnessProfileExport::new().with_rider_type_string("Custom Type");

        assert_eq!(profile.rider_type, Some("Custom Type".to_string()));
    }

    #[test]
    fn test_fitness_profile_export_with_vo2max() {
        let vo2max = Vo2maxExport::new(55.0, "Well-Trained", "FTP-based");
        let profile = FitnessProfileExport::new().with_vo2max(vo2max);

        assert!(profile.vo2max.is_some());
        assert_eq!(profile.vo2max.as_ref().unwrap().vo2max, 55.0);
        assert!(profile.has_data());
    }

    #[test]
    fn test_fitness_profile_export_with_power_profile() {
        let power_profile = PowerProfileExport::new(180.0, 125.0, 92.0);
        let profile = FitnessProfileExport::new().with_power_profile(power_profile);

        assert!(profile.power_profile.is_some());
        assert_eq!(profile.power_profile.as_ref().unwrap().neuromuscular_pct, 180.0);
        assert!(profile.has_data());
    }

    #[test]
    fn test_fitness_profile_export_with_updated_at() {
        let timestamp = Utc::now();
        let profile = FitnessProfileExport::new().with_updated_at(timestamp);

        assert_eq!(profile.updated_at, Some(timestamp));
    }

    #[test]
    fn test_fitness_profile_export_full_builder() {
        let timestamp = Utc::now();
        let vo2max = Vo2maxExport::new(58.0, "Well-Trained", "FTP-based");
        let power_profile = PowerProfileExport::new(175.0, 128.0, 94.0);

        let profile = FitnessProfileExport::new()
            .with_ftp(275)
            .with_rider_type(RiderType::TimeTrialist)
            .with_vo2max(vo2max)
            .with_power_profile(power_profile)
            .with_updated_at(timestamp);

        assert_eq!(profile.ftp_watts, Some(275));
        assert_eq!(profile.rider_type, Some("Time Trialist".to_string()));
        assert!(profile.vo2max.is_some());
        assert!(profile.power_profile.is_some());
        assert_eq!(profile.updated_at, Some(timestamp));
        assert!(profile.has_data());
    }

    #[test]
    fn test_rider_type_to_string() {
        assert_eq!(FitnessProfileExport::rider_type_to_string(RiderType::Sprinter), "Sprinter");
        assert_eq!(FitnessProfileExport::rider_type_to_string(RiderType::Pursuiter), "Pursuiter");
        assert_eq!(FitnessProfileExport::rider_type_to_string(RiderType::TimeTrialist), "Time Trialist");
        assert_eq!(FitnessProfileExport::rider_type_to_string(RiderType::AllRounder), "All-Rounder");
        assert_eq!(FitnessProfileExport::rider_type_to_string(RiderType::Unknown), "Unknown");
    }

    #[test]
    fn test_fitness_profile_export_serialize_empty() {
        let profile = FitnessProfileExport::new();
        let json = serde_json::to_string(&profile).expect("should serialize");

        // Empty profile should have minimal JSON (all fields skipped)
        assert!(!json.contains("vo2max"));
        assert!(!json.contains("ftp_watts"));
        assert!(!json.contains("rider_type"));
        assert!(!json.contains("power_profile"));
        assert!(!json.contains("updated_at"));
    }

    #[test]
    fn test_fitness_profile_export_serialize_with_data() {
        let profile = FitnessProfileExport::new()
            .with_ftp(280)
            .with_rider_type(RiderType::AllRounder);

        let json = serde_json::to_string(&profile).expect("should serialize");

        assert!(json.contains("ftp_watts"));
        assert!(json.contains("rider_type"));
        assert!(json.contains("280"));
        assert!(json.contains("All-Rounder"));
    }

    #[test]
    fn test_fitness_profile_export_serialize_deserialize_roundtrip() {
        let vo2max = Vo2maxExport::new(55.0, "Well-Trained", "FTP-based");
        let power_profile = PowerProfileExport::new(175.0, 128.0, 94.0);
        let profile = FitnessProfileExport::new()
            .with_ftp(270)
            .with_rider_type(RiderType::Pursuiter)
            .with_vo2max(vo2max)
            .with_power_profile(power_profile);

        let json = serde_json::to_string_pretty(&profile).expect("should serialize");
        let deserialized: FitnessProfileExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.ftp_watts, Some(270));
        assert_eq!(deserialized.rider_type, Some("Pursuiter".to_string()));
        assert!(deserialized.vo2max.is_some());
        assert_eq!(deserialized.vo2max.as_ref().unwrap().vo2max, 55.0);
        assert!(deserialized.power_profile.is_some());
        assert_eq!(deserialized.power_profile.as_ref().unwrap().neuromuscular_pct, 175.0);
    }

    // ============ FitnessProfileExport Integration with AnalyticsExport Tests ============

    #[test]
    fn test_analytics_export_with_fitness_profile() {
        let fitness_profile = FitnessProfileExport::new()
            .with_ftp(265)
            .with_rider_type(RiderType::Sprinter);

        let export = AnalyticsExport::new("test-user").with_fitness_profile(fitness_profile);

        assert!(export.fitness_profile.is_some());
        let fp = export.fitness_profile.as_ref().unwrap();
        assert_eq!(fp.ftp_watts, Some(265));
        assert_eq!(fp.rider_type, Some("Sprinter".to_string()));
    }

    #[test]
    fn test_analytics_export_without_fitness_profile_skips_field() {
        let export = AnalyticsExport::new("test-user");
        let json = serde_json::to_string(&export).expect("should serialize");

        assert!(!json.contains("\"fitness_profile\""));
    }

    #[test]
    fn test_analytics_export_with_fitness_profile_includes_field() {
        let fitness_profile = FitnessProfileExport::new().with_ftp(280);
        let export = AnalyticsExport::new("test-user").with_fitness_profile(fitness_profile);
        let json = serde_json::to_string(&export).expect("should serialize");

        assert!(json.contains("\"fitness_profile\""));
        assert!(json.contains("\"ftp_watts\""));
        assert!(json.contains("280"));
    }

    #[test]
    fn test_analytics_export_with_fitness_profile_roundtrip() {
        let timestamp = Utc::now();
        let vo2max = Vo2maxExport::with_timestamp(60.0, "Elite", "5-minute power", timestamp);
        let power_profile = PowerProfileExport::new(190.0, 135.0, 98.0);
        let fitness_profile = FitnessProfileExport::new()
            .with_ftp(290)
            .with_rider_type(RiderType::TimeTrialist)
            .with_vo2max(vo2max)
            .with_power_profile(power_profile);

        let export = AnalyticsExport::new("test-user").with_fitness_profile(fitness_profile);

        let json = serde_json::to_string_pretty(&export).expect("should serialize");
        let deserialized: AnalyticsExport = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.user_id, export.user_id);
        assert!(deserialized.fitness_profile.is_some());

        let fp = deserialized.fitness_profile.unwrap();
        assert_eq!(fp.ftp_watts, Some(290));
        assert_eq!(fp.rider_type, Some("Time Trialist".to_string()));
        assert!(fp.vo2max.is_some());
        assert!(fp.power_profile.is_some());
    }

    #[test]
    fn test_analytics_export_with_all_data_types_including_fitness() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let training_load = TrainingLoadExport::from_days(vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.0,
            75.0,
            80.0,
            5.0,
        )]);
        let cp_model = CpModelExport::new(250, 20000, 0.98);
        let fitness_profile = FitnessProfileExport::new()
            .with_ftp(275)
            .with_rider_type(RiderType::AllRounder);

        let export = AnalyticsExport::new("test-user")
            .with_pdc(pdc)
            .with_training_load(training_load)
            .with_cp_model(cp_model)
            .with_fitness_profile(fitness_profile);

        assert!(export.pdc.is_some());
        assert!(export.training_load.is_some());
        assert!(export.cp_model.is_some());
        assert!(export.fitness_profile.is_some());

        let json = serde_json::to_string(&export).expect("should serialize");
        assert!(json.contains("\"pdc\""));
        assert!(json.contains("\"training_load\""));
        assert!(json.contains("\"cp_model\""));
        assert!(json.contains("\"fitness_profile\""));
    }

    #[test]
    fn test_fitness_profile_export_equality() {
        let profile1 = FitnessProfileExport::new().with_ftp(280);
        let profile2 = FitnessProfileExport::new().with_ftp(280);
        let profile3 = FitnessProfileExport::new().with_ftp(290);

        assert_eq!(profile1, profile2);
        assert_ne!(profile1, profile3);
    }

    #[test]
    fn test_power_profile_export_equality() {
        let profile1 = PowerProfileExport::new(175.0, 125.0, 90.0);
        let profile2 = PowerProfileExport::new(175.0, 125.0, 90.0);
        let profile3 = PowerProfileExport::new(180.0, 125.0, 90.0);

        assert_eq!(profile1, profile2);
        assert_ne!(profile1, profile3);
    }

    #[test]
    fn test_vo2max_export_equality() {
        let vo2max1 = Vo2maxExport::new(55.0, "Trained", "FTP-based");
        let vo2max2 = Vo2maxExport::new(55.0, "Trained", "FTP-based");
        let vo2max3 = Vo2maxExport::new(60.0, "Trained", "FTP-based");

        assert_eq!(vo2max1, vo2max2);
        assert_ne!(vo2max1, vo2max3);
    }

    // ============ export_json() Tests ============

    #[test]
    fn test_export_json_empty_export() {
        let export = AnalyticsExport::new("test-user");
        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Should be pretty-printed (contains newlines)
        assert!(json.contains('\n'));

        // Should contain required fields
        assert!(json.contains("\"exported_at\""));
        assert!(json.contains("\"export_version\""));
        assert!(json.contains("\"user_id\""));
        assert!(json.contains("\"test-user\""));
    }

    #[test]
    fn test_export_json_with_pdc() {
        let pdc = PdcExport::from_points(vec![
            PdcPointExport::new(60, 350),
            PdcPointExport::new(300, 280),
        ]);
        let export = AnalyticsExport::new("user-123").with_pdc(pdc);
        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Should contain PDC data
        assert!(json.contains("\"pdc\""));
        assert!(json.contains("\"points\""));
        assert!(json.contains("\"duration_secs\""));
        assert!(json.contains("\"power_watts\""));
    }

    #[test]
    fn test_export_json_with_training_load() {
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
        let export = AnalyticsExport::new("user-456").with_training_load(training_load);
        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Should contain training load data
        assert!(json.contains("\"training_load\""));
        assert!(json.contains("\"days\""));
        assert!(json.contains("\"tss\""));
        assert!(json.contains("\"atl\""));
        assert!(json.contains("\"ctl\""));
        assert!(json.contains("\"tsb\""));
    }

    #[test]
    fn test_export_json_with_cp_model() {
        let cp_model = CpModelExport::new(250, 20000, 0.98);
        let export = AnalyticsExport::new("user-789").with_cp_model(cp_model);
        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Should contain CP model data
        assert!(json.contains("\"cp_model\""));
        assert!(json.contains("\"cp_watts\""));
        assert!(json.contains("\"w_prime_joules\""));
        assert!(json.contains("\"r_squared\""));
    }

    #[test]
    fn test_export_json_with_fitness_profile() {
        let vo2max = Vo2maxExport::new(55.0, "Well-Trained", "FTP-based");
        let power_profile = PowerProfileExport::new(175.0, 128.0, 94.0);
        let fitness_profile = FitnessProfileExport::new()
            .with_ftp(275)
            .with_rider_type(RiderType::TimeTrialist)
            .with_vo2max(vo2max)
            .with_power_profile(power_profile);

        let export = AnalyticsExport::new("user-fitness").with_fitness_profile(fitness_profile);
        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Should contain fitness profile data
        assert!(json.contains("\"fitness_profile\""));
        assert!(json.contains("\"ftp_watts\""));
        assert!(json.contains("\"rider_type\""));
        assert!(json.contains("\"vo2max\""));
        assert!(json.contains("\"power_profile\""));
        assert!(json.contains("Time Trialist"));
    }

    #[test]
    fn test_export_json_with_all_data() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let training_load = TrainingLoadExport::from_days(vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.0,
            75.0,
            80.0,
            5.0,
        )]);
        let cp_model = CpModelExport::new(250, 20000, 0.98);
        let fitness_profile = FitnessProfileExport::new().with_ftp(280);

        let export = AnalyticsExport::new("full-export-user")
            .with_pdc(pdc)
            .with_training_load(training_load)
            .with_cp_model(cp_model)
            .with_fitness_profile(fitness_profile);

        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Should contain all data sections
        assert!(json.contains("\"pdc\""));
        assert!(json.contains("\"training_load\""));
        assert!(json.contains("\"cp_model\""));
        assert!(json.contains("\"fitness_profile\""));
    }

    #[test]
    fn test_export_json_is_pretty_printed() {
        let export = AnalyticsExport::new("test-user");
        let result = export.export_json();

        assert!(result.is_ok());
        let json = result.unwrap();

        // Pretty-printed JSON should have:
        // - Multiple lines
        // - Indentation (spaces at beginning of lines)
        let lines: Vec<&str> = json.lines().collect();
        assert!(lines.len() > 1, "Pretty-printed JSON should have multiple lines");

        // Check that there's indentation (lines starting with spaces)
        let has_indentation = lines.iter().any(|line| line.starts_with("  "));
        assert!(has_indentation, "Pretty-printed JSON should have indentation");
    }

    #[test]
    fn test_export_json_roundtrip() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let cp_model = CpModelExport::new(250, 20000, 0.98);
        let fitness_profile = FitnessProfileExport::new()
            .with_ftp(280)
            .with_rider_type(RiderType::AllRounder);

        let export = AnalyticsExport::new("roundtrip-user")
            .with_pdc(pdc)
            .with_cp_model(cp_model)
            .with_fitness_profile(fitness_profile);

        // Export to JSON
        let json = export.export_json().expect("should export");

        // Parse back
        let deserialized: AnalyticsExport =
            serde_json::from_str(&json).expect("should deserialize");

        // Verify data integrity
        assert_eq!(deserialized.user_id, "roundtrip-user");
        assert!(deserialized.pdc.is_some());
        assert!(deserialized.cp_model.is_some());
        assert!(deserialized.fitness_profile.is_some());

        let pdc = deserialized.pdc.unwrap();
        assert_eq!(pdc.len(), 1);
        assert_eq!(pdc.points[0].duration_secs, 60);
        assert_eq!(pdc.points[0].power_watts, 350);

        let cp = deserialized.cp_model.unwrap();
        assert_eq!(cp.cp_watts, 250);
        assert_eq!(cp.w_prime_joules, 20000);

        let fp = deserialized.fitness_profile.unwrap();
        assert_eq!(fp.ftp_watts, Some(280));
        assert_eq!(fp.rider_type, Some("All-Rounder".to_string()));
    }
}
