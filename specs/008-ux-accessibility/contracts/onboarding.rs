//! Onboarding Module Contract
//!
//! Public API for the first-time user onboarding wizard.

// ============================================================================
// Onboarding Wizard
// ============================================================================

/// Manages the onboarding wizard flow.
pub trait OnboardingWizard {
    /// Check if onboarding should be shown (first-time user or reset).
    fn should_show_onboarding(&self) -> bool;

    /// Get the current step.
    fn current_step(&self) -> OnboardingStep;

    /// Get information about the current step.
    fn step_info(&self, step: OnboardingStep) -> StepInfo;

    /// Move to the next step.
    fn next_step(&mut self) -> Result<OnboardingStep, OnboardingError>;

    /// Move to the previous step.
    fn prev_step(&mut self) -> Result<OnboardingStep, OnboardingError>;

    /// Skip to a specific step.
    fn go_to_step(&mut self, step: OnboardingStep) -> Result<(), OnboardingError>;

    /// Skip the entire onboarding.
    fn skip_onboarding(&mut self);

    /// Complete the onboarding.
    fn complete_onboarding(&mut self);

    /// Reset onboarding (can be triggered from settings).
    fn reset_onboarding(&mut self);

    /// Check if onboarding is completed.
    fn is_completed(&self) -> bool;

    /// Check if onboarding was skipped.
    fn was_skipped(&self) -> bool;

    /// Get the total number of steps.
    fn total_steps(&self) -> usize;

    /// Get progress as percentage (0-100).
    fn progress_percent(&self) -> u8;
}

/// Onboarding wizard steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OnboardingStep {
    Welcome = 0,
    SensorSetup = 1,
    ProfileSetup = 2,
    FtpConfiguration = 3,
    UiTour = 4,
    Complete = 5,
}

impl OnboardingStep {
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Welcome),
            1 => Some(Self::SensorSetup),
            2 => Some(Self::ProfileSetup),
            3 => Some(Self::FtpConfiguration),
            4 => Some(Self::UiTour),
            5 => Some(Self::Complete),
            _ => None,
        }
    }

    pub fn index(&self) -> u8 {
        *self as u8
    }
}

/// Information about an onboarding step.
#[derive(Clone, Debug)]
pub struct StepInfo {
    /// Step identifier
    pub step: OnboardingStep,

    /// Step title (for header)
    pub title: String,

    /// Step description (for body)
    pub description: String,

    /// Primary action button label
    pub primary_action: String,

    /// Secondary action (skip) label, if applicable
    pub secondary_action: Option<String>,

    /// Whether this step can be skipped
    pub can_skip: bool,

    /// Whether this step has been completed
    pub is_completed: bool,

    /// Glossary terms used in this step
    pub glossary_terms: Vec<String>,
}

// ============================================================================
// Glossary
// ============================================================================

/// Provides cycling terminology definitions for tooltips.
pub trait Glossary {
    /// Get the definition for a term.
    fn get_definition(&self, term: &str) -> Option<&GlossaryEntry>;

    /// Get all glossary terms.
    fn all_terms(&self) -> &[GlossaryEntry];

    /// Search for terms matching a query.
    fn search(&self, query: &str) -> Vec<&GlossaryEntry>;
}

/// A glossary entry with term and definition.
#[derive(Clone, Debug)]
pub struct GlossaryEntry {
    /// The term (e.g., "FTP", "FTMS", "TSS")
    pub term: String,

    /// Short definition (for tooltip)
    pub short_definition: String,

    /// Long definition (for glossary screen)
    pub long_definition: String,

    /// Related terms
    pub related: Vec<String>,

    /// Category (e.g., "Power", "Sensors", "Training")
    pub category: GlossaryCategory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlossaryCategory {
    Power,
    HeartRate,
    Sensors,
    Training,
    Metrics,
    Equipment,
}

/// Standard glossary terms to include:
/// - FTP: Functional Threshold Power
/// - FTMS: Fitness Machine Service (BLE protocol)
/// - ANT+: Wireless sensor protocol
/// - TSS: Training Stress Score
/// - NP: Normalized Power
/// - IF: Intensity Factor
/// - ERG Mode: Electronic resistance control
/// - Smart Trainer: Controllable indoor trainer
/// - Power Meter: Device measuring cycling power
/// - Cadence: Pedaling rate (RPM)
/// - Heart Rate Zones: Training intensity zones based on HR
/// - Power Zones: Training intensity zones based on power

// ============================================================================
// Step-Specific Interfaces
// ============================================================================

/// Sensor setup step actions.
pub trait SensorSetupStep {
    /// Start sensor discovery.
    fn start_discovery(&mut self);

    /// Stop sensor discovery.
    fn stop_discovery(&mut self);

    /// Get discovered sensors.
    fn discovered_sensors(&self) -> &[DiscoveredSensor];

    /// Connect to a sensor.
    fn connect_sensor(&mut self, sensor_id: &str) -> Result<(), SensorError>;

    /// Check if at least one sensor is connected.
    fn has_connected_sensor(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct DiscoveredSensor {
    pub id: String,
    pub name: String,
    pub sensor_type: SensorType,
    pub signal_strength: i8,
    pub is_connected: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum SensorType {
    SmartTrainer,
    PowerMeter,
    HeartRateMonitor,
    CadenceSensor,
    SpeedSensor,
}

/// Profile setup step actions.
pub trait ProfileSetupStep {
    /// Set user name.
    fn set_name(&mut self, name: &str);

    /// Set user weight.
    fn set_weight(&mut self, weight_kg: f32) -> Result<(), ValidationError>;

    /// Set user height.
    fn set_height(&mut self, height_cm: u16);

    /// Set unit preference.
    fn set_units(&mut self, units: Units);

    /// Validate the profile is complete.
    fn is_complete(&self) -> bool;
}

/// FTP configuration step actions.
pub trait FtpSetupStep {
    /// Set FTP value directly.
    fn set_ftp(&mut self, ftp: u16) -> Result<(), ValidationError>;

    /// Start an FTP test (navigates to workout).
    fn start_ftp_test(&mut self);

    /// Use auto-detected FTP from recent rides.
    fn use_auto_detected_ftp(&mut self) -> Option<u16>;

    /// Skip FTP setup (use default).
    fn skip_with_default(&mut self);
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum OnboardingError {
    #[error("Cannot go to previous step from first step")]
    AtFirstStep,

    #[error("Cannot go to next step from last step")]
    AtLastStep,

    #[error("Step not accessible: {0:?}")]
    StepNotAccessible(OnboardingStep),

    #[error("Persistence error: {0}")]
    PersistenceError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Value out of range")]
    OutOfRange,

    #[error("Required field missing")]
    Required,
}

#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    #[error("Sensor not found")]
    NotFound,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}
