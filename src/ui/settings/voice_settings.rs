//! Voice control settings UI panel widget.
//!
//! Feature 018: Add voice control settings to settings screen.
//! Provides enable/disable toggle, activation mode selection,
//! push-to-talk key binding, voice feedback toggle, and model download.

use egui::{Align, Color32, Layout, ProgressBar, RichText, Ui, Key};

#[cfg(feature = "voice-control")]
use crate::voice::{
    ModelState, VoskModelManager, PushToTalkKey, VoiceActivation,
    DEFAULT_PUSH_TO_TALK_KEY,
};

#[cfg(not(feature = "voice-control"))]
use crate::storage::config::VoiceActivation;

/// Actions that can be triggered from the voice settings panel.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceSettingsAction {
    /// Voice settings were modified
    SettingsChanged(VoiceSettings),
    /// User requested to download the voice model
    DownloadModel,
    /// User cancelled model download
    CancelDownload,
    /// User toggled voice control enabled
    ToggleVoiceControl(bool),
    /// User changed activation mode
    ChangeActivationMode(VoiceActivation),
    /// User changed push-to-talk key binding
    ChangePushToTalkKey(Key),
    /// User toggled voice feedback
    ToggleVoiceFeedback(bool),
    /// User toggled TTS confirmation
    ToggleTtsConfirmation(bool),
}

/// Current voice control settings.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceSettings {
    /// Whether voice control is enabled.
    pub enabled: bool,
    /// Voice activation mode.
    pub activation_mode: VoiceActivation,
    /// Push-to-talk key binding.
    pub push_to_talk_key: Key,
    /// Whether to play audio feedback tones.
    pub audio_feedback_enabled: bool,
    /// Whether to use TTS for command confirmation.
    pub tts_confirmation_enabled: bool,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            activation_mode: VoiceActivation::WakeWord,
            #[cfg(feature = "voice-control")]
            push_to_talk_key: DEFAULT_PUSH_TO_TALK_KEY,
            #[cfg(not(feature = "voice-control"))]
            push_to_talk_key: Key::F4,
            audio_feedback_enabled: true,
            tts_confirmation_enabled: true,
        }
    }
}

/// Response from rendering the voice settings panel.
#[derive(Debug, Default)]
pub struct VoiceSettingsResponse {
    /// Action triggered by user interaction.
    pub action: Option<VoiceSettingsAction>,
    /// Whether any setting was changed (triggers auto-save).
    pub settings_changed: bool,
}

/// Status of the voice model.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceModelStatus {
    /// Model state is unknown.
    Unknown,
    /// Model is not installed.
    NotInstalled,
    /// Model is currently downloading.
    Downloading {
        /// Progress percentage (0-100).
        progress_percent: u8,
        /// Bytes downloaded so far.
        bytes_received: u64,
        /// Total bytes to download.
        total_bytes: Option<u64>,
    },
    /// Model is being extracted.
    Extracting,
    /// Model is installed and ready.
    Ready,
    /// Model installation failed.
    Error {
        /// Error message.
        message: String,
    },
}

impl Default for VoiceModelStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[cfg(feature = "voice-control")]
impl From<ModelState> for VoiceModelStatus {
    fn from(state: ModelState) -> Self {
        match state {
            ModelState::Unknown => VoiceModelStatus::Unknown,
            ModelState::NotInstalled => VoiceModelStatus::NotInstalled,
            ModelState::Downloading { progress_percent } => VoiceModelStatus::Downloading {
                progress_percent,
                bytes_received: 0,
                total_bytes: None,
            },
            ModelState::Extracting => VoiceModelStatus::Extracting,
            ModelState::Ready => VoiceModelStatus::Ready,
            ModelState::Error => VoiceModelStatus::Error {
                message: "Model installation failed".to_string(),
            },
        }
    }
}

/// Configuration for the voice settings panel display.
#[derive(Debug, Clone)]
pub struct VoiceSettingsPanelConfig {
    /// Show section header and collapse button.
    pub show_header: bool,
    /// Whether the section is expanded (for collapsible mode).
    pub expanded: bool,
    /// Current model status for download progress.
    pub model_status: VoiceModelStatus,
    /// Whether voice-control feature is compiled in.
    pub feature_available: bool,
}

