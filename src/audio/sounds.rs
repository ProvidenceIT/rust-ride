//! Sound Asset Catalog
//!
//! Defines all available sound assets for the audio engine with fallback
//! to generated tones when audio files are not available.

use std::path::PathBuf;
use std::time::Duration;

use super::tones::{frequencies, CuePattern, Tone};

/// Sound asset identifier
///
/// Each variant represents a named sound effect that can be played
/// by the audio engine. When audio files are available, they will be
/// loaded from the assets/sounds directory. Otherwise, a generated
/// tone sequence will be used as a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundAsset {
    // Countdown sounds
    /// Countdown tick (played at each second during countdown)
    CountdownTick,
    /// Final 3 seconds countdown beep (slightly higher pitch)
    CountdownFinal3,
    /// Final 2 seconds countdown beep (higher pitch)
    CountdownFinal2,
    /// Final 1 second countdown beep (highest pitch, most urgent)
    CountdownFinal1,
    /// Countdown complete / GO! sound
    CountdownGo,

    // Interval sounds
    /// Interval is about to change (warning tone)
    IntervalWarning,
    /// Interval has changed (transition sound)
    IntervalChange,
    /// Rest interval started (calming tone)
    IntervalRest,
    /// Work interval started (energetic tone)
    IntervalWork,

    // Achievement sounds
    /// Bronze achievement unlocked
    AchievementBronze,
    /// Silver achievement unlocked
    AchievementSilver,
    /// Gold achievement unlocked
    AchievementGold,
    /// Platinum achievement unlocked (rare, special sound)
    AchievementPlatinum,
    /// Level up notification
    LevelUp,

    // Milestone sounds
    /// Distance milestone reached (5km, 10km, etc.)
    MilestoneDistance,
    /// Time milestone reached (15min, 30min, etc.)
    MilestoneTime,
    /// Calorie milestone reached
    MilestoneCalories,
    /// Personal record achieved
    PersonalRecord,

    // Workout lifecycle sounds
    /// Workout is starting
    WorkoutStart,
    /// Workout paused
    WorkoutPause,
    /// Workout resumed
    WorkoutResume,
    /// Workout completed successfully
    WorkoutComplete,
    /// Workout cancelled/stopped early
    WorkoutStop,

    // Zone change sounds
    /// Entering a higher power zone
    ZoneUp,
    /// Entering a lower power zone
    ZoneDown,

    // Alert sounds
    /// General notification
    Notification,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Success confirmation
    Success,

    // Connection sounds
    /// Device connected
    DeviceConnected,
    /// Device disconnected
    DeviceDisconnected,
}

