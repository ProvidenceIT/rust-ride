//! Audio settings UI panel widget.
//!
//! T064: Add audio alert settings to settings screen.
//! Provides sliders for volume categories, toggles for audio features,
//! and test buttons to preview sounds.

use egui::{Align, Color32, Layout, RichText, Slider, Ui};

use crate::audio::{AudioCategory, AudioConfig, CuePattern, MuteState, VoiceInfo};

/// Actions that can be triggered from the audio settings panel.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioSettingsAction {
    /// Audio settings were modified
    SettingsChanged(AudioConfig),
    /// User requested to test/preview a sound type
    TestSound(AudioTestType),
    /// User toggled global mute
    ToggleMute,
    /// User toggled category mute
    ToggleCategoryMute(AudioCategory),
}

/// Types of audio that can be tested/previewed.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioTestType {
    /// Test voice with current settings
    Voice {
        voice_id: Option<String>,
        volume: f32,
        rate: f32,
    },
    /// Test countdown sounds
    Countdown,
    /// Test achievement chime (with tier)
    Achievement { tier: String },
    /// Test milestone sound
    Milestone,
    /// Test a specific tone pattern
    TonePattern(CuePattern),
}

/// Response from rendering the audio settings panel.
#[derive(Debug, Default)]
pub struct AudioSettingsResponse {
    /// Action triggered by user interaction
    pub action: Option<AudioSettingsAction>,
    /// Whether any setting was changed (triggers auto-save)
    pub settings_changed: bool,
}

/// Configuration for the audio settings panel display.
#[derive(Debug, Clone)]
pub struct AudioSettingsPanelConfig {
    /// Show detailed/advanced options (expanded mode)
    pub show_advanced: bool,
    /// Available system voices for TTS selection
    pub available_voices: Vec<VoiceInfo>,
    /// Show section header and collapse button
    pub show_header: bool,
    /// Whether the section is expanded (for collapsible mode)
    pub expanded: bool,
}

impl Default for AudioSettingsPanelConfig {
    fn default() -> Self {
        Self {
            show_advanced: false,
            available_voices: Vec::new(),
            show_header: true,
            expanded: true,
        }
    }
}

/// Audio settings panel for configuring audio playback options.
///
/// Provides:
/// - Volume sliders for each audio category (master, voice, effects, countdown, achievement, milestone)
/// - Toggles for enabling/disabling audio features
/// - Mute controls (global and per-category)
/// - Test buttons to preview sounds
pub struct AudioSettingsPanel;

