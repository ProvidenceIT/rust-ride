# Analytics Module API Contract

**Module**: `src/metrics/analytics/`
**Feature**: 003-training-analytics
**Date**: 2025-12-25

## Overview

This contract defines the public API for the analytics module. All analytics calculations are exposed as library functions, decoupled from storage and UI concerns.

---

## Module Structure

```rust
// src/metrics/analytics/mod.rs
pub mod pdc;
pub mod critical_power;
pub mod ftp_detection;
pub mod training_load;
pub mod vo2max;
pub mod rider_type;
pub mod sweet_spot;

// Re-exports
pub use pdc::{PowerDurationCurve, MmpCalculator};
pub use critical_power::{CpModel, CpFitter};
pub use ftp_detection::{FtpDetector, FtpEstimate, FtpConfidence};
pub use training_load::{TrainingLoadCalculator, DailyLoad, Acwr};
pub use vo2max::{Vo2maxCalculator, Vo2maxResult};
pub use rider_type::{RiderClassifier, RiderType, PowerProfile};
pub use sweet_spot::{SweetSpotRecommender, WorkoutRecommendation};
```

---

## 1. Power Duration Curve (pdc.rs)

### Types

```rust
/// A single point on the power duration curve
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdcPoint {
    /// Duration in seconds
    pub duration_secs: u32,
    /// Maximum average power at this duration
    pub power_watts: u16,
}

/// Power Duration Curve - collection of max power values at each duration
#[derive(Debug, Clone)]
pub struct PowerDurationCurve {
    /// User's PDC points, sorted by duration
    points: Vec<PdcPoint>,
}

/// Calculator for extracting Mean Maximal Power from ride samples
pub struct MmpCalculator {
    // Internal state for efficient windowed calculations
}
```

### Functions

```rust
impl PowerDurationCurve {
    /// Create empty PDC
    pub fn new() -> Self;

    /// Create PDC from existing points
    pub fn from_points(points: Vec<PdcPoint>) -> Self;

    /// Get power at a specific duration (interpolates if needed)
    pub fn power_at(&self, duration_secs: u32) -> Option<u16>;

    /// Get all points for charting
    pub fn points(&self) -> &[PdcPoint];

    /// Update PDC with new ride data, returns which points changed
    pub fn update(&mut self, new_points: &[PdcPoint]) -> Vec<PdcPoint>;

    /// Filter to date range
    pub fn filter_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self;

    /// Check if sufficient data for CP calculation
    pub fn has_sufficient_data_for_cp(&self) -> bool;
}

impl MmpCalculator {
    /// Create new calculator for specified durations
    pub fn new(durations: &[u32]) -> Self;

    /// Create with standard durations (1s to 5h)
    pub fn standard() -> Self;

    /// Calculate MMP from power samples (1-second intervals assumed)
    /// Returns max power at each configured duration
    pub fn calculate(&self, power_samples: &[u16]) -> Vec<PdcPoint>;

    /// Calculate single duration MMP (for focused queries)
    pub fn calculate_single(&self, power_samples: &[u16], duration_secs: u32) -> Option<u16>;
}
```

### Example Usage

```rust
// Extract MMP from a ride
let calculator = MmpCalculator::standard();
let samples: Vec<u16> = ride.samples.iter()
    .filter_map(|s| s.power_watts.map(|p| p as u16))
    .collect();
let ride_mmp = calculator.calculate(&samples);

// Update user's PDC
let mut pdc = PowerDurationCurve::from_points(existing_points);
let updated = pdc.update(&ride_mmp);

// Get 5-minute power
let ftp_power = pdc.power_at(300); // 5 min = 300 seconds
```

---

## 2. Critical Power Model (critical_power.rs)

### Types

```rust
/// Critical Power model parameters
#[derive(Debug, Clone, Copy)]
pub struct CpModel {
    /// Critical Power in watts
    pub cp: u16,
    /// W' (anaerobic capacity) in joules
    pub w_prime: u32,
    /// Model fit quality (R² value, 0-1)
    pub r_squared: f32,
}

/// Model fitting errors
#[derive(Debug, thiserror::Error)]
pub enum CpFitError {
    #[error("Insufficient data points (need at least 3, got {0})")]
    InsufficientData(usize),
    #[error("Invalid duration range (need 2-20 minute efforts)")]
    InvalidDurationRange,
    #[error("Model fitting failed: {0}")]
    FittingFailed(String),
}

/// CP model fitter
pub struct CpFitter {
    /// Minimum duration for fitting (default: 120s / 2 min)
    min_duration: u32,
    /// Maximum duration for fitting (default: 1200s / 20 min)
    max_duration: u32,
}
```

### Functions