impl Default for VoiceSettingsPanelConfig {
    fn default() -> Self {
        Self {
            show_header: true,
            expanded: true,
            model_status: VoiceModelStatus::Unknown,
            #[cfg(feature = "voice-control")]
            feature_available: true,
            #[cfg(not(feature = "voice-control"))]
            feature_available: false,
        }
    }
}

/// Voice control settings panel for configuring voice recognition options.
///
/// Provides:
/// - Enable/disable toggle for voice control
/// - Activation mode selection (Always On, Wake Word, Push-to-Talk, Off)
/// - Push-to-talk key binding configuration
/// - Audio feedback and TTS confirmation toggles
/// - Model download button with progress indicator
pub struct VoiceSettingsPanel;

impl VoiceSettingsPanel {
    /// Render the voice settings panel.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `settings` - Mutable reference to the voice settings
    /// * `panel_config` - Panel display configuration
    ///
    /// # Returns
    /// Response containing any triggered action and whether settings changed
    pub fn show(
        ui: &mut Ui,
        settings: &mut VoiceSettings,
        panel_config: &VoiceSettingsPanelConfig,
    ) -> VoiceSettingsResponse {
        let mut response = VoiceSettingsResponse::default();
        let original_settings = settings.clone();

        if panel_config.show_header {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width() - 16.0);
                Self::render_header(ui, settings);

                if panel_config.expanded {
                    Self::render_content(ui, settings, panel_config, &mut response);
                }
            });
        } else {
            Self::render_content(ui, settings, panel_config, &mut response);
        }

        // Check if any settings changed
        response.settings_changed = *settings != original_settings;
        if response.settings_changed && response.action.is_none() {
            response.action = Some(VoiceSettingsAction::SettingsChanged(settings.clone()));
        }

        response
    }

    /// Render the section header with title and status.
    fn render_header(ui: &mut Ui, settings: &VoiceSettings) {
        ui.horizontal(|ui| {
            let icon = if settings.enabled { "🎤" } else { "🔇" };
            ui.label(RichText::new(format!("{} Voice Control Settings", icon)).size(18.0).strong());
        });
    }

    /// Render the main settings content.
    fn render_content(
        ui: &mut Ui,
        settings: &mut VoiceSettings,
        panel_config: &VoiceSettingsPanelConfig,
        response: &mut VoiceSettingsResponse,
    ) {
        ui.add_space(8.0);

        // Feature availability warning
        if !panel_config.feature_available {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").color(Color32::YELLOW));
                ui.label(
                    RichText::new("Voice control feature not compiled. Rebuild with --features voice-control")
                        .color(Color32::GRAY)
                        .italics(),
                );
            });
            ui.add_space(8.0);
        }

        // Voice control master toggle
        Self::render_enable_toggle(ui, settings, panel_config, response);

        // All other controls disabled when voice control is disabled or feature unavailable
        let controls_enabled = settings.enabled && panel_config.feature_available;
        ui.add_enabled_ui(controls_enabled, |ui| {
            ui.add_space(12.0);

            // Model status and download section
            Self::render_model_section(ui, panel_config, response);

            ui.add_space(16.0);

            // Activation mode selection
            Self::render_activation_mode(ui, settings, response);

            // Push-to-talk key binding (only if push-to-talk mode selected)
            if settings.activation_mode == VoiceActivation::PushToTalk {
                ui.add_space(12.0);
                Self::render_push_to_talk_key(ui, settings, response);
            }

            ui.add_space(16.0);

            // Feedback settings
            Self::render_feedback_settings(ui, settings, response);
        });
    }

    /// Render the enable/disable toggle.
    fn render_enable_toggle(
        ui: &mut Ui,
        settings: &mut VoiceSettings,
        panel_config: &VoiceSettingsPanelConfig,
        response: &mut VoiceSettingsResponse,
    ) {
        ui.horizontal(|ui| {
            let was_enabled = settings.enabled;

            ui.add_enabled_ui(panel_config.feature_available, |ui| {
                if ui
                    .checkbox(&mut settings.enabled, "Enable Voice Control")
                    .on_hover_text("Control workouts with voice commands like 'pause', 'resume', 'skip'")
                    .changed()
                {
                    response.action = Some(VoiceSettingsAction::ToggleVoiceControl(settings.enabled));
                }
            });

            if settings.enabled && !was_enabled {
                // Just enabled - may need to download model
            }
        });
    }

    /// Render the model status and download section.
    fn render_model_section(
        ui: &mut Ui,
        panel_config: &VoiceSettingsPanelConfig,
        response: &mut VoiceSettingsResponse,
    ) {
        ui.label(RichText::new("Voice Recognition Model").strong());
        ui.add_space(4.0);

        match &panel_config.model_status {
            VoiceModelStatus::Unknown => {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new("Checking...").color(Color32::GRAY).italics());
                });
            }
            VoiceModelStatus::NotInstalled => {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new("Not installed").color(Color32::YELLOW));
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("The Vosk speech recognition model (~50MB) needs to be downloaded for voice control to work.")
                        .color(Color32::GRAY)
                        .small(),
                );
                ui.add_space(8.0);
                if ui
                    .button("📥 Download Model")
                    .on_hover_text("Download the speech recognition model for offline voice control")
                    .clicked()
                {
                    response.action = Some(VoiceSettingsAction::DownloadModel);
                }
            }
            VoiceModelStatus::Downloading {
                progress_percent,
                bytes_received,
                total_bytes,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new("Downloading...").color(Color32::from_rgb(100, 200, 255)));
                });
                ui.add_space(4.0);

                // Progress bar
                let progress = *progress_percent as f32 / 100.0;
                ui.add(ProgressBar::new(progress).text(format!("{}%", progress_percent)));

                // Size info
                if let Some(total) = total_bytes {
                    ui.label(
                        RichText::new(format!(
                            "{} / {} downloaded",
                            format_bytes(*bytes_received),
                            format_bytes(*total)
                        ))
                        .color(Color32::GRAY)
                        .small(),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("{} downloaded", format_bytes(*bytes_received)))
                            .color(Color32::GRAY)
                            .small(),
                    );
                }

                ui.add_space(4.0);
                if ui
                    .button("Cancel")
                    .on_hover_text("Cancel the model download")
                    .clicked()
                {
                    response.action = Some(VoiceSettingsAction::CancelDownload);
                }
            }
            VoiceModelStatus::Extracting => {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new("Extracting...").color(Color32::from_rgb(100, 200, 255)));
                });
                ui.add_space(4.0);
                ui.add(ProgressBar::new(0.95).text("Extracting model files..."));
            }
            VoiceModelStatus::Ready => {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new("✓ Ready").color(Color32::from_rgb(100, 255, 100)));
                });
                ui.label(
                    RichText::new("Voice recognition model is installed and ready to use.")
                        .color(Color32::GRAY)
                        .small(),
                );
            }
            VoiceModelStatus::Error { message } => {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(RichText::new("✗ Error").color(Color32::from_rgb(255, 100, 100)));
                });
                ui.label(RichText::new(message).color(Color32::from_rgb(255, 150, 150)).small());
                ui.add_space(8.0);
                if ui
                    .button("🔄 Retry Download")
                    .on_hover_text("Try downloading the model again")
                    .clicked()
                {
                    response.action = Some(VoiceSettingsAction::DownloadModel);
                }
            }
        }
    }

    /// Render the activation mode selection.
    fn render_activation_mode(
        ui: &mut Ui,
        settings: &mut VoiceSettings,
        response: &mut VoiceSettingsResponse,
    ) {
        ui.label(RichText::new("Activation Mode").strong());
        ui.add_space(4.0);

        let modes = [
            (VoiceActivation::WakeWord, "Wake Word", "Say \"Hey Rust Ride\" or \"OK Ride\" to start listening"),
            (VoiceActivation::PushToTalk, "Push to Talk", "Hold a key to speak commands"),
            (VoiceActivation::AlwaysOn, "Always Listening", "Continuously listen for commands (uses more CPU)"),
            (VoiceActivation::Off, "Off", "Disable voice activation"),
        ];

        for (mode, label, tooltip) in modes {
            if ui
                .selectable_label(settings.activation_mode == mode, label)
                .on_hover_text(tooltip)
                .clicked()
            {
                settings.activation_mode = mode;
                response.action = Some(VoiceSettingsAction::ChangeActivationMode(mode));
            }
        }
    }

    /// Render the push-to-talk key binding selector.
    fn render_push_to_talk_key(
        ui: &mut Ui,
        settings: &mut VoiceSettings,
        response: &mut VoiceSettingsResponse,
    ) {
        ui.label(RichText::new("Push-to-Talk Key").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Key binding:");

            // Common key options for push-to-talk
            let key_options = [
                (Key::F4, "F4"),
                (Key::F5, "F5"),
                (Key::F6, "F6"),
                (Key::F7, "F7"),
                (Key::F8, "F8"),
                (Key::Space, "Space"),
                (Key::Tab, "Tab"),
            ];

            let current_key_name = key_display_name(settings.push_to_talk_key);
            egui::ComboBox::from_id_salt("ptt_key_selection")
                .selected_text(current_key_name)
                .show_ui(ui, |ui| {
                    for (key, name) in key_options {
                        if ui
                            .selectable_label(settings.push_to_talk_key == key, name)
                            .clicked()
                        {
                            settings.push_to_talk_key = key;
                            response.action = Some(VoiceSettingsAction::ChangePushToTalkKey(key));
                        }
                    }
                });
        });

        ui.label(
            RichText::new("Hold this key while speaking your command")
                .color(Color32::GRAY)
                .small(),
        );
    }

    /// Render the feedback settings section.
    fn render_feedback_settings(
        ui: &mut Ui,
        settings: &mut VoiceSettings,
        response: &mut VoiceSettingsResponse,
    ) {
        ui.label(RichText::new("Feedback Settings").strong());
        ui.add_space(4.0);

        // Audio feedback tones
        if ui
            .checkbox(&mut settings.audio_feedback_enabled, "Audio Feedback Tones")
            .on_hover_text("Play tones when wake word detected and commands recognized")
            .changed()
        {
            response.action = Some(VoiceSettingsAction::ToggleVoiceFeedback(settings.audio_feedback_enabled));
        }

        // TTS confirmation
        if ui
            .checkbox(&mut settings.tts_confirmation_enabled, "Speak Command Confirmation")
            .on_hover_text("Use text-to-speech to confirm recognized commands (e.g., 'Pausing')")
            .changed()
        {
            response.action = Some(VoiceSettingsAction::ToggleTtsConfirmation(settings.tts_confirmation_enabled));
        }
    }

    /// Render a compact voice status for embedding in other UIs.
    ///
    /// Shows just the voice control toggle and activation mode.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `enabled` - Current enabled state
    /// * `mode` - Current activation mode
    ///
    /// # Returns
    /// Tuple of (new_enabled, new_mode) if changed, None otherwise
    pub fn compact_status(
        ui: &mut Ui,
        enabled: bool,
        mode: VoiceActivation,
    ) -> Option<(bool, VoiceActivation)> {
        let mut new_enabled = enabled;
        let mut new_mode = mode;
        let mut changed = false;

        ui.horizontal(|ui| {
            let icon = if enabled { "🎤" } else { "🔇" };
            if ui.button(icon).on_hover_text(if enabled { "Disable voice control" } else { "Enable voice control" }).clicked() {
                new_enabled = !enabled;
                changed = true;
            }

            if enabled {
                ui.label(format!("Voice: {}", mode));
            } else {
                ui.label(RichText::new("Voice: Off").color(Color32::GRAY));
            }
        });

        if changed {
            Some((new_enabled, new_mode))
        } else {
            None
        }
    }
}