impl SoundAsset {
    /// Get all available sound assets
    pub fn all() -> &'static [SoundAsset] {
        &[
            // Countdown
            SoundAsset::CountdownTick,
            SoundAsset::CountdownFinal3,
            SoundAsset::CountdownFinal2,
            SoundAsset::CountdownFinal1,
            SoundAsset::CountdownGo,
            // Intervals
            SoundAsset::IntervalWarning,
            SoundAsset::IntervalChange,
            SoundAsset::IntervalRest,
            SoundAsset::IntervalWork,
            // Achievements
            SoundAsset::AchievementBronze,
            SoundAsset::AchievementSilver,
            SoundAsset::AchievementGold,
            SoundAsset::AchievementPlatinum,
            SoundAsset::LevelUp,
            // Milestones
            SoundAsset::MilestoneDistance,
            SoundAsset::MilestoneTime,
            SoundAsset::MilestoneCalories,
            SoundAsset::PersonalRecord,
            // Workout lifecycle
            SoundAsset::WorkoutStart,
            SoundAsset::WorkoutPause,
            SoundAsset::WorkoutResume,
            SoundAsset::WorkoutComplete,
            SoundAsset::WorkoutStop,
            // Zone changes
            SoundAsset::ZoneUp,
            SoundAsset::ZoneDown,
            // Alerts
            SoundAsset::Notification,
            SoundAsset::Warning,
            SoundAsset::Error,
            SoundAsset::Success,
            // Connections
            SoundAsset::DeviceConnected,
            SoundAsset::DeviceDisconnected,
        ]
    }

    /// Get the canonical name for this sound (used for file lookup)
    pub fn name(&self) -> &'static str {
        match self {
            // Countdown
            SoundAsset::CountdownTick => "countdown_tick",
            SoundAsset::CountdownFinal3 => "countdown_3",
            SoundAsset::CountdownFinal2 => "countdown_2",
            SoundAsset::CountdownFinal1 => "countdown_1",
            SoundAsset::CountdownGo => "countdown_go",
            // Intervals
            SoundAsset::IntervalWarning => "interval_warning",
            SoundAsset::IntervalChange => "interval_change",
            SoundAsset::IntervalRest => "interval_rest",
            SoundAsset::IntervalWork => "interval_work",
            // Achievements
            SoundAsset::AchievementBronze => "achievement_bronze",
            SoundAsset::AchievementSilver => "achievement_silver",
            SoundAsset::AchievementGold => "achievement_gold",
            SoundAsset::AchievementPlatinum => "achievement_platinum",
            SoundAsset::LevelUp => "level_up",
            // Milestones
            SoundAsset::MilestoneDistance => "milestone_distance",
            SoundAsset::MilestoneTime => "milestone_time",
            SoundAsset::MilestoneCalories => "milestone_calories",
            SoundAsset::PersonalRecord => "personal_record",
            // Workout lifecycle
            SoundAsset::WorkoutStart => "workout_start",
            SoundAsset::WorkoutPause => "workout_pause",
            SoundAsset::WorkoutResume => "workout_resume",
            SoundAsset::WorkoutComplete => "workout_complete",
            SoundAsset::WorkoutStop => "workout_stop",
            // Zone changes
            SoundAsset::ZoneUp => "zone_up",
            SoundAsset::ZoneDown => "zone_down",
            // Alerts
            SoundAsset::Notification => "notification",
            SoundAsset::Warning => "warning",
            SoundAsset::Error => "error",
            SoundAsset::Success => "success",
            // Connections
            SoundAsset::DeviceConnected => "device_connected",
            SoundAsset::DeviceDisconnected => "device_disconnected",
        }
    }

    /// Get the expected file path for this sound asset
    pub fn file_path(&self) -> PathBuf {
        PathBuf::from("assets/sounds").join(format!("{}.wav", self.name()))
    }

    /// Get the category this sound belongs to
    pub fn category(&self) -> SoundCategory {
        match self {
            SoundAsset::CountdownTick
            | SoundAsset::CountdownFinal3
            | SoundAsset::CountdownFinal2
            | SoundAsset::CountdownFinal1
            | SoundAsset::CountdownGo => SoundCategory::Countdown,

            SoundAsset::IntervalWarning
            | SoundAsset::IntervalChange
            | SoundAsset::IntervalRest
            | SoundAsset::IntervalWork => SoundCategory::Interval,

            SoundAsset::AchievementBronze
            | SoundAsset::AchievementSilver
            | SoundAsset::AchievementGold
            | SoundAsset::AchievementPlatinum
            | SoundAsset::LevelUp => SoundCategory::Achievement,

            SoundAsset::MilestoneDistance
            | SoundAsset::MilestoneTime
            | SoundAsset::MilestoneCalories
            | SoundAsset::PersonalRecord => SoundCategory::Milestone,

            SoundAsset::WorkoutStart
            | SoundAsset::WorkoutPause
            | SoundAsset::WorkoutResume
            | SoundAsset::WorkoutComplete
            | SoundAsset::WorkoutStop => SoundCategory::Workout,

            SoundAsset::ZoneUp | SoundAsset::ZoneDown => SoundCategory::Zone,

            SoundAsset::Notification
            | SoundAsset::Warning
            | SoundAsset::Error
            | SoundAsset::Success => SoundCategory::Alert,

            SoundAsset::DeviceConnected | SoundAsset::DeviceDisconnected => {
                SoundCategory::Connection
            }
        }
    }

    /// Get the fallback tone sequence when no audio file is available
    pub fn fallback_tones(&self) -> Vec<Tone> {
        match self {
            // Countdown tones - short, distinct beeps with increasing urgency
            SoundAsset::CountdownTick => vec![Tone::new(frequencies::MEDIUM, 50)],

            SoundAsset::CountdownFinal3 => vec![Tone::new(523.25, 80)], // C5

            SoundAsset::CountdownFinal2 => vec![Tone::new(587.33, 80)], // D5

            SoundAsset::CountdownFinal1 => vec![Tone::new(659.25, 100)], // E5

            SoundAsset::CountdownGo => vec![
                Tone::new(frequencies::HIGH, 100),
                Tone::pause(50),
                Tone::new(frequencies::VERY_HIGH, 200),
            ],

            // Interval tones
            SoundAsset::IntervalWarning => vec![
                Tone::new(frequencies::ALERT, 50),
                Tone::pause(50),
                Tone::new(frequencies::ALERT, 50),
            ],

            SoundAsset::IntervalChange => CuePattern::DoubleBeep.tones(),

            SoundAsset::IntervalRest => vec![
                Tone::new(frequencies::HIGH, 100),
                Tone::pause(30),
                Tone::new(frequencies::MEDIUM, 100),
                Tone::pause(30),
                Tone::new(frequencies::LOW, 200),
            ],

            SoundAsset::IntervalWork => vec![
                Tone::new(frequencies::LOW, 100),
                Tone::pause(30),
                Tone::new(frequencies::MEDIUM, 100),
                Tone::pause(30),
                Tone::new(frequencies::HIGH, 200),
            ],

            // Achievement tones - increasingly elaborate
            SoundAsset::AchievementBronze => vec![
                Tone::new(523.25, 100), // C5
                Tone::pause(50),
                Tone::new(659.25, 150), // E5
            ],

            SoundAsset::AchievementSilver => vec![
                Tone::new(523.25, 80),  // C5
                Tone::pause(30),
                Tone::new(659.25, 80),  // E5
                Tone::pause(30),
                Tone::new(783.99, 150), // G5
            ],

            SoundAsset::AchievementGold => vec![
                Tone::new(523.25, 80),   // C5
                Tone::pause(30),
                Tone::new(659.25, 80),   // E5
                Tone::pause(30),
                Tone::new(783.99, 80),   // G5
                Tone::pause(30),
                Tone::new(1046.50, 200), // C6
            ],

            SoundAsset::AchievementPlatinum => vec![
                Tone::new(523.25, 60),   // C5
                Tone::pause(20),
                Tone::new(659.25, 60),   // E5
                Tone::pause(20),
                Tone::new(783.99, 60),   // G5
                Tone::pause(20),
                Tone::new(1046.50, 60),  // C6
                Tone::pause(20),
                Tone::new(1318.51, 300), // E6
            ],

            SoundAsset::LevelUp => vec![
                Tone::new(392.00, 80), // G4
                Tone::pause(50),
                Tone::new(523.25, 80), // C5
                Tone::pause(50),
                Tone::new(659.25, 80), // E5
                Tone::pause(50),
                Tone::new(783.99, 80), // G5
                Tone::pause(50),
                Tone::new(1046.50, 250), // C6
            ],

            // Milestone tones - celebratory but not too long
            SoundAsset::MilestoneDistance => vec![
                Tone::new(frequencies::SUCCESS, 100),
                Tone::pause(50),
                Tone::new(frequencies::SUCCESS * 1.25, 150),
            ],

            SoundAsset::MilestoneTime => vec![
                Tone::new(frequencies::MEDIUM, 80),
                Tone::pause(40),
                Tone::new(frequencies::HIGH, 120),
            ],

            SoundAsset::MilestoneCalories => vec![
                Tone::new(frequencies::MEDIUM, 80),
                Tone::pause(40),
                Tone::new(frequencies::VERY_HIGH, 120),
            ],

            SoundAsset::PersonalRecord => vec![
                Tone::new(783.99, 100),  // G5
                Tone::pause(50),
                Tone::new(880.00, 100),  // A5
                Tone::pause(50),
                Tone::new(987.77, 100),  // B5
                Tone::pause(50),
                Tone::new(1046.50, 300), // C6
            ],

            // Workout lifecycle tones
            SoundAsset::WorkoutStart => CuePattern::TripleBeep.tones(),

            SoundAsset::WorkoutPause => vec![
                Tone::new(frequencies::MEDIUM, 200),
                Tone::pause(100),
                Tone::new(frequencies::LOW, 300),
            ],

            SoundAsset::WorkoutResume => vec![
                Tone::new(frequencies::LOW, 200),
                Tone::pause(100),
                Tone::new(frequencies::MEDIUM, 300),
            ],

            SoundAsset::WorkoutComplete => CuePattern::Success.tones(),

            SoundAsset::WorkoutStop => vec![Tone::new(frequencies::LOW, 400)],

            // Zone change tones
            SoundAsset::ZoneUp => CuePattern::Ascending.tones(),

            SoundAsset::ZoneDown => CuePattern::Descending.tones(),

            // Alert tones
            SoundAsset::Notification => CuePattern::SingleBeep.tones(),

            SoundAsset::Warning => CuePattern::Alert.tones(),

            SoundAsset::Error => CuePattern::Error.tones(),

            SoundAsset::Success => CuePattern::Success.tones(),

            // Connection tones
            SoundAsset::DeviceConnected => vec![
                Tone::new(frequencies::LOW, 100),
                Tone::pause(50),
                Tone::new(frequencies::HIGH, 150),
            ],

            SoundAsset::DeviceDisconnected => vec![
                Tone::new(frequencies::HIGH, 100),
                Tone::pause(50),
                Tone::new(frequencies::LOW, 150),
            ],
        }
    }

    /// Get the approximate duration of the fallback tones
    pub fn fallback_duration(&self) -> Duration {
        let total_ms: u64 = self.fallback_tones().iter().map(|t| t.duration_ms).sum();
        Duration::from_millis(total_ms)
    }

    /// Parse a sound asset from its name
    pub fn from_name(name: &str) -> Option<SoundAsset> {
        match name {
            // Countdown
            "countdown_tick" => Some(SoundAsset::CountdownTick),
            "countdown_3" => Some(SoundAsset::CountdownFinal3),
            "countdown_2" => Some(SoundAsset::CountdownFinal2),
            "countdown_1" => Some(SoundAsset::CountdownFinal1),
            "countdown_go" => Some(SoundAsset::CountdownGo),
            // Intervals
            "interval_warning" => Some(SoundAsset::IntervalWarning),
            "interval_change" => Some(SoundAsset::IntervalChange),
            "interval_rest" => Some(SoundAsset::IntervalRest),
            "interval_work" => Some(SoundAsset::IntervalWork),
            // Achievements
            "achievement_bronze" => Some(SoundAsset::AchievementBronze),
            "achievement_silver" => Some(SoundAsset::AchievementSilver),
            "achievement_gold" => Some(SoundAsset::AchievementGold),
            "achievement_platinum" => Some(SoundAsset::AchievementPlatinum),
            "level_up" => Some(SoundAsset::LevelUp),
            // Milestones
            "milestone_distance" => Some(SoundAsset::MilestoneDistance),
            "milestone_time" => Some(SoundAsset::MilestoneTime),
            "milestone_calories" => Some(SoundAsset::MilestoneCalories),
            "personal_record" => Some(SoundAsset::PersonalRecord),
            // Workout lifecycle
            "workout_start" => Some(SoundAsset::WorkoutStart),
            "workout_pause" => Some(SoundAsset::WorkoutPause),
            "workout_resume" => Some(SoundAsset::WorkoutResume),
            "workout_complete" => Some(SoundAsset::WorkoutComplete),
            "workout_stop" => Some(SoundAsset::WorkoutStop),
            // Zone changes
            "zone_up" => Some(SoundAsset::ZoneUp),
            "zone_down" => Some(SoundAsset::ZoneDown),
            // Alerts
            "notification" => Some(SoundAsset::Notification),
            "warning" => Some(SoundAsset::Warning),
            "error" => Some(SoundAsset::Error),
            "success" => Some(SoundAsset::Success),
            // Connections
            "device_connected" => Some(SoundAsset::DeviceConnected),
            "device_disconnected" => Some(SoundAsset::DeviceDisconnected),
            _ => None,
        }
    }
}

