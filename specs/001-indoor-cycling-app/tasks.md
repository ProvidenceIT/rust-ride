# Tasks: RustRide Indoor Cycling Application

**Input**: Design documents from `/specs/001-indoor-cycling-app/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests included per plan.md TDD approach for core calculations and sensor protocols.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and Rust project structure

- [x] T001 Create Rust project with `cargo init --name rustride` at repository root
- [x] T002 Configure Cargo.toml with all dependencies from research.md in Cargo.toml
- [x] T003 [P] Create module directory structure per plan.md: src/sensors/, src/workouts/, src/recording/, src/metrics/, src/storage/, src/ui/
- [x] T004 [P] Create test directory structure: tests/unit/, tests/integration/, tests/fixtures/workouts/, tests/fixtures/rides/
- [x] T005 [P] Configure rustfmt.toml and clippy.toml for code style
- [x] T006 [P] Create src/lib.rs with module declarations
- [x] T007 Create src/main.rs with basic eframe application skeleton

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Database & Configuration

- [x] T008 Define database schema SQL in src/storage/schema.rs (users, sensors, workouts, rides, ride_samples tables)
- [x] T009 Implement Database struct with connection and migration in src/storage/database.rs
- [x] T010 [P] Implement Config loading from TOML in src/storage/config.rs
- [x] T011 [P] Create default config template in assets/default_config.toml

### Core Types

- [x] T012 [P] Define SensorType, Protocol, ConnectionState enums in src/sensors/types.rs
- [x] T013 [P] Define PowerTarget, SegmentType, WorkoutStatus enums in src/workouts/types.rs
- [x] T014 [P] Define RideSample, RecordingStatus structs in src/recording/types.rs
- [x] T015 [P] Define PowerZones, HRZones, ZoneRange structs in src/metrics/zones.rs
- [x] T016 Define UserProfile struct with FTP, zones, preferences in src/storage/config.rs

### Metrics Foundation (shared by all stories)

- [x] T017 Implement Coggan 7-zone power zone calculation from FTP in src/metrics/zones.rs
- [x] T018 [P] Implement Karvonen HR zone calculation in src/metrics/zones.rs
- [x] T019 [P] Implement 3-second rolling average in src/metrics/smoothing.rs
- [x] T020 [P] Implement power spike filtering (>2000W) in src/metrics/smoothing.rs

### Error Types

- [x] T021 [P] Define SensorError enum in src/sensors/types.rs
- [x] T022 [P] Define WorkoutError, WorkoutParseError enums in src/workouts/types.rs
- [x] T023 [P] Define RecorderError, ExportError enums in src/recording/types.rs

### Logging & Tracing

- [x] T024 Configure tracing subscriber in src/main.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Connect Smart Trainer and Start Free Ride (Priority: P1) 🎯 MVP

**Goal**: User can discover BLE smart trainer, connect, start free ride, and see real-time power/cadence/speed metrics.

**Independent Test**: Launch app → Sensor discovery → Connect trainer → Start free ride → See metrics updating at 1Hz.

### Tests for User Story 1

- [x] T025 [P] [US1] Unit test for FTMS data parsing in tests/unit/ftms_parser_test.rs
- [x] T026 [P] [US1] Unit test for sensor discovery filtering in tests/unit/sensor_discovery_test.rs
- [x] T027 [P] [US1] Integration test with mock BLE adapter in tests/integration/sensor_mock.rs

### Sensor Module Implementation

- [x] T028 [P] [US1] Define DiscoveredSensor, SensorState, SensorEvent types in src/sensors/types.rs
- [x] T029 [P] [US1] Define SensorConfig struct in src/sensors/types.rs
- [x] T030 [US1] Implement SensorManager struct with btleplug adapter initialization in src/sensors/manager.rs
- [x] T031 [US1] Implement start_discovery() with FTMS/CPS/HRS service UUID filtering in src/sensors/manager.rs
- [x] T032 [US1] Implement stop_discovery() in src/sensors/manager.rs
- [x] T033 [US1] Implement connect() with characteristic subscription in src/sensors/manager.rs
- [x] T034 [US1] Implement disconnect() in src/sensors/manager.rs
- [x] T035 [US1] Implement event channel for SensorEvent streaming in src/sensors/manager.rs

### FTMS Protocol Implementation

- [x] T036 [US1] Implement Indoor Bike Data (0x2AD2) notification parsing in src/sensors/ftms.rs
- [x] T037 [US1] Implement FTMS service/characteristic UUID constants in src/sensors/ftms.rs
- [x] T038 [US1] Extract power, cadence, speed from FTMS Indoor Bike Data in src/sensors/ftms.rs

### Metrics Calculator (minimal for US1)

- [x] T039 [US1] Implement MetricsCalculator struct with reset() in src/metrics/calculator.rs
- [x] T040 [US1] Implement process() to aggregate SensorReading into AggregatedMetrics in src/metrics/calculator.rs
- [x] T041 [US1] Define AggregatedMetrics, PowerMetrics structs in src/metrics/calculator.rs

### UI - Home Screen

- [x] T042 [US1] Create App struct with egui state in src/app.rs
- [x] T043 [US1] Implement home screen with "Start Free Ride" and "Sensors" buttons in src/ui/screens/home.rs
- [x] T044 [US1] Implement screen navigation state machine in src/ui/mod.rs

### UI - Sensor Setup Screen

- [x] T045 [US1] Implement sensor discovery list widget in src/ui/screens/sensor_setup.rs
- [x] T046 [US1] Implement sensor pairing confirmation dialog in src/ui/screens/sensor_setup.rs
- [x] T047 [US1] Implement connection status indicators in src/ui/widgets/sensor_status.rs

### UI - Ride Screen (Free Ride)

- [x] T048 [US1] Implement ride screen layout with metric panels in src/ui/screens/ride.rs
- [x] T049 [US1] Implement metric display widget (large readable numbers) in src/ui/widgets/metric_display.rs
- [x] T050 [US1] Wire sensor data to UI via crossbeam channel in src/app.rs
- [x] T051 [US1] Implement "End Ride" button in src/ui/screens/ride.rs

### Theme & Polish

- [x] T052 [US1] Implement dark theme colors in src/ui/theme.rs
- [x] T053 [US1] Implement keyboard shortcut: Space for pause in src/ui/screens/ride.rs

**Checkpoint**: User Story 1 complete - can connect trainer, start free ride, see live metrics

---

## Phase 4: User Story 2 - Execute Structured Workout with ERG Mode (Priority: P2)

**Goal**: User can import .zwo workout, start workout, and trainer automatically adjusts resistance for each interval.

**Independent Test**: Import .zwo file → Start workout → Verify trainer sets target power → Verify smooth transitions between intervals.

### Tests for User Story 2

- [x] T054 [P] [US2] Unit test for .zwo parsing with sample files in tests/unit/workout_parser_test.rs
- [x] T055 [P] [US2] Unit test for .mrc parsing in tests/unit/workout_parser_test.rs
- [x] T056 [P] [US2] Unit test for WorkoutEngine state machine in tests/unit/workout_engine_test.rs
- [ ] T057 [P] [US2] Integration test for workout execution in tests/integration/workout_execution_test.rs

### Workout Types

- [x] T058 [P] [US2] Define Workout, WorkoutSegment structs in src/workouts/types.rs
- [x] T059 [P] [US2] Define WorkoutState, SegmentProgress structs in src/workouts/types.rs

### Workout Parsers

- [x] T060 [US2] Implement .zwo XML parser with quick-xml in src/workouts/parser_zwo.rs
- [x] T061 [P] [US2] Implement .mrc/.erg text parser in src/workouts/parser_mrc.rs
- [x] T062 [US2] Add sample .zwo workout files to tests/fixtures/workouts/

### Workout Engine

- [x] T063 [US2] Implement WorkoutEngine struct with load(), start(), pause(), resume(), stop() in src/workouts/engine.rs
- [x] T064 [US2] Implement tick() for time progression and segment transitions in src/workouts/engine.rs
- [x] T065 [US2] Implement skip_segment() and extend_segment() in src/workouts/engine.rs
- [x] T066 [US2] Implement power ramp calculation for smooth transitions (3s default) in src/workouts/engine.rs
- [x] T067 [US2] Implement adjust_power() for manual +/- offset in src/workouts/engine.rs

### ERG Mode Control

- [x] T068 [US2] Implement FTMS Control Point (0x2AD9) write for target power in src/sensors/ftms.rs
- [x] T069 [US2] Implement request_control() command in src/sensors/ftms.rs
- [x] T070 [US2] Implement start_training() command in src/sensors/ftms.rs
- [x] T071 [US2] Implement set_target_power() command in src/sensors/ftms.rs
- [x] T072 [US2] Add SensorConnection::set_target_power() method in src/sensors/manager.rs

### UI - Workout Library Screen

- [x] T073 [US2] Implement workout library list with name, duration, TSS in src/ui/screens/workout_library.rs
- [x] T074 [US2] Implement workout import file picker in src/ui/screens/workout_library.rs
- [x] T075 [US2] Implement workout preview with profile graph in src/ui/widgets/workout_graph.rs

### UI - Ride Screen (Workout Mode)

- [x] T076 [US2] Extend ride screen with workout progress bar in src/ui/screens/ride.rs
- [x] T077 [US2] Display current interval, target power, time remaining in src/ui/screens/ride.rs
- [x] T078 [US2] Implement pause/resume/skip interval buttons in src/ui/screens/ride.rs
- [x] T079 [US2] Implement keyboard shortcuts: +/- for power adjustment in src/ui/screens/ride.rs

### Workout Storage

- [ ] T080 [US2] Implement workout CRUD in database in src/storage/database.rs

**Checkpoint**: User Story 2 complete - can import workouts, execute with ERG mode

---

## Phase 5: User Story 3 - Record and Export Ride Data (Priority: P3)

**Goal**: User's ride data is automatically recorded at 1-second resolution with auto-save, and can be exported to .fit/.tcx for Strava/Garmin.

**Independent Test**: Complete ride → Verify samples captured → Export to .fit → Upload to Strava successfully.

### Tests for User Story 3

- [ ] T081 [P] [US3] Unit test for TCX export format in tests/unit/tcx_export_test.rs
- [ ] T082 [P] [US3] Unit test for ride summary calculations in tests/unit/ride_summary_test.rs
- [ ] T083 [P] [US3] Integration test for ride recording in tests/integration/ride_recording_test.rs

### Recording Types

- [ ] T084 [P] [US3] Define Ride struct with summary fields in src/recording/types.rs
- [ ] T085 [P] [US3] Define RecorderConfig struct in src/recording/types.rs
- [ ] T086 [P] [US3] Define LiveRideSummary struct in src/recording/types.rs

### Ride Recorder

- [ ] T087 [US3] Implement RideRecorder struct with start(), record_sample(), finish() in src/recording/recorder.rs
- [ ] T088 [US3] Implement auto-save to autosave table every 30 seconds in src/recording/recorder.rs
- [ ] T089 [US3] Implement crash recovery with has_recovery_data(), recover() in src/recording/recorder.rs
- [ ] T090 [US3] Implement get_live_summary() for real-time stats in src/recording/recorder.rs

### Summary Calculations

- [ ] T091 [US3] Implement Normalized Power calculation (30s rolling, 4th power) in src/metrics/calculator.rs
- [ ] T092 [US3] Implement TSS calculation in src/metrics/calculator.rs
- [ ] T093 [US3] Implement IF (Intensity Factor) calculation in src/metrics/calculator.rs
- [ ] T094 [US3] Implement calorie estimation from power in src/metrics/calculator.rs

### Export - TCX

- [ ] T095 [US3] Implement TCX XML structure generation with quick-xml in src/recording/exporter_tcx.rs
- [ ] T096 [US3] Include power data in TCX ActivityExtension/TPX in src/recording/exporter_tcx.rs
- [ ] T097 [US3] Validate TCX output against schema in tests/unit/tcx_export_test.rs

### Export - CSV

- [ ] T098 [P] [US3] Implement CSV export of raw samples in src/recording/exporter_csv.rs

### Ride Storage

- [ ] T099 [US3] Implement ride CRUD in database in src/storage/database.rs
- [ ] T100 [US3] Implement ride_samples bulk insert in src/storage/database.rs

### UI - Ride Summary Screen

- [ ] T101 [US3] Create ride summary screen with stats display in src/ui/screens/ride_summary.rs
- [ ] T102 [US3] Implement export button with format selection in src/ui/screens/ride_summary.rs
- [ ] T103 [US3] Implement save/discard controls in src/ui/screens/ride_summary.rs

**Checkpoint**: User Story 3 complete - rides recorded, exported, and uploadable

---

## Phase 6: User Story 4 - Display Real-Time Training Metrics (Priority: P4)

**Goal**: User sees comprehensive, clear metrics display with power zones, HR zones, and derived metrics (NP, TSS, IF).

**Independent Test**: Ride with sensors → Verify all metrics displayed → Verify zone colors match configured FTP/HR.

### Tests for User Story 4

- [ ] T104 [P] [US4] Unit test for zone determination from power/HR in tests/unit/zones_test.rs

### Zone Calculation

- [ ] T105 [US4] Implement current_power_zone() in src/metrics/calculator.rs
- [ ] T106 [US4] Implement current_hr_zone() in src/metrics/calculator.rs
- [ ] T107 [US4] Add zone colors (7 for power, 5 for HR) in src/metrics/zones.rs

### Enhanced Metrics Display

- [ ] T108 [US4] Implement zone indicator widget with color band in src/ui/widgets/zone_indicator.rs
- [ ] T109 [US4] Display running NP, TSS, IF on ride screen in src/ui/screens/ride.rs
- [ ] T110 [US4] Implement full-screen mode toggle in src/ui/screens/ride.rs

### Dashboard Customization

- [ ] T111 [US4] Implement configurable metric panel layout in src/ui/screens/ride.rs
- [ ] T112 [US4] Save dashboard layout to config in src/storage/config.rs

**Checkpoint**: User Story 4 complete - rich metrics display with zones

---

## Phase 7: User Story 5 - Configure User Profile and Training Zones (Priority: P5)

**Goal**: User can configure FTP, HR, weight, and customize power/HR zones for accurate training metrics.

**Independent Test**: Open settings → Enter FTP → Verify power zones auto-calculate → Complete ride → Verify TSS uses configured FTP.

### Tests for User Story 5

- [ ] T113 [P] [US5] Unit test for zone calculation from FTP in tests/unit/zones_test.rs
- [ ] T114 [P] [US5] Unit test for HR zone calculation in tests/unit/zones_test.rs

### Profile Management

- [ ] T115 [US5] Implement UserProfile CRUD in database in src/storage/database.rs
- [ ] T116 [US5] Implement profile loading on app startup in src/app.rs
- [ ] T117 [US5] Implement FTP validation (50-600W) in src/storage/config.rs

### Zone Customization

- [ ] T118 [US5] Allow custom zone boundaries override in src/metrics/zones.rs
- [ ] T119 [US5] Persist custom zones to config in src/storage/config.rs

### UI - Settings Screen

- [ ] T120 [US5] Create settings screen with profile section in src/ui/screens/settings.rs
- [ ] T121 [US5] Implement FTP, max HR, resting HR, weight, height inputs in src/ui/screens/settings.rs
- [ ] T122 [US5] Implement power zone editor with auto-calculate toggle in src/ui/screens/settings.rs
- [ ] T123 [US5] Implement HR zone editor in src/ui/screens/settings.rs
- [ ] T124 [US5] Implement unit preference toggle (metric/imperial) in src/ui/screens/settings.rs
- [ ] T125 [US5] Implement theme toggle (dark/light) in src/ui/screens/settings.rs

### Light Theme

- [ ] T126 [P] [US5] Implement light theme colors in src/ui/theme.rs

**Checkpoint**: User Story 5 complete - profile and zones configurable

---

## Phase 8: User Story 6 - Browse Ride History and Analyze Past Rides (Priority: P6)

**Goal**: User can browse past rides, view detailed statistics, and re-export rides.

**Independent Test**: Complete multiple rides → Open history → Filter by date → Open ride detail → View charts → Re-export.

### Tests for User Story 6

- [ ] T127 [P] [US6] Unit test for ride query with date filtering in tests/unit/ride_query_test.rs

### Ride History Queries

- [ ] T128 [US6] Implement list_rides() with pagination in src/storage/database.rs
- [ ] T129 [US6] Implement get_ride_with_samples() in src/storage/database.rs
- [ ] T130 [US6] Implement filter_rides_by_date() in src/storage/database.rs
- [ ] T131 [US6] Implement delete_ride() in src/storage/database.rs

### UI - Ride History Screen

- [ ] T132 [US6] Create ride history list screen in src/ui/screens/ride_history.rs
- [ ] T133 [US6] Display date, duration, distance, avg power per ride in src/ui/screens/ride_history.rs
- [ ] T134 [US6] Implement date range filter UI in src/ui/screens/ride_history.rs

### UI - Ride Detail Screen

- [ ] T135 [US6] Create ride detail screen with summary stats in src/ui/screens/ride_detail.rs
- [ ] T136 [US6] Implement power/HR/cadence charts over time with egui_plot in src/ui/screens/ride_detail.rs
- [ ] T137 [US6] Implement min-max downsampling for chart performance in src/ui/screens/ride_detail.rs
- [ ] T138 [US6] Implement re-export button in src/ui/screens/ride_detail.rs
- [ ] T139 [US6] Implement delete ride with confirmation in src/ui/screens/ride_detail.rs

**Checkpoint**: User Story 6 complete - full ride history browsing

---

## Phase 9: User Story 7 - Connect Additional Sensors (Priority: P7)

**Goal**: User can connect multiple sensors simultaneously (trainer + HR + cadence) and select primary data sources.

**Independent Test**: Pair HR monitor alongside trainer → Start ride → Verify both data streams received → Configure primary source.

### Tests for User Story 7

- [ ] T140 [P] [US7] Unit test for multi-sensor data aggregation in tests/unit/sensor_aggregation_test.rs

### Multi-Sensor Support

- [ ] T141 [US7] Implement Cycling Power Service (0x1818) parsing in src/sensors/cycling_power.rs
- [ ] T142 [US7] Implement Heart Rate Service (0x180D) parsing in src/sensors/heart_rate.rs
- [ ] T143 [US7] Extend SensorManager for simultaneous connections in src/sensors/manager.rs
- [ ] T144 [US7] Implement data source priority selection in src/sensors/manager.rs

### Sensor Persistence

- [ ] T145 [US7] Implement sensor CRUD in database in src/storage/database.rs
- [ ] T146 [US7] Implement auto-reconnect on app startup in src/sensors/manager.rs
- [ ] T147 [US7] Save is_primary flag per sensor type in src/storage/database.rs

### UI - Multi-Sensor Setup

- [ ] T148 [US7] Show all discovered sensors with type icons in src/ui/screens/sensor_setup.rs
- [ ] T149 [US7] Show all connected sensors with status in src/ui/screens/sensor_setup.rs
- [ ] T150 [US7] Implement primary source selector when conflicts exist in src/ui/screens/sensor_setup.rs

**Checkpoint**: User Story 7 complete - multi-sensor support working

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

### FIT Export (Secondary format)

- [ ] T151 [P] Research fit-rust crate capabilities for FIT write in src/recording/exporter_fit.rs
- [ ] T152 Implement FIT export if crate supports creation in src/recording/exporter_fit.rs

### Edge Cases & Error Handling

- [ ] T153 Implement trainer disconnect mid-workout handling in src/workouts/engine.rs
- [ ] T154 Implement no-sensors-found troubleshooting tips in src/ui/screens/sensor_setup.rs
- [ ] T155 Implement invalid workout file error display in src/ui/screens/workout_library.rs
- [ ] T156 Implement storage-full warning in src/recording/recorder.rs
- [ ] T157 Implement crash recovery prompt on startup in src/app.rs

### Performance Optimization

- [ ] T158 Profile memory usage during active ride
- [ ] T159 Profile CPU usage during active ride
- [ ] T160 Optimize chart rendering for large ride history

### Documentation

- [ ] T161 [P] Update README.md with build and usage instructions
- [ ] T162 [P] Validate quickstart.md setup instructions work

### Final Validation

- [ ] T163 Run full test suite: cargo test
- [ ] T164 Run clippy lint check: cargo clippy
- [ ] T165 Test on Windows 10/11
- [ ] T166 Test on macOS
- [ ] T167 Test on Linux

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-9)**: All depend on Foundational phase completion
  - US1 (P1) can start after Foundation
  - US2 (P2) can start after Foundation (may use US1 sensor components)
  - US3 (P3) can start after Foundation
  - US4 (P4) depends on US1 (needs metrics display foundation)
  - US5 (P5) can start after Foundation
  - US6 (P6) depends on US3 (needs ride data to display)
  - US7 (P7) depends on US1 (extends sensor capabilities)
- **Polish (Phase 10)**: Depends on desired user stories being complete

### User Story Dependencies

| Story | Can Start After | Notes |
|-------|-----------------|-------|
| US1 (P1) | Foundation | Core sensor + free ride MVP |
| US2 (P2) | Foundation | Uses US1 sensor module |
| US3 (P3) | Foundation | Recording independent of workout |
| US4 (P4) | US1 | Extends ride screen with zones |
| US5 (P5) | Foundation | Profile independent |
| US6 (P6) | US3 | Needs rides to browse |
| US7 (P7) | US1 | Extends sensor capabilities |

### Within Each User Story

1. Tests (if included) written and FAIL first
2. Types/Models before services
3. Core logic before UI
4. UI before polish

### Parallel Opportunities

- All tasks marked [P] within a phase can run in parallel
- After Foundation, US1, US2, US3, and US5 can start in parallel
- Within US1: T028-T029 (types), T042-T044 (UI home), T048-T049 (UI ride) can parallelize
- Within US2: T054-T057 (tests), T058-T059 (types), T060-T061 (parsers) can parallelize

---

## Parallel Example: User Story 1

```bash
# Launch all tests for US1 together:
Task: "Unit test for FTMS data parsing in tests/unit/ftms_parser_test.rs"
Task: "Unit test for sensor discovery filtering in tests/unit/sensor_discovery_test.rs"