/// Format bytes as human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Get a display name for a key.
fn key_display_name(key: Key) -> &'static str {
    match key {
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Tab => "Tab",
        Key::Enter => "Enter",
        Key::Escape => "Escape",
        Key::Space => "Space",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_settings_action_eq() {
        let settings = VoiceSettings::default();
        let action1 = VoiceSettingsAction::SettingsChanged(settings.clone());
        let action2 = VoiceSettingsAction::SettingsChanged(settings);
        assert_eq!(action1, action2);

        assert_eq!(
            VoiceSettingsAction::DownloadModel,
            VoiceSettingsAction::DownloadModel
        );

        assert_eq!(
            VoiceSettingsAction::ToggleVoiceControl(true),
            VoiceSettingsAction::ToggleVoiceControl(true)
        );
    }

    #[test]
    fn test_voice_settings_default() {
        let settings = VoiceSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.activation_mode, VoiceActivation::WakeWord);
        assert_eq!(settings.push_to_talk_key, Key::F4);
        assert!(settings.audio_feedback_enabled);
        assert!(settings.tts_confirmation_enabled);
    }

    #[test]
    fn test_voice_settings_response_default() {
        let response = VoiceSettingsResponse::default();
        assert!(response.action.is_none());
        assert!(!response.settings_changed);
    }

    #[test]
    fn test_panel_config_default() {
        let config = VoiceSettingsPanelConfig::default();
        assert!(config.show_header);
        assert!(config.expanded);
        assert_eq!(config.model_status, VoiceModelStatus::Unknown);
    }

    #[test]
    fn test_voice_model_status_default() {
        let status = VoiceModelStatus::default();
        assert_eq!(status, VoiceModelStatus::Unknown);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(50 * 1024 * 1024), "50.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_key_display_name() {
        assert_eq!(key_display_name(Key::F4), "F4");
        assert_eq!(key_display_name(Key::Space), "Space");
        assert_eq!(key_display_name(Key::Tab), "Tab");
        assert_eq!(key_display_name(Key::A), "A");
        assert_eq!(key_display_name(Key::Num0), "0");
    }

    #[test]
    fn test_voice_activation_modes() {
        // Test all modes are available
        let modes = [
            VoiceActivation::WakeWord,
            VoiceActivation::PushToTalk,
            VoiceActivation::AlwaysOn,
            VoiceActivation::Off,
        ];

        for mode in modes {
            let settings = VoiceSettings {
                activation_mode: mode,
                ..Default::default()
            };
            assert_eq!(settings.activation_mode, mode);
        }
    }

    #[test]
    fn test_voice_settings_clone() {
        let settings = VoiceSettings {
            enabled: true,
            activation_mode: VoiceActivation::PushToTalk,
            push_to_talk_key: Key::F5,
            audio_feedback_enabled: false,
            tts_confirmation_enabled: true,
        };

        let cloned = settings.clone();
        assert_eq!(settings, cloned);
    }

    #[test]
    fn test_voice_model_status_variants() {
        let unknown = VoiceModelStatus::Unknown;
        assert_eq!(unknown, VoiceModelStatus::Unknown);

        let not_installed = VoiceModelStatus::NotInstalled;
        assert_eq!(not_installed, VoiceModelStatus::NotInstalled);

        let downloading = VoiceModelStatus::Downloading {
            progress_percent: 50,
            bytes_received: 1024,
            total_bytes: Some(2048),
        };
        if let VoiceModelStatus::Downloading { progress_percent, bytes_received, total_bytes } = downloading {
            assert_eq!(progress_percent, 50);
            assert_eq!(bytes_received, 1024);
            assert_eq!(total_bytes, Some(2048));
        }

        let extracting = VoiceModelStatus::Extracting;
        assert_eq!(extracting, VoiceModelStatus::Extracting);

        let ready = VoiceModelStatus::Ready;
        assert_eq!(ready, VoiceModelStatus::Ready);

        let error = VoiceModelStatus::Error {
            message: "Test error".to_string(),
        };
        if let VoiceModelStatus::Error { message } = error {
            assert_eq!(message, "Test error");
        }
    }
}
