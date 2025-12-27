# Research: UX & Accessibility

**Feature**: 008-ux-accessibility
**Date**: 2025-12-27

## 1. Keyboard Navigation in egui

### Decision
Use egui's built-in focus system with custom focus ring styling and shortcut handling via `egui::Context::input()`.

### Rationale
- egui provides native Tab/Shift+Tab navigation between widgets
- Focus can be managed programmatically via `Response::request_focus()` and `Context::memory().focus()`
- Custom focus indicators can be styled via `Visuals::selection.stroke`
- Keyboard shortcuts can be captured via `ctx.input(|i| i.key_pressed(Key::Escape))`

### Alternatives Considered
- **Custom focus system**: Rejected - would duplicate egui functionality and create maintenance burden
- **Third-party accessibility crate**: None mature enough for egui; egui's built-in system is sufficient

### Implementation Notes
- Create `FocusManager` to track focus order and handle modal focus trapping
- Implement `KeyboardShortcuts` registry for application-wide shortcut handling
- Add `?` key binding to show shortcut overlay

---

## 2. Screen Reader Integration (NVDA, VoiceOver, Orca)

### Decision
Use `accesskit` crate for cross-platform accessibility tree integration, which egui already supports via `eframe`.

### Rationale
- accesskit is the standard Rust accessibility crate, designed to work with egui/eframe
- Provides native integration with platform accessibility APIs (UI Automation on Windows, NSAccessibility on macOS, AT-SPI on Linux)
- egui has built-in accesskit support via `eframe::AccessKitActivation`
- Enables NVDA, VoiceOver, and Orca screen readers automatically

### Alternatives Considered
- **Direct platform APIs**: Rejected - would require platform-specific code for each OS
- **Custom TTS announcements only**: Rejected - doesn't integrate with screen reader navigation

### Implementation Notes
- Enable accesskit in eframe options: `native_options.accesskit = Some(AccessKitActivation::OnDemand)`
- Add accessible labels to all widgets using `.accessibility()`
- Use live regions for dynamic announcements (interval changes, metrics hotkey)
- Test with NVDA on Windows, VoiceOver on macOS, Orca on Linux

---

## 3. Voice Control / Speech Recognition

### Decision
Use `vosk` crate for offline speech recognition with fallback to system speech recognition on supported platforms.

### Rationale
- Vosk provides offline, local speech recognition (no cloud dependency)
- Small model size (~50MB) suitable for desktop application
- Cross-platform (Windows, macOS, Linux)
- Low latency suitable for real-time voice commands
- 95%+ accuracy achievable in quiet environments with limited command vocabulary

### Alternatives Considered
- **Cloud-based (Google/Azure Speech)**: Rejected - requires internet, privacy concerns, latency
- **Platform-native only (Windows Speech, macOS Dictation)**: Rejected - inconsistent API, not available on Linux
- **whisper.cpp**: Rejected - larger model size, higher latency for continuous listening

### Implementation Notes
- Use compact English model (~50MB download on first run)
- Implement wake-word detection or push-to-talk as options
- Command vocabulary: "start ride", "pause", "resume", "end ride", "skip interval"
- Display "Voice Unavailable" indicator if Vosk fails to initialize
- Add dependency: `vosk = "0.4"`

---

## 4. Internationalization (fluent-rs)

### Decision
Use `fluent-rs` for message formatting with Fluent Translation List (.ftl) files.

### Rationale
- fluent-rs is Mozilla's Rust implementation of Project Fluent
- Supports complex pluralization, gender, and locale-specific formatting
- .ftl files are easy to translate and version control
- Runtime language switching without restart
- Used by Firefox and other major projects

### Alternatives Considered
- **gettext/rust-gettext**: Rejected - older format, less expressive than Fluent
- **i18n-embed**: Rejected - embed-only approach, harder to update translations
- **Simple key-value JSON**: Rejected - lacks pluralization and formatting features

### Implementation Notes
- Add dependencies: `fluent = "0.16"`, `fluent-langneg = "0.14"`, `unic-langid = "0.9"`
- Store translations in `src/i18n/locales/{locale}/main.ftl`
- Initial languages: en-US (base), es, fr, de, it
- Create `I18n` struct with `get(key, args)` method for message lookup
- Detect system locale on first launch via `sys-locale` crate

---

## 5. Colorblind-Safe Palettes

### Decision
Use established colorblind-safe palettes from research, specifically Paul Tol's scheme and IBM Design colorblind palette.

### Rationale
- Paul Tol's color schemes are scientifically designed for color vision deficiency
- Tested and validated across protanopia, deuteranopia, and tritanopia
- Provides distinct colors for power zones (7 zones) that remain distinguishable
- High contrast variants available for WCAG AAA compliance

### Alternatives Considered
- **ColorBrewer palettes**: Good for maps but limited to fewer distinct colors
- **Custom palette design**: Rejected - requires extensive testing with colorblind users

