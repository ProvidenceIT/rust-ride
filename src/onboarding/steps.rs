//! Onboarding wizard steps.
//!
//! T052-T056: Step UI implementations for onboarding wizard.

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};
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
            OnboardingStep::Welcome => "Let's get you set up for your first ride.",
            OnboardingStep::SensorSetup => {
                "Connect your smart trainer, power meter, or heart rate monitor."
            }
            OnboardingStep::ProfileSetup => "Enter your details to personalize your training.",
            OnboardingStep::FtpConfiguration => {
                "Set your Functional Threshold Power for accurate training zones."
            }
            OnboardingStep::UiTour => "Let's explore the main features of RustRide.",
            OnboardingStep::Complete => "You're ready to start riding.",
        }
    }

    /// Check if this step can be skipped.
    pub fn is_skippable(&self) -> bool {
        match self {
            OnboardingStep::Welcome => false,
            OnboardingStep::SensorSetup => true, // Can connect later
            OnboardingStep::ProfileSetup => true, // Can use defaults
            OnboardingStep::FtpConfiguration => true, // Can estimate or test later
            OnboardingStep::UiTour => true,      // Optional tour
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

/// Actions that can result from a step UI.
#[derive(Debug, Clone, PartialEq)]
pub enum StepAction {
    /// Continue to next step
    Next,
    /// Go back to previous step
    Back,
    /// Skip this step
    Skip,
    /// Finish onboarding
    Finish,
    /// No action
    None,
}

/// T052: Welcome step UI.
pub struct WelcomeStepUi;

impl WelcomeStepUi {
    /// Render the welcome step.
    pub fn show(ui: &mut Ui) -> StepAction {
        let mut action = StepAction::None;

        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // App logo/icon placeholder
            ui.label(RichText::new("🚴").size(80.0));
            ui.add_space(20.0);

            // Title
            ui.label(RichText::new("Welcome to RustRide").size(32.0).strong());
            ui.add_space(10.0);

            // Description
            ui.label(
                RichText::new("Your open-source, self-hosted indoor cycling companion")
                    .size(18.0)
                    .color(Color32::GRAY),
            );
            ui.add_space(40.0);

            // Feature highlights
            ui.group(|ui| {
                ui.set_min_width(400.0);
                ui.vertical(|ui| {
                    Self::feature_item(
                        ui,
                        "🔌",
                        "Connect smart trainers, power meters, and HR monitors",
                    );
                    Self::feature_item(ui, "📊", "Track your training with real-time metrics");
                    Self::feature_item(ui, "🏋️", "Execute structured workouts with ERG mode");
                    Self::feature_item(ui, "📈", "Analyze your progress with detailed analytics");
                });
            });

            ui.add_space(40.0);

            // Get started button
            if ui
                .add_sized(
                    Vec2::new(200.0, 44.0),
                    egui::Button::new(RichText::new("Get Started").size(16.0)),
                )
                .clicked()
            {
                action = StepAction::Next;
            }
        });

        action
    }

    fn feature_item(ui: &mut Ui, icon: &str, text: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(24.0));
            ui.add_space(10.0);
            ui.label(RichText::new(text).size(14.0));
        });
        ui.add_space(8.0);
    }
}

/// T053: Sensor setup step UI.
#[derive(Default)]
pub struct SensorSetupStepUi {
    /// Whether scanning is active
    pub scanning: bool,
    /// Discovered sensors
    pub discovered_sensors: Vec<SensorInfo>,
    /// Selected sensor IDs
    pub selected_sensors: Vec<String>,
}

/// Basic sensor info for display.
#[derive(Debug, Clone)]
pub struct SensorInfo {
    /// Sensor ID
    pub id: String,
    /// Sensor name
    pub name: String,
    /// Sensor type
    pub sensor_type: String,
    /// Signal strength (if available)
    pub signal_strength: Option<i8>,
}

