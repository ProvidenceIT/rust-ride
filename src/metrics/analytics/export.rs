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

    /// Export the PDC data to CSV format.
    ///
    /// Returns a CSV string with headers: `duration_secs,power_watts,achieved_at`
    ///
    /// The `achieved_at` column contains ISO 8601 timestamps when available,
    /// or is empty when the timestamp is not tracked.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pdc = PdcExport::from_points(vec![
    ///     PdcPointExport::new(60, 350),
    ///     PdcPointExport::new(300, 280),
    /// ]);
    /// let csv = pdc.to_csv();
    /// // csv contains:
    /// // duration_secs,power_watts,achieved_at
    /// // 60,350,
    /// // 300,280,
    /// ```
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("duration_secs,power_watts,achieved_at\n");

        for point in &self.points {
            csv.push_str(&format!(
                "{},{},{}\n",
                point.duration_secs,
                point.power_watts,
                point
                    .achieved_at
                    .map_or(String::new(), |ts| ts.to_rfc3339()),
            ));
        }

        csv
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

    /// Filter training load data to a specific date range.
    ///
    /// Returns a new `TrainingLoadExport` containing only the days within
    /// the specified date range (inclusive on both ends).
    ///
    /// # Arguments
    ///
    /// * `start_date` - The start of the date range (inclusive)
    /// * `end_date` - The end of the date range (inclusive)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let training_load = TrainingLoadExport::from_days(vec![
    ///     DailyLoadExport::new(NaiveDate::from_ymd_opt(2024, 6, 13).unwrap(), 90.0, 70.0, 75.0, 5.0),
    ///     DailyLoadExport::new(NaiveDate::from_ymd_opt(2024, 6, 14).unwrap(), 100.0, 75.0, 78.0, 3.0),
    ///     DailyLoadExport::new(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(), 80.0, 78.0, 79.0, 1.0),
    ///     DailyLoadExport::new(NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(), 110.0, 85.0, 80.0, -5.0),
    /// ]);
    /// let start = NaiveDate::from_ymd_opt(2024, 6, 14).unwrap();
    /// let end = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    /// let filtered = training_load.filter_by_date_range(start, end);
    /// assert_eq!(filtered.len(), 2); // Only June 14 and 15
    /// ```
    pub fn filter_by_date_range(&self, start_date: NaiveDate, end_date: NaiveDate) -> Self {
        let filtered_days: Vec<DailyLoadExport> = self
            .days
            .iter()
            .filter(|d| d.date >= start_date && d.date <= end_date)
            .cloned()
            .collect();
        Self { days: filtered_days }
    }

    /// Export the training load data to CSV format.
    ///
    /// Returns a CSV string with headers: `date,tss,atl,ctl,tsb,acwr`
    ///
    /// The `acwr` column contains the Acute:Chronic Workload Ratio when available,
    /// or is empty when CTL is zero (division by zero).
    ///
    /// Dates are formatted as ISO 8601 (YYYY-MM-DD).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let training_load = TrainingLoadExport::from_days(vec![
    ///     DailyLoadExport::new(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(), 100.0, 75.0, 80.0, 5.0),
    ///     DailyLoadExport::with_acwr(NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(), 80.0, 78.0, 81.0, 3.0, 0.96),
    /// ]);
    /// let csv = training_load.to_csv();
    /// // csv contains:
    /// // date,tss,atl,ctl,tsb,acwr
    /// // 2024-06-15,100.00,75.00,80.00,5.00,
    /// // 2024-06-16,80.00,78.00,81.00,3.00,0.96
    /// ```
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("date,tss,atl,ctl,tsb,acwr\n");

        for day in &self.days {
            csv.push_str(&format!(
                "{},{:.2},{:.2},{:.2},{:.2},{}\n",
                day.date,
                day.tss,
                day.atl,
                day.ctl,
                day.tsb,
                day.acwr.map_or(String::new(), |v| format!("{:.2}", v)),
            ));
        }

        csv
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

    /// Export Power Duration Curve data for a user to CSV format.
    ///
    /// Returns a CSV string with headers: `duration_secs,power_watts,achieved_at`
    ///
    /// The CSV format is suitable for import into spreadsheet applications
    /// or analysis tools. Timestamps are in ISO 8601 format (RFC 3339).
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID to export PDC data for
    ///
    /// # Returns
    ///
    /// Returns a CSV string containing the PDC data, or an error if:
    /// - A database error occurs during data retrieval
    /// - No PDC data is available for the user
    ///
    /// # Example
    ///
    /// ```ignore
    /// let exporter = AnalyticsExporter::new(db);
    /// let csv = exporter.export_pdc_csv(user_id)?;
    /// std::fs::write("pdc_export.csv", csv)?;
    /// ```
    pub fn export_pdc_csv(&self, user_id: Uuid) -> Result<String, ExportError> {
        let conn = self.db.connection();
        let store = AnalyticsStore::new(conn);

        // Load PDC data
        let pdc = store
            .load_pdc(&user_id)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        if pdc.is_empty() {
            return Err(ExportError::InsufficientData(
                "No PDC data available for user".to_string(),
            ));
        }

        // Convert to export format
        let pdc_points: Vec<PdcPointExport> = pdc
            .points()
            .map(|p| PdcPointExport::from(p.clone()))
            .collect();
        let pdc_export = PdcExport::from_points(pdc_points);

        Ok(pdc_export.to_csv())
    }

    /// Export training load history for a user to CSV format.
    ///
    /// Returns a CSV string with headers: `date,tss,atl,ctl,tsb,acwr`
    ///
    /// The CSV format is suitable for import into spreadsheet applications
    /// or analysis tools. Dates are in ISO 8601 format (YYYY-MM-DD).
    /// Floating point values are formatted with 2 decimal places.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID to export training load data for
    /// * `start_date` - The start date of the export range (inclusive)
    /// * `end_date` - The end date of the export range (inclusive)
    ///
    /// # Returns
    ///
    /// Returns a CSV string containing the training load data ordered
    /// chronologically, or an error if:
    /// - A database error occurs during data retrieval
    /// - No training load data is available for the user in the date range
    ///
    /// # Example
    ///
    /// ```ignore
    /// let exporter = AnalyticsExporter::new(db);
    /// let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    /// let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    /// let csv = exporter.export_training_load_csv(user_id, start, end)?;
    /// std::fs::write("training_load_export.csv", csv)?;
    /// ```
    pub fn export_training_load_csv(
        &self,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<String, ExportError> {
        let conn = self.db.connection();
        let store = AnalyticsStore::new(conn);

        // Load training load history for the specified date range
        let load_history = store
            .load_training_load_history(&user_id, start_date, end_date)
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        if load_history.is_empty() {
            return Err(ExportError::InsufficientData(
                "No training load data available for user in the specified date range".to_string(),
            ));
        }

        // Convert to export format
        let daily_loads: Vec<DailyLoadExport> = load_history
            .into_iter()
            .map(|(date, load)| DailyLoadExport::from_daily_load(date, load))
            .collect();
        let training_load_export = TrainingLoadExport::from_days(daily_loads);

        Ok(training_load_export.to_csv())
    }

    /// Build an analytics export with configurable options.
    ///
    /// Similar to [`build_export`], but allows filtering which data types
    /// to include and specifying a date range for training load data.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID to export analytics for
    /// * `options` - Export options controlling which data to include and date filtering
    ///
    /// # Returns
    ///
    /// Returns an [`AnalyticsExport`] containing the requested analytics data,
    /// or an error if the export fails.
    pub fn build_export_with_options(
        &self,
        user_id: Uuid,
        options: &ExportOptions,
    ) -> Result<AnalyticsExport, ExportError> {
        let conn = self.db.connection();
        let store = AnalyticsStore::new(conn);

        let mut export = AnalyticsExport::new(user_id.to_string());

        // Load PDC data if requested
        if options.include_pdc {
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
        }

        // Load training load history if requested
        if options.include_training_load {
            let end_date = options
                .end_date
                .unwrap_or_else(|| Utc::now().date_naive());
            let start_date = options
                .start_date
                .unwrap_or_else(|| end_date - chrono::Duration::days(365));

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
        }

        // Load current CP model if requested
        if options.include_cp_model {
            if let Some(cp_model) = store
                .load_current_cp_model(&user_id)
                .map_err(|e| ExportError::DatabaseError(e.to_string()))?
            {
                export = export.with_cp_model(CpModelExport::from(cp_model));
            }
        }

        // Build fitness profile if requested
        if options.include_fitness_profile {
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
        }

        Ok(export)
    }

    /// Export analytics data with configurable options to pretty-printed JSON.
    ///
    /// This method allows selective export of analytics data, controlling
    /// which data types to include and specifying a date range for filtering.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID to export analytics for
    /// * `options` - Export options controlling which data to include and date filtering
    ///
    /// # Returns
    ///
    /// Returns a pretty-printed JSON string containing the requested analytics
    /// data for the user, or an error if building or serializing fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let exporter = AnalyticsExporter::new(db);
    ///
    /// // Export only PDC and training load for a specific date range
    /// let options = ExportOptions::new()
    ///     .with_pdc(true)
    ///     .with_training_load(true)
    ///     .with_cp_model(false)
    ///     .with_fitness_profile(false)
    ///     .with_date_range(
    ///         NaiveDate::from_ymd_opt(2024, 1, 1),
    ///         NaiveDate::from_ymd_opt(2024, 6, 30),
    ///     );
    ///
    /// let json = exporter.export_json_with_options(user_id, &options)?;
    /// ```
    pub fn export_json_with_options(
        &self,
        user_id: Uuid,
        options: &ExportOptions,
    ) -> Result<String, ExportError> {
        let export = self.build_export_with_options(user_id, options)?;
        export.export_json()
    }
}