### Palettes to Implement
1. **Default**: Current zone colors (no modification needed for normal vision)
2. **Protanopia**: Blue-yellow dominant, avoid red-green confusion
3. **Deuteranopia**: Similar to protanopia, different red-green confusion pattern
4. **Tritanopia**: Blue-yellow confusion, use red-green instead
5. **High Contrast**: Black background, maximum contrast colors meeting AAA (7:1 ratio)

### Implementation Notes
- Define `ColorPalette` enum with palette selection
- Store as user preference in accessibility settings
- Apply palette via `ZoneColors::for_palette(ColorPalette)` method
- Add pattern fills (stripes, dots) for charts as secondary indicator

---

## 6. System Theme Detection

### Decision
Use `dark-light` crate for cross-platform system theme detection.

### Rationale
- Simple, focused crate that detects system dark/light mode
- Supports Windows (registry), macOS (NSAppearance), Linux (GNOME, KDE, GTK settings)
- Provides callback for theme change notifications
- Minimal dependencies

### Alternatives Considered
- **Platform-specific code**: Rejected - maintenance burden across three platforms
- **Polling system settings**: Rejected - inefficient, delayed response

### Implementation Notes
- Add dependency: `dark-light = "1.0"`
- Poll every 5 seconds (or use platform notification if available)
- `ThemePreference` enum: `FollowSystem`, `Light`, `Dark`
- Smooth theme transition during rides (don't interrupt data display)

---

## 7. Touch Gesture Support

### Decision
Use egui's built-in touch and multi-touch support via `egui::PointerState` and gesture detection.

### Rationale
- egui supports touch input natively on Windows (touch-enabled monitors) and via wgpu
- Multi-touch gestures (pinch-zoom) can be detected via `ctx.multi_touch()`
- Minimum touch target size can be enforced via widget sizing

### Alternatives Considered
- **Custom touch layer**: Rejected - duplicates egui functionality
- **Platform-specific touch APIs**: Rejected - egui abstraction is sufficient

### Implementation Notes
- Enforce 44x44 minimum hit area on all buttons via custom `AccessibleButton` widget
- Implement pinch-zoom for graphs via `MultiTouchInfo::zoom_delta()`
- Swipe detection: track touch start/end positions, calculate direction
- Add `TouchGestureHandler` for swipe navigation between screens

---

## 8. Audio Cue Synthesis

### Decision
Use `rodio` (already in dependencies) for audio playback with synthesized tones via `rodio::source::SineWave`.

### Rationale
- rodio already in project dependencies for audio playback
- SineWave source can generate distinct tones without external audio files
- Low latency (<100ms) achievable with direct audio device access
- Cross-platform support

### Alternatives Considered
- **Pre-recorded WAV files**: Acceptable alternative, but increases binary size
- **MIDI synthesis**: Overkill for simple beep sounds

### Implementation Notes
- Interval transition: 440Hz (A4) for 200ms
- Zone change up: Ascending two-tone (440Hz → 523Hz)
- Zone change down: Descending two-tone (523Hz → 440Hz)
- Independent volume control stored in preferences
- Respect system mute state

---

## 9. Layout Profile Persistence

### Decision
Use existing SQLite database with new `layout_profiles` table.

### Rationale
- SQLite already used for ride history and sensor data
- Structured storage better than TOML for multiple named profiles
- Supports atomic updates and constraints (max 10 profiles)

### Alternatives Considered
- **TOML file per profile**: Rejected - file management complexity, no atomic updates
- **JSON blob in config**: Rejected - harder to query and manage limits

### Implementation Notes
- Table: `layout_profiles(id, name, layout_json, is_default, created_at, updated_at)`
- Layout JSON: `{ "widgets": [{ "type": "power", "x": 0, "y": 0, "w": 2, "h": 1 }] }`
- Enforce max 10 profiles at application layer with clear error message
- One default profile always exists, cannot be deleted

---

## 10. Onboarding Wizard State Machine

### Decision
Implement a simple finite state machine for wizard steps with persistence in SQLite.

### Rationale
- Wizard has linear flow with optional skip/resume
- State can be serialized as current step index plus completion flags
- SQLite persistence allows resume after app restart

### Alternatives Considered
- **In-memory only**: Rejected - loses progress on crash/restart
- **Complex state machine library**: Overkill for linear wizard flow

### Implementation Notes
- Steps: Welcome → Sensor Setup → Profile Setup → FTP Configuration → UI Tour → Complete
- Each step has: title, description, actions, skip option
- Store: `onboarding_state(user_id, current_step, completed, skipped_at)`
- Glossary tooltips via `RichText::on_hover_text()` for terms like "FTP", "FTMS", "ANT+"

---

## Dependencies Summary

### New Dependencies
```toml
# Accessibility
accesskit = "0.13"  # Already supported by eframe

# Voice Recognition (optional feature)
vosk = "0.4"

# Internationalization
fluent = "0.16"
fluent-langneg = "0.14"
unic-langid = "0.9"
sys-locale = "0.3"

# Theme Detection
dark-light = "1.0"
```

### Existing Dependencies (no changes needed)
- rodio 0.17 - Audio cues
- rusqlite 0.31 - Layout profile storage
- egui 0.33 - UI framework with touch and accessibility support
- tts 0.26 - Text-to-speech for screen reader fallback