# Launch all type definitions together:
Task: "Define DiscoveredSensor, SensorState, SensorEvent types in src/sensors/types.rs"
Task: "Define SensorConfig struct in src/sensors/types.rs"

# Launch UI screens in parallel (different files):
Task: "Implement home screen in src/ui/screens/home.rs"
Task: "Implement sensor setup screen in src/ui/screens/sensor_setup.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test trainer connection + free ride independently
5. Deploy/demo if ready - this is a functional cycling computer

### Incremental Delivery

1. Setup + Foundation → Foundation ready
2. Add US1 → Test → Deploy (MVP: free ride with trainer)
3. Add US2 → Test → Deploy (structured workouts with ERG)
4. Add US3 → Test → Deploy (ride recording + export)
5. Add US4-US7 → Polish → Full MVP

### Suggested MVP Scope

**Minimum**: User Story 1 only (45 tasks)
- User can connect trainer, start free ride, see live metrics
- Delivers immediate value for unstructured training

**Recommended MVP**: User Stories 1-3 (85 tasks)
- Adds structured workouts with ERG mode
- Adds ride recording and Strava export
- Complete core training application

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Tasks** | 167 |
| **Setup Phase** | 7 tasks |
| **Foundational Phase** | 17 tasks |
| **User Story 1 (P1)** | 29 tasks |
| **User Story 2 (P2)** | 27 tasks |
| **User Story 3 (P3)** | 23 tasks |
| **User Story 4 (P4)** | 9 tasks |
| **User Story 5 (P5)** | 14 tasks |
| **User Story 6 (P6)** | 13 tasks |
| **User Story 7 (P7)** | 11 tasks |
| **Polish Phase** | 17 tasks |
| **Parallel Opportunities** | ~40% of tasks marked [P] |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
