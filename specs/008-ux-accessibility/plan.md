# Implementation Plan: UX & Accessibility

**Branch**: `008-ux-accessibility` | **Date**: 2025-12-27 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/008-ux-accessibility/spec.md`

## Summary

Implement comprehensive UX improvements and accessibility features for the RustRide indoor cycling application. This includes keyboard navigation, unit preferences (Imperial/Metric), colorblind and high contrast modes, onboarding wizard, theme auto-detection, customizable UI layouts, audio feedback, TV/Flow display modes, internationalization (5 languages), screen reader support, voice control, and touch/gesture support. Target WCAG 2.1 AA compliance baseline with high contrast mode meeting AAA.

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**: egui 0.33, eframe 0.33 (GUI), rodio 0.17 (audio), tts 0.26 (text-to-speech), fluent-rs (i18n)
**Storage**: SQLite via rusqlite 0.31 (preferences, layout profiles, onboarding state)
**Testing**: cargo test, integration tests for accessibility validation
**Target Platform**: Windows, macOS, Linux desktop (cross-platform via eframe/wgpu)
**Project Type**: Single desktop application
**Performance Goals**: Theme/unit changes < 1 second, audio cues < 100ms latency, 60 fps UI
**Constraints**: WCAG 2.1 AA compliance, voice recognition 95% accuracy in quiet environments, all touch targets >= 44x44 pixels
**Scale/Scope**: 13 features (P1-P13), 39 functional requirements, 15 success criteria

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Gate | Status | Notes |
|------|--------|-------|
| Single desktop application | PASS | No new projects; extends existing RustRide |
| Library-first design | PASS | New modules (accessibility, i18n, onboarding) are self-contained |
| Test-first (TDD) | PASS | Will define tests before implementation |
| CLI exposure | N/A | GUI feature, not applicable |
| Observability | PASS | Use existing tracing infrastructure |
| Simplicity | PASS | Extends existing config/theme system; minimal new abstractions |

## Project Structure

### Documentation (this feature)

```text
specs/008-ux-accessibility/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── accessibility/           # NEW: Accessibility subsystem
│   ├── mod.rs              # Module exports
│   ├── focus.rs            # Focus management, keyboard navigation
│   ├── screen_reader.rs    # Screen reader integration (NVDA, VoiceOver, Orca)
│   ├── colorblind.rs       # Colorblind palette definitions
│   ├── high_contrast.rs    # High contrast theme (WCAG AAA)
│   └── voice_control.rs    # Voice command recognition
│
├── i18n/                   # NEW: Internationalization
│   ├── mod.rs              # Fluent-rs integration
│   ├── loader.rs           # Translation file loader
│   └── locales/            # Translation bundles (.ftl files)
│       ├── en-US/
│       ├── es/
│       ├── fr/
│       ├── de/
│       └── it/
│
├── onboarding/             # NEW: Onboarding wizard
│   ├── mod.rs              # Wizard state machine
│   ├── steps.rs            # Individual step definitions
│   └── glossary.rs         # Cycling terminology tooltips
│
├── ui/
│   ├── layout/             # NEW: Layout customization
│   │   ├── mod.rs          # Layout profile management
│   │   ├── editor.rs       # Drag-and-drop layout editor
│   │   └── profiles.rs     # Profile persistence (max 10)
│   ├── display_modes/      # NEW: TV Mode, Flow Mode
│   │   ├── mod.rs
│   │   ├── tv_mode.rs      # Large display optimization
│   │   └── flow_mode.rs    # Minimal distraction mode
│   ├── theme.rs            # EXTEND: Add system theme detection, colorblind palettes
│   └── widgets/            # EXTEND: Accessible widget variants
│
├── audio/                  # EXTEND: Add interval/zone audio cues
│   ├── alerts.rs           # EXTEND: Zone change, interval alerts
│   └── cues.rs             # NEW: Synthesized beep sounds
│
├── storage/
│   └── config.rs           # EXTEND: Add accessibility settings, layout profiles
│
└── input/                  # NEW: Input abstraction
    ├── mod.rs              # Unified input handling
    ├── keyboard.rs         # Keyboard shortcuts, focus management
    ├── touch.rs            # Touch gestures, pinch-zoom
    └── gestures.rs         # Swipe navigation

tests/
├── integration/
│   ├── accessibility/      # Keyboard nav, screen reader, focus tests
│   └── i18n/               # Translation coverage tests
└── unit/
    ├── colorblind_test.rs  # Palette distinguishability
    └── layout_test.rs      # Profile serialization
```

**Structure Decision**: Extend existing single-project structure with new modules for accessibility (`src/accessibility/`), internationalization (`src/i18n/`), onboarding (`src/onboarding/`), and input abstraction (`src/input/`). Layout customization integrates into `src/ui/layout/`. Display modes in `src/ui/display_modes/`.

## Complexity Tracking

No constitution violations requiring justification. All features integrate cleanly into existing architecture.