impl std::fmt::Display for SoundAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Category of sound assets for volume control and configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundCategory {
    /// Countdown sounds (tick, 3, 2, 1, go)
    Countdown,
    /// Interval change sounds
    Interval,
    /// Achievement unlock sounds
    Achievement,
    /// Milestone sounds
    Milestone,
    /// Workout lifecycle sounds (start, pause, complete)
    Workout,
    /// Zone change sounds
    Zone,
    /// General alert sounds
    Alert,
    /// Device connection sounds
    Connection,
}

impl SoundCategory {
    /// Get all categories
    pub fn all() -> &'static [SoundCategory] {
        &[
            SoundCategory::Countdown,
            SoundCategory::Interval,
            SoundCategory::Achievement,
            SoundCategory::Milestone,
            SoundCategory::Workout,
            SoundCategory::Zone,
            SoundCategory::Alert,
            SoundCategory::Connection,
        ]
    }

    /// Get a human-readable name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            SoundCategory::Countdown => "Countdown",
            SoundCategory::Interval => "Intervals",
            SoundCategory::Achievement => "Achievements",
            SoundCategory::Milestone => "Milestones",
            SoundCategory::Workout => "Workout",
            SoundCategory::Zone => "Zone Changes",
            SoundCategory::Alert => "Alerts",
            SoundCategory::Connection => "Connections",
        }
    }

    /// Get all sound assets in this category
    pub fn sounds(&self) -> Vec<SoundAsset> {
        SoundAsset::all()
            .iter()
            .filter(|s| s.category() == *self)
            .copied()
            .collect()
    }
}

