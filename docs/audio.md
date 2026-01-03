# RustRide Audio System Documentation

The RustRide audio system provides rich audio feedback for workouts including countdown sounds,
achievement chimes, milestone celebrations, voice announcements, and zone change alerts. It uses
[rodio](https://github.com/RustAudio/rodio) for audio playback and system TTS for voice.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Architecture Overview](#architecture-overview)
3. [Configuration](#configuration)
4. [Audio Categories](#audio-categories)
5. [Sound Assets](#sound-assets)
6. [Extending the System](#extending-the-system)
7. [Troubleshooting](#troubleshooting)
8. [Example Code](#example-code)

---

## Quick Start

### Basic Audio Engine Setup

```rust
use rustride::audio::{AudioConfig, AudioEngine, DefaultAudioEngine};

// Create audio engine with default configuration
let config = AudioConfig::default();
let engine = DefaultAudioEngine::new(config);

// Initialize the engine (connects to audio devices)
engine.initialize()?;

// Play a tone
engine.play_tone(440, 200).await?;

// Speak text
engine.speak("Interval starting").await?;

// Play a sound effect
engine.play_sound("countdown_tick").await?;
```

### Using the Workout Audio Bridge

```rust
use rustride::audio::{
    AudioEngine, DefaultAudioEngine, WorkoutAudioBridge,
    WorkoutAudioBridgeConfig, AudioConfig
};
use rustride::workouts::WorkoutEvent;

let audio_config = AudioConfig::default();
let engine = Arc::new(DefaultAudioEngine::new(audio_config));
engine.initialize()?;

let bridge_config = WorkoutAudioBridgeConfig::default();
let bridge = WorkoutAudioBridge::new(bridge_config, engine.clone());

// Handle workout events
let event = WorkoutEvent::IntervalCountdown { seconds_remaining: 3 };
bridge.handle_event(&event).await;
```

---

## Architecture Overview

The audio system is organized into several layers:

```
┌────────────────────────────────────────────────────────┐
│                    Application Layer                    │
│  (WorkoutAudioBridge, AchievementAudioBridge, etc.)    │
├────────────────────────────────────────────────────────┤
│                    Audio Engine Layer                   │
│  (DefaultAudioEngine - queue, priority, mixing)        │
├────────────────────────────────────────────────────────┤
│                    Backend Layer                        │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │ RodioAudioBackend│  │ThreadSafeTTS     │            │
│  │ (tones, sounds)  │  │(voice synthesis) │            │
│  └──────────────────┘  └──────────────────┘            │
├────────────────────────────────────────────────────────┤
│                  Platform Layer                         │
│  Windows: WASAPI | macOS: CoreAudio | Linux: ALSA/Pulse│
└────────────────────────────────────────────────────────┘
```

### Key Components

| Component | Purpose |
|-----------|---------|
| `AudioEngine` | Trait defining the audio playback interface |
| `DefaultAudioEngine` | Main implementation with priority queue, volume mixing |
| `RodioAudioBackend` | Low-level audio playback using rodio |
| `ThreadSafeTtsProvider` | Text-to-speech for voice announcements |
| `WorkoutAudioBridge` | Connects workout events to audio feedback |
| `AchievementAudioBridge` | Plays achievement unlock sounds |
| `MilestoneAudioBridge` | Handles milestone celebration audio |

---

## Configuration

### AudioConfig

The main configuration structure for the audio system:

```rust
use rustride::audio::AudioConfig;

let mut config = AudioConfig::default();

// Master settings
config.enabled = true;                    // Master enable for all audio
config.volume = 80;                       // Master volume (0-100)

// Voice/TTS settings
config.voice_enabled = true;              // Enable text-to-speech
config.voice_volume = 100;                // Voice volume (0-100)
config.preferred_voice = None;            // System voice ID (OS-specific)
config.speech_rate = 1.0;                 // Speech rate multiplier (0.5-2.0)

// Sound effects
config.sound_effects_enabled = true;      // Enable sound effects
config.sound_effects_volume = 80;         // Sound effects volume (0-100)

// Countdown sounds
config.countdown_enabled = true;          // Enable countdown audio
config.countdown_volume = 100;            // Countdown volume (0-100)

// Achievements
config.achievements_enabled = true;       // Enable achievement sounds
config.achievement_volume = 100;          // Achievement volume (0-100)

// Milestones
config.milestones_enabled = true;         // Enable milestone sounds
config.milestone_volume = 70;             // Milestone volume (default: 70% - subtle)
config.personal_record_sounds_enabled = true;  // Personal record fanfare

// Alert interval (prevents audio spam)
config.min_alert_interval_ms = 3000;      // Minimum 3s between alerts
```

### Mute Controls

The audio system supports both global and per-category muting:

```rust
// Global mute (silences all audio, preserves volume settings)
config.muted = false;

// Per-category mutes
config.voice_muted = false;
config.sound_effects_muted = false;
config.countdown_muted = false;
config.achievement_muted = false;
config.milestone_muted = false;
```

### Timing Configuration

Control audio queue behavior and timing:

```rust
use rustride::audio::AudioTimingConfig;

let timing = AudioTimingConfig {
    max_queue_size: 20,              // Maximum queued audio items
    countdown_max_age_ms: 500,       // Countdown sounds expire after 500ms
    sound_max_age_ms: 3000,          // Regular sounds expire after 3s
    speech_max_age_ms: 10000,        // Speech expires after 10s
    min_audio_gap_ms: 50,            // Minimum gap between audio items
    aggressive_cleanup: true,         // Auto-cleanup expired items
    queue_pressure_threshold: 70,     // Drop low-priority at 70% queue capacity
};
```

### WorkoutAudioBridgeConfig

Configure countdown behavior:

```rust
use rustride::audio::WorkoutAudioBridgeConfig;

let config = WorkoutAudioBridgeConfig {
    enabled: true,
    countdown_sounds_enabled: true,
    countdown_voice_enabled: true,
    countdown_thresholds: vec![10, 5, 3, 2, 1],      // Which seconds to announce
    countdown_voice_thresholds: vec![10, 5],         // Which get voice + tone
    voice_interval_changes: true,
    voice_zone_changes: true,
};
```

---

## Audio Categories

Audio is organized into categories for independent volume control:

| Category | Description | Default Volume |
|----------|-------------|----------------|
| `Voice` | TTS announcements | 100% |
| `SoundEffect` | General sound effects | 80% |
| `Countdown` | Interval countdown tones | 100% |
| `Achievement` | Achievement unlock chimes | 100% |
| `Milestone` | Distance/time/calorie milestones | 70% (subtle) |

### Using Categories

```rust
use rustride::audio::{AudioCategory, AudioEngine, AudioItem};

// Play a sound with a specific category
engine.play_sound_with_category("countdown_tick", AudioCategory::Countdown).await?;

// Play a tone with a specific category
engine.play_tone_with_category(440, 200, AudioCategory::Achievement).await?;

// Set category-specific volume
engine.set_category_volume(AudioCategory::Countdown, 90);

// Get category volume
let vol = engine.get_category_volume(AudioCategory::Achievement);

// Mute/unmute specific category
engine.mute_category(AudioCategory::Milestone);
engine.unmute_category(AudioCategory::Milestone);
```

---

## Sound Assets

### Available Sounds

Sounds are organized by category. The system uses WAV files from `assets/sounds/` when available,
with automatic fallback to generated tones.

#### Countdown Sounds
- `countdown_tick` - Regular countdown tick
- `countdown_3`, `countdown_2`, `countdown_1` - Final countdown with increasing urgency
- `countdown_go` - "GO!" sound

#### Interval Sounds
- `interval_warning` - Interval change approaching
- `interval_change` - Interval has changed
- `interval_rest` - Rest interval started
- `interval_work` - Work interval started

#### Achievement Sounds
- `achievement_bronze`, `achievement_silver`, `achievement_gold`, `achievement_platinum`
- `level_up` - Level up notification

#### Milestone Sounds
- `milestone_distance` - Distance milestone (5km, 10km, etc.)
- `milestone_time` - Time milestone (15min, 30min, etc.)
- `milestone_calories` - Calorie milestone
- `personal_record` - Personal record achieved

#### Workout Lifecycle
- `workout_start`, `workout_pause`, `workout_resume`
- `workout_complete`, `workout_stop`

#### Zone Changes
- `zone_up` - Moving to higher power zone
- `zone_down` - Moving to lower power zone

#### Alerts
- `notification`, `warning`, `error`, `success`

#### Connections
- `device_connected`, `device_disconnected`

### Adding Custom Sound Files

1. Create WAV files (16-bit, 44100 Hz recommended)
2. Place in `assets/sounds/` with the naming convention:

```
assets/sounds/
├── countdown_tick.wav
├── countdown_3.wav
├── countdown_2.wav
├── countdown_1.wav
├── countdown_go.wav
├── achievement_bronze.wav
├── achievement_silver.wav
├── achievement_gold.wav
├── achievement_platinum.wav
└── ...
```

If a sound file is missing, the system automatically uses generated tones as fallback.

---

## Extending the System

### Adding New Sound Assets

1. Add the sound to `SoundAsset` enum in `src/audio/sounds.rs`:

```rust
pub enum SoundAsset {
    // ... existing sounds ...

    /// My new custom sound
    MyNewSound,
}
```

2. Implement the required methods:

```rust
impl SoundAsset {
    pub fn name(&self) -> &'static str {
        match self {
            // ... existing ...
            SoundAsset::MyNewSound => "my_new_sound",
        }
    }

    pub fn category(&self) -> SoundCategory {
        match self {
            // ... existing ...
            SoundAsset::MyNewSound => SoundCategory::Alert,
        }
    }

    pub fn fallback_tones(&self) -> Vec<Tone> {
        match self {
            // ... existing ...
            SoundAsset::MyNewSound => vec![
                Tone::new(440.0, 100),  // A4, 100ms
                Tone::pause(50),
                Tone::new(523.25, 150), // C5, 150ms
            ],
        }
    }
}
```

3. Add to `from_name()` for string parsing:

```rust
pub fn from_name(name: &str) -> Option<SoundAsset> {
    match name {
        // ... existing ...
        "my_new_sound" => Some(SoundAsset::MyNewSound),
        _ => None,
    }
}
```

### Adding New Cue Patterns

Add to `CuePattern` enum in `src/audio/tones.rs`:

```rust
pub enum CuePattern {
    // ... existing patterns ...

    /// My custom pattern
    MyPattern,
}

impl CuePattern {
    pub fn tones(&self) -> Vec<Tone> {
        match self {
            // ... existing ...
            CuePattern::MyPattern => vec![
                Tone::new(frequencies::LOW, 100),
                Tone::pause(30),
                Tone::new(frequencies::HIGH, 200),
            ],
        }
    }
}
```

### Creating a Custom Audio Bridge

```rust
use std::sync::Arc;
use rustride::audio::{AudioEngine, AudioItem, AudioPriority, AudioCategory};

pub struct MyCustomBridge<E: AudioEngine> {
    engine: Arc<E>,
    enabled: bool,
}

impl<E: AudioEngine> MyCustomBridge<E> {
    pub fn new(engine: Arc<E>) -> Self {
        Self { engine, enabled: true }
    }

    pub async fn handle_custom_event(&self, event_type: &str) {
        if !self.enabled {
            return;
        }

        match event_type {
            "important" => {
                // High-priority voice + sound
                self.engine.queue(
                    AudioItem::urgent_speech("Important event!")
                );
                self.engine.queue(
                    AudioItem::tone(880, 200)
                        .with_priority(AudioPriority::High)
                        .with_category(AudioCategory::SoundEffect)
                );
            }
            "notification" => {
                // Normal priority sound
                self.engine.queue(
                    AudioItem::sound("notification")
                );
            }
            _ => {}
        }
    }
}
```

---

## Troubleshooting

### Audio Device Issues

#### Windows

| Issue | Solution |
|-------|----------|
| No audio output | Check Windows Sound Settings, ensure output device is selected |
| Audio crackling | Update audio drivers, check sample rate settings |
| Device not found | Restart Windows Audio service (services.msc) |
| Permission denied | Run as administrator or check audio permissions |

**Windows Audio Troubleshooter:**
1. Settings -> System -> Sound
2. Click "Troubleshoot" under Output

#### macOS

| Issue | Solution |
|-------|----------|
| No audio | Check System Preferences -> Sound for output device |
| Exclusive access error | Close other audio applications |
| Permission denied | Grant audio access in System Preferences -> Security & Privacy |
| CoreAudio issues | Run `sudo killall coreaudiod` to reset |

#### Linux

| Issue | Solution |
|-------|----------|
| No audio | Check PulseAudio: `systemctl --user status pulseaudio` |
| Permission denied | Add user to audio group: `sudo usermod -aG audio $USER` |
| ALSA errors | Verify configuration: `aplay -l` to list devices |
| PipeWire systems | Restart: `systemctl --user restart pipewire pipewire-pulse` |

**Install required packages (Debian/Ubuntu):**
```bash
sudo apt install pulseaudio alsa-utils
```

### Common Issues

#### Audio Queue Overload

**Symptoms:** Sounds are delayed or missing, queue pressure warnings in logs

**Solutions:**
1. Check `QueueStats`:
   ```rust
   let stats = engine.get_queue_stats();
   if !stats.is_healthy() {
       tracing::warn!("Queue status: {}", stats.status_string());
   }
   ```

2. Adjust timing configuration:
   ```rust
   let mut timing = engine.get_timing_config();
   timing.max_queue_size = 30;  // Increase if needed
   timing.aggressive_cleanup = true;
   engine.set_timing_config(timing);
   ```

3. Reset queue statistics:
   ```rust
   engine.reset_queue_stats();
   ```

#### Countdown Sounds Not Playing

**Symptoms:** Countdown tones are missing or delayed

**Causes:** Countdown sounds have a short expiration time (500ms) to ensure timing accuracy.

**Solutions:**
1. Ensure countdown is enabled:
   ```rust
   config.countdown_enabled = true;
   config.countdown_muted = false;
   ```

2. Check queue health - if queue is overloaded, time-critical sounds expire.

#### TTS Not Working

1. Check TTS is enabled and not muted:
   ```rust
   config.voice_enabled = true;
   config.voice_muted = false;
   ```

2. Initialize the engine properly:
   ```rust
   engine.initialize()?;  // Required for TTS setup
   ```

3. Check platform-specific requirements:
   - **Linux**: Install `speech-dispatcher`: `sudo apt install speech-dispatcher`
   - **macOS**: TTS should work out of the box
   - **Windows**: TTS should work out of the box (SAPI)

#### Hot-Plug Recovery

The audio system can recover when audio devices are connected/disconnected:

```rust
// Check device status
let status = engine.get_device_status();
if !status.available {
    println!("Device unavailable: {}", status.state_description());

    // Try recovery
    if engine.try_device_recovery() {
        println!("Recovery attempted");
    }
}

// Get troubleshooting hints
for hint in engine.get_troubleshooting_hints() {
    println!("  - {}", hint);
}

// Reset recovery counter (after user reconnects device)
engine.reset_device_recovery();
```

---

## Example Code

### Complete Workout Audio Setup

```rust
use std::sync::Arc;
use rustride::audio::{
    AudioConfig, AudioEngine, DefaultAudioEngine,
    WorkoutAudioBridge, WorkoutAudioBridgeConfig,
    AchievementAudioBridge, AchievementAudioBridgeConfig,
    MilestoneAudioBridge, MilestoneAudioBridgeConfig,
};
use rustride::workouts::WorkoutEvent;

async fn setup_workout_audio() -> anyhow::Result<()> {
    // Create and configure audio engine
    let mut audio_config = AudioConfig::default();
    audio_config.volume = 80;
    audio_config.countdown_volume = 100;  // Ensure countdown is loud
    audio_config.milestone_volume = 60;   // Subtle milestones

    let engine = Arc::new(DefaultAudioEngine::new(audio_config));
    engine.initialize()?;

    // Create bridges
    let workout_config = WorkoutAudioBridgeConfig::default();
    let workout_bridge = WorkoutAudioBridge::new(workout_config, engine.clone());

    let achievement_config = AchievementAudioBridgeConfig::default();
    let achievement_bridge = AchievementAudioBridge::new(achievement_config, engine.clone());

    let milestone_config = MilestoneAudioBridgeConfig::default();
    let milestone_bridge = MilestoneAudioBridge::new(milestone_config, engine.clone());

    // Handle events
    let countdown_event = WorkoutEvent::IntervalCountdown { seconds_remaining: 3 };
    workout_bridge.handle_event(&countdown_event).await;

    Ok(())
}
```

### Custom Priority Audio

```rust
use rustride::audio::{AudioItem, AudioPriority, AudioCategory};

// Low priority - can be dropped if queue is full
let ambient = AudioItem::sound("notification")
    .with_priority(AudioPriority::Low)
    .with_category(AudioCategory::SoundEffect);
engine.queue(ambient);

// Normal priority - standard behavior
let chime = AudioItem::tone(523, 150)
    .with_priority(AudioPriority::Normal)
    .with_category(AudioCategory::Achievement);
engine.queue(chime);

// High priority - interrupts lower priority audio
let alert = AudioItem::urgent_speech("Interval change in 5 seconds");
engine.queue(alert);

// Critical priority - interrupts everything
let emergency = AudioItem::speech("Workout paused due to sensor disconnect")
    .with_priority(AudioPriority::Critical);
engine.queue(emergency);

// Process the queue
engine.process_queue().await;
```

### Time-Critical Countdown Tones

```rust
use std::time::Duration;
use rustride::audio::{AudioItem, AudioCategory};

// Standard countdown tone (500ms expiration)
let tone = AudioItem::countdown_tone(587, 100);  // D5, 100ms
engine.queue(tone);

// Custom timing for very time-sensitive scenarios
let urgent = AudioItem::countdown_tone_with_timing(
    880,    // Frequency: A5
    150,    // Duration: 150ms
    200,    // Expires after 200ms in queue
);
engine.queue(urgent);

// The countdown tone has AudioCategory::Countdown
// and respects countdown_volume settings
```

### Volume Control Example

```rust
use rustride::audio::{AudioCategory, AudioEngine};

// Set master volume
engine.set_volume(75);

// Set per-category volumes
engine.set_category_volume(AudioCategory::Voice, 100);
engine.set_category_volume(AudioCategory::SoundEffect, 80);
engine.set_category_volume(AudioCategory::Countdown, 90);
engine.set_category_volume(AudioCategory::Achievement, 85);
engine.set_category_volume(AudioCategory::Milestone, 60);

// Mute/unmute globally
engine.mute();
assert!(engine.is_muted());

engine.unmute();
assert!(!engine.is_muted());

// Toggle mute (returns new state)
let now_muted = engine.toggle_mute();

// Category-specific muting
engine.mute_category(AudioCategory::Milestone);
engine.unmute_category(AudioCategory::Milestone);

// Get mute state for UI
let mute_state = engine.get_mute_state();
println!("Status: {}", mute_state.display_string());
println!("Icon: {}", mute_state.icon_hint());
```

### Subscribing to Audio Events

```rust
use rustride::audio::AudioEvent;

let mut rx = engine.subscribe_events();

tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match event {
            AudioEvent::SpeechStarted { text } => {
                println!("Speaking: {}", text);
            }
            AudioEvent::SpeechCompleted => {
                println!("Speech finished");
            }
            AudioEvent::SoundPlayed { name } => {
                println!("Played sound: {}", name);
            }
            AudioEvent::ItemExpired { audio_type, age_ms } => {
                tracing::warn!("Audio expired after {}ms: {}", age_ms, audio_type);
            }
            AudioEvent::QueuePressure { current_size, max_size } => {
                tracing::warn!("Queue pressure: {}/{}", current_size, max_size);
            }
            AudioEvent::Error { message } => {
                tracing::error!("Audio error: {}", message);
            }
            _ => {}
        }
    }
});
```

### Device Status Monitoring

```rust
use rustride::audio::{Platform, HotPlugConfig};
use std::time::Duration;

// Get platform info
let platform = engine.get_platform();
println!("Platform: {:?}", platform);
println!("Audio backend: {}", platform.audio_backend_name());

// Check device availability
if !engine.is_device_available() {
    println!("Audio device not available");

    // Get troubleshooting hints
    for hint in engine.get_troubleshooting_hints() {
        println!("  - {}", hint);
    }

    // Attempt recovery
    engine.try_device_recovery();
}

// Configure hot-plug behavior
let hot_plug = HotPlugConfig {
    enabled: true,
    retry_interval: Duration::from_secs(10),
    max_consecutive_failures: 5,
    backoff_multiplier: 1.5,
    max_backoff: Duration::from_secs(120),
};
engine.set_hot_plug_config(hot_plug);

// Get detailed device status
let status = engine.get_device_status();
println!("Available: {}", status.available);
println!("State: {}", status.state_description());
println!("Recovery count: {}", status.recovery_count);
```

---

## API Reference

For complete API documentation, run:

```bash
cargo doc --open
```

Key modules:
- `rustride::audio` - Main audio module exports
- `rustride::audio::engine` - AudioEngine trait and DefaultAudioEngine
- `rustride::audio::backend` - RodioAudioBackend for low-level audio
- `rustride::audio::sounds` - SoundAsset catalog
- `rustride::audio::tones` - CuePattern and tone generation
- `rustride::audio::workout_bridge` - WorkoutAudioBridge for workout events
- `rustride::audio::achievement_bridge` - Achievement audio handling
- `rustride::audio::milestone_bridge` - Milestone celebration audio