impl SensorSetupStepUi {
    /// Create a new sensor setup UI.
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the sensor setup step.
    pub fn show(&mut self, ui: &mut Ui) -> StepAction {
        let mut action = StepAction::None;

        ui.vertical(|ui| {
            ui.add_space(20.0);

            // Header
            ui.label(RichText::new("Connect Your Sensors").size(24.0).strong());
            ui.add_space(10.0);
            ui.label(
                RichText::new("RustRide can connect to Bluetooth smart trainers, power meters, and heart rate monitors.")
                    .color(Color32::GRAY),
            );
            ui.add_space(20.0);

            // Scan button
            ui.horizontal(|ui| {
                let scan_text = if self.scanning {
                    "Scanning..."
                } else {
                    "Scan for Sensors"
                };
                if ui
                    .add_enabled(
                        !self.scanning,
                        egui::Button::new(scan_text).min_size(Vec2::new(150.0, 36.0)),
                    )
                    .clicked()
                {
                    self.scanning = true;
                    // Note: Actual scanning would be triggered here
                }

                if self.scanning {
                    ui.spinner();
                }
            });

            ui.add_space(20.0);

            // Discovered sensors list
            ui.group(|ui| {
                ui.set_min_height(200.0);
                ui.set_min_width(ui.available_width() - 20.0);

                if self.discovered_sensors.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("No sensors discovered yet. Click 'Scan' to find nearby sensors.")
                                .color(Color32::GRAY),
                        );
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for sensor in &self.discovered_sensors {
                            let is_selected = self.selected_sensors.contains(&sensor.id);
                            ui.horizontal(|ui| {
                                let icon = match sensor.sensor_type.as_str() {
                                    "trainer" => "🚲",
                                    "power" => "⚡",
                                    "heartrate" => "❤️",
                                    "cadence" => "🔄",
                                    _ => "📡",
                                };
                                ui.label(RichText::new(icon).size(20.0));
                                ui.add_space(10.0);

                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&sensor.name).strong());
                                    ui.label(
                                        RichText::new(&sensor.sensor_type)
                                            .size(12.0)
                                            .color(Color32::GRAY),
                                    );
                                });

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let btn_text = if is_selected { "Connected" } else { "Connect" };
                                    if ui.button(btn_text).clicked() && !is_selected {
                                        self.selected_sensors.push(sensor.id.clone());
                                    }
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });
                }
            });

            ui.add_space(20.0);

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    action = StepAction::Back;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_sized(Vec2::new(100.0, 36.0), egui::Button::new("Continue"))
                        .clicked()
                    {
                        action = StepAction::Next;
                    }

                    if ui.button("Skip for now").clicked() {
                        action = StepAction::Skip;
                    }
                });
            });
        });

        action
    }
}

/// T054: Profile setup step UI.
pub struct ProfileSetupStepUi {
    /// User's name
    pub name: String,
    /// Weight in kg
    pub weight_kg: String,
    /// Height in cm
    pub height_cm: String,
    /// Date of birth (optional)
    pub date_of_birth: String,
    /// Selected gender (optional)
    pub gender: Option<String>,
    /// Use metric units
    pub use_metric: bool,
}

impl Default for ProfileSetupStepUi {
    fn default() -> Self {
        Self {
            name: String::new(),
            weight_kg: "75.0".to_string(),
            height_cm: "175".to_string(),
            date_of_birth: String::new(),
            gender: None,
            use_metric: true,
        }
    }
}

impl ProfileSetupStepUi {
    /// Create a new profile setup UI.
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the profile setup step.
    pub fn show(&mut self, ui: &mut Ui) -> StepAction {
        let mut action = StepAction::None;

        ui.vertical(|ui| {
            ui.add_space(20.0);

            // Header
            ui.label(RichText::new("Create Your Profile").size(24.0).strong());
            ui.add_space(10.0);
            ui.label(
                RichText::new(
                    "Enter your details for personalized training zones and calorie calculations.",
                )
                .color(Color32::GRAY),
            );
            ui.add_space(30.0);

            // Profile form
            egui::Grid::new("profile_grid")
                .num_columns(2)
                .spacing([20.0, 12.0])
                .show(ui, |ui| {
                    // Name
                    ui.label("Name:");
                    ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(250.0));
                    ui.end_row();

                    // Unit preference
                    ui.label("Units:");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.use_metric, "Metric (kg, km)")
                            .clicked()
                        {
                            self.use_metric = true;
                        }
                        if ui
                            .selectable_label(!self.use_metric, "Imperial (lbs, mi)")
                            .clicked()
                        {
                            self.use_metric = false;
                        }
                    });
                    ui.end_row();

                    // Weight
                    let weight_label = if self.use_metric {
                        "Weight (kg):"
                    } else {
                        "Weight (lbs):"
                    };
                    ui.label(weight_label);
                    ui.add(egui::TextEdit::singleline(&mut self.weight_kg).desired_width(100.0));
                    ui.end_row();

                    // Height
                    let height_label = if self.use_metric {
                        "Height (cm):"
                    } else {
                        "Height (in):"
                    };
                    ui.label(height_label);
                    ui.add(egui::TextEdit::singleline(&mut self.height_cm).desired_width(100.0));
                    ui.end_row();

                    // Gender (optional)
                    ui.label("Gender (optional):");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.gender.as_deref() == Some("male"), "Male")
                            .clicked()
                        {
                            self.gender = Some("male".to_string());
                        }
                        if ui
                            .selectable_label(self.gender.as_deref() == Some("female"), "Female")
                            .clicked()
                        {
                            self.gender = Some("female".to_string());
                        }
                        if ui
                            .selectable_label(self.gender.is_none(), "Prefer not to say")
                            .clicked()
                        {
                            self.gender = None;
                        }
                    });
                    ui.end_row();
                });

            ui.add_space(30.0);

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    action = StepAction::Back;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_sized(Vec2::new(100.0, 36.0), egui::Button::new("Continue"))
                        .clicked()
                    {
                        action = StepAction::Next;
                    }

                    if ui.button("Skip for now").clicked() {
                        action = StepAction::Skip;
                    }
                });
            });
        });

        action
    }
}