/// Options for configuring analytics data export.
///
/// Controls which data types to include in the export and allows
/// filtering by date range. Use the builder pattern to configure options.
///
/// # Example
///
/// ```ignore
/// let options = ExportOptions::new()
///     .with_pdc(true)
///     .with_training_load(true)
///     .with_cp_model(false)
///     .with_fitness_profile(false)
///     .with_date_range(
///         NaiveDate::from_ymd_opt(2024, 1, 1),
///         NaiveDate::from_ymd_opt(2024, 6, 30),
///     );
/// ```
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Include Power Duration Curve data in export.
    pub include_pdc: bool,
    /// Include training load history in export.
    pub include_training_load: bool,
    /// Include Critical Power model data in export.
    pub include_cp_model: bool,
    /// Include fitness profile data (VO2max, FTP, rider type) in export.
    pub include_fitness_profile: bool,
    /// Start date for filtering training load history (inclusive).
    pub start_date: Option<NaiveDate>,
    /// End date for filtering training load history (inclusive).
    pub end_date: Option<NaiveDate>,
}

impl ExportOptions {
    /// Create new export options with all data types included.
    ///
    /// By default, all data types are included and no date filtering is applied.
    pub fn new() -> Self {
        Self {
            include_pdc: true,
            include_training_load: true,
            include_cp_model: true,
            include_fitness_profile: true,
            start_date: None,
            end_date: None,
        }
    }

    /// Set whether to include PDC data.
    pub fn with_pdc(mut self, include: bool) -> Self {
        self.include_pdc = include;
        self
    }

    /// Set whether to include training load data.
    pub fn with_training_load(mut self, include: bool) -> Self {
        self.include_training_load = include;
        self
    }

    /// Set whether to include CP model data.
    pub fn with_cp_model(mut self, include: bool) -> Self {
        self.include_cp_model = include;
        self
    }

    /// Set whether to include fitness profile data.
    pub fn with_fitness_profile(mut self, include: bool) -> Self {
        self.include_fitness_profile = include;
        self
    }

    /// Set the date range for filtering training load data.
    ///
    /// Both dates are inclusive. If either is None, the corresponding
    /// bound is not applied.
    pub fn with_date_range(
        mut self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Self {
        self.start_date = start_date;
        self.end_date = end_date;
        self
    }

    /// Set only the start date for filtering.
    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.start_date = Some(start_date);
        self
    }

    /// Set only the end date for filtering.
    pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Create options that include only PDC data.
    pub fn pdc_only() -> Self {
        Self::new()
            .with_pdc(true)
            .with_training_load(false)
            .with_cp_model(false)
            .with_fitness_profile(false)
    }

    /// Create options that include only training load data.
    pub fn training_load_only() -> Self {
        Self::new()
            .with_pdc(false)
            .with_training_load(true)
            .with_cp_model(false)
            .with_fitness_profile(false)
    }

    /// Create options that include only CP model data.
    pub fn cp_model_only() -> Self {
        Self::new()
            .with_pdc(false)
            .with_training_load(false)
            .with_cp_model(true)
            .with_fitness_profile(false)
    }

    /// Create options that include only fitness profile data.
    pub fn fitness_profile_only() -> Self {
        Self::new()
            .with_pdc(false)
            .with_training_load(false)
            .with_cp_model(false)
            .with_fitness_profile(true)
    }
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self::new()
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

    // ============ PdcExport CSV Tests ============

    #[test]
    fn test_pdc_export_to_csv_empty() {
        let pdc = PdcExport::new();
        let csv = pdc.to_csv();

        // Should only contain the header
        assert_eq!(csv, "duration_secs,power_watts,achieved_at\n");
    }