impl std::fmt::Display for SoundCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Sound asset definition with metadata
#[derive(Debug, Clone)]
pub struct SoundDefinition {
    /// The sound asset identifier
    pub asset: SoundAsset,
    /// Human-readable description
    pub description: &'static str,
    /// Whether this sound is critical (should play even when others are muted)
    pub critical: bool,
    /// Default volume multiplier (0.0-1.0) relative to category volume
    pub volume_multiplier: f32,
}

impl SoundDefinition {
    /// Get the definition for a sound asset
    pub fn for_asset(asset: SoundAsset) -> Self {
        match asset {
            // Countdown sounds
            SoundAsset::CountdownTick => SoundDefinition {
                asset,
                description: "Countdown tick for each second",
                critical: false,
                volume_multiplier: 0.8,
            },
            SoundAsset::CountdownFinal3 => SoundDefinition {
                asset,
                description: "3 seconds remaining",
                critical: true,
                volume_multiplier: 0.9,
            },
            SoundAsset::CountdownFinal2 => SoundDefinition {
                asset,
                description: "2 seconds remaining",
                critical: true,
                volume_multiplier: 0.95,
            },
            SoundAsset::CountdownFinal1 => SoundDefinition {
                asset,
                description: "1 second remaining",
                critical: true,
                volume_multiplier: 1.0,
            },
            SoundAsset::CountdownGo => SoundDefinition {
                asset,
                description: "Go! Interval starting now",
                critical: true,
                volume_multiplier: 1.0,
            },

            // Interval sounds
            SoundAsset::IntervalWarning => SoundDefinition {
                asset,
                description: "Interval change approaching",
                critical: false,
                volume_multiplier: 0.9,
            },
            SoundAsset::IntervalChange => SoundDefinition {
                asset,
                description: "Interval has changed",
                critical: true,
                volume_multiplier: 1.0,
            },
            SoundAsset::IntervalRest => SoundDefinition {
                asset,
                description: "Rest interval started",
                critical: false,
                volume_multiplier: 0.8,
            },
            SoundAsset::IntervalWork => SoundDefinition {
                asset,
                description: "Work interval started",
                critical: true,
                volume_multiplier: 1.0,
            },

            // Achievement sounds
            SoundAsset::AchievementBronze => SoundDefinition {
                asset,
                description: "Bronze achievement unlocked",
                critical: false,
                volume_multiplier: 0.8,
            },
            SoundAsset::AchievementSilver => SoundDefinition {
                asset,
                description: "Silver achievement unlocked",
                critical: false,
                volume_multiplier: 0.85,
            },
            SoundAsset::AchievementGold => SoundDefinition {
                asset,
                description: "Gold achievement unlocked",
                critical: false,
                volume_multiplier: 0.9,
            },
            SoundAsset::AchievementPlatinum => SoundDefinition {
                asset,
                description: "Platinum achievement unlocked",
                critical: false,
                volume_multiplier: 1.0,
            },
            SoundAsset::LevelUp => SoundDefinition {
                asset,
                description: "Level up notification",
                critical: false,
                volume_multiplier: 1.0,
            },

            // Milestone sounds
            SoundAsset::MilestoneDistance => SoundDefinition {
                asset,
                description: "Distance milestone reached",
                critical: false,
                volume_multiplier: 0.7,
            },
            SoundAsset::MilestoneTime => SoundDefinition {
                asset,
                description: "Time milestone reached",
                critical: false,
                volume_multiplier: 0.7,
            },
            SoundAsset::MilestoneCalories => SoundDefinition {
                asset,
                description: "Calorie milestone reached",
                critical: false,
                volume_multiplier: 0.7,
            },
            SoundAsset::PersonalRecord => SoundDefinition {
                asset,
                description: "Personal record achieved",
                critical: false,
                volume_multiplier: 1.0,
            },

            // Workout lifecycle sounds
            SoundAsset::WorkoutStart => SoundDefinition {
                asset,
                description: "Workout is starting",
                critical: true,
                volume_multiplier: 1.0,
            },
            SoundAsset::WorkoutPause => SoundDefinition {
                asset,
                description: "Workout paused",
                critical: false,
                volume_multiplier: 0.8,
            },
            SoundAsset::WorkoutResume => SoundDefinition {
                asset,
                description: "Workout resumed",
                critical: false,
                volume_multiplier: 0.9,
            },
            SoundAsset::WorkoutComplete => SoundDefinition {
                asset,
                description: "Workout completed successfully",
                critical: true,
                volume_multiplier: 1.0,
            },
            SoundAsset::WorkoutStop => SoundDefinition {
                asset,
                description: "Workout stopped",
                critical: false,
                volume_multiplier: 0.8,
            },

            // Zone change sounds
            SoundAsset::ZoneUp => SoundDefinition {
                asset,
                description: "Entered higher power zone",
                critical: false,
                volume_multiplier: 0.9,
            },
            SoundAsset::ZoneDown => SoundDefinition {
                asset,
                description: "Entered lower power zone",
                critical: false,
                volume_multiplier: 0.9,
            },

            // Alert sounds
            SoundAsset::Notification => SoundDefinition {
                asset,
                description: "General notification",
                critical: false,
                volume_multiplier: 0.7,
            },
            SoundAsset::Warning => SoundDefinition {
                asset,
                description: "Warning alert",
                critical: true,
                volume_multiplier: 1.0,
            },
            SoundAsset::Error => SoundDefinition {
                asset,
                description: "Error alert",
                critical: true,
                volume_multiplier: 1.0,
            },
            SoundAsset::Success => SoundDefinition {
                asset,
                description: "Success confirmation",
                critical: false,
                volume_multiplier: 0.8,
            },

            // Connection sounds
            SoundAsset::DeviceConnected => SoundDefinition {
                asset,
                description: "Device connected",
                critical: false,
                volume_multiplier: 0.7,
            },
            SoundAsset::DeviceDisconnected => SoundDefinition {
                asset,
                description: "Device disconnected",
                critical: true,
                volume_multiplier: 0.9,
            },
        }
    }
}

