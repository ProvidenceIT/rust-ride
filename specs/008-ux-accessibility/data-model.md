# Data Model: UX & Accessibility

**Feature**: 008-ux-accessibility
**Date**: 2025-12-27

## Entities

### AccessibilitySettings

Stores user accessibility preferences. Subset of UserPreferences specific to accessibility features.

```rust
/// Accessibility configuration stored per user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    /// Colorblind mode selection
    pub color_mode: ColorMode,

    /// High contrast mode enabled
    pub high_contrast: bool,

    /// Screen reader optimizations enabled
    pub screen_reader_enabled: bool,

    /// Voice control enabled
    pub voice_control_enabled: bool,

    /// Voice control activation mode
    pub voice_activation: VoiceActivation,

    /// Keyboard shortcut overlay key (default: F1 or ?)
    pub shortcut_help_key: KeyCode,

    /// Focus indicator style
    pub focus_indicator: FocusIndicatorStyle,

    /// Reduce motion animations
    pub reduce_motion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    /// Default color palette
    Normal,
    /// Optimized for protanopia (red-green, red-blind)
    Protanopia,
    /// Optimized for deuteranopia (red-green, green-blind)
    Deuteranopia,
    /// Optimized for tritanopia (blue-yellow)
    Tritanopia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceActivation {
    /// Always listening for commands
    AlwaysOn,
    /// Requires push-to-talk key
    PushToTalk,
    /// Disabled
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusIndicatorStyle {
    /// Standard 2px outline
    Standard,
    /// Bold 4px outline for visibility
    Bold,
    /// Animated pulsing outline
    Animated,
}
```

### ThemePreference

Extends existing Theme enum to support system-following mode.

```rust
/// Theme selection preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemePreference {
    /// Follow system dark/light mode
    #[default]
    FollowSystem,
    /// Always use light theme
    Light,
    /// Always use dark theme
    Dark,
}
```

### LayoutProfile

Named collection of widget positions and sizes for the ride screen.

```rust
/// A saved dashboard layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutProfile {
    /// Unique identifier
    pub id: Uuid,

    /// User-defined profile name
    pub name: String,

    /// Widget placements on the dashboard
    pub widgets: Vec<WidgetPlacement>,

    /// Whether this is the default profile
    pub is_default: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

/// Placement of a single widget on the dashboard grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPlacement {
    /// Type of metric to display
    pub metric_type: MetricType,

    /// Grid column (0-based)
    pub column: u8,

    /// Grid row (0-based)
    pub row: u8,

    /// Width in grid units (1-4)
    pub width: u8,

    /// Height in grid units (1-2)
    pub height: u8,

    /// Display size tier
    pub size_tier: WidgetSizeTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetSizeTier {
    /// Large display (primary metrics)
    Primary,
    /// Medium display (secondary metrics)
    Secondary,
    /// Small display (tertiary metrics)
    Tertiary,
}
```

### OnboardingState

Tracks user's progress through the onboarding wizard.

```rust
/// Onboarding wizard progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingState {
    /// Current step in the wizard (0-indexed)
    pub current_step: u8,

    /// Whether onboarding has been completed
    pub completed: bool,

    /// Timestamp when onboarding was skipped (if applicable)
    pub skipped_at: Option<DateTime<Utc>>,

    /// Steps that have been individually completed
    pub completed_steps: Vec<OnboardingStep>,

    /// First launch timestamp
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingStep {
    Welcome,
    SensorSetup,
    ProfileSetup,
    FtpConfiguration,
    UiTour,
    Complete,
}
```

### AudioSettings

Audio feedback configuration.

```rust
/// Audio feedback settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Master audio enabled
    pub enabled: bool,

    /// Audio cue volume (0.0 - 1.0)
    pub volume: f32,

    /// Play sound on interval transitions
    pub interval_cues: bool,

    /// Play sound on zone changes
    pub zone_change_cues: bool,

    /// Play sound on workout start/end
    pub workout_cues: bool,

    /// Custom audio profile selection
    pub profile: AudioProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AudioProfile {
    /// Simple beep tones
    #[default]
    Simple,
    /// Melodic tones
    Melodic,
    /// Minimal (only critical alerts)
    Minimal,
}
```

### DisplayMode

Current display mode for the ride screen.

```rust
/// Display mode for the ride screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DisplayMode {
    /// Standard dashboard layout
    #[default]
    Normal,
    /// Large display optimized (TV Mode)
    TvMode,
    /// Minimal distraction (Flow Mode)
    FlowMode,
}

/// Flow Mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowModeSettings {
    /// Primary metric to display in Flow Mode
    pub primary_metric: MetricType,

    /// Show 3D world background
    pub show_world: bool,

    /// Show interval notifications
    pub show_interval_alerts: bool,
}
```

