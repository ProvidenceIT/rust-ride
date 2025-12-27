# Quickstart: UX & Accessibility Feature

**Feature**: 008-ux-accessibility
**Date**: 2025-12-27

## Prerequisites

- Rust 1.75+ stable toolchain
- Platform: Windows 10+, macOS 12+, or Linux with X11/Wayland
- For voice control testing: Working microphone

## Setup

### 1. Switch to Feature Branch

```bash
git checkout 008-ux-accessibility
```

### 2. Install New Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Voice Recognition (optional)
vosk = { version = "0.4", optional = true }

# Internationalization
fluent = "0.16"
fluent-langneg = "0.14"
unic-langid = "0.9"
sys-locale = "0.3"

# Theme Detection
dark-light = "1.0"

[features]
voice-control = ["vosk"]
```

### 3. Download Voice Model (Optional)

For voice control support, download the Vosk model on first run:

```bash
# Models are downloaded automatically, or manually:
# https://alphacephei.com/vosk/models
# Place in: ~/.config/rustride/vosk-model/
```

### 4. Build

```bash
# Standard build
cargo build

# With voice control
cargo build --features voice-control

# Release build
cargo build --release
```

### 5. Run Tests

```bash
# All tests
cargo test

# Accessibility tests only
cargo test accessibility

# I18n tests only
cargo test i18n

# Layout tests only
cargo test layout
```

## Development Workflow

### Creating New Modules

The feature adds these new module directories:

```
src/
├── accessibility/   # Focus, screen reader, colorblind, voice
├── i18n/           # Internationalization
├── onboarding/     # First-run wizard
├── input/          # Keyboard, touch, gesture handling
└── ui/
    ├── layout/         # Custom dashboard layouts
    └── display_modes/  # TV Mode, Flow Mode
```

### Adding Translations

1. Create translation file: `src/i18n/locales/{locale}/main.ftl`
2. Add messages in Fluent format:

```ftl
# src/i18n/locales/es/main.ftl
button-start-ride = Iniciar Paseo
ride-duration = { $hours } horas, { $minutes } minutos
```

3. Use in code:

```rust
use crate::i18n::t;

let label = t!("button-start-ride");
let duration = t!("ride-duration", "hours" => 1, "minutes" => 30);
```

### Testing Accessibility

1. **Keyboard Navigation**: Unplug mouse, use Tab/Shift+Tab to navigate
2. **Screen Reader**: Test with NVDA (Windows), VoiceOver (macOS), or Orca (Linux)
3. **Colorblind**: Use browser extensions or OS colorblind simulation

```bash
# Run accessibility validation
cargo test --test accessibility_validation
```

### Testing Display Modes

```rust
// Toggle TV Mode
app.display_mode_manager.toggle_tv_mode();

// Enter Flow Mode
app.display_mode_manager.enter_flow_mode();

// Hotkeys: T for TV Mode, F for Flow Mode
```

## Key Files

| File | Purpose |
|------|---------|
| `src/accessibility/mod.rs` | Accessibility module entry |
| `src/accessibility/focus.rs` | Focus manager implementation |
| `src/accessibility/colorblind.rs` | Color palette definitions |
| `src/i18n/mod.rs` | Translation service |
| `src/i18n/locales/` | Translation files (.ftl) |
| `src/onboarding/mod.rs` | Onboarding wizard state machine |
| `src/ui/layout/profiles.rs` | Layout profile storage |
| `src/ui/display_modes/flow_mode.rs` | Flow Mode renderer |
| `src/storage/config.rs` | Extended preferences (modify) |
| `src/ui/theme.rs` | Extended theme system (modify) |

## Database Migrations

New tables are created on first run:

```sql
-- Run via src/storage/migrations.rs
CREATE TABLE layout_profiles (...);
CREATE TABLE onboarding_state (...);
CREATE TABLE user_preferences (...);
```

## Testing Checklist

- [ ] Tab navigation works through all screens
- [ ] Focus indicators visible on all interactive elements
- [ ] Escape closes modals/overlays
- [ ] ? key shows shortcut overlay
- [ ] Unit toggle changes all metrics immediately
- [ ] Colorblind modes apply without restart
- [ ] High contrast meets 7:1 ratio
- [ ] Screen reader announces button labels
- [ ] Screen reader announces interval changes
- [ ] Metrics hotkey announces current values
- [ ] Voice commands recognized (if enabled)
- [ ] "Voice Unavailable" shown when needed
- [ ] Layout profiles save/load correctly
- [ ] Max 10 profiles enforced
- [ ] TV Mode scales text appropriately
- [ ] Flow Mode shows single metric
- [ ] Audio cues play on intervals
- [ ] Language switch works at runtime
- [ ] Onboarding shows for new users
- [ ] Onboarding can be skipped/resumed

## Debugging

### Common Issues

**No audio cues playing**
- Check system volume
- Verify `audio.enabled = true` in settings
- Check `rodio` device initialization

**Voice control unavailable**
- Ensure built with `--features voice-control`
- Check microphone permissions
- Verify Vosk model is downloaded

**Screen reader not announcing**
- Enable accesskit in eframe options
- Check accessible labels on widgets
- Test with actual screen reader, not simulated

**Theme not following system**
- Verify `dark-light` crate is detecting correctly
- Check `theme_preference = FollowSystem` in settings

### Logging

Enable verbose logging:

```bash
RUST_LOG=rustride=debug cargo run
```

Key log prefixes:
- `accessibility::` - Focus and screen reader events
- `i18n::` - Translation loading
- `audio::` - Audio cue playback
- `layout::` - Profile changes

## Performance Targets

| Operation | Target |
|-----------|--------|
| Theme change | < 1 second |
| Unit conversion | < 100ms |
| Audio cue latency | < 100ms |
| Focus navigation | < 16ms (60fps) |
| Language switch | < 500ms |

## Reference

- [Spec](./spec.md) - Feature requirements
- [Data Model](./data-model.md) - Entity definitions
- [Research](./research.md) - Technology decisions
- [Contracts](./contracts/) - API definitions