```rust
impl CpFitter {
    /// Create with default settings (2-20 min range)
    pub fn new() -> Self;

    /// Create with custom duration range
    pub fn with_range(min_secs: u32, max_secs: u32) -> Self;

    /// Fit CP model from PDC points
    pub fn fit(&self, pdc: &PowerDurationCurve) -> Result<CpModel, CpFitError>;

    /// Fit from explicit (duration, power) pairs
    pub fn fit_points(&self, points: &[(u32, u16)]) -> Result<CpModel, CpFitError>;
}

impl CpModel {
    /// Predict time to exhaustion at given power
    /// Returns None if power <= CP (theoretically infinite)
    pub fn time_to_exhaustion(&self, power_watts: u16) -> Option<Duration>;

    /// Predict sustainable power for given duration
    pub fn power_at_duration(&self, duration: Duration) -> u16;

    /// Calculate remaining W' after work at given power/duration
    pub fn w_prime_remaining(&self, power_watts: u16, duration: Duration) -> i32;
}
```

### Example Usage

```rust
let fitter = CpFitter::new();
match fitter.fit(&pdc) {
    Ok(model) => {
        println!("CP: {} watts", model.cp);
        println!("W': {} kJ", model.w_prime / 1000);

        // How long can I hold 350W?
        if let Some(tte) = model.time_to_exhaustion(350) {
            println!("TTE at 350W: {:?}", tte);
        }
    }
    Err(CpFitError::InsufficientData(n)) => {
        println!("Need more efforts (have {})", n);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## 3. FTP Detection (ftp_detection.rs)

### Types

```rust
/// FTP detection confidence level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FtpConfidence {
    /// 3+ recent quality efforts
    High,
    /// 2+ efforts
    Medium,
    /// Limited data
    Low,
}

/// Method used to detect FTP
#[derive(Debug, Clone, Copy)]
pub enum FtpMethod {
    /// 95% of 20-minute power
    TwentyMinute,
    /// Average of extended duration efforts
    ExtendedDuration,
    /// Derived from CP model
    CriticalPower,
}

/// FTP estimate result
#[derive(Debug, Clone)]
pub struct FtpEstimate {
    pub ftp_watts: u16,
    pub method: FtpMethod,
    pub confidence: FtpConfidence,
    pub supporting_data: Vec<(u32, u16)>, // (duration, power) pairs used
}

/// FTP detector
pub struct FtpDetector {
    /// Minimum weeks of data required
    min_weeks: u8,
    /// How recent data must be (in days)
    recency_days: u32,
}
```

### Functions

```rust
impl FtpDetector {
    /// Create with default settings
    pub fn new() -> Self;

    /// Detect FTP from PDC
    pub fn detect(&self, pdc: &PowerDurationCurve) -> Option<FtpEstimate>;

    /// Detect using CP model (more accurate if available)
    pub fn detect_from_cp(&self, cp_model: &CpModel) -> FtpEstimate;

    /// Check if FTP estimate differs significantly from current
    pub fn is_significant_change(&self, current_ftp: u16, new_estimate: &FtpEstimate) -> bool;

    /// Get change percentage
    pub fn change_percent(&self, current: u16, new: u16) -> f32;
}

impl FtpEstimate {
    /// Check if this should trigger user notification
    pub fn should_notify(&self, current_ftp: u16) -> bool;
}
```

---

## 4. Training Load (training_load.rs)

### Types

```rust
/// Daily training load values
#[derive(Debug, Clone, Copy)]
pub struct DailyLoad {
    /// Total TSS for the day
    pub tss: f32,
    /// Acute Training Load (7-day)
    pub atl: f32,
    /// Chronic Training Load (42-day)
    pub ctl: f32,
    /// Training Stress Balance
    pub tsb: f32,
}

/// Acute:Chronic Workload Ratio result
#[derive(Debug, Clone, Copy)]
pub struct Acwr {
    pub ratio: f32,
    pub status: AcwrStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AcwrStatus {
    Undertrained,  // < 0.8
    Optimal,       // 0.8 - 1.3
    Caution,       // 1.3 - 1.5
    HighRisk,      // > 1.5
}

/// Training load calculator
pub struct TrainingLoadCalculator {
    /// ATL decay constant (default: 7 days)
    atl_days: f32,
    /// CTL decay constant (default: 42 days)
    ctl_days: f32,
}
```

### Functions

```rust
impl TrainingLoadCalculator {
    /// Create with default constants (7/42 day)
    pub fn new() -> Self;

    /// Create with custom decay constants
    pub fn with_constants(atl_days: f32, ctl_days: f32) -> Self;

    /// Calculate training load for a date given previous day's values and today's TSS
    pub fn calculate_day(&self, prev: DailyLoad, today_tss: f32) -> DailyLoad;

    /// Calculate full history from daily TSS values
    pub fn calculate_history(&self, daily_tss: &[(NaiveDate, f32)]) -> Vec<(NaiveDate, DailyLoad)>;

    /// Calculate ACWR from current ATL/CTL
    pub fn acwr(&self, atl: f32, ctl: f32) -> Acwr;
}

impl Acwr {
    /// Get color for UI display
    pub fn color(&self) -> (u8, u8, u8); // RGB