/// T055: FTP configuration step UI.
pub struct FtpConfigurationStepUi {
    /// FTP value
    pub ftp: String,
    /// Max heart rate
    pub max_hr: String,
    /// Resting heart rate
    pub resting_hr: String,
    /// FTP source
    pub ftp_source: FtpSource,
}

/// Source of FTP value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FtpSource {
    /// Manually entered
    #[default]
    Manual,
    /// Based on weight estimation
    Estimated,
    /// From a previous test
    Tested,
}

impl Default for FtpConfigurationStepUi {
    fn default() -> Self {
        Self {
            ftp: "200".to_string(),
            max_hr: "185".to_string(),
            resting_hr: "60".to_string(),
            ftp_source: FtpSource::Manual,
        }
    }
}

impl FtpConfigurationStepUi {
    /// Create a new FTP configuration UI.
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the FTP configuration step.
    pub fn show(&mut self, ui: &mut Ui) -> StepAction {
        let mut action = StepAction::None;

        ui.vertical(|ui| {
            ui.add_space(20.0);

            // Header
            ui.label(RichText::new("Configure Your Power Zones").size(24.0).strong());
            ui.add_space(10.0);
            ui.label(
                RichText::new("Your FTP (Functional Threshold Power) determines your training zones.")
                    .color(Color32::GRAY),
            );
            ui.add_space(30.0);

            // FTP info box
            ui.group(|ui| {
                ui.set_min_width(ui.available_width() - 20.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("💡").size(24.0));
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("What is FTP?").strong());
                        ui.label(
                            RichText::new(
                                "FTP is the maximum power you can sustain for approximately one hour. \
                                 It's used to calculate your training zones for structured workouts.",
                            )
                            .size(13.0)
                            .color(Color32::GRAY),
                        );
                    });
                });
            });

            ui.add_space(20.0);

            // FTP input
            egui::Grid::new("ftp_grid")
                .num_columns(2)
                .spacing([20.0, 12.0])
                .show(ui, |ui| {
                    // FTP source selection
                    ui.label("How do you know your FTP?");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.ftp_source == FtpSource::Tested, "I've tested it")
                            .clicked()
                        {
                            self.ftp_source = FtpSource::Tested;
                        }
                        if ui
                            .selectable_label(self.ftp_source == FtpSource::Estimated, "Estimate for me")
                            .clicked()
                        {
                            self.ftp_source = FtpSource::Estimated;
                            // Estimate based on typical values
                            self.ftp = "200".to_string();
                        }
                        if ui
                            .selectable_label(self.ftp_source == FtpSource::Manual, "I'll enter it")
                            .clicked()
                        {
                            self.ftp_source = FtpSource::Manual;
                        }
                    });
                    ui.end_row();

                    // FTP value
                    ui.label("FTP (watts):");
                    ui.add(egui::TextEdit::singleline(&mut self.ftp).desired_width(100.0));
                    ui.end_row();

                    ui.add_space(10.0);
                    ui.end_row();

                    // Heart rate (optional)
                    ui.label("Max Heart Rate (optional):");
                    ui.add(egui::TextEdit::singleline(&mut self.max_hr).desired_width(100.0));
                    ui.end_row();

                    ui.label("Resting Heart Rate (optional):");
                    ui.add(egui::TextEdit::singleline(&mut self.resting_hr).desired_width(100.0));
                    ui.end_row();
                });

            ui.add_space(30.0);

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    action = StepAction::Back;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_sized(Vec2::new(100.0, 36.0), egui::Button::new("Continue"))
                        .clicked()
                    {
                        action = StepAction::Next;
                    }

                    if ui.button("Skip for now").clicked() {
                        action = StepAction::Skip;
                    }
                });
            });
        });

        action
    }
}

