//! Onboarding wizard steps.

use serde::{Deserialize, Serialize};

/// Steps in the onboarding wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum OnboardingStep {
    /// Welcome screen with overview
    #[default]
    Welcome,
    /// Sensor discovery and connection
    SensorSetup,
    /// User profile creation (name, weight, etc.)
    ProfileSetup,
    /// FTP configuration with optional test
    FtpConfiguration,
    /// UI tour and feature highlights
    UiTour,
    /// Completion screen
    Complete,
}

impl OnboardingStep {
    /// Get all steps in order.
    pub fn all() -> &'static [OnboardingStep] {
        &[
            OnboardingStep::Welcome,
            OnboardingStep::SensorSetup,
            OnboardingStep::ProfileSetup,
            OnboardingStep::FtpConfiguration,
            OnboardingStep::UiTour,
            OnboardingStep::Complete,
        ]
    }

    /// Get the step index (0-based).
    pub fn index(&self) -> usize {
        Self::all().iter().position(|s| s == self).unwrap_or(0)
    }

    /// Get the next step, if any.
    pub fn next(&self) -> Option<OnboardingStep> {
        let steps = Self::all();
        let idx = self.index();
        if idx + 1 < steps.len() {
            Some(steps[idx + 1])
        } else {
            None
        }
    }

    /// Get the previous step, if any.
    pub fn previous(&self) -> Option<OnboardingStep> {
        let steps = Self::all();
        let idx = self.index();
        if idx > 0 {
            Some(steps[idx - 1])
        } else {
            None
        }
    }

    /// Get the title for this step.
    pub fn title(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "Welcome to RustRide",
            OnboardingStep::SensorSetup => "Sensor Setup",
            OnboardingStep::ProfileSetup => "Profile Setup",
            OnboardingStep::FtpConfiguration => "FTP Configuration",
            OnboardingStep::UiTour => "UI Tour",
            OnboardingStep::Complete => "All Set!",
        }
    }

    /// Get the description for this step.
    pub fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => {
                "Let's get you set up for your first ride."
            }
            OnboardingStep::SensorSetup => {
                "Connect your smart trainer, power meter, or heart rate monitor."
            }
            OnboardingStep::ProfileSetup => {
                "Enter your details to personalize your training."
            }
            OnboardingStep::FtpConfiguration => {
                "Set your Functional Threshold Power for accurate training zones."
            }
            OnboardingStep::UiTour => {
                "Let's explore the main features of RustRide."
            }
            OnboardingStep::Complete => {
                "You're ready to start riding."
            }
        }
    }

    /// Check if this step can be skipped.
    pub fn is_skippable(&self) -> bool {
        match self {
            OnboardingStep::Welcome => false,
            OnboardingStep::SensorSetup => true, // Can connect later
            OnboardingStep::ProfileSetup => true, // Can use defaults
            OnboardingStep::FtpConfiguration => true, // Can estimate or test later
            OnboardingStep::UiTour => true, // Optional tour
            OnboardingStep::Complete => false,
        }
    }

    /// Check if this is the first step.
    pub fn is_first(&self) -> bool {
        *self == OnboardingStep::Welcome
    }

    /// Check if this is the last step.
    pub fn is_last(&self) -> bool {
        *self == OnboardingStep::Complete
    }
}

impl std::fmt::Display for OnboardingStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}