impl AudioSettingsPanel {
    /// Render the audio settings panel.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `config` - Mutable reference to the audio configuration
    /// * `panel_config` - Panel display configuration (advanced mode, available voices, etc.)
    ///
    /// # Returns
    /// Response containing any triggered action and whether settings changed
    pub fn show(
        ui: &mut Ui,
        config: &mut AudioConfig,
        panel_config: &AudioSettingsPanelConfig,
    ) -> AudioSettingsResponse {
        let mut response = AudioSettingsResponse::default();
        let original_config = config.clone();

        if panel_config.show_header {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width() - 16.0);
                Self::render_header(ui, config, &mut response);

                if panel_config.expanded {
                    Self::render_content(ui, config, panel_config, &mut response);
                }
            });
        } else {
            Self::render_content(ui, config, panel_config, &mut response);
        }

        // Check if any settings changed
        response.settings_changed = Self::settings_differ(&original_config, config);
        if response.settings_changed && response.action.is_none() {
            response.action = Some(AudioSettingsAction::SettingsChanged(config.clone()));
        }

        response
    }

    /// Render the section header with title and mute status.
    fn render_header(ui: &mut Ui, config: &AudioConfig, response: &mut AudioSettingsResponse) {
        ui.horizontal(|ui| {
            // Title with mute icon
            let mute_state = MuteState::from_config(config);
            let icon = if mute_state.globally_muted {
                "🔇"
            } else if mute_state.any_muted() {
                "🔉"
            } else {
                "🔊"
            };

            ui.label(RichText::new(format!("{} Audio Settings", icon)).size(18.0).strong());

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Global mute toggle
                let mute_text = if config.muted { "Unmute All" } else { "Mute All" };
                if ui.button(mute_text).clicked() {
                    config.muted = !config.muted;
                    response.action = Some(AudioSettingsAction::ToggleMute);
                }
            });
        });
    }

    /// Render the main settings content.
    fn render_content(
        ui: &mut Ui,
        config: &mut AudioConfig,
        panel_config: &AudioSettingsPanelConfig,
        response: &mut AudioSettingsResponse,
    ) {
        ui.add_space(8.0);

        // Master audio toggle
        ui.horizontal(|ui| {
            ui.checkbox(&mut config.enabled, "Enable Audio");
            if config.muted {
                ui.label(RichText::new("(Muted)").color(Color32::GRAY).italics());
            }
        });

        // All controls are disabled when audio is disabled or globally muted
        ui.add_enabled_ui(config.enabled && !config.muted, |ui| {
            ui.add_space(12.0);

            // Volume sliders section
            Self::render_volume_section(ui, config, panel_config, response);

            ui.add_space(16.0);

            // Feature toggles section
            Self::render_feature_toggles(ui, config);

            ui.add_space(16.0);

            // Test sounds section
            Self::render_test_section(ui, config, panel_config, response);

            // Advanced options (if enabled)
            if panel_config.show_advanced {
                ui.add_space(16.0);
                Self::render_advanced_section(ui, config, panel_config, response);
            }
        });
    }

    /// Render volume sliders for each audio category.
    fn render_volume_section(
        ui: &mut Ui,
        config: &mut AudioConfig,
        panel_config: &AudioSettingsPanelConfig,
        response: &mut AudioSettingsResponse,
    ) {
        ui.label(RichText::new("Volume Levels").strong());
        ui.add_space(4.0);

        // Master volume
        Self::render_volume_slider(
            ui,
            "Master Volume",
            &mut config.volume,
            None,
            "Controls overall audio loudness",
        );

        ui.add_space(8.0);

        // Category volumes in a grid
        egui::Grid::new("audio_volume_grid")
            .num_columns(3)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                // Voice volume
                Self::render_category_volume_row(
                    ui,
                    "Voice",
                    &mut config.voice_volume,
                    &mut config.voice_muted,
                    config.voice_enabled,
                    AudioCategory::Voice,
                    response,
                );
                ui.end_row();

                // Sound effects volume
                Self::render_category_volume_row(
                    ui,
                    "Sound Effects",
                    &mut config.sound_effects_volume,
                    &mut config.sound_effects_muted,
                    config.sound_effects_enabled,
                    AudioCategory::SoundEffect,
                    response,
                );
                ui.end_row();

                // Countdown volume
                Self::render_category_volume_row(
                    ui,
                    "Countdown",
                    &mut config.countdown_volume,
                    &mut config.countdown_muted,
                    config.countdown_enabled,
                    AudioCategory::Countdown,
                    response,
                );
                ui.end_row();

                // Achievement volume
                Self::render_category_volume_row(
                    ui,
                    "Achievement",
                    &mut config.achievement_volume,
                    &mut config.achievement_muted,
                    config.achievements_enabled,
                    AudioCategory::Achievement,
                    response,
                );
                ui.end_row();

                // Milestone volume
                Self::render_category_volume_row(
                    ui,
                    "Milestone",
                    &mut config.milestone_volume,
                    &mut config.milestone_muted,
                    config.milestones_enabled,
                    AudioCategory::Milestone,
                    response,
                );
                ui.end_row();
            });

        // Show effective volume hint
        if panel_config.show_advanced {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Note: Category volumes are multiplied by master volume")
                    .weak()
                    .size(11.0),
            );
        }
    }

    /// Render a volume slider with optional tooltip.
    fn render_volume_slider(
        ui: &mut Ui,
        label: &str,
        volume: &mut u8,
        muted: Option<bool>,
        tooltip: &str,
    ) {
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));

            // Show mute indicator if muted
            if let Some(true) = muted {
                ui.label(RichText::new("🔇").color(Color32::GRAY));
            }

            let mut volume_f32 = *volume as f32;
            let slider = Slider::new(&mut volume_f32, 0.0..=100.0)
                .suffix("%")
                .show_value(true);

            if ui.add(slider).on_hover_text(tooltip).changed() {
                *volume = volume_f32 as u8;
            }
        });
    }

    /// Render a volume row for a category with mute toggle.
    fn render_category_volume_row(
        ui: &mut Ui,
        label: &str,
        volume: &mut u8,
        muted: &mut bool,
        enabled: bool,
        category: AudioCategory,
        response: &mut AudioSettingsResponse,
    ) {
        // Label with enabled status
        let label_text = if enabled {
            RichText::new(format!("{}:", label))
        } else {
            RichText::new(format!("{}:", label)).color(Color32::GRAY)
        };
        ui.label(label_text);

        // Volume slider (disabled if category not enabled or muted)
        ui.add_enabled_ui(enabled && !*muted, |ui| {
            let mut volume_f32 = *volume as f32;
            let slider = Slider::new(&mut volume_f32, 0.0..=100.0)
                .suffix("%")
                .show_value(true);

            if ui.add(slider).changed() {
                *volume = volume_f32 as u8;
            }
        });

        // Mute toggle button
        let mute_icon = if *muted { "🔇" } else { "🔊" };
        let mute_tooltip = if *muted {
            format!("Unmute {}", label.to_lowercase())
        } else {
            format!("Mute {}", label.to_lowercase())
        };

        if ui.button(mute_icon).on_hover_text(mute_tooltip).clicked() {
            *muted = !*muted;
            response.action = Some(AudioSettingsAction::ToggleCategoryMute(category));
        }
    }

    /// Render toggles for audio features.
    fn render_feature_toggles(ui: &mut Ui, config: &mut AudioConfig) {
        ui.label(RichText::new("Audio Features").strong());
        ui.add_space(4.0);

        egui::Grid::new("audio_features_grid")
            .num_columns(2)
            .spacing([32.0, 4.0])
            .show(ui, |ui| {
                // Voice announcements
                ui.checkbox(&mut config.voice_enabled, "Voice Announcements")
                    .on_hover_text("Spoken alerts during workouts and rides");

                // Sound effects
                ui.checkbox(&mut config.sound_effects_enabled, "Sound Effects")
                    .on_hover_text("Audio feedback for actions and events");
                ui.end_row();

                // Countdown sounds
                ui.checkbox(&mut config.countdown_enabled, "Countdown Sounds")
                    .on_hover_text("Audio countdown before interval changes");

                // Achievement chimes
                ui.checkbox(&mut config.achievements_enabled, "Achievement Chimes")
                    .on_hover_text("Celebratory sounds when unlocking achievements");
                ui.end_row();

                // Milestone sounds
                ui.checkbox(&mut config.milestones_enabled, "Milestone Sounds")
                    .on_hover_text("Subtle chimes for distance/time/calorie milestones");

                // Personal record sounds
                ui.checkbox(&mut config.personal_record_sounds_enabled, "Personal Record Sounds")
                    .on_hover_text("Celebratory sounds when setting personal records");
                ui.end_row();
            });
    }

    /// Render test/preview buttons for each audio type.
    fn render_test_section(
        ui: &mut Ui,
        config: &AudioConfig,
        panel_config: &AudioSettingsPanelConfig,
        response: &mut AudioSettingsResponse,
    ) {
        ui.label(RichText::new("Test Sounds").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Test voice
            ui.add_enabled_ui(config.voice_enabled, |ui| {
                if ui
                    .button("🗣 Test Voice")
                    .on_hover_text("Preview voice announcement")
                    .clicked()
                {
                    response.action = Some(AudioSettingsAction::TestSound(AudioTestType::Voice {
                        voice_id: config.preferred_voice.clone(),
                        volume: config.voice_volume as f32 / 100.0,
                        rate: config.speech_rate,
                    }));
                }
            });

            // Test countdown
            ui.add_enabled_ui(config.countdown_enabled, |ui| {
                if ui
                    .button("⏱ Test Countdown")
                    .on_hover_text("Preview countdown sounds")
                    .clicked()
                {
                    response.action = Some(AudioSettingsAction::TestSound(AudioTestType::Countdown));
                }
            });
        });

        ui.horizontal(|ui| {
            // Test achievement (with tier selector if advanced mode)
            ui.add_enabled_ui(config.achievements_enabled, |ui| {
                if panel_config.show_advanced {
                    // Show individual tier buttons
                    for tier in &["Bronze", "Silver", "Gold", "Platinum"] {
                        if ui
                            .button(format!("🏆 {}", tier))
                            .on_hover_text(format!("Preview {} achievement chime", tier.to_lowercase()))
                            .clicked()
                        {
                            response.action = Some(AudioSettingsAction::TestSound(
                                AudioTestType::Achievement {
                                    tier: tier.to_string(),
                                },
                            ));
                        }
                    }
                } else {
                    // Simple test button
                    if ui
                        .button("🏆 Test Achievement")
                        .on_hover_text("Preview achievement chime")
                        .clicked()
                    {
                        response.action = Some(AudioSettingsAction::TestSound(
                            AudioTestType::Achievement {
                                tier: "Gold".to_string(),
                            },
                        ));
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            // Test milestone
            ui.add_enabled_ui(config.milestones_enabled, |ui| {
                if ui
                    .button("🎯 Test Milestone")
                    .on_hover_text("Preview milestone sound")
                    .clicked()
                {
                    response.action = Some(AudioSettingsAction::TestSound(AudioTestType::Milestone));
                }
            });

            // Test personal record
            ui.add_enabled_ui(config.personal_record_sounds_enabled, |ui| {
                if ui
                    .button("⭐ Test Personal Record")
                    .on_hover_text("Preview personal record fanfare")
                    .clicked()
                {
                    response.action = Some(AudioSettingsAction::TestSound(
                        AudioTestType::TonePattern(CuePattern::PersonalRecord),
                    ));
                }
            });
        });
    }

    /// Render advanced settings section.
    fn render_advanced_section(
        ui: &mut Ui,
        config: &mut AudioConfig,
        panel_config: &AudioSettingsPanelConfig,
        response: &mut AudioSettingsResponse,
    ) {
        ui.collapsing("Advanced Audio Settings", |ui| {
            // Speech rate slider
            ui.horizontal(|ui| {
                ui.label("Speech Rate:");
                let slider = Slider::new(&mut config.speech_rate, 0.5..=2.0)
                    .show_value(true)
                    .custom_formatter(|v, _| format!("{:.1}x", v));
                ui.add(slider)
                    .on_hover_text("Speed of voice announcements (1.0x = normal)");
            });

            ui.add_space(8.0);

            // Voice selection dropdown (if voices available)
            if !panel_config.available_voices.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Voice:");

                    // Get selected voice display text
                    let selected_text = if let Some(ref voice_id) = config.preferred_voice {
                        panel_config
                            .available_voices
                            .iter()
                            .find(|v| &v.id == voice_id)
                            .map(|v| format!("{} ({})", v.name, v.language))
                            .unwrap_or_else(|| voice_id.clone())
                    } else {
                        panel_config
                            .available_voices
                            .iter()
                            .find(|v| v.is_default)
                            .map(|v| format!("{} ({})", v.name, v.language))
                            .unwrap_or_else(|| "System Default".to_string())
                    };

                    egui::ComboBox::from_id_salt("audio_voice_selection")
                        .selected_text(&selected_text)
                        .width(250.0)
                        .show_ui(ui, |ui| {
                            // System Default option
                            if ui
                                .selectable_label(config.preferred_voice.is_none(), "System Default")
                                .clicked()
                            {
                                config.preferred_voice = None;
                            }

                            ui.separator();

                            // Available voices
                            for voice in &panel_config.available_voices {
                                let is_selected = config
                                    .preferred_voice
                                    .as_ref()
                                    .map(|id| id == &voice.id)
                                    .unwrap_or(false);

                                let label = format!("{} ({})", voice.name, voice.language);
                                if ui.selectable_label(is_selected, &label).clicked() {
                                    config.preferred_voice = Some(voice.id.clone());
                                }
                            }
                        });
                });
            }

            ui.add_space(8.0);

            // Alert interval setting
            ui.horizontal(|ui| {
                ui.label("Min Alert Interval:");
                let mut interval_secs = config.min_alert_interval_ms as f32 / 1000.0;
                let slider = Slider::new(&mut interval_secs, 0.5..=10.0)
                    .suffix("s")
                    .show_value(true);
                if ui
                    .add(slider)
                    .on_hover_text("Minimum time between audio alerts to prevent spam")
                    .changed()
                {
                    config.min_alert_interval_ms = (interval_secs * 1000.0) as u32;
                }
            });

            ui.add_space(8.0);

            // Per-category mute status display
            ui.label(RichText::new("Category Status").strong());
            let mute_state = MuteState::from_config(config);
            ui.horizontal(|ui| {
                for category in AudioCategory::all() {
                    let is_muted = mute_state.is_category_muted(*category);
                    let icon = if is_muted { "🔇" } else { "🔊" };
                    let color = if is_muted {
                        Color32::GRAY
                    } else {
                        Color32::WHITE
                    };
                    ui.label(
                        RichText::new(format!("{} {}", icon, category.display_name()))
                            .color(color)
                            .size(11.0),
                    );
                }
            });
        });
    }

    /// Compare two configs to detect changes.
    fn settings_differ(a: &AudioConfig, b: &AudioConfig) -> bool {
        a.enabled != b.enabled
            || a.volume != b.volume
            || a.voice_enabled != b.voice_enabled
            || a.voice_volume != b.voice_volume
            || a.preferred_voice != b.preferred_voice
            || (a.speech_rate - b.speech_rate).abs() > 0.01
            || a.sound_effects_enabled != b.sound_effects_enabled
            || a.sound_effects_volume != b.sound_effects_volume
            || a.min_alert_interval_ms != b.min_alert_interval_ms
            || a.countdown_enabled != b.countdown_enabled
            || a.countdown_volume != b.countdown_volume
            || a.milestones_enabled != b.milestones_enabled
            || a.milestone_volume != b.milestone_volume
            || a.personal_record_sounds_enabled != b.personal_record_sounds_enabled
            || a.achievements_enabled != b.achievements_enabled
            || a.achievement_volume != b.achievement_volume
            || a.muted != b.muted
            || a.voice_muted != b.voice_muted
            || a.sound_effects_muted != b.sound_effects_muted
            || a.countdown_muted != b.countdown_muted
            || a.achievement_muted != b.achievement_muted
            || a.milestone_muted != b.milestone_muted
    }

    /// Render a compact volume control for embedding in other UIs.
    ///
    /// Shows just the master volume slider with mute button.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `volume` - Current master volume (0-100)
    /// * `muted` - Current mute state
    ///
    /// # Returns
    /// Tuple of (new_volume, new_muted) if changed, None otherwise
    pub fn compact_volume(ui: &mut Ui, volume: u8, muted: bool) -> Option<(u8, bool)> {
        let mut new_volume = volume;
        let mut new_muted = muted;
        let mut changed = false;

        ui.horizontal(|ui| {
            let icon = if muted { "🔇" } else { "🔊" };
            if ui.button(icon).clicked() {
                new_muted = !muted;
                changed = true;
            }

            ui.add_enabled_ui(!muted, |ui| {
                let mut volume_f32 = volume as f32;
                if ui
                    .add(
                        Slider::new(&mut volume_f32, 0.0..=100.0)
                            .show_value(false)
                            .clamp_to_range(true),
                    )
                    .changed()
                {
                    new_volume = volume_f32 as u8;
                    changed = true;
                }
            });

            ui.label(format!("{}%", if muted { 0 } else { volume }));
        });

        if changed {
            Some((new_volume, new_muted))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_settings_action_eq() {
        let config = AudioConfig::default();
        let action1 = AudioSettingsAction::SettingsChanged(config.clone());
        let action2 = AudioSettingsAction::SettingsChanged(config);
        assert_eq!(action1, action2);

        assert_eq!(
            AudioSettingsAction::ToggleMute,
            AudioSettingsAction::ToggleMute
        );
    }

    #[test]
    fn test_audio_test_type_eq() {
        let test1 = AudioTestType::Countdown;
        let test2 = AudioTestType::Countdown;
        assert_eq!(test1, test2);

        let test3 = AudioTestType::Achievement {
            tier: "Gold".to_string(),
        };
        let test4 = AudioTestType::Achievement {
            tier: "Gold".to_string(),
        };
        assert_eq!(test3, test4);
    }

    #[test]
    fn test_audio_settings_response_default() {
        let response = AudioSettingsResponse::default();
        assert!(response.action.is_none());
        assert!(!response.settings_changed);
    }

    #[test]
    fn test_panel_config_default() {
        let config = AudioSettingsPanelConfig::default();
        assert!(!config.show_advanced);
        assert!(config.available_voices.is_empty());
        assert!(config.show_header);
        assert!(config.expanded);
    }

    #[test]
    fn test_settings_differ_volume() {
        let config1 = AudioConfig::default();
        let mut config2 = AudioConfig::default();

        assert!(!AudioSettingsPanel::settings_differ(&config1, &config2));

        config2.volume = 50;
        assert!(AudioSettingsPanel::settings_differ(&config1, &config2));
    }

    #[test]
    fn test_settings_differ_mute() {
        let config1 = AudioConfig::default();
        let mut config2 = AudioConfig::default();

        config2.muted = true;
        assert!(AudioSettingsPanel::settings_differ(&config1, &config2));

        config2.muted = false;
        config2.voice_muted = true;
        assert!(AudioSettingsPanel::settings_differ(&config1, &config2));
    }

    #[test]
    fn test_settings_differ_enabled() {
        let config1 = AudioConfig::default();
        let mut config2 = AudioConfig::default();

        config2.countdown_enabled = false;
        assert!(AudioSettingsPanel::settings_differ(&config1, &config2));
    }
}