/// T056: UI tour step.
#[derive(Default)]
pub struct UiTourStepUi {
    /// Current tour item
    pub current_item: usize,
}

impl UiTourStepUi {
    /// Create a new UI tour.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get tour items.
    fn tour_items() -> &'static [(&'static str, &'static str, &'static str)] {
        &[
            (
                "🏠",
                "Home Screen",
                "Start rides, access workouts, and view your training history.",
            ),
            (
                "🚴",
                "Ride Screen",
                "See real-time metrics during your ride including power, heart rate, and cadence.",
            ),
            (
                "📊",
                "Analytics",
                "Review your training progress, power curves, and fitness trends.",
            ),
            (
                "⚙️",
                "Settings",
                "Customize your experience, connect sensors, and configure training zones.",
            ),
            (
                "⌨️",
                "Keyboard Shortcuts",
                "Press F1 or ? anytime to see available keyboard shortcuts.",
            ),
        ]
    }

    /// Render the UI tour step.
    pub fn show(&mut self, ui: &mut Ui) -> StepAction {
        let mut action = StepAction::None;
        let items = Self::tour_items();

        ui.vertical(|ui| {
            ui.add_space(20.0);

            // Header
            ui.label(RichText::new("Quick Tour").size(24.0).strong());
            ui.add_space(10.0);
            ui.label(
                RichText::new("Let's take a quick look at the main features.").color(Color32::GRAY),
            );
            ui.add_space(30.0);

            // Progress indicator
            ui.horizontal(|ui| {
                for (i, _) in items.iter().enumerate() {
                    let color = if i == self.current_item {
                        Color32::from_rgb(66, 133, 244)
                    } else if i < self.current_item {
                        Color32::from_rgb(52, 168, 83)
                    } else {
                        Color32::GRAY
                    };
                    ui.add(
                        egui::widgets::ProgressBar::new(1.0)
                            .fill(color)
                            .desired_width(30.0),
                    );
                    if i < items.len() - 1 {
                        ui.add_space(5.0);
                    }
                }
            });

            ui.add_space(30.0);

            // Current tour item
            if let Some((icon, title, description)) = items.get(self.current_item) {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width() - 20.0);
                    ui.set_min_height(150.0);

                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new(*icon).size(48.0));
                        ui.add_space(15.0);
                        ui.label(RichText::new(*title).size(20.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new(*description).size(14.0).color(Color32::GRAY));
                        ui.add_space(20.0);
                    });
                });
            }

            ui.add_space(30.0);

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    if self.current_item > 0 {
                        self.current_item -= 1;
                    } else {
                        action = StepAction::Back;
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.current_item < items.len() - 1 {
                        if ui
                            .add_sized(Vec2::new(100.0, 36.0), egui::Button::new("Next"))
                            .clicked()
                        {
                            self.current_item += 1;
                        }
                    } else if ui
                        .add_sized(Vec2::new(100.0, 36.0), egui::Button::new("Finish"))
                        .clicked()
                    {
                        action = StepAction::Next;
                    }

                    if ui.button("Skip Tour").clicked() {
                        action = StepAction::Skip;
                    }
                });
            });
        });

        action
    }
}

/// Complete step UI (simple congratulations screen).
pub struct CompleteStepUi;

impl CompleteStepUi {
    /// Render the complete step.
    pub fn show(ui: &mut Ui) -> StepAction {
        let mut action = StepAction::None;

        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            // Success icon
            ui.label(RichText::new("🎉").size(80.0));
            ui.add_space(30.0);

            // Title
            ui.label(RichText::new("You're All Set!").size(32.0).strong());
            ui.add_space(15.0);

            // Description
            ui.label(
                RichText::new("You've completed the setup. Time to start riding!")
                    .size(18.0)
                    .color(Color32::GRAY),
            );
            ui.add_space(40.0);

            // Start riding button
            if ui
                .add_sized(
                    Vec2::new(200.0, 50.0),
                    egui::Button::new(RichText::new("Start Riding").size(18.0)),
                )
                .clicked()
            {
                action = StepAction::Finish;
            }
        });

        action
    }
}
