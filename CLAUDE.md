# RustRide - Claude Code Context

## Project Overview

RustRide is an open-source, self-hosted indoor cycling training application built in Rust. It provides BLE sensor connectivity (smart trainers, power meters, HR monitors), structured workout execution with ERG mode control, real-time metrics display, ride recording, and export to standard formats (.fit, .tcx).

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust stable (1.75+) |
| GUI Framework | egui + eframe (0.33+) |
| Accessibility | accesskit (built into eframe) |
| BLE Communication | btleplug |
| Audio | rodio |
| Voice Control | vosk (optional feature) |
| Async Runtime | tokio |
| Database | SQLite via rusqlite |
| Configuration | TOML via toml crate |
| Internationalization | fluent, fluent-langneg, unic-langid |
| Theme Detection | dark-light |
| Logging | tracing |
| Serialization | serde |

## Project Structure

```
src/
├── main.rs              # Application entry point
├── app.rs               # egui application state
├── lib.rs               # Library exports
├── accessibility/       # Accessibility features (WCAG 2.1 AA/AAA)
│   ├── colorblind.rs    # Colorblind-safe palettes
│   ├── focus.rs         # Keyboard focus management
│   ├── high_contrast.rs # High contrast mode
│   ├── screen_reader.rs # Screen reader support (accesskit)
│   └── voice_control.rs # Voice control (optional Vosk feature)
├── audio/               # Audio system (rodio)
│   ├── engine.rs        # Audio engine
│   └── tones.rs         # Tone generation for cues
├── i18n/                # Internationalization (fluent)
├── input/               # Input handling
│   ├── keyboard.rs      # Keyboard shortcuts
│   ├── touch.rs         # Touch input
│   └── gestures.rs      # Gesture recognition
├── onboarding/          # First-run onboarding wizard
├── sensors/             # BLE/ANT+ sensor management
├── workouts/            # Workout parsing and execution
├── recording/           # Ride recording and file export
├── metrics/             # Training metric calculations
├── storage/             # SQLite and config persistence
└── ui/                  # egui screens and widgets
    ├── screens/         # Main application screens
    ├── widgets/         # Reusable UI components
    ├── display_modes/   # Flow mode, TV mode
    ├── layout/          # Widget layout management
    └── theme.rs         # Theme detection and management

tests/
├── unit/               # Unit tests
├── integration/        # Integration tests
└── fixtures/           # Test data (workouts, rides)

specs/                  # Feature specifications
├── 001-indoor-cycling-app/  # MVP cycling features
├── 008-ux-accessibility/    # UX & Accessibility features
│   ├── spec.md         # Feature specification
│   ├── plan.md         # Implementation plan
│   └── tasks.md        # Implementation tasks
```

## Key Modules

### sensors/
BLE sensor discovery and connection using btleplug. Supports:
- FTMS (Fitness Machine Service) for smart trainers
- Cycling Power Service for power meters
- Heart Rate Service for HR monitors

### workouts/
Workout file parsing (.zwo, .mrc) and execution engine. Manages:
- Workout state machine (start, pause, resume, skip)
- ERG mode power target calculations
- Interval timing and transitions

### recording/
Ride data capture and export. Features:
- 1-second sample recording
- Auto-save for crash recovery (30-second intervals)
- Export to TCX, FIT, GPX, CSV formats

### metrics/
Training metric calculations:
- Normalized Power (NP)
- Training Stress Score (TSS)
- Intensity Factor (IF)
- Power and HR zone calculations

### storage/
Data persistence:
- SQLite for rides, workouts, sensors
- TOML for user configuration
- Auto-migrations on startup

## Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo test               # Run all tests
cargo test sensors       # Test specific module

# Run
cargo run                # Debug mode
cargo run --release      # Release mode

# Code quality
cargo fmt                # Format code
cargo clippy             # Lint
```

## Important Patterns

### Thread Communication
- Use `crossbeam::channel` for sensor data (BLE → UI)
- Use `Arc<Mutex<T>>` for shared state (ride samples)
- Call `ctx.request_repaint()` from background threads to wake UI

### BLE Protocol
- FTMS Control Point writes for ERG mode: `[0x05, watts_lo, watts_hi]`
- Always request control (`[0x00]`) before setting targets
- Handle automatic reconnection on signal loss

### Metrics Calculations
- Normalized Power: 30s rolling average → 4th power → mean → 4th root
- TSS = (hours × IF²) × 100
- Filter power spikes > 2000W as noise

## Configuration Locations

- **Windows**: `%APPDATA%\rustride\`
- **macOS**: `~/Library/Application Support/rustride/`
- **Linux**: `~/.config/rustride/` (config), `~/.local/share/rustride/` (data)

## Accessibility Features

The application follows WCAG 2.1 AA/AAA guidelines:

### Keyboard Navigation
- Tab/Shift+Tab for focus navigation
- Enter/Space for button activation
- Escape to close modals
- F1/? to show keyboard shortcuts overlay
- Ctrl+M for screen reader metrics announcement

### Visual Accessibility
- High contrast mode (7:1 WCAG AAA ratio)
- Colorblind-safe palettes (deuteranopia, protanopia, tritanopia)
- System theme detection and follow (dark/light)
- Minimum 44x44px touch targets

### Screen Reader Support
- AccessKit integration (NVDA, VoiceOver, Orca)
- Live regions for interval changes
- Immediate alert announcements
- Focus management for modal dialogs

### Voice Control (optional)
Enable with `--features voice-control`:
- Commands: start, pause, resume, end, skip
- Increase/decrease power targets
- Status announcements

### Touch/Gesture Support
- Swipe navigation between screens
- Pinch-zoom for charts
- 44x44 minimum touch targets
- Visual touch feedback (ripple/scale/highlight)

## Current Feature: 001-indoor-cycling-app

MVP implementation covering:
- P1: Connect smart trainer and start free ride
- P2: Execute structured workouts with ERG mode
- P3: Record and export ride data
- P4: Real-time training metrics display
- P5: User profile and training zones
- P6: Ride history browsing
- P7: Multi-sensor support

See `specs/001-indoor-cycling-app/` for full specifications.
