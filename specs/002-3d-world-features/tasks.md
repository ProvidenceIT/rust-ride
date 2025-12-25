# Tasks: 3D Virtual World & Complete Feature Implementation

**Input**: Design documents from `/specs/002-3d-world-features/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (New Dependencies & Structure)

**Purpose**: Add new dependencies and create module structure for 3D world

- [x] T001 Add new dependencies to Cargo.toml (glam, gltf, bytemuck, image, rfd)
- [x] T002 [P] Create src/world/ module structure with mod.rs
- [x] T003 [P] Create assets/ directory structure for 3D models and textures
- [x] T004 [P] Add database migration for autosave table in src/storage/database.rs
- [x] T005 [P] Add database migration for avatars table in src/storage/database.rs
- [x] T006 Verify build compiles with new dependencies

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**Warning**: No user story work can begin until this phase is complete

- [x] T007 Implement SensorError enum with thiserror in src/sensors/types.rs
- [x] T008 [P] Implement RecorderError enum with thiserror in src/recording/types.rs
- [x] T009 [P] Implement WorkoutError enum with thiserror in src/workouts/mod.rs
- [x] T010 [P] Implement ConfigError enum with thiserror in src/storage/config.rs
- [x] T011 [P] Implement WorldError enum with thiserror in src/world/mod.rs
- [x] T012 Implement SensorCommand and SensorEvent channel messages in src/sensors/types.rs
- [x] T013 [P] Implement AutosaveData via save_autosave/load_autosave in src/storage/database.rs
- [x] T014 [P] Implement Ride struct with summary fields in src/recording/types.rs
- [x] T015 Run cargo build to verify all foundational types compile

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Complete Sensor Control (Priority: P1) MVP

**Goal**: Enable BLE discovery, connection, and disconnection for smart trainers and sensors

**Independent Test**: Launch app, scan for BLE devices, connect a trainer, verify data flows, disconnect

### Implementation for User Story 1

- [x] T016 [US1] Implement start_discovery() with btleplug adapter.start_scan() in src/sensors/manager.rs
- [x] T017 [US1] Implement stop_discovery() with adapter.stop_scan() in src/sensors/manager.rs
- [x] T018 [US1] Implement connect() with peripheral connection and service discovery in src/sensors/manager.rs
- [x] T019 [US1] Implement disconnect() with peripheral.disconnect() in src/sensors/manager.rs
- [x] T020 [US1] Implement get_sensor_states() to return all known sensor states in src/sensors/manager.rs
- [x] T021 [US1] Implement has_controllable_trainer() check in src/sensors/manager.rs
- [x] T022 [US1] Add SensorState struct with ConnectionStatus enum in src/sensors/types.rs
- [x] T023 [US1] Wire UI "Start Scanning" button to send SensorCommand::StartDiscovery in src/ui/screens/sensor_setup.rs
- [x] T024 [US1] Wire UI device list to show discovered sensors from SensorEvent channel in src/ui/screens/sensor_setup.rs
- [x] T025 [US1] Wire UI device tap to send SensorCommand::Connect in src/ui/screens/sensor_setup.rs
- [x] T026 [US1] Wire UI "Disconnect" button to send SensorCommand::Disconnect in src/ui/screens/sensor_setup.rs
- [x] T027 [US1] Display sensor status bar on home screen in src/ui/screens/home.rs
- [x] T028 [US1] Add signal strength (RSSI) display during discovery in src/ui/screens/sensor_setup.rs
- [x] T029 [US1] Add auto-reconnect logic (3 retries) on unexpected disconnect in src/sensors/manager.rs
- [x] T030 [US1] Add 30-second discovery timeout with auto-stop in src/sensors/manager.rs

**Checkpoint**: User Story 1 complete - sensors can be discovered, connected, and disconnected

---

## Phase 4: User Story 2 - Ride Recording & Persistence (Priority: P1) MVP

**Goal**: Save rides to database, implement autosave and crash recovery

**Independent Test**: Start ride, record data, end ride, close app, reopen, verify ride in history

### Implementation for User Story 2

- [x] T031 [US2] Implement save_ride() to persist Ride and RideSamples to database in src/recording/recorder.rs
- [x] T032 [US2] Implement enable_autosave() to start 30-second autosave timer in src/recording/recorder.rs
- [x] T033 [US2] Implement upsert_autosave() database operation in src/storage/database.rs
- [x] T034 [US2] Implement get_autosave() database operation in src/storage/database.rs
- [x] T035 [US2] Implement clear_autosave() database operation in src/storage/database.rs
- [x] T036 [US2] Implement has_recovery_data() check in src/recording/recorder.rs
- [x] T037 [US2] Implement recover_ride() to load and return RecoverableRide in src/recording/recorder.rs
- [x] T038 [US2] Implement discard_recovery() to clear autosave data in src/recording/recorder.rs
- [x] T039 [US2] Add recovery dialog on app startup in src/app.rs
- [x] T040 [US2] Wire "End Ride" button to call save_ride() in src/ui/screens/ride.rs
- [x] T041 [US2] Calculate NP, TSS, IF, max_speed on save in src/recording/recorder.rs
- [x] T042 [US2] Add confirmation dialog before ending ride in src/ui/screens/ride.rs
- [x] T043 [US2] Navigate to ride summary after successful save in src/ui/screens/ride.rs

**Checkpoint**: User Story 2 complete - rides persist and can be recovered after crash

---

## Phase 5: User Story 3 - Ride History & Details (Priority: P2)

**Goal**: Browse past rides, view details with zone distribution, export and delete

**Independent Test**: Complete multiple rides, view history, tap to see details, export one ride

### Implementation for User Story 3

- [x] T044 [US3] Implement get_all_rides() returning Vec<RideSummary> in src/storage/database.rs (list_rides exists)
- [x] T045 [US3] Implement get_ride_detail() with samples and zones in src/storage/database.rs (get_ride_with_samples exists)
- [x] T046 [US3] Implement delete_ride() with cascade to samples in src/storage/database.rs
- [x] T047 [US3] Display ride list in reverse chronological order in src/ui/screens/ride_history.rs
- [x] T048 [US3] Show summary metrics (date, duration, distance, avg power) per ride in src/ui/screens/ride_history.rs
- [x] T049 [US3] Navigate to detail view on ride tap in src/ui/screens/ride_history.rs
- [x] T050 [US3] Display comprehensive metrics in detail view in src/ui/screens/ride_detail.rs
- [x] T051 [US3] Implement power zones distribution chart in src/ui/screens/ride_detail.rs
- [x] T052 [US3] Implement HR zones distribution chart in src/ui/screens/ride_detail.rs
- [x] T053 [US3] Add max_speed calculation from samples for TCX export in src/recording/exporter_tcx.rs
- [x] T054 [US3] Wire "Export TCX" button with file save dialog in src/ui/screens/ride_detail.rs
- [x] T055 [US3] Wire "Export CSV" button with file save dialog in src/ui/screens/ride_detail.rs
- [x] T056 [US3] Wire "Delete" button with confirmation dialog in src/ui/screens/ride_detail.rs

**Checkpoint**: User Story 3 complete - full ride history browsing, export, and deletion

---

## Phase 6: User Story 4 - Workout Library & Execution (Priority: P2)

**Goal**: Import workout files via native dialog, display library, execute with ERG mode

**Independent Test**: Import .zwo file, view in library, start workout, verify ERG targets change

### Implementation for User Story 4

- [x] T057 [US4] Implement pick_workout_file() using rfd::AsyncFileDialog in src/workouts/mod.rs
- [x] T058 [US4] Implement WorkoutLibrary struct with new(), get_all() in src/workouts/library.rs
- [x] T059 [US4] Implement import_file() to parse and save workout in src/workouts/library.rs
- [x] T060 [US4] Implement save_workout() database operation in src/storage/database.rs (insert_workout)
- [x] T061 [US4] Implement get_workout_summaries() database operation in src/storage/database.rs (list_workouts)
- [x] T062 [US4] Implement get_workout() database operation in src/storage/database.rs
- [x] T063 [US4] Implement delete_workout() database operation in src/storage/database.rs
- [x] T064 [US4] Implement WorkoutSummary with IntervalPreview for visual bars in src/workouts/types.rs
- [x] T065 [US4] Generate preview_data during import in src/workouts/library.rs
- [x] T066 [US4] Display workout library grid/list in src/ui/screens/workout_library.rs
- [x] T067 [US4] Show workout cards with name, duration, interval preview in src/ui/screens/workout_library.rs
- [x] T068 [US4] Wire "Import" button to open native file dialog in src/ui/screens/workout_library.rs
- [x] T069 [US4] Show loading state during import and toast on success/failure in src/ui/screens/workout_library.rs
- [x] T070 [US4] Wire workout selection to start workout execution in src/ui/screens/workout_library.rs
- [x] T071 [US4] Send ERG power targets via SensorCommand::SetErgTarget in src/workouts/engine.rs
- [x] T072 [US4] Implement pause/resume/skip controls during workout in src/ui/screens/ride.rs

**Checkpoint**: User Story 4 complete - workouts can be imported and executed with ERG mode

---

## Phase 7: User Story 5 - Settings & User Profile (Priority: P2)

**Goal**: Configure FTP, max HR, weight, units, theme with persistence

**Independent Test**: Change FTP, save, start ride, verify power zones use new FTP

### Implementation for User Story 5

- [x] T073 [US5] Implement UserSettings, UserProfile, DisplaySettings, TrainingSettings structs in src/storage/config.rs
- [x] T074 [US5] Implement PowerZoneConfig with from_ftp() and get_zone() in src/metrics/zones.rs
- [x] T075 [US5] Implement HrZoneConfig with from_max_hr() and get_zone() in src/metrics/zones.rs
- [x] T076 [US5] Implement SettingsManager with load() and save() in src/storage/config.rs
- [x] T077 [US5] Implement settings persistence to TOML file in src/storage/config.rs
- [x] T078 [US5] Implement light theme colors in ThemeColors::light() in src/ui/theme.rs
- [x] T079 [US5] Implement apply_theme() to apply colors to egui context in src/ui/theme.rs
- [x] T080 [US5] Build settings screen UI with profile fields in src/ui/screens/settings.rs
- [x] T081 [US5] Add FTP input with validation (50-500W) in src/ui/screens/settings.rs
- [x] T082 [US5] Add Max HR input with validation (100-220) in src/ui/screens/settings.rs
- [x] T083 [US5] Add Weight input with validation (30-200kg) in src/ui/screens/settings.rs
- [x] T084 [US5] Add unit system dropdown (Metric/Imperial) in src/ui/screens/settings.rs
- [x] T085 [US5] Add theme toggle (Light/Dark) with immediate preview in src/ui/screens/settings.rs
- [x] T086 [US5] Add "View Power Zones" and "View HR Zones" buttons in src/ui/screens/settings.rs
- [x] T087 [US5] Wire Save button to persist settings in src/ui/screens/settings.rs
- [x] T088 [US5] Auto-recalculate zones when FTP or Max HR changes in src/ui/screens/settings.rs

**Checkpoint**: User Story 5 complete - settings persist and affect calculations

---

## Phase 8: User Story 6 - 3D Virtual World Riding (Priority: P3)

**Goal**: Render 3D environment with avatar moving based on power output

**Independent Test**: Connect trainer, start 3D ride, verify avatar moves when pedaling

### Implementation for User Story 6

- [x] T089 [US6] Implement PhysicsEngine with calculate_speed() in src/world/physics.rs
- [x] T090 [US6] Implement Route struct with get_position(), get_gradient() in src/world/route.rs
- [x] T091 [US6] Implement Waypoint struct in src/world/route.rs
- [x] T092 [US6] Implement Camera with follow() and view_projection() in src/world/camera.rs
- [x] T093 [US6] Implement Avatar struct with update() and set_cadence() in src/world/avatar.rs
- [x] T094 [US6] Implement Transform struct with position, rotation, scale in src/world/scene.rs
- [x] T095 [US6] Implement Scene struct with terrain, road, avatar, scenery in src/world/scene.rs
- [x] T096 [US6] Implement Renderer with wgpu pipeline initialization in src/world/renderer.rs
- [x] T097 [US6] Implement Renderer::render() to output to texture in src/world/renderer.rs
- [x] T098 [US6] Implement World3D::new() to initialize from WorldDefinition in src/world/mod.rs
- [x] T099 [US6] Implement World3D::update() for physics/avatar/camera updates in src/world/mod.rs
- [x] T100 [US6] Implement World3D::render() to return egui TextureId in src/world/mod.rs
- [x] T101 [US6] Implement World3D::get_stats() for HUD data in src/world/mod.rs
- [x] T102 [US6] Create basic terrain rendering with ground plane in src/world/terrain.rs
- [x] T103 [US6] Create road rendering along route path in src/world/terrain.rs
- [x] T104 [US6] Create sky rendering with gradient in src/world/terrain.rs
- [x] T105 [US6] Integrate 3D view into ride screen in src/ui/screens/ride.rs
- [x] T106 [US6] Implement HUD overlay with speed, distance, elevation in src/world/hud.rs
- [x] T107 [US6] Wire power data to World3D::update() in src/ui/screens/ride.rs
- [x] T108 [US6] Ensure ride data saves normally when 3D ride ends in src/ui/screens/ride.rs

**Checkpoint**: User Story 6 complete - 3D world renders with avatar responding to power

---

## Phase 9: User Story 7 - 3D World Selection & Routes (Priority: P3)

**Goal**: Multiple worlds with different themes, route selection with elevation profiles

**Independent Test**: View world selection, choose different worlds, verify distinct visuals

### Implementation for User Story 7

- [x] T109 [US7] Implement WorldDefinition struct with routes list in src/world/worlds/mod.rs
- [x] T110 [US7] Implement RouteDefinition struct with distance, elevation in src/world/worlds/mod.rs
- [x] T111 [US7] Create countryside world definition in src/world/worlds/countryside.rs
- [x] T112 [US7] Create mountains world definition in src/world/worlds/mountains.rs
- [x] T113 [US7] Create coastal world definition in src/world/worlds/coastal.rs
- [x] T114 [P] [US7] Create countryside world JSON data in assets/worlds/countryside.json
- [x] T115 [P] [US7] Create mountains world JSON data in assets/worlds/mountains.json
- [x] T116 [P] [US7] Create coastal world JSON data in assets/worlds/coastal.json
- [x] T117 [US7] Implement world loader to parse JSON definitions in src/world/worlds/mod.rs
- [x] T118 [US7] Create world selection screen in src/ui/screens/world_select.rs
- [x] T119 [US7] Display world cards with preview images and descriptions in src/ui/screens/world_select.rs
- [x] T120 [US7] Display route list when world is selected in src/ui/screens/world_select.rs
- [x] T121 [US7] Show route distance and elevation profile in src/ui/screens/world_select.rs
- [x] T122 [US7] Wire route selection to start 3D ride in src/ui/screens/world_select.rs
- [x] T123 [US7] Add HUD element showing route progress and remaining distance in src/world/hud.rs

**Checkpoint**: User Story 7 complete - 3 distinct worlds with selectable routes

---

## Phase 10: User Story 8 - Avatar Customization (Priority: P4)

**Goal**: Customize avatar jersey color and bike style with persistence

**Independent Test**: Change jersey color, save, start ride, verify avatar displays customization

### Implementation for User Story 8

- [x] T124 [US8] Implement AvatarConfig struct with jersey_color, bike_style in src/world/avatar.rs
- [x] T125 [US8] Implement BikeStyle enum (RoadBike, TimeTrial, Gravel) in src/world/avatar.rs
- [x] T126 [US8] Implement save_avatar() database operation in src/storage/database.rs
- [x] T127 [US8] Implement get_avatar() database operation in src/storage/database.rs
- [x] T128 [US8] Create avatar customization screen in src/ui/screens/avatar.rs
- [x] T129 [US8] Add jersey color picker (10+ color options) in src/ui/screens/avatar.rs
- [x] T130 [US8] Add bike style selector with previews in src/ui/screens/avatar.rs
- [x] T131 [US8] Show live avatar preview as customization changes in src/ui/screens/avatar.rs
- [x] T132 [US8] Wire Save button to persist avatar config in src/ui/screens/avatar.rs
- [x] T133 [US8] Load avatar config when starting 3D ride in src/ui/screens/ride.rs
- [x] T134 [US8] Apply jersey color to avatar model material in src/world/avatar.rs
- [x] T135 [US8] Switch bike model based on BikeStyle selection in src/world/avatar.rs

**Checkpoint**: User Story 8 complete - avatar customization persists and renders

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T136 [P] Add Bluetooth disabled error handling with user message in src/sensors/manager.rs
- [x] T137 [P] Add sensor disconnect mid-ride handling with reconnect notification in src/ui/screens/ride.rs
- [x] T138 [P] Add storage full warning before ride starts in src/ui/screens/ride.rs
- [x] T139 [P] Add malformed workout file error display in src/ui/screens/workout_library.rs
- [x] T140 [P] Add GPU capability detection and 2D fallback mode in src/world/mod.rs
- [x] T141 [P] Add 3D world load timeout (30s) with retry option in src/world/mod.rs
- [x] T142 [P] Add trainer power loss detection during ERG mode in src/workouts/engine.rs
- [x] T143 Run cargo fmt to format all code
- [x] T144 Run cargo clippy and fix all warnings
- [x] T145 Run cargo test to verify all existing tests pass
- [x] T146 Validate quickstart.md instructions work on clean checkout

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-10)**: All depend on Foundational phase completion
  - US1 and US2 are P1 (MVP) - implement first in sequence
  - US3, US4, US5 are P2 - can proceed in parallel after P1
  - US6, US7 are P3 - can proceed in parallel after P2
  - US8 is P4 - implement last
- **Polish (Phase 11)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (Sensor Control)**: Foundation only - no other story dependencies
- **US2 (Ride Persistence)**: Foundation only - independent of US1
- **US3 (Ride History)**: Requires US2 (rides must be saved to browse history)
- **US4 (Workout Library)**: Requires US1 (sensors needed for ERG mode)
- **US5 (Settings)**: Foundation only - but affects US3/US4 calculations
- **US6 (3D World)**: Requires US1 (sensors for power data), US2 (ride saving), US5 (weight for physics)
- **US7 (World Selection)**: Requires US6 (3D world must exist)
- **US8 (Avatar Custom)**: Requires US6 (avatar must exist)

### Within Each User Story

- Models/types before services
- Services before UI components
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel
- US1 and US2 can run in parallel (both P1, no dependencies)
- US3, US4, US5 can run in parallel after US2 completes
- US6 and US7 can start once US5 completes
- All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# After T007 (SensorError), these can run in parallel:
Task: "T016 [US1] Implement start_discovery()"
Task: "T017 [US1] Implement stop_discovery()"
Task: "T018 [US1] Implement connect()"
Task: "T019 [US1] Implement disconnect()"

# UI tasks can run in parallel after manager implementations:
Task: "T023 [US1] Wire UI Start Scanning button"
Task: "T024 [US1] Wire UI device list"
Task: "T025 [US1] Wire UI device tap"
```

