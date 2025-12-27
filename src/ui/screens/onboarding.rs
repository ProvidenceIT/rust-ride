//! Onboarding screen for first-time user experience.
//!
//! T059: Integrate onboarding check on app startup.

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::onboarding::steps::{
    CompleteStepUi, FtpConfigurationStepUi, OnboardingStep, ProfileSetupStepUi, SensorSetupStepUi,
    StepAction, UiTourStepUi, WelcomeStepUi,
};
use crate::onboarding::OnboardingWizard;

/// Onboarding screen that wraps the wizard.
pub struct OnboardingScreen {
    /// Wizard controller
    wizard: OnboardingWizard,
    /// Sensor setup UI state
    sensor_ui: SensorSetupStepUi,
    /// Profile setup UI state
    profile_ui: ProfileSetupStepUi,
    /// FTP configuration UI state
    ftp_ui: FtpConfigurationStepUi,
    /// UI tour state
    tour_ui: UiTourStepUi,
}

impl Default for OnboardingScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardingScreen {
    /// Create a new onboarding screen.
    pub fn new() -> Self {
        Self {
            wizard: OnboardingWizard::new(),
            sensor_ui: SensorSetupStepUi::new(),
            profile_ui: ProfileSetupStepUi::new(),
            ftp_ui: FtpConfigurationStepUi::new(),
            tour_ui: UiTourStepUi::new(),
        }
    }

    /// Create from existing wizard state.
    pub fn from_wizard(wizard: OnboardingWizard) -> Self {
        Self {
            wizard,
            sensor_ui: SensorSetupStepUi::new(),
            profile_ui: ProfileSetupStepUi::new(),
            ftp_ui: FtpConfigurationStepUi::new(),
            tour_ui: UiTourStepUi::new(),
        }
    }

    /// Check if onboarding should be shown.
    pub fn should_show(&self) -> bool {
        self.wizard.should_show()
    }

    /// Mark onboarding as complete (for persistence).
    pub fn is_complete(&self) -> bool {
        self.wizard.state().completed
    }

    /// Get the wizard state for persistence.
    pub fn wizard_state(&self) -> &crate::onboarding::OnboardingState {
        self.wizard.state()
    }

    /// Get profile data collected during onboarding.
    pub fn get_profile_data(&self) -> OnboardingProfileData {
        OnboardingProfileData {
            name: self.profile_ui.name.clone(),
            weight_kg: self.profile_ui.weight_kg.parse().unwrap_or(75.0),
            height_cm: self.profile_ui.height_cm.parse().unwrap_or(175.0),
            use_metric: self.profile_ui.use_metric,
            ftp: self.ftp_ui.ftp.parse().unwrap_or(200),
            max_hr: self.ftp_ui.max_hr.parse().ok(),
            resting_hr: self.ftp_ui.resting_hr.parse().ok(),
        }
    }

    /// Show the onboarding screen.
    ///
    /// Returns true when onboarding is complete and the app should proceed to home.
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let mut completed = false;

        // Header with progress
        ui.horizontal(|ui| {
            ui.heading("RustRide Setup");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Progress indicator
                let progress = self.wizard.state().progress_percent();
                ui.label(
                    RichText::new(format!("{}% complete", progress))
                        .size(14.0)
                        .color(Color32::GRAY),
                );

                // Progress bar
                let steps = OnboardingStep::all();
                let current_idx = self.wizard.current_step().index();
                for (i, _step) in steps.iter().enumerate() {
                    let color = if i < current_idx {
                        Color32::from_rgb(52, 168, 83) // Green for completed
                    } else if i == current_idx {
                        Color32::from_rgb(66, 133, 244) // Blue for current
                    } else {
                        Color32::from_rgb(100, 100, 110) // Gray for pending
                    };

                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(20.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color);
                }
            });
        });

        ui.separator();

        // Main content area with current step
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let action = match self.wizard.current_step() {
                OnboardingStep::Welcome => WelcomeStepUi::show(ui),
                OnboardingStep::SensorSetup => self.sensor_ui.show(ui),
                OnboardingStep::ProfileSetup => self.profile_ui.show(ui),
                OnboardingStep::FtpConfiguration => self.ftp_ui.show(ui),
                OnboardingStep::UiTour => self.tour_ui.show(ui),
                OnboardingStep::Complete => CompleteStepUi::show(ui),
            };

            // Handle step action
            match action {
                StepAction::Next => {
                    self.wizard.next_step();
                }
                StepAction::Back => {
                    self.wizard.previous_step();
                }
                StepAction::Skip => {
                    self.wizard.next_step();
                }
                StepAction::Finish => {
                    completed = true;
                }
                StepAction::None => {}
            }
        });

        completed
    }

    /// Restart the onboarding process.
    pub fn restart(&mut self) {
        self.wizard.restart();
        self.sensor_ui = SensorSetupStepUi::new();
        self.profile_ui = ProfileSetupStepUi::new();
        self.ftp_ui = FtpConfigurationStepUi::new();
        self.tour_ui = UiTourStepUi::new();
    }
}

/// Profile data collected during onboarding.
#[derive(Debug, Clone)]
pub struct OnboardingProfileData {
    /// User name
    pub name: String,
    /// Weight in kg
    pub weight_kg: f64,
    /// Height in cm
    pub height_cm: f64,
    /// Use metric units
    pub use_metric: bool,
    /// FTP in watts
    pub ftp: u16,
    /// Max heart rate (optional)
    pub max_hr: Option<u8>,
    /// Resting heart rate (optional)
    pub resting_hr: Option<u8>,
}