/// Sound asset catalog - provides access to all sound definitions
pub struct SoundCatalog;

impl SoundCatalog {
    /// Get all sound definitions
    pub fn all_definitions() -> Vec<SoundDefinition> {
        SoundAsset::all()
            .iter()
            .map(|asset| SoundDefinition::for_asset(*asset))
            .collect()
    }

    /// Get sound definitions for a category
    pub fn definitions_for_category(category: SoundCategory) -> Vec<SoundDefinition> {
        category
            .sounds()
            .iter()
            .map(|asset| SoundDefinition::for_asset(*asset))
            .collect()
    }

    /// Get all critical sounds (should play even when others muted)
    pub fn critical_sounds() -> Vec<SoundAsset> {
        SoundAsset::all()
            .iter()
            .filter(|asset| SoundDefinition::for_asset(**asset).critical)
            .copied()
            .collect()
    }

    /// Check if a sound name is valid
    pub fn is_valid_sound(name: &str) -> bool {
        SoundAsset::from_name(name).is_some()
    }

    /// Get all sound names
    pub fn all_sound_names() -> Vec<&'static str> {
        SoundAsset::all().iter().map(|a| a.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_asset_all() {
        let all = SoundAsset::all();
        assert!(!all.is_empty());
        // Should have at least countdown, achievement, and workout sounds
        assert!(all.len() >= 20);
    }

    #[test]
    fn test_sound_asset_name_roundtrip() {
        for asset in SoundAsset::all() {
            let name = asset.name();
            let parsed = SoundAsset::from_name(name);
            assert_eq!(parsed, Some(*asset), "Failed roundtrip for {}", name);
        }
    }

    #[test]
    fn test_sound_asset_file_path() {
        let tick = SoundAsset::CountdownTick;
        let path = tick.file_path();
        assert_eq!(path.to_str().unwrap(), "assets/sounds/countdown_tick.wav");
    }

    #[test]
    fn test_sound_category() {
        assert_eq!(
            SoundAsset::CountdownTick.category(),
            SoundCategory::Countdown
        );
        assert_eq!(
            SoundAsset::AchievementGold.category(),
            SoundCategory::Achievement
        );
        assert_eq!(SoundAsset::WorkoutStart.category(), SoundCategory::Workout);
    }

    #[test]
    fn test_sound_category_sounds() {
        let countdown_sounds = SoundCategory::Countdown.sounds();
        assert!(countdown_sounds.contains(&SoundAsset::CountdownTick));
        assert!(countdown_sounds.contains(&SoundAsset::CountdownGo));
        assert!(!countdown_sounds.contains(&SoundAsset::AchievementGold));
    }

    #[test]
    fn test_fallback_tones_not_empty() {
        for asset in SoundAsset::all() {
            let tones = asset.fallback_tones();
            assert!(!tones.is_empty(), "No fallback tones for {}", asset.name());
        }
    }

    #[test]
    fn test_fallback_duration() {
        let tick = SoundAsset::CountdownTick;
        let duration = tick.fallback_duration();
        assert!(duration.as_millis() > 0);
        assert!(duration.as_millis() < 1000); // Should be a short beep
    }

    #[test]
    fn test_sound_definition() {
        let def = SoundDefinition::for_asset(SoundAsset::CountdownGo);
        assert_eq!(def.asset, SoundAsset::CountdownGo);
        assert!(def.critical); // Countdown go is critical
        assert!(def.volume_multiplier > 0.0 && def.volume_multiplier <= 1.0);
    }

    #[test]
    fn test_sound_catalog_critical_sounds() {
        let critical = SoundCatalog::critical_sounds();
        assert!(critical.contains(&SoundAsset::CountdownGo));
        assert!(critical.contains(&SoundAsset::IntervalChange));
        assert!(critical.contains(&SoundAsset::Warning));
    }

    #[test]
    fn test_sound_catalog_valid_sound() {
        assert!(SoundCatalog::is_valid_sound("countdown_tick"));
        assert!(SoundCatalog::is_valid_sound("achievement_gold"));
        assert!(!SoundCatalog::is_valid_sound("invalid_sound"));
    }

    #[test]
    fn test_sound_catalog_all_definitions() {
        let definitions = SoundCatalog::all_definitions();
        assert_eq!(definitions.len(), SoundAsset::all().len());
    }

    #[test]
    fn test_achievement_tones_increase_complexity() {
        let bronze = SoundAsset::AchievementBronze.fallback_tones();
        let silver = SoundAsset::AchievementSilver.fallback_tones();
        let gold = SoundAsset::AchievementGold.fallback_tones();
        let platinum = SoundAsset::AchievementPlatinum.fallback_tones();

        // Higher tiers should have more elaborate sequences
        assert!(silver.len() >= bronze.len());
        assert!(gold.len() >= silver.len());
        assert!(platinum.len() >= gold.len());
    }

    #[test]
    fn test_countdown_tones_increase_urgency() {
        let tick = SoundAsset::CountdownTick.fallback_tones()[0];
        let final3 = SoundAsset::CountdownFinal3.fallback_tones()[0];
        let final2 = SoundAsset::CountdownFinal2.fallback_tones()[0];
        let final1 = SoundAsset::CountdownFinal1.fallback_tones()[0];

        // Final countdown tones should have increasing frequency (higher pitch = more urgent)
        assert!(final3.frequency_hz > tick.frequency_hz);
        assert!(final2.frequency_hz > final3.frequency_hz);
        assert!(final1.frequency_hz > final2.frequency_hz);
    }

    #[test]
    fn test_zone_up_down_patterns() {
        let up_tones = SoundAsset::ZoneUp.fallback_tones();
        let down_tones = SoundAsset::ZoneDown.fallback_tones();

        // Zone up should have ascending frequency pattern
        let up_freqs: Vec<f32> = up_tones
            .iter()
            .filter(|t| !t.is_pause())
            .map(|t| t.frequency_hz)
            .collect();
        assert!(up_freqs.len() >= 2);
        for i in 1..up_freqs.len() {
            assert!(
                up_freqs[i] >= up_freqs[i - 1],
                "Zone up should have ascending frequencies"
            );
        }

        // Zone down should have descending frequency pattern
        let down_freqs: Vec<f32> = down_tones
            .iter()
            .filter(|t| !t.is_pause())
            .map(|t| t.frequency_hz)
            .collect();
        assert!(down_freqs.len() >= 2);
        for i in 1..down_freqs.len() {
            assert!(
                down_freqs[i] <= down_freqs[i - 1],
                "Zone down should have descending frequencies"
            );
        }
    }

    #[test]
    fn test_sound_category_display_names() {
        for category in SoundCategory::all() {
            let name = category.display_name();
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn test_all_categories_have_sounds() {
        for category in SoundCategory::all() {
            let sounds = category.sounds();
            assert!(
                !sounds.is_empty(),
                "Category {} has no sounds",
                category.display_name()
            );
        }
    }

    #[test]
    fn test_sound_asset_display() {
        let asset = SoundAsset::CountdownTick;
        let display = format!("{}", asset);
        assert_eq!(display, "countdown_tick");
    }

    #[test]
    fn test_unknown_sound_name() {
        assert!(SoundAsset::from_name("unknown_sound").is_none());
        assert!(SoundAsset::from_name("").is_none());
        assert!(SoundAsset::from_name("COUNTDOWN_TICK").is_none()); // Case sensitive
    }
}
