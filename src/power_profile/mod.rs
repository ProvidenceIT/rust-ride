//! Power profile module for 4D power analysis.
//!
//! Provides multi-duration power profiling with rolling 90-day and lifetime bests,
//! strength/weakness analysis, and rider type classification.

mod analysis;
mod comparison;
mod lifetime;
mod manager;
mod mmp_adapter;
mod profile;
mod ride_integration;
mod rider_type;
mod rolling;
mod types;

pub use analysis::{DurationStrength, EnergySystem, ProfileAnalysis, StrengthLevel};
pub use comparison::{
    DurationComparison, ProfileComparer, ProfileComparison, ReferenceCurve, ReferenceLevel,
    female_reference_wpk, male_reference_wpk,
};
pub use lifetime::{LifetimeBest, LifetimeBestTracker, LifetimeCheckResult, build_lifetime_from_history};
pub use manager::{PowerProfileManager, PowerProfileManagerBuilder, RideProcessResult};
pub use mmp_adapter::{MmpAdapter, RideMmpProcessor};
pub use profile::{PowerProfile, PowerProfilePoint};
pub use rider_type::{RiderClassification, RiderType};
pub use rolling::{
    RidePowerData, RollingWindowCalculator, RollingWindowConfig, RollingWindowUpdate,
    RollingWindowUpdater,
};
pub use ride_integration::{
    extract_power_samples, process_ride_for_profiles, ride_samples_to_power_data,
    HistoricalRideProcessor, PowerProfileUpdateSummary,
};
pub use types::{duration_label, get_duration_bucket, is_standard_duration, ProfileType, PROFILE_DURATIONS};