    #[test]
    fn test_pdc_export_to_csv_single_point() {
        let pdc = PdcExport::from_points(vec![PdcPointExport::new(60, 350)]);
        let csv = pdc.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "duration_secs,power_watts,achieved_at");
        assert_eq!(lines[1], "60,350,");
    }

    #[test]
    fn test_pdc_export_to_csv_multiple_points() {
        let points = vec![
            PdcPointExport::new(60, 350),
            PdcPointExport::new(180, 310),
            PdcPointExport::new(300, 280),
        ];
        let pdc = PdcExport::from_points(points);
        let csv = pdc.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "duration_secs,power_watts,achieved_at");
        assert_eq!(lines[1], "60,350,");
        assert_eq!(lines[2], "180,310,");
        assert_eq!(lines[3], "300,280,");
    }

    #[test]
    fn test_pdc_export_to_csv_with_timestamps() {
        let timestamp1 = DateTime::parse_from_rfc3339("2024-06-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let timestamp2 = DateTime::parse_from_rfc3339("2024-06-16T14:45:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let points = vec![
            PdcPointExport::with_timestamp(60, 350, timestamp1),
            PdcPointExport::new(180, 310), // No timestamp
            PdcPointExport::with_timestamp(300, 280, timestamp2),
        ];
        let pdc = PdcExport::from_points(points);
        let csv = pdc.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "duration_secs,power_watts,achieved_at");
        // First point has timestamp
        assert!(lines[1].starts_with("60,350,2024-06-15T10:30:00"));
        // Second point has no timestamp
        assert_eq!(lines[2], "180,310,");
        // Third point has timestamp
        assert!(lines[3].starts_with("300,280,2024-06-16T14:45:00"));
    }

    #[test]
    fn test_pdc_export_to_csv_header_format() {
        let pdc = PdcExport::new();
        let csv = pdc.to_csv();

        // Verify header is exactly as expected
        assert!(csv.starts_with("duration_secs,power_watts,achieved_at\n"));
    }

    #[test]
    fn test_pdc_export_to_csv_sorted_by_duration() {
        // Points added out of order
        let points = vec![
            PdcPointExport::new(300, 280),
            PdcPointExport::new(60, 350),
            PdcPointExport::new(180, 310),
        ];
        let pdc = PdcExport::from_points(points);
        let csv = pdc.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        // Points should be sorted by duration in the CSV
        assert!(lines[1].starts_with("60,"));
        assert!(lines[2].starts_with("180,"));
        assert!(lines[3].starts_with("300,"));
    }

    #[test]
    fn test_pdc_export_to_csv_timestamp_rfc3339_format() {
        let timestamp = DateTime::parse_from_rfc3339("2024-06-15T10:30:00+00:00")
            .unwrap()
            .with_timezone(&Utc);

        let points = vec![PdcPointExport::with_timestamp(60, 350, timestamp)];
        let pdc = PdcExport::from_points(points);
        let csv = pdc.to_csv();

        // Verify the timestamp is in RFC 3339 format
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[1].contains("2024-06-15T10:30:00"));
        // Should end with Z for UTC
        assert!(lines[1].ends_with("+00:00") || lines[1].ends_with("Z"));
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

    // ============ TrainingLoadExport CSV Tests ============

    #[test]
    fn test_training_load_export_to_csv_empty() {
        let training_load = TrainingLoadExport::new();
        let csv = training_load.to_csv();

        // Should only contain the header
        assert_eq!(csv, "date,tss,atl,ctl,tsb,acwr\n");
    }

    #[test]
    fn test_training_load_export_to_csv_single_day() {
        let days = vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.0,
            75.0,
            80.0,
            5.0,
        )];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "date,tss,atl,ctl,tsb,acwr");
        assert_eq!(lines[1], "2024-06-15,100.00,75.00,80.00,5.00,");
    }

    #[test]
    fn test_training_load_export_to_csv_multiple_days() {
        let days = vec![
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
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                120.0,
                85.0,
                82.0,
                -3.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "date,tss,atl,ctl,tsb,acwr");
        assert_eq!(lines[1], "2024-06-15,100.00,75.00,80.00,5.00,");
        assert_eq!(lines[2], "2024-06-16,80.00,78.00,81.00,3.00,");
        assert_eq!(lines[3], "2024-06-17,120.00,85.00,82.00,-3.00,");
    }

    #[test]
    fn test_training_load_export_to_csv_with_acwr() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ), // No ACWR (will be None)
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                80.0,
                78.0,
                81.0,
                3.0,
                0.96,
            ), // With ACWR
            DailyLoadExport::from_daily_load(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                DailyLoad {
                    tss: 100.0,
                    atl: 90.0,
                    ctl: 85.0,
                    tsb: -5.0,
                },
            ), // ACWR calculated: 90/85 = 1.058...
        ];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "date,tss,atl,ctl,tsb,acwr");
        // First day has no ACWR
        assert_eq!(lines[1], "2024-06-15,100.00,75.00,80.00,5.00,");
        // Second day has explicit ACWR
        assert_eq!(lines[2], "2024-06-16,80.00,78.00,81.00,3.00,0.96");
        // Third day has calculated ACWR
        assert!(lines[3].starts_with("2024-06-17,100.00,90.00,85.00,-5.00,1.0"));
    }

    #[test]
    fn test_training_load_export_to_csv_header_format() {
        let training_load = TrainingLoadExport::new();
        let csv = training_load.to_csv();

        // Verify header is exactly as expected
        assert!(csv.starts_with("date,tss,atl,ctl,tsb,acwr\n"));
    }

    #[test]
    fn test_training_load_export_to_csv_sorted_chronologically() {
        // Days added out of order
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
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        // Days should be sorted chronologically in the CSV
        assert!(lines[1].starts_with("2024-06-15,"));
        assert!(lines[2].starts_with("2024-06-16,"));
        assert!(lines[3].starts_with("2024-06-17,"));
    }

    #[test]
    fn test_training_load_export_to_csv_decimal_precision() {
        let days = vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            100.123456,  // Should be rounded to 2 decimal places
            75.5,        // Should show as 75.50
            80.999,      // Should be rounded to 81.00
            5.0,         // Should show as 5.00
        )];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        // Values should have exactly 2 decimal places
        assert_eq!(lines[1], "2024-06-15,100.12,75.50,81.00,5.00,");
    }

    #[test]
    fn test_training_load_export_to_csv_negative_tsb() {
        let days = vec![DailyLoadExport::new(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            150.0,
            110.0,
            85.0,
            -25.0,  // Negative TSB indicates fatigue
        )];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        // Negative TSB should be formatted correctly
        assert_eq!(lines[1], "2024-06-15,150.00,110.00,85.00,-25.00,");
    }

    #[test]
    fn test_training_load_export_to_csv_iso8601_date_format() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(),  // Single digit day
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(),  // December
                100.0,
                75.0,
                80.0,
                5.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        // Dates should be in ISO 8601 format (YYYY-MM-DD) with zero-padding
        assert!(lines[1].starts_with("2024-01-05,"));
        assert!(lines[2].starts_with("2024-12-25,"));
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

    // ============ ExportOptions Tests ============

    #[test]
    fn test_export_options_new_defaults() {
        let options = ExportOptions::new();

        assert!(options.include_pdc);
        assert!(options.include_training_load);
        assert!(options.include_cp_model);
        assert!(options.include_fitness_profile);
        assert!(options.start_date.is_none());
        assert!(options.end_date.is_none());
    }

    #[test]
    fn test_export_options_default_trait() {
        let options = ExportOptions::default();

        assert!(options.include_pdc);
        assert!(options.include_training_load);
        assert!(options.include_cp_model);
        assert!(options.include_fitness_profile);
    }

    #[test]
    fn test_export_options_with_pdc() {
        let options = ExportOptions::new().with_pdc(false);
        assert!(!options.include_pdc);
        assert!(options.include_training_load);

        let options = ExportOptions::new().with_pdc(true);
        assert!(options.include_pdc);
    }

    #[test]
    fn test_export_options_with_training_load() {
        let options = ExportOptions::new().with_training_load(false);
        assert!(!options.include_training_load);
        assert!(options.include_pdc);

        let options = ExportOptions::new().with_training_load(true);
        assert!(options.include_training_load);
    }

    #[test]
    fn test_export_options_with_cp_model() {
        let options = ExportOptions::new().with_cp_model(false);
        assert!(!options.include_cp_model);
        assert!(options.include_pdc);

        let options = ExportOptions::new().with_cp_model(true);
        assert!(options.include_cp_model);
    }

    #[test]
    fn test_export_options_with_fitness_profile() {
        let options = ExportOptions::new().with_fitness_profile(false);
        assert!(!options.include_fitness_profile);
        assert!(options.include_pdc);

        let options = ExportOptions::new().with_fitness_profile(true);
        assert!(options.include_fitness_profile);
    }

    #[test]
    fn test_export_options_with_date_range() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

        let options = ExportOptions::new().with_date_range(Some(start), Some(end));

        assert_eq!(options.start_date, Some(start));
        assert_eq!(options.end_date, Some(end));
    }

    #[test]
    fn test_export_options_with_date_range_partial() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let options = ExportOptions::new().with_date_range(Some(start), None);
        assert_eq!(options.start_date, Some(start));
        assert!(options.end_date.is_none());

        let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();
        let options = ExportOptions::new().with_date_range(None, Some(end));
        assert!(options.start_date.is_none());
        assert_eq!(options.end_date, Some(end));
    }

    #[test]
    fn test_export_options_with_start_date() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let options = ExportOptions::new().with_start_date(start);

        assert_eq!(options.start_date, Some(start));
        assert!(options.end_date.is_none());
    }

    #[test]
    fn test_export_options_with_end_date() {
        let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();
        let options = ExportOptions::new().with_end_date(end);

        assert!(options.start_date.is_none());
        assert_eq!(options.end_date, Some(end));
    }

    #[test]
    fn test_export_options_pdc_only() {
        let options = ExportOptions::pdc_only();

        assert!(options.include_pdc);
        assert!(!options.include_training_load);
        assert!(!options.include_cp_model);
        assert!(!options.include_fitness_profile);
    }

    #[test]
    fn test_export_options_training_load_only() {
        let options = ExportOptions::training_load_only();

        assert!(!options.include_pdc);
        assert!(options.include_training_load);
        assert!(!options.include_cp_model);
        assert!(!options.include_fitness_profile);
    }

    #[test]
    fn test_export_options_cp_model_only() {
        let options = ExportOptions::cp_model_only();

        assert!(!options.include_pdc);
        assert!(!options.include_training_load);
        assert!(options.include_cp_model);
        assert!(!options.include_fitness_profile);
    }

    #[test]
    fn test_export_options_fitness_profile_only() {
        let options = ExportOptions::fitness_profile_only();

        assert!(!options.include_pdc);
        assert!(!options.include_training_load);
        assert!(!options.include_cp_model);
        assert!(options.include_fitness_profile);
    }

    #[test]
    fn test_export_options_chained_builders() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

        let options = ExportOptions::new()
            .with_pdc(true)
            .with_training_load(true)
            .with_cp_model(false)
            .with_fitness_profile(false)
            .with_date_range(Some(start), Some(end));

        assert!(options.include_pdc);
        assert!(options.include_training_load);
        assert!(!options.include_cp_model);
        assert!(!options.include_fitness_profile);
        assert_eq!(options.start_date, Some(start));
        assert_eq!(options.end_date, Some(end));
    }

    #[test]
    fn test_export_options_clone() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let options = ExportOptions::new()
            .with_pdc(false)
            .with_start_date(start);

        let cloned = options.clone();

        assert_eq!(cloned.include_pdc, options.include_pdc);
        assert_eq!(cloned.include_training_load, options.include_training_load);
        assert_eq!(cloned.include_cp_model, options.include_cp_model);
        assert_eq!(cloned.include_fitness_profile, options.include_fitness_profile);
        assert_eq!(cloned.start_date, options.start_date);
        assert_eq!(cloned.end_date, options.end_date);
    }

    #[test]
    fn test_export_options_debug() {
        let options = ExportOptions::new();
        let debug_str = format!("{:?}", options);

        assert!(debug_str.contains("ExportOptions"));
        assert!(debug_str.contains("include_pdc"));
        assert!(debug_str.contains("include_training_load"));
    }

    #[test]
    fn test_export_options_training_load_only_with_date_range() {
        let start = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 3, 31).unwrap();

        let options = ExportOptions::training_load_only()
            .with_date_range(Some(start), Some(end));

        assert!(!options.include_pdc);
        assert!(options.include_training_load);
        assert!(!options.include_cp_model);
        assert!(!options.include_fitness_profile);
        assert_eq!(options.start_date, Some(start));
        assert_eq!(options.end_date, Some(end));
    }

    // ============ P3.1: Comprehensive JSON Serialization Tests ============

    #[test]
    fn test_json_export_full_roundtrip_with_all_data_types() {
        // Create a comprehensive export with ALL data types including training load
        let timestamp = Utc::now();

        // PDC with timestamps
        let pdc = PdcExport::from_points(vec![
            PdcPointExport::with_timestamp(5, 1200, timestamp),
            PdcPointExport::new(60, 350),
            PdcPointExport::with_timestamp(300, 280, timestamp),
            PdcPointExport::new(1200, 250),
        ]);

        // Training load with ACWR
        let training_load = TrainingLoadExport::from_days(vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 13).unwrap(),
                0.0,
                50.0,
                60.0,
                10.0,
            ),
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 14).unwrap(),
                120.0,
                65.0,
                62.0,
                -3.0,
                1.05,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                80.0,
                70.0,
                65.0,
                -5.0,
            ),
        ]);

        // CP model with timestamp
        let cp_model = CpModelExport::with_timestamp(250, 20000, 0.98, timestamp);

        // Comprehensive fitness profile
        let vo2max = Vo2maxExport::with_timestamp(55.0, "Well-Trained", "FTP-based", timestamp);
        let power_profile = PowerProfileExport::new(175.0, 128.0, 94.0);
        let fitness_profile = FitnessProfileExport::new()
            .with_ftp(275)
            .with_rider_type(RiderType::TimeTrialist)
            .with_vo2max(vo2max)
            .with_power_profile(power_profile)
            .with_updated_at(timestamp);

        let export = AnalyticsExport::new("comprehensive-test-user")
            .with_pdc(pdc)
            .with_training_load(training_load)
            .with_cp_model(cp_model)
            .with_fitness_profile(fitness_profile);

        // Export to JSON
        let json = export.export_json().expect("should export to JSON");

        // Deserialize back
        let deserialized: AnalyticsExport =
            serde_json::from_str(&json).expect("should deserialize from JSON");

        // Verify metadata
        assert_eq!(deserialized.user_id, "comprehensive-test-user");
        assert_eq!(deserialized.export_version, AnalyticsExport::CURRENT_VERSION);

        // Verify PDC roundtrip
        let pdc = deserialized.pdc.expect("should have PDC data");
        assert_eq!(pdc.len(), 4);
        assert_eq!(pdc.points[0].duration_secs, 5);
        assert_eq!(pdc.points[0].power_watts, 1200);
        assert!(pdc.points[0].achieved_at.is_some());
        assert_eq!(pdc.points[1].duration_secs, 60);
        assert!(pdc.points[1].achieved_at.is_none());

        // Verify training load roundtrip
        let tl = deserialized.training_load.expect("should have training load data");
        assert_eq!(tl.len(), 3);
        assert_eq!(tl.days[0].date, NaiveDate::from_ymd_opt(2024, 6, 13).unwrap());
        assert!(tl.days[0].acwr.is_none());
        assert_eq!(tl.days[1].date, NaiveDate::from_ymd_opt(2024, 6, 14).unwrap());
        assert!((tl.days[1].tss - 120.0).abs() < 0.001);
        assert!(tl.days[1].acwr.is_some());
        assert!((tl.days[1].acwr.unwrap() - 1.05).abs() < 0.001);

        // Verify CP model roundtrip
        let cp = deserialized.cp_model.expect("should have CP model data");
        assert_eq!(cp.cp_watts, 250);
        assert_eq!(cp.w_prime_joules, 20000);
        assert!((cp.r_squared - 0.98).abs() < 0.001);
        assert!(cp.calculated_at.is_some());

        // Verify fitness profile roundtrip
        let fp = deserialized.fitness_profile.expect("should have fitness profile");
        assert_eq!(fp.ftp_watts, Some(275));
        assert_eq!(fp.rider_type, Some("Time Trialist".to_string()));
        assert!(fp.updated_at.is_some());

        let vo2max = fp.vo2max.expect("should have VO2max");
        assert!((vo2max.vo2max - 55.0).abs() < 0.001);
        assert_eq!(vo2max.classification, "Well-Trained");
        assert_eq!(vo2max.method, "FTP-based");
        assert!(vo2max.calculated_at.is_some());

        let pp = fp.power_profile.expect("should have power profile");
        assert!((pp.neuromuscular_pct - 175.0).abs() < 0.001);
        assert!((pp.anaerobic_pct - 128.0).abs() < 0.001);
        assert!((pp.vo2max_pct - 94.0).abs() < 0.001);
        assert!((pp.threshold_pct - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_json_export_validates_export_version() {
        let export = AnalyticsExport::new("version-test-user");
        let json = export.export_json().expect("should export");

        // Parse as raw JSON to verify version format
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        let version = parsed["export_version"]
            .as_str()
            .expect("export_version should be a string");
        assert_eq!(version, "1.0");

        // Verify roundtrip preserves version
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.export_version, "1.0");
    }

    #[test]
    fn test_json_export_validates_timestamp_format() {
        let export = AnalyticsExport::new("timestamp-test-user");
        let json = export.export_json().expect("should export");

        // Parse as raw JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        let exported_at = parsed["exported_at"]
            .as_str()
            .expect("exported_at should be a string");

        // Verify it's a valid ISO 8601 / RFC 3339 timestamp
        let parsed_ts: Result<DateTime<Utc>, _> = exported_at.parse();
        assert!(parsed_ts.is_ok(), "exported_at should be valid RFC3339");

        // Timestamp should be recent (within last minute)
        let ts = parsed_ts.unwrap();
        let diff = Utc::now() - ts;
        assert!(diff.num_seconds() < 60, "timestamp should be recent");
    }

    #[test]
    fn test_json_export_nested_structures_with_all_timestamps() {
        let timestamp = Utc::now();

        // Create export with timestamps on all nested structures
        let pdc = PdcExport::from_points(vec![
            PdcPointExport::with_timestamp(60, 350, timestamp),
        ]);
        let cp_model = CpModelExport::with_timestamp(250, 20000, 0.95, timestamp);
        let vo2max = Vo2maxExport::with_timestamp(52.0, "Trained", "Critical Power-based", timestamp);
        let fitness_profile = FitnessProfileExport::new()
            .with_vo2max(vo2max)
            .with_ftp(280)
            .with_updated_at(timestamp);

        let export = AnalyticsExport::new("nested-timestamps-user")
            .with_pdc(pdc)
            .with_cp_model(cp_model)
            .with_fitness_profile(fitness_profile);

        let json = export.export_json().expect("should export");

        // Parse and verify all timestamp fields are present
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        // Check PDC point timestamp
        assert!(parsed["pdc"]["points"][0]["achieved_at"].is_string());

        // Check CP model timestamp
        assert!(parsed["cp_model"]["calculated_at"].is_string());

        // Check fitness profile timestamps
        assert!(parsed["fitness_profile"]["updated_at"].is_string());
        assert!(parsed["fitness_profile"]["vo2max"]["calculated_at"].is_string());
    }

    #[test]
    fn test_json_export_omits_none_optional_fields() {
        // Create minimal export without optional data
        let export = AnalyticsExport::new("minimal-user");
        let json = export.export_json().expect("should export");

        // Parse as raw JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        // Optional fields should be absent (not null)
        assert!(parsed.get("pdc").is_none());
        assert!(parsed.get("training_load").is_none());
        assert!(parsed.get("cp_model").is_none());
        assert!(parsed.get("fitness_profile").is_none());
    }

    #[test]
    fn test_json_export_pdc_points_sorted_by_duration() {
        // Create PDC with unsorted points
        let pdc = PdcExport::from_points(vec![
            PdcPointExport::new(300, 280),
            PdcPointExport::new(5, 1200),
            PdcPointExport::new(60, 400),
            PdcPointExport::new(1200, 250),
        ]);
        let export = AnalyticsExport::new("pdc-sort-test").with_pdc(pdc);

        let json = export.export_json().expect("should export");
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();

        let pdc = deserialized.pdc.unwrap();
        let durations: Vec<u32> = pdc.points.iter().map(|p| p.duration_secs).collect();

        // Verify sorted ascending
        assert_eq!(durations, vec![5, 60, 300, 1200]);
    }

    #[test]
    fn test_json_export_training_load_sorted_chronologically() {
        // Create training load with unsorted days
        let training_load = TrainingLoadExport::from_days(vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                90.0,
                75.0,
                70.0,
                -5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                65.0,
                60.0,
                -5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                80.0,
                70.0,
                65.0,
                -5.0,
            ),
        ]);
        let export = AnalyticsExport::new("tl-sort-test").with_training_load(training_load);

        let json = export.export_json().expect("should export");
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();

        let tl = deserialized.training_load.unwrap();
        let dates: Vec<NaiveDate> = tl.days.iter().map(|d| d.date).collect();

        // Verify sorted chronologically
        assert_eq!(
            dates,
            vec![
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
            ]
        );
    }

    #[test]
    fn test_json_export_fitness_profile_rider_type_string() {
        // Test that rider type is exported as human-readable string, not enum variant
        let fitness_profile = FitnessProfileExport::new()
            .with_rider_type(RiderType::Pursuiter);

        let export = AnalyticsExport::new("rider-type-test")
            .with_fitness_profile(fitness_profile);

        let json = export.export_json().expect("should export");

        // Should contain human-readable string, not "Pursuiter" raw enum name
        assert!(json.contains("\"Pursuiter\""));

        // Verify roundtrip
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();
        let fp = deserialized.fitness_profile.unwrap();
        assert_eq!(fp.rider_type, Some("Pursuiter".to_string()));
    }

    #[test]
    fn test_json_export_preserves_floating_point_precision() {
        let training_load = TrainingLoadExport::from_days(vec![DailyLoadExport::with_acwr(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            123.456,
            78.901,
            82.345,
            -3.444,
            0.958,
        )]);

        let export = AnalyticsExport::new("precision-test")
            .with_training_load(training_load);

        let json = export.export_json().expect("should export");
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();

        let tl = deserialized.training_load.unwrap();
        let day = &tl.days[0];

        // Verify floating point values are preserved within f32 precision
        assert!((day.tss - 123.456).abs() < 0.001);
        assert!((day.atl - 78.901).abs() < 0.001);
        assert!((day.ctl - 82.345).abs() < 0.001);
        assert!((day.tsb - (-3.444)).abs() < 0.001);
        assert!((day.acwr.unwrap() - 0.958).abs() < 0.001);
    }

    #[test]
    fn test_json_export_handles_negative_values() {
        // Negative TSB is common (fatigue > fitness)
        let training_load = TrainingLoadExport::from_days(vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                150.0,
                95.0,
                70.0,
                -25.0, // Very fatigued
            ),
        ]);

        let export = AnalyticsExport::new("negative-test")
            .with_training_load(training_load);

        let json = export.export_json().expect("should export");
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();

        let tl = deserialized.training_load.unwrap();
        assert!((tl.days[0].tsb - (-25.0)).abs() < 0.001);
    }

    #[test]
    fn test_json_export_large_values() {
        // Test with realistic large values
        let pdc = PdcExport::from_points(vec![
            PdcPointExport::new(1, 2000), // Very high sprint power
            PdcPointExport::new(3600, 200), // 1-hour power
        ]);

        let cp_model = CpModelExport::new(280, 35000, 0.99);

        let export = AnalyticsExport::new("large-values-test")
            .with_pdc(pdc)
            .with_cp_model(cp_model);

        let json = export.export_json().expect("should export");
        let deserialized: AnalyticsExport = serde_json::from_str(&json).unwrap();

        let pdc = deserialized.pdc.unwrap();
        assert_eq!(pdc.points[0].power_watts, 2000);
        assert_eq!(pdc.points[1].duration_secs, 3600);

        let cp = deserialized.cp_model.unwrap();
        assert_eq!(cp.w_prime_joules, 35000);
    }

    // ============ P3.3: Training Load CSV Export with Date Filtering Tests ============

    #[test]
    fn test_training_load_filter_by_date_range_basic() {
        // Create training load data spanning a week
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
                80.0,
                60.0,
                70.0,
                10.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 11).unwrap(),
                90.0,
                65.0,
                72.0,
                7.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 12).unwrap(),
                100.0,
                70.0,
                74.0,
                4.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 13).unwrap(),
                110.0,
                75.0,
                76.0,
                1.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 14).unwrap(),
                120.0,
                80.0,
                78.0,
                -2.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Filter to middle 3 days
        let start = NaiveDate::from_ymd_opt(2024, 6, 11).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 13).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert_eq!(filtered.len(), 3);
        assert_eq!(
            filtered.days[0].date,
            NaiveDate::from_ymd_opt(2024, 6, 11).unwrap()
        );
        assert_eq!(
            filtered.days[1].date,
            NaiveDate::from_ymd_opt(2024, 6, 12).unwrap()
        );
        assert_eq!(
            filtered.days[2].date,
            NaiveDate::from_ymd_opt(2024, 6, 13).unwrap()
        );
    }

    #[test]
    fn test_training_load_filter_by_date_range_inclusive_boundaries() {
        // Verify that start and end dates are both inclusive
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                110.0,
                80.0,
                82.0,
                2.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                90.0,
                78.0,
                83.0,
                5.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Filter to exact range - should include both boundary dates
        let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 17).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered.days[0].date, start);
        assert_eq!(filtered.days[2].date, end);
    }

    #[test]
    fn test_training_load_filter_by_date_range_single_day() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                110.0,
                80.0,
                82.0,
                2.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                90.0,
                78.0,
                83.0,
                5.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Filter to a single day
        let date = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
        let filtered = training_load.filter_by_date_range(date, date);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.days[0].date, date);
        assert!((filtered.days[0].tss - 110.0).abs() < 0.001);
    }

    #[test]
    fn test_training_load_filter_by_date_range_empty_result() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                110.0,
                80.0,
                82.0,
                2.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Filter to range with no data
        let start = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 31).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert!(filtered.is_empty());
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_training_load_filter_by_date_range_before_data() {
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

        // Filter to range before available data
        let start = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 5, 31).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_training_load_filter_by_date_range_after_data() {
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

        // Filter to range after available data
        let start = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 31).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_training_load_filter_preserves_data_values() {
        let days = vec![
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.5,
                75.25,
                80.75,
                5.5,
                0.93,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                110.0,
                80.0,
                82.0,
                2.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let filtered = training_load.filter_by_date_range(date, date);

        assert_eq!(filtered.len(), 1);
        let day = &filtered.days[0];
        assert!((day.tss - 100.5).abs() < 0.001);
        assert!((day.atl - 75.25).abs() < 0.001);
        assert!((day.ctl - 80.75).abs() < 0.001);
        assert!((day.tsb - 5.5).abs() < 0.001);
        assert!(day.acwr.is_some());
        assert!((day.acwr.unwrap() - 0.93).abs() < 0.001);
    }

    #[test]
    fn test_training_load_filter_then_csv_export() {
        // Test the full workflow: filter then export to CSV
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
                80.0,
                60.0,
                70.0,
                10.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 11).unwrap(),
                90.0,
                65.0,
                72.0,
                7.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 12).unwrap(),
                100.0,
                70.0,
                74.0,
                4.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 13).unwrap(),
                110.0,
                75.0,
                76.0,
                1.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Filter to 2 days
        let start = NaiveDate::from_ymd_opt(2024, 6, 11).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 12).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        // Export filtered data to CSV
        let csv = filtered.to_csv();
        let lines: Vec<&str> = csv.lines().collect();

        // Should have header + 2 data rows
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "date,tss,atl,ctl,tsb,acwr");
        assert!(lines[1].starts_with("2024-06-11,"));
        assert!(lines[2].starts_with("2024-06-12,"));
    }

    #[test]
    fn test_training_load_filter_csv_with_all_columns() {
        // Test that CSV export includes all columns after filtering
        let days = vec![
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                150.0,
                95.0,
                85.0,
                -10.0,
                1.12,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let filtered = training_load.filter_by_date_range(date, date);
        let csv = filtered.to_csv();
        let lines: Vec<&str> = csv.lines().collect();

        assert_eq!(lines.len(), 2);
        // Verify all columns present with correct values
        // date,tss,atl,ctl,tsb,acwr
        let data_line = lines[1];
        let parts: Vec<&str> = data_line.split(',').collect();
        assert_eq!(parts.len(), 6);
        assert_eq!(parts[0], "2024-06-15"); // date
        assert_eq!(parts[1], "150.00"); // tss
        assert_eq!(parts[2], "95.00"); // atl
        assert_eq!(parts[3], "85.00"); // ctl
        assert_eq!(parts[4], "-10.00"); // tsb
        assert_eq!(parts[5], "1.12"); // acwr
    }

    #[test]
    fn test_training_load_filter_csv_maintains_chronological_order() {
        // Create data in non-chronological order
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
        // from_days should sort them
        let training_load = TrainingLoadExport::from_days(days);

        let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 17).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        // Verify sorting is maintained after filtering
        assert_eq!(
            filtered.days[0].date,
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
        assert_eq!(
            filtered.days[1].date,
            NaiveDate::from_ymd_opt(2024, 6, 16).unwrap()
        );
        assert_eq!(
            filtered.days[2].date,
            NaiveDate::from_ymd_opt(2024, 6, 17).unwrap()
        );

        // Verify CSV also maintains order
        let csv = filtered.to_csv();
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[1].starts_with("2024-06-15,"));
        assert!(lines[2].starts_with("2024-06-16,"));
        assert!(lines[3].starts_with("2024-06-17,"));
    }

    #[test]
    fn test_training_load_filter_csv_decimal_precision_preserved() {
        // Test that decimal precision is maintained through filter + CSV export
        let days = vec![
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                123.456, // Should round to 123.46
                78.991,  // Should round to 78.99
                82.005,  // Should round to 82.01 (rounding)
                3.009,   // Should round to 3.01
                0.963,   // Should round to 0.96
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let filtered = training_load.filter_by_date_range(date, date);
        let csv = filtered.to_csv();

        let lines: Vec<&str> = csv.lines().collect();
        // Values should have 2 decimal places
        assert_eq!(lines[1], "2024-06-15,123.46,78.99,82.00,3.01,0.96");
    }

    #[test]
    fn test_training_load_filter_empty_training_load() {
        let training_load = TrainingLoadExport::new();

        let start = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert!(filtered.is_empty());
        let csv = filtered.to_csv();
        // Should still have header
        assert_eq!(csv, "date,tss,atl,ctl,tsb,acwr\n");
    }

    #[test]
    fn test_training_load_filter_partial_overlap_start() {
        // Range overlaps with beginning of data
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                110.0,
                80.0,
                82.0,
                2.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                90.0,
                78.0,
                83.0,
                5.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Range starts before data, ends in middle
        let start = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert_eq!(filtered.len(), 2);
        assert_eq!(
            filtered.days[0].date,
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
        assert_eq!(
            filtered.days[1].date,
            NaiveDate::from_ymd_opt(2024, 6, 16).unwrap()
        );
    }

    #[test]
    fn test_training_load_filter_partial_overlap_end() {
        // Range overlaps with end of data
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                100.0,
                75.0,
                80.0,
                5.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(),
                110.0,
                80.0,
                82.0,
                2.0,
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 17).unwrap(),
                90.0,
                78.0,
                83.0,
                5.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);

        // Range starts in middle, ends after data
        let start = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 25).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        assert_eq!(filtered.len(), 2);
        assert_eq!(
            filtered.days[0].date,
            NaiveDate::from_ymd_opt(2024, 6, 16).unwrap()
        );
        assert_eq!(
            filtered.days[1].date,
            NaiveDate::from_ymd_opt(2024, 6, 17).unwrap()
        );
    }

    #[test]
    fn test_training_load_filter_date_range_method_after_filter() {
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
                80.0,
                60.0,
                70.0,
                10.0,
            ),
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

        // Filter to middle date only
        let start = NaiveDate::from_ymd_opt(2024, 6, 12).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 18).unwrap();
        let filtered = training_load.filter_by_date_range(start, end);

        // date_range() should reflect filtered data
        let range = filtered.date_range().unwrap();
        assert_eq!(range.0, NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
        assert_eq!(range.1, NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
    }

    #[test]
    fn test_training_load_csv_export_all_columns_format() {
        // Comprehensive test of CSV format with all column types
        let days = vec![
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                0.0,    // Zero TSS (rest day)
                50.0,
                60.0,
                10.0,
            ),
            DailyLoadExport::with_acwr(
                NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                200.0,   // High TSS
                75.0,
                62.0,
                -13.0,   // Negative TSB
                1.21,    // High ACWR (injury risk zone)
            ),
            DailyLoadExport::new(
                NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                50.0,
                70.0,
                63.0,
                -7.0,
            ),
        ];
        let training_load = TrainingLoadExport::from_days(days);
        let csv = training_load.to_csv();

        // Verify CSV structure
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 data rows

        // Verify header
        assert_eq!(lines[0], "date,tss,atl,ctl,tsb,acwr");

        // Verify zero values are formatted correctly
        assert!(lines[1].contains("0.00,50.00,60.00,10.00,"));

        // Verify ACWR is present when available
        assert!(lines[2].ends_with(",1.21"));

        // Verify ACWR is empty when not available
        assert!(lines[3].ends_with(","));

        // Verify negative values
        assert!(lines[2].contains("-13.00"));
    }

    // ============ P3.4: Export Error Handling Tests ============

    #[test]
    fn test_export_error_user_not_found_contains_uuid() {
        let user_id = Uuid::new_v4();
        let error = ExportError::UserNotFound(user_id);

        // Error message should contain the user ID
        let error_msg = error.to_string();
        assert!(error_msg.contains(&user_id.to_string()));
        assert!(error_msg.contains("User not found"));
    }

    #[test]
    fn test_export_error_user_not_found_display_format() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let error = ExportError::UserNotFound(user_id);

        let error_msg = format!("{}", error);
        assert_eq!(
            error_msg,
            "User not found: 550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_export_error_insufficient_data_with_descriptive_message() {
        let error = ExportError::InsufficientData("No PDC data available for user".to_string());

        let error_msg = error.to_string();
        assert!(error_msg.contains("Insufficient data"));
        assert!(error_msg.contains("No PDC data available for user"));
    }

    #[test]
    fn test_export_error_insufficient_data_for_training_load() {
        let error = ExportError::InsufficientData(
            "No training load data available for user in the specified date range".to_string(),
        );

        let error_msg = error.to_string();
        assert!(error_msg.contains("Insufficient data"));
        assert!(error_msg.contains("training load"));
        assert!(error_msg.contains("date range"));
    }

    #[test]
    fn test_export_error_serialization_failed_contains_details() {
        let error = ExportError::SerializationFailed(
            "invalid type: expected map, found string".to_string(),
        );

        let error_msg = error.to_string();
        assert!(error_msg.contains("Serialization failed"));
        assert!(error_msg.contains("invalid type"));
    }

    #[test]
    fn test_export_error_database_error_contains_message() {
        let error = ExportError::DatabaseError("Connection refused".to_string());

        let error_msg = error.to_string();
        assert!(error_msg.contains("Database error"));
        assert!(error_msg.contains("Connection refused"));
    }

    #[test]
    fn test_export_error_debug_format() {
        let user_id = Uuid::new_v4();
        let error = ExportError::UserNotFound(user_id);

        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("UserNotFound"));
    }

    #[test]
    fn test_export_error_is_std_error() {
        // Verify that ExportError implements std::error::Error
        fn assert_error<E: std::error::Error>(_: &E) {}

        let error = ExportError::UserNotFound(Uuid::new_v4());
        assert_error(&error);

        let error = ExportError::InsufficientData("test".to_string());
        assert_error(&error);

        let error = ExportError::SerializationFailed("test".to_string());
        assert_error(&error);

        let error = ExportError::DatabaseError("test".to_string());
        assert_error(&error);
    }

    #[test]
    fn test_export_error_distinct_variants() {
        // Test that each error variant produces distinct messages
        let user_id = Uuid::new_v4();
        let errors = vec![
            ExportError::UserNotFound(user_id),
            ExportError::InsufficientData("test message".to_string()),
            ExportError::SerializationFailed("test message".to_string()),
            ExportError::DatabaseError("test message".to_string()),
        ];

        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();

        // All error messages should be unique (different prefixes)
        assert!(messages[0].starts_with("User not found"));
        assert!(messages[1].starts_with("Insufficient data"));
        assert!(messages[2].starts_with("Serialization failed"));
        assert!(messages[3].starts_with("Database error"));
    }

    #[test]
    fn test_export_error_empty_message_handling() {
        // Ensure errors handle empty messages gracefully
        let error = ExportError::InsufficientData(String::new());
        let error_msg = error.to_string();
        assert!(error_msg.contains("Insufficient data"));
        // Should still be valid even with empty inner message
        assert!(!error_msg.is_empty());
    }

    #[test]
    fn test_export_error_long_message_handling() {
        // Ensure errors handle long messages correctly
        let long_message = "a".repeat(1000);
        let error = ExportError::DatabaseError(long_message.clone());

        let error_msg = error.to_string();
        assert!(error_msg.contains(&long_message));
        assert!(error_msg.len() > 1000);
    }

    #[test]
    fn test_export_error_special_characters_in_message() {
        // Test that error messages handle special characters
        let message = "Error: Connection failed (code: 42) - user's data \"corrupted\"";
        let error = ExportError::DatabaseError(message.to_string());

        let error_msg = error.to_string();
        assert!(error_msg.contains(message));
        assert!(error_msg.contains("user's"));
        assert!(error_msg.contains("\"corrupted\""));
    }

    #[test]
    fn test_export_error_user_not_found_different_uuids() {
        // Test that different UUIDs produce different error messages
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let error1 = ExportError::UserNotFound(uuid1);
        let error2 = ExportError::UserNotFound(uuid2);

        assert_ne!(error1.to_string(), error2.to_string());
        assert!(error1.to_string().contains(&uuid1.to_string()));
        assert!(error2.to_string().contains(&uuid2.to_string()));
    }

    #[test]
    fn test_export_error_informative_pdc_error() {
        // Test the actual error message used in export_pdc_csv
        let error = ExportError::InsufficientData("No PDC data available for user".to_string());

        let msg = error.to_string();
        // Message should be informative enough to understand the problem
        assert!(msg.contains("PDC"));
        assert!(msg.contains("data"));
        assert!(msg.contains("user"));
    }

    #[test]
    fn test_export_error_informative_training_load_error() {
        // Test the actual error message used in export_training_load_csv
        let error = ExportError::InsufficientData(
            "No training load data available for user in the specified date range".to_string(),
        );

        let msg = error.to_string();
        // Message should explain what's missing and why
        assert!(msg.contains("training load"));
        assert!(msg.contains("date range"));
        assert!(msg.contains("user"));
    }
}