---

## Parallel Example: World Asset Creation

```bash
# These JSON files can all be created in parallel:
Task: "T114 [P] [US7] Create countryside.json"
Task: "T115 [P] [US7] Create mountains.json"
Task: "T116 [P] [US7] Create coastal.json"
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Sensor Control)
4. Complete Phase 4: User Story 2 (Ride Persistence)
5. **STOP and VALIDATE**: Sensors work, rides save, crash recovery works
6. Deploy/demo if ready - this is a functional training app!

### P2 Features (After MVP)

7. Complete Phase 5: User Story 3 (Ride History)
8. Complete Phase 6: User Story 4 (Workout Library)
9. Complete Phase 7: User Story 5 (Settings)
10. **VALIDATE**: Full feature set minus 3D world

### 3D World (After P2)

11. Complete Phase 8: User Story 6 (3D World)
12. Complete Phase 9: User Story 7 (World Selection)
13. Complete Phase 10: User Story 8 (Avatar Customization)
14. Complete Phase 11: Polish

### Incremental Delivery

Each phase adds value without breaking previous phases:
- After US1+US2: Functional training app
- After US3: Browse past rides
- After US4: Structured workouts
- After US5: Personalized settings
- After US6: Immersive 3D experience
- After US7: Multiple worlds
- After US8: Personalized avatar

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Total: 146 tasks across 11 phases