### LocaleSettings

Internationalization preferences.

```rust
/// Language and locale settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSettings {
    /// Selected language code (e.g., "en-US", "es", "fr")
    pub language: String,

    /// Use system language on startup
    pub follow_system: bool,

    /// Date format preference
    pub date_format: DateFormat,

    /// Number format (decimal separator)
    pub number_format: NumberFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DateFormat {
    #[default]
    /// YYYY-MM-DD
    Iso,
    /// MM/DD/YYYY
    UsFormat,
    /// DD/MM/YYYY
    EuFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NumberFormat {
    #[default]
    /// 1,234.56
    Comma,
    /// 1.234,56
    Period,
}
```

## Extended UserPreferences

The existing `UserProfile` struct should be extended to include these new settings:

```rust
/// Extended user preferences (addition to existing UserProfile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    // ... existing fields (units, theme) ...

    /// Theme preference (extended from Theme to ThemePreference)
    pub theme_preference: ThemePreference,

    /// Accessibility settings
    pub accessibility: AccessibilitySettings,

    /// Audio feedback settings
    pub audio: AudioSettings,

    /// Display mode settings
    pub display_mode: DisplayMode,

    /// Flow mode configuration
    pub flow_mode: FlowModeSettings,

    /// Locale/language settings
    pub locale: LocaleSettings,

    /// Active layout profile ID
    pub active_layout_id: Option<Uuid>,
}
```

## Database Schema

### New Tables

```sql
-- Layout profiles (max 10 per user enforced at application layer)
CREATE TABLE IF NOT EXISTS layout_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    layout_json TEXT NOT NULL,  -- Serialized Vec<WidgetPlacement>
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Onboarding state (single row table)
CREATE TABLE IF NOT EXISTS onboarding_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Ensure single row
    current_step INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    skipped_at TEXT,
    completed_steps TEXT NOT NULL DEFAULT '[]',  -- JSON array
    started_at TEXT NOT NULL
);

-- Extended user preferences (extends existing user_profile)
-- Add columns to existing table or create preferences table
CREATE TABLE IF NOT EXISTS user_preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    theme_preference TEXT NOT NULL DEFAULT 'follow_system',
    accessibility_json TEXT NOT NULL DEFAULT '{}',
    audio_json TEXT NOT NULL DEFAULT '{}',
    display_mode TEXT NOT NULL DEFAULT 'normal',
    flow_mode_json TEXT NOT NULL DEFAULT '{}',
    locale_json TEXT NOT NULL DEFAULT '{}',
    active_layout_id TEXT,
    FOREIGN KEY (active_layout_id) REFERENCES layout_profiles(id)
);
```

## State Transitions

### OnboardingState

```
[Not Started] → Welcome → SensorSetup → ProfileSetup → FtpConfiguration → UiTour → [Completed]
                  ↓           ↓              ↓               ↓              ↓
                [Skip] ────────────────────────────────────────────────────[Skipped]

Resume: Can resume from any step if not completed/skipped
Restart: Can restart from Welcome at any time from settings
```

### DisplayMode

```
Normal ←→ TvMode
   ↕          ↕
FlowMode ←────┘

Transitions:
- Normal → TvMode: User toggles TV Mode setting
- Normal → FlowMode: User activates Flow Mode (hotkey or menu)
- TvMode → FlowMode: User activates Flow Mode
- FlowMode → Normal/TvMode: User exits Flow Mode (any input or hotkey)
```

## Validation Rules

| Entity | Field | Rule |
|--------|-------|------|
| LayoutProfile | name | 1-50 characters, unique per user |
| LayoutProfile | widgets | 1-12 widgets, no overlapping positions |
| WidgetPlacement | column | 0-3 (4-column grid) |
| WidgetPlacement | row | 0-5 (6-row max grid) |
| WidgetPlacement | width | 1-4 |
| WidgetPlacement | height | 1-2 |
| AudioSettings | volume | 0.0-1.0 |
| LocaleSettings | language | Must be supported locale (en-US, es, fr, de, it) |
| OnboardingState | current_step | 0-5 (matches OnboardingStep enum) |

## Relationships

```
UserPreferences (1) ──→ (0..1) LayoutProfile (active)
UserPreferences (1) ──→ (1) AccessibilitySettings
UserPreferences (1) ──→ (1) AudioSettings
UserPreferences (1) ──→ (1) LocaleSettings
UserPreferences (1) ──→ (1) FlowModeSettings

User (1) ──→ (0..10) LayoutProfile
User (1) ──→ (1) OnboardingState
```