    /// Get recommendation text
    pub fn recommendation(&self) -> &'static str;
}
```

---

## 5. VO2max Estimation (vo2max.rs)

### Types

```rust
/// VO2max calculation result
#[derive(Debug, Clone, Copy)]
pub struct Vo2maxResult {
    /// VO2max in ml/kg/min
    pub vo2max: f32,
    /// Percentile vs age/gender norms
    pub percentile: u8,
    /// 5-min power used
    pub power_5min: u16,
    /// Weight used
    pub weight_kg: f32,
}

/// VO2max calculator
pub struct Vo2maxCalculator {
    // Reference tables for percentile lookup
}
```

### Functions

```rust
impl Vo2maxCalculator {
    /// Create calculator with default reference tables
    pub fn new() -> Self;

    /// Calculate VO2max from 5-minute power and weight
    pub fn calculate(&self, power_5min: u16, weight_kg: f32) -> f32;

    /// Calculate with percentile for age/gender
    pub fn calculate_with_percentile(
        &self,
        power_5min: u16,
        weight_kg: f32,
        age: u8,
        is_male: bool,
    ) -> Vo2maxResult;

    /// Get percentile only (for existing VO2max value)
    pub fn percentile(&self, vo2max: f32, age: u8, is_male: bool) -> u8;
}
```

---

## 6. Rider Type Classification (rider_type.rs)

### Types

```rust
/// Rider classification types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiderType {
    Sprinter,
    Pursuiter,
    Climber,
    TimeTrialist,
    AllRounder,
}

/// Power profile scores (0-1 normalized)
#[derive(Debug, Clone, Copy)]
pub struct PowerProfile {
    /// 5-second power score
    pub neuromuscular: f32,
    /// 1-minute power score
    pub anaerobic: f32,
    /// 5-minute power score
    pub vo2max: f32,
    /// 20-minute power score
    pub threshold: f32,
}

/// Rider classifier
pub struct RiderClassifier {
    /// Reference power values by category
}
```

### Functions

```rust
impl RiderClassifier {
    /// Create with default reference values
    pub fn new() -> Self;

    /// Classify rider type from PDC
    pub fn classify(&self, pdc: &PowerDurationCurve, weight_kg: f32) -> RiderType;

    /// Get full power profile
    pub fn profile(&self, pdc: &PowerDurationCurve, weight_kg: f32) -> PowerProfile;

    /// Get classification explanation
    pub fn explain(&self, profile: &PowerProfile) -> String;
}

impl RiderType {
    /// Get training focus recommendations
    pub fn training_focus(&self) -> &'static str;

    /// Get event type recommendations
    pub fn recommended_events(&self) -> &'static [&'static str];
}
```

---

## 7. Sweet Spot Recommendations (sweet_spot.rs)

### Types

```rust
/// Workout recommendation
#[derive(Debug, Clone)]
pub struct WorkoutRecommendation {
    /// Target power range (low, high)
    pub power_range: (u16, u16),
    /// Suggested interval count
    pub intervals: u8,
    /// Suggested interval duration in minutes
    pub interval_minutes: u8,
    /// Rest between intervals in minutes
    pub rest_minutes: u8,
    /// Total workout duration estimate
    pub total_minutes: u16,
    /// Additional notes/warnings
    pub notes: Vec<String>,
}

/// Sweet spot workout recommender
pub struct SweetSpotRecommender {
    /// Sweet spot percentage range (default: 0.88-0.93)
    ss_low: f32,
    ss_high: f32,
}
```

### Functions

```rust
impl SweetSpotRecommender {
    /// Create with default settings
    pub fn new() -> Self;

    /// Generate recommendation based on FTP and training load
    pub fn recommend(
        &self,
        ftp: u16,
        ctl: f32,
        acwr: Acwr,
        available_minutes: Option<u16>,
    ) -> WorkoutRecommendation;

    /// Get power zone for sweet spot at given FTP
    pub fn zone(&self, ftp: u16) -> (u16, u16);

    /// Check if current power is in sweet spot range
    pub fn is_in_zone(&self, power: u16, ftp: u16) -> bool;
}
```

---

## Error Handling

All modules use `thiserror` for error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Calculation failed: {0}")]
    CalculationFailed(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] rusqlite::Error),
}

pub type AnalyticsResult<T> = Result<T, AnalyticsError>;
```

---

## Thread Safety

All calculator types are `Send + Sync` where applicable:

- `PowerDurationCurve`: `Clone + Send + Sync`
- `CpModel`: `Copy + Send + Sync`
- `MmpCalculator`: `Send + Sync` (stateless after creation)
- All calculators are safe to use from multiple threads

---

## Testing Requirements

Each module must include:

1. **Unit tests** for pure calculation functions
2. **Property tests** for edge cases (zero values, extremes)
3. **Regression tests** against known reference values

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cp_model_tte() {
        let model = CpModel { cp: 250, w_prime: 20000, r_squared: 0.98 };

        // At CP, TTE is infinite
        assert!(model.time_to_exhaustion(250).is_none());

        // Above CP, TTE is finite
        let tte = model.time_to_exhaustion(300).unwrap();
        assert_eq!(tte.as_secs(), 400); // W' / (300 - 250) = 20000 / 50 = 400s
    }
}
```
