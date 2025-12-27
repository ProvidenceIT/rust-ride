# Tasks: UX & Accessibility

**Input**: Design documents from `/specs/008-ux-accessibility/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and module structure for UX & Accessibility features

- [x] T001 Add new dependencies to Cargo.toml (fluent, fluent-langneg, unic-langid, sys-locale, dark-light)
- [x] T002 [P] Create src/accessibility/mod.rs with module structure and exports
- [x] T003 [P] Create src/i18n/mod.rs with module structure and exports
- [x] T004 [P] Create src/onboarding/mod.rs with module structure and exports
- [x] T005 [P] Create src/input/mod.rs with module structure and exports
- [x] T006 [P] Create src/ui/layout/mod.rs with module structure and exports
- [x] T007 [P] Create src/ui/display_modes/mod.rs with module structure and exports
- [x] T008 Register all new modules in src/lib.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T009 Extend AccessibilitySettings struct in src/storage/config.rs per data-model.md
- [x] T010 [P] Extend AudioSettings struct in src/storage/config.rs per data-model.md
- [x] T011 [P] Add ThemePreference enum (FollowSystem, Light, Dark) to src/storage/config.rs
- [x] T012 [P] Add LocaleSettings struct to src/storage/config.rs per data-model.md
- [x] T013 [P] Add DisplayMode and FlowModeSettings to src/storage/config.rs per data-model.md
- [x] T014 Extend UserPreferences struct to include all new settings in src/storage/config.rs
- [x] T015 Create database migration for layout_profiles table in src/storage/migrations.rs
- [x] T016 [P] Create database migration for onboarding_state table in src/storage/migrations.rs
- [x] T017 [P] Create database migration for user_preferences table in src/storage/migrations.rs
- [x] T018 Create LayoutProfile struct and WidgetPlacement in src/ui/layout/profiles.rs
- [x] T019 Implement layout profile CRUD operations in src/ui/layout/profiles.rs (max 10 limit)

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Keyboard-Only Navigation (Priority: P1) 🎯 MVP

**Goal**: Enable full keyboard navigation through all application screens with visible focus indicators

**Independent Test**: Unplug mouse and navigate all screens using only Tab, Shift+Tab, Enter, Space, arrow keys

### Implementation for User Story 1

- [x] T020 [P] [US1] Create FocusManager struct for tracking focus order in src/accessibility/focus.rs
- [x] T021 [P] [US1] Implement FocusIndicatorStyle enum and focus ring rendering in src/accessibility/focus.rs
- [x] T022 [US1] Implement Tab/Shift+Tab navigation handler in src/input/keyboard.rs
- [x] T023 [US1] Implement Enter/Space button activation in src/input/keyboard.rs
- [x] T024 [US1] Implement Escape key for modal/overlay closing in src/input/keyboard.rs
- [x] T025 [US1] Create KeyboardShortcuts registry in src/input/keyboard.rs
- [x] T026 [US1] Implement shortcut guide overlay (? or F1 key) in src/ui/widgets/shortcut_overlay.rs
- [x] T027 [US1] Add focus trap for modal dialogs in src/accessibility/focus.rs
- [x] T028 [US1] Update all existing button widgets to support focus in src/ui/widgets/
- [x] T029 [US1] Integrate focus manager with main app loop in src/app.rs

**Checkpoint**: User Story 1 complete - keyboard navigation fully functional

---

## Phase 4: User Story 2 - Imperial/Metric Unit Toggle (Priority: P2)

**Goal**: Allow users to toggle between Imperial and Metric unit systems with immediate display updates

**Independent Test**: Switch units and verify speed, distance, elevation display correctly in both systems

### Implementation for User Story 2

- [x] T030 [US2] Extend Units enum with conversion methods in src/storage/config.rs
- [x] T031 [US2] Add unit preference UI toggle in src/ui/screens/settings.rs
- [x] T032 [US2] Implement format_speed() with unit awareness in src/metrics/calculator.rs
- [x] T033 [US2] Implement format_distance() with unit awareness in src/metrics/calculator.rs
- [x] T034 [US2] Implement format_elevation() with unit awareness in src/metrics/calculator.rs
- [x] T035 [US2] Update metric_display widget to use unit-aware formatting in src/ui/widgets/metric_display.rs
- [x] T036 [US2] Update ride export to include unit metadata in src/recording/export.rs
- [x] T037 [US2] Ensure unit changes propagate immediately without restart in src/app.rs

**Checkpoint**: User Story 2 complete - unit toggle works throughout application

---

## Phase 5: User Story 3 - High Contrast and Colorblind Modes (Priority: P3)

**Goal**: Provide colorblind-safe palettes and high contrast mode meeting WCAG AAA

**Independent Test**: Enable each colorblind mode and verify zone colors are distinguishable

### Implementation for User Story 3

- [x] T038 [P] [US3] Create ColorMode enum (Normal, Protanopia, Deuteranopia, Tritanopia) in src/accessibility/colorblind.rs
- [x] T039 [P] [US3] Define protanopia-safe color palette in src/accessibility/colorblind.rs
- [x] T040 [P] [US3] Define deuteranopia-safe color palette in src/accessibility/colorblind.rs
- [x] T041 [P] [US3] Define tritanopia-safe color palette in src/accessibility/colorblind.rs
- [x] T042 [US3] Implement ColorPaletteProvider trait in src/accessibility/colorblind.rs
- [x] T043 [US3] Create HighContrastTheme with WCAG AAA ratios (7:1) in src/accessibility/high_contrast.rs
- [x] T044 [US3] Add contrast ratio calculation utility in src/accessibility/high_contrast.rs
- [x] T045 [US3] Update zone_colors module to use active palette in src/ui/theme.rs
- [ ] T046 [US3] Add pattern fills for charts (stripes, dots) as secondary indicators in src/ui/widgets/
- [x] T047 [US3] Add colorblind/contrast mode UI in accessibility settings screen in src/ui/screens/settings.rs
- [x] T048 [US3] Ensure immediate mode switch without restart in src/app.rs

**Checkpoint**: User Story 3 complete - all colorblind and high contrast modes functional

---

## Phase 6: User Story 4 - Onboarding Tutorial (Priority: P4)

**Goal**: Guide first-time users through sensor setup, profile, FTP, and UI with glossary tooltips

**Independent Test**: Reset user data and complete full onboarding flow

### Implementation for User Story 4

- [x] T049 [P] [US4] Create OnboardingState struct and persistence in src/onboarding/mod.rs
- [x] T050 [P] [US4] Create OnboardingStep enum (Welcome, SensorSetup, ProfileSetup, FtpConfiguration, UiTour, Complete) in src/onboarding/steps.rs
- [x] T051 [US4] Implement OnboardingWizard state machine in src/onboarding/mod.rs
- [x] T052 [US4] Create Welcome step UI in src/onboarding/steps.rs
- [x] T053 [US4] Create SensorSetup step UI with discovery integration in src/onboarding/steps.rs
- [x] T054 [US4] Create ProfileSetup step UI in src/onboarding/steps.rs
- [x] T055 [US4] Create FtpConfiguration step UI in src/onboarding/steps.rs
- [x] T056 [US4] Create UiTour step UI in src/onboarding/steps.rs
- [x] T057 [US4] Implement glossary tooltips for cycling terms (FTP, FTMS, ANT+, TSS) in src/onboarding/glossary.rs
- [x] T058 [US4] Add skip/resume/restart functionality in src/onboarding/mod.rs
- [x] T059 [US4] Integrate onboarding check on app startup in src/app.rs
- [x] T060 [US4] Add "Restart Onboarding" option in settings screen in src/ui/screens/settings.rs

**Checkpoint**: User Story 4 complete - onboarding wizard fully functional

---

## Phase 7: User Story 5 - Dark/Light Theme Auto-Detection (Priority: P5)

**Goal**: Automatically follow system theme with smooth transitions and manual override

**Independent Test**: Change system theme and verify app theme changes within 5 seconds

### Implementation for User Story 5

- [x] T061 [US5] Integrate dark-light crate for system theme detection in src/ui/theme.rs
- [x] T062 [US5] Implement ThemePreference enum handling (FollowSystem, Light, Dark) in src/ui/theme.rs
- [x] T063 [US5] Add system theme polling (5 second interval) in src/ui/theme.rs
- [x] T064 [US5] Implement smooth theme transition animation in src/ui/theme.rs
- [x] T065 [US5] Add theme preference UI (Follow System / Light / Dark) in src/ui/screens/settings.rs
- [x] T066 [US5] Ensure theme changes don't disrupt active ride display in src/ui/screens/ride.rs

**Checkpoint**: User Story 5 complete - theme auto-detection works seamlessly

---

## Phase 8: User Story 6 - Customizable UI Layout (Priority: P6)

**Goal**: Enable drag-and-drop widget arrangement with named profiles (max 10)

**Independent Test**: Enter edit mode, rearrange widgets, save profile, verify persistence across restart

### Implementation for User Story 6

- [x] T067 [US6] Create LayoutEditor struct for edit mode in src/ui/layout/editor.rs
- [x] T068 [US6] Implement drag-and-drop widget movement in src/ui/layout/editor.rs
- [x] T069 [US6] Implement widget resize functionality in src/ui/layout/editor.rs
- [x] T070 [US6] Add collision detection for widget placement in src/ui/layout/editor.rs
- [x] T071 [US6] Create layout profile save/load UI in src/ui/layout/mod.rs
- [x] T072 [US6] Implement profile naming dialog in src/ui/layout/mod.rs
- [x] T073 [US6] Implement profile deletion with confirmation in src/ui/layout/mod.rs
- [x] T074 [US6] Implement LayoutRenderer for dashboard in src/ui/layout/mod.rs
- [x] T075 [US6] Add layout profile selector to ride screen in src/ui/layout/mod.rs
- [x] T076 [US6] Ensure default profile always exists and cannot be deleted in src/ui/layout/profiles.rs

**Checkpoint**: User Story 6 complete - layout customization fully functional

---

## Phase 9: User Story 7 - Audio Feedback System (Priority: P7)

**Goal**: Provide audio cues for interval transitions and zone changes with volume control

**Independent Test**: Enable audio feedback, start workout with intervals, verify sounds play

### Implementation for User Story 7

- [x] T077 [P] [US7] Create ToneGenerator using rodio SineWave in src/audio/tones.rs
- [x] T078 [P] [US7] Define tone frequencies and durations for cues in src/audio/tones.rs
- [x] T079 [US7] Implement AudioCueSystem trait in src/audio/alerts.rs
- [x] T080 [US7] Add interval transition cue (IntervalTransition enum) in src/audio/alerts.rs
- [x] T081 [US7] Add zone change cue (ascending/descending tones) in src/audio/tones.rs
- [x] T082 [US7] Implement ZoneChangeDetector with debouncing in src/audio/tones.rs
- [x] T083 [US7] Add volume control independent of system volume in src/audio/tones.rs
- [x] T084 [US7] Add audio settings UI in src/ui/screens/settings.rs
- [x] T085 [US7] Integrate audio cues with workout engine in src/workouts/engine.rs
- [x] T086 [US7] Respect system mute state in src/audio/tones.rs

**Checkpoint**: User Story 7 complete - audio feedback works for intervals and zones

---

## Phase 10: User Story 8 - Large Display / TV Mode (Priority: P8)

**Goal**: Optimize interface for 65"+ displays at 3+ meter viewing distance

**Independent Test**: Enable TV Mode and verify all text readable from 3 meters

### Implementation for User Story 8

- [x] T087 [US8] Create TvModeRenderer with font scaling (48pt+ primary) in src/ui/display_modes/tv_mode.rs
- [x] T088 [US8] Define TvModeLayout with simplified metrics in src/ui/display_modes/tv_mode.rs
- [x] T089 [US8] Implement enlarged button sizes for TV Mode in src/ui/display_modes/tv_mode.rs
- [x] T090 [US8] Add TV Mode toggle hotkey (T key) in src/input/keyboard.rs
- [x] T091 [US8] Implement metrics de-emphasis/hiding for clutter reduction in src/ui/display_modes/tv_mode.rs
- [x] T092 [US8] Add TV Mode toggle in settings UI in src/ui/screens/settings.rs
- [x] T093 [US8] Ensure smooth transition between Normal and TV Mode in src/ui/display_modes/mod.rs

**Checkpoint**: User Story 8 complete - TV Mode optimized for large displays

---

## Phase 11: User Story 9 - Flow Mode / Minimal Distractions (Priority: P9)

**Goal**: Single large metric display with optional 3D world background

**Independent Test**: Activate Flow Mode mid-ride, verify only selected metric displays

### Implementation for User Story 9

- [x] T094 [US9] Create FlowModeRenderer in src/ui/display_modes/flow_mode.rs
- [x] T095 [US9] Implement single-metric centered display in src/ui/display_modes/flow_mode.rs
- [x] T096 [US9] Add Flow Mode settings (primary metric, world background) in src/ui/display_modes/flow_mode.rs
- [x] T097 [US9] Implement brief interval notification overlay with fade in src/ui/display_modes/flow_mode.rs
- [x] T098 [US9] Add Flow Mode toggle hotkey (F key) in src/input/keyboard.rs
- [x] T099 [US9] Add metric cycle hotkey (M key) for Flow Mode in src/input/keyboard.rs
- [ ] T100 [US9] Integrate 3D world expansion in Flow Mode in src/ui/display_modes/flow_mode.rs
- [x] T101 [US9] Add Escape or any input to exit Flow Mode in src/input/keyboard.rs

**Checkpoint**: User Story 9 complete - Flow Mode provides minimal distraction experience

---

## Phase 12: User Story 10 - Multi-Language Support (Priority: P10)

**Goal**: Runtime language switching with 5 languages (EN, ES, FR, DE, IT)

**Independent Test**: Switch to each language and verify all UI text displays correctly

### Implementation for User Story 10

- [x] T102 [P] [US10] Setup fluent-rs integration in src/i18n/mod.rs
- [x] T103 [P] [US10] Create TranslationLoader for .ftl files in src/i18n/loader.rs
- [x] T104 [P] [US10] Create English (en-US) translation file in src/i18n/locales/en-US/main.ftl
- [x] T105 [P] [US10] Create Spanish translation file in src/i18n/locales/es/main.ftl
- [x] T106 [P] [US10] Create French translation file in src/i18n/locales/fr/main.ftl
- [x] T107 [P] [US10] Create German translation file in src/i18n/locales/de/main.ftl
- [x] T108 [P] [US10] Create Italian translation file in src/i18n/locales/it/main.ftl
- [x] T109 [US10] Implement system locale detection via sys-locale in src/i18n/mod.rs
- [x] T110 [US10] Implement runtime language switching without restart in src/i18n/mod.rs
- [x] T111 [US10] Implement English fallback for missing translations in src/i18n/mod.rs
- [x] T112 [US10] Create t!() macro for translation lookups in src/i18n/mod.rs
- [x] T113 [US10] Add language selector UI in settings screen in src/ui/screens/settings.rs
- [ ] T114 [US10] Replace all hardcoded UI strings with t!() calls across src/ui/

**Checkpoint**: User Story 10 complete - application fully localized in 5 languages

---

## Phase 13: User Story 11 - Screen Reader Support (Priority: P11)

**Goal**: Full accessibility via NVDA, VoiceOver, Orca with dynamic announcements

**Independent Test**: Complete full ride workflow using screen reader without visual monitor

### Implementation for User Story 11

- [x] T115 [US11] Enable accesskit in eframe native options in src/main.rs
- [x] T116 [US11] Create ScreenReaderSupport trait implementation in src/accessibility/screen_reader.rs
- [x] T117 [US11] Add accessible labels to all buttons in src/ui/widgets/
- [x] T118 [US11] Add accessible labels to all form inputs in src/ui/widgets/
- [x] T119 [US11] Implement live region for interval change announcements in src/accessibility/screen_reader.rs
- [x] T120 [US11] Implement metrics hotkey (e.g., Ctrl+M) for on-demand announcement in src/accessibility/screen_reader.rs
- [x] T121 [US11] Add accessible labels to metric displays in src/ui/widgets/metric_display.rs
- [x] T122 [US11] Ensure all alerts/errors announced immediately in src/accessibility/screen_reader.rs
- [x] T123 [US11] Test and fix focus order for screen reader navigation in src/accessibility/focus.rs

**Checkpoint**: User Story 11 complete - screen reader users can complete full workflows

---

## Phase 14: User Story 12 - Voice Control (Priority: P12)

**Goal**: Hands-free control via voice commands with 95% accuracy

**Independent Test**: Issue all voice commands and verify correct responses without keyboard/mouse

### Implementation for User Story 12

- [x] T124 [US12] Add vosk as optional feature dependency in Cargo.toml
- [x] T125 [US12] Create VoiceControl trait and VoiceCommand enum in src/accessibility/voice_control.rs
- [x] T126 [US12] Implement Vosk model initialization (download on first run) in src/accessibility/voice_control.rs
- [x] T127 [US12] Implement command vocabulary recognition (start, pause, resume, end, skip) in src/accessibility/voice_control.rs
- [x] T128 [US12] Add VoiceControlState enum (Ready, Listening, Unavailable) in src/accessibility/voice_control.rs
- [x] T129 [US12] Implement "Voice Unavailable" indicator UI in src/ui/widgets/voice_indicator.rs
- [x] T130 [US12] Add visual/audio confirmation of recognized commands in src/accessibility/voice_control.rs
- [x] T131 [US12] Add voice control settings (enable, activation mode) in src/ui/screens/settings.rs
- [x] T132 [US12] Integrate voice commands with ride control in src/app.rs
- [x] T133 [US12] Handle graceful degradation when voice unavailable in src/accessibility/voice_control.rs

**Checkpoint**: User Story 12 complete - voice control functional for all supported commands

---

## Phase 15: User Story 13 - Touch/Gesture Support (Priority: P13)

**Goal**: Full touch support with 44x44 minimum targets, swipe nav, pinch-zoom

**Independent Test**: Complete all workflows using touch-only input on touchscreen device

### Implementation for User Story 13

- [x] T134 [US13] Create TouchGestureHandler in src/input/touch.rs
- [x] T135 [US13] Implement swipe detection (direction, distance) in src/input/gestures.rs
- [x] T136 [US13] Implement pinch-zoom detection via multi_touch() in src/input/gestures.rs
- [x] T137 [US13] Create AccessibleButton widget wrapper ensuring 44x44 minimum in src/ui/widgets/accessible_button.rs
- [x] T138 [US13] Implement swipe navigation between screens in src/input/gestures.rs
- [ ] T139 [US13] Implement pinch-zoom for graphs in src/ui/widgets/
- [ ] T140 [US13] Update all buttons to use AccessibleButton wrapper in src/ui/widgets/
- [ ] T141 [US13] Add touch feedback visual effects in src/ui/widgets/accessible_button.rs
- [ ] T142 [US13] Integrate gesture handler with main app input loop in src/app.rs

**Checkpoint**: User Story 13 complete - full touch/gesture support functional

---

## Phase 16: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T143 [P] Update CLAUDE.md with new modules and dependencies
- [ ] T144 [P] Verify all 44x44 touch targets across application
- [ ] T145 [P] Verify WCAG 2.1 AA compliance across all screens
- [ ] T146 Run all translation files through validation (< 1% missing)
- [ ] T147 Performance test: theme changes < 1 second
- [ ] T148 Performance test: audio cues < 100ms latency
- [ ] T149 Integration test: keyboard navigation through all screens
- [ ] T150 Integration test: screen reader workflow completion
- [ ] T151 Run quickstart.md validation checklist

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-15)**: All depend on Foundational phase completion
  - Stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → ... → P13)
- **Polish (Phase 16)**: Depends on all user stories being complete

### User Story Dependencies

| Story | Dependencies | Notes |
|-------|--------------|-------|
| US1 (Keyboard Nav) | Foundational only | Core accessibility, enables US11 |
| US2 (Units) | Foundational only | Independent feature |
| US3 (Colorblind) | Foundational only | Independent feature |
| US4 (Onboarding) | Foundational only | May use US1 keyboard nav |
| US5 (Theme) | Foundational only | Independent feature |
| US6 (Layout) | Foundational only | Uses T018-T019 from Foundational |
| US7 (Audio) | Foundational only | Independent feature |
| US8 (TV Mode) | Foundational only | Independent feature |
| US9 (Flow Mode) | Foundational only | May integrate with US8 |
| US10 (i18n) | Foundational only | Touches all UI strings |
| US11 (Screen Reader) | US1 (keyboard nav) | Builds on focus management |
| US12 (Voice) | Foundational only | Independent feature |
| US13 (Touch) | Foundational only | Independent feature |

### Parallel Opportunities

Within each story, tasks marked [P] can run in parallel:
- Phase 1: T002-T007 (module creation)
- Phase 2: T010-T013, T015-T017 (config structs, migrations)
- Phase 3: T020-T021 (focus structs)
- Phase 5: T038-T041 (colorblind palettes)
- Phase 6: T049-T050 (onboarding structs)
- Phase 9: T077-T078 (audio tone generation)
- Phase 12: T102-T108 (translation files)

---

## Parallel Example: User Story 3 (Colorblind Modes)

```bash
# Launch all palette definitions in parallel:
Task: "Define protanopia-safe color palette in src/accessibility/colorblind.rs"
Task: "Define deuteranopia-safe color palette in src/accessibility/colorblind.rs"
Task: "Define tritanopia-safe color palette in src/accessibility/colorblind.rs"

# Then implement the provider (depends on palettes):
Task: "Implement ColorPaletteProvider trait in src/accessibility/colorblind.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Keyboard Navigation)
4. **STOP and VALIDATE**: Test keyboard navigation independently
5. Deploy/demo if ready - users can now navigate without mouse

### Incremental Delivery (Recommended)

1. Setup + Foundational → Foundation ready
2. US1 (Keyboard) → Core accessibility ✓
3. US2 (Units) → International users ✓
4. US3 (Colorblind) → Color vision accessibility ✓
5. US4 (Onboarding) → New user experience ✓
6. Continue in priority order...

### Parallel Team Strategy (3 developers)

After Foundational phase:
- Developer A: US1 → US4 → US7 → US10
- Developer B: US2 → US5 → US8 → US11
- Developer C: US3 → US6 → US9 → US12 → US13

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Tasks** | 151 |
| **Setup Tasks** | 8 |
| **Foundational Tasks** | 11 |
| **User Story Tasks** | 123 |
| **Polish Tasks** | 9 |
| **Parallel Opportunities** | 35 tasks marked [P] |

### Tasks per User Story

| Story | Task Count | Key Files |
|-------|------------|-----------|
| US1 (Keyboard) | 10 | src/accessibility/focus.rs, src/input/keyboard.rs |
| US2 (Units) | 8 | src/storage/config.rs, src/metrics/calculator.rs |
| US3 (Colorblind) | 11 | src/accessibility/colorblind.rs, high_contrast.rs |
| US4 (Onboarding) | 12 | src/onboarding/*.rs |
| US5 (Theme) | 6 | src/ui/theme.rs |
| US6 (Layout) | 10 | src/ui/layout/*.rs |
| US7 (Audio) | 10 | src/audio/cues.rs, alerts.rs |
| US8 (TV Mode) | 7 | src/ui/display_modes/tv_mode.rs |
| US9 (Flow Mode) | 8 | src/ui/display_modes/flow_mode.rs |
| US10 (i18n) | 13 | src/i18n/*.rs, locales/ |
| US11 (Screen Reader) | 9 | src/accessibility/screen_reader.rs |
| US12 (Voice) | 10 | src/accessibility/voice_control.rs |
| US13 (Touch) | 9 | src/input/touch.rs, gestures.rs |

### Independent Test Criteria

Each user story can be tested in isolation:
- **US1**: Navigate without mouse using keyboard only
- **US2**: Toggle units and verify displays
- **US3**: Enable colorblind mode and verify zone colors
- **US4**: Reset data and complete onboarding
- **US5**: Change system theme and verify app follows
- **US6**: Create/save/load layout profiles
- **US7**: Start workout and hear audio cues
- **US8**: Enable TV Mode and verify readability at distance
- **US9**: Activate Flow Mode and verify single metric
- **US10**: Switch language and verify all text
- **US11**: Complete workflow with screen reader only
- **US12**: Issue voice commands without keyboard
- **US13**: Complete workflow with touch only

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- US11 (Screen Reader) depends on US1 (Keyboard) for focus management
