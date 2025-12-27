# Tasks: Hardware Integration

**Input**: Design documents from `/specs/007-hardware-integration/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story (14 stories across 5 priority tiers) to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new dependencies and create module structure

- [x] T001 Add new dependencies to Cargo.toml: ant-rs, tts, rodio, rumqttc, tokio-tungstenite, qrcode, hidapi, reqwest, keyring, oauth2, ffmpeg-next
- [x] T002 [P] Create src/sensors/ant/ directory structure per plan.md
- [x] T003 [P] Create src/audio/ directory structure per plan.md
- [x] T004 [P] Create src/integrations/ directory structure with mqtt/, weather/, sync/, streaming/ subdirs
- [x] T005 [P] Create src/hid/ directory structure per plan.md
- [x] T006 [P] Create src/video/ directory structure per plan.md
- [x] T007 [P] Create tests/integration/ and tests/unit/ test file stubs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T008 Extend src/sensors/types.rs with SensorProtocol enum (Ble, AntPlus) and new SensorType variants (SmO2, Imu)
- [x] T009 Add database migrations for new tables in src/storage/schema.rs: ant_dongles, dual_protocol_bindings, fan_profiles, hid_devices, button_mappings, streaming_sessions, platform_syncs, sync_records, video_syncs
- [x] T010 Extend existing sensors table with protocol, serial_number, preferred_protocol columns in src/storage/schema.rs
- [x] T011 [P] Create src/storage/hardware_store.rs with CRUD operations for new entities (AntDongle, FanProfile, HidDevice, ButtonMapping, PlatformSync, VideoSync)
- [x] T012 [P] Create src/sensors/ant/mod.rs with module exports and trait definitions
- [x] T013 [P] Create src/audio/mod.rs with AudioEngine trait and module exports
- [x] T014 [P] Create src/integrations/mod.rs with module exports for mqtt, weather, sync, streaming
- [x] T015 [P] Create src/hid/mod.rs with HidDeviceManager trait and module exports
- [x] T016 [P] Create src/video/mod.rs with VideoPlayer trait and module exports
- [x] T017 Update src/lib.rs to export new modules: sensors::ant, audio, integrations, hid, video
- [x] T018 Update src/app.rs to initialize new module managers in application state

**Checkpoint**: Foundation ready - user story implementation can now begin ✅

---

## Phase 3: User Story 1 - ANT+ Sensor Connection (Priority: P1) 🎯 MVP

**Goal**: Users with ANT+ sensors can discover, pair, and receive data from legacy equipment

**Independent Test**: Connect ANT+ USB dongle, discover sensors, pair power meter, verify data streams during free ride

### Implementation for User Story 1

- [x] T019 [P] [US1] Create AntDongle and DongleStatus types in src/sensors/ant/dongle.rs
- [x] T020 [P] [US1] Create AntChannel and ChannelStatus types in src/sensors/ant/channels.rs
- [x] T021 [US1] Implement AntDongleManager trait with scan_dongles(), initialize_dongle(), release_dongle() in src/sensors/ant/dongle.rs
- [x] T022 [US1] Implement USB VID/PID detection for common ANT+ dongles (Garmin, Suunto, Dynastream) in src/sensors/ant/dongle.rs
- [x] T023 [US1] Implement AntChannelManager trait with allocate_channel(), start_search(), close_channel() in src/sensors/ant/channels.rs
- [x] T024 [P] [US1] Create src/sensors/ant/profiles/mod.rs with profile trait definitions
- [x] T025 [P] [US1] Implement ANT+ Power (PWR) profile parser in src/sensors/ant/profiles/power.rs
- [x] T026 [P] [US1] Implement ANT+ Heart Rate (HRM) profile parser in src/sensors/ant/profiles/hr.rs
- [x] T027 [P] [US1] Implement ANT+ FE-C profile parser in src/sensors/ant/profiles/fec.rs
- [x] T028 [US1] Create DualProtocolBinding type and DualProtocolDetector trait in src/sensors/ant/duplex.rs
- [x] T029 [US1] Implement duplicate sensor detection (match by serial number or manufacturer ID) in src/sensors/ant/duplex.rs
- [x] T030 [US1] Integrate ANT+ sensor discovery into existing src/sensors/manager.rs
- [x] T031 [US1] Update src/ui/screens/sensor_setup.rs to show ANT+ dongle status and enable ANT+ scanning toggle
- [x] T032 [US1] Update src/ui/screens/sensor_setup.rs to display dual-protocol sensors with protocol choice dialog
- [x] T033 [US1] Add ANT+ sensor event handling to main sensor event loop in src/sensors/manager.rs
- [x] T034 [US1] Write unit tests for ANT+ profile parsers in tests/unit/ant_profiles.rs

**Checkpoint**: ANT+ sensors discoverable and functional alongside BLE sensors

---

## Phase 4: User Story 2 - Smart Trainer Incline/Slope Mode (Priority: P1)

**Goal**: Smart trainer simulates realistic gradients during virtual rides

**Independent Test**: Load route with gradients, verify trainer resistance changes correspond to gradient values

### Implementation for User Story 2

- [x] T035 [P] [US2] Create InclineConfig and GradientState types in src/sensors/incline.rs
- [x] T036 [US2] Implement InclineController trait with set_gradient(), calculate_effective_gradient() in src/sensors/incline.rs
- [x] T037 [US2] Implement FTMS slope command building (0x11 command with signed 16-bit gradient) in src/sensors/incline.rs
- [x] T038 [US2] Add gradient calculation formula: resistance based on gradient, rider weight, bike weight in src/sensors/incline.rs
- [x] T039 [US2] Implement gradient smoothing for transition between slope changes in src/sensors/incline.rs
- [x] T040 [US2] Add intensity scaling (50%-150%) to gradient calculations in src/sensors/incline.rs
- [x] T041 [US2] Integrate incline controller with existing FTMS trainer connection in src/sensors/ftms.rs
- [x] T042 [US2] Update src/ui/screens/settings.rs with incline mode settings: enable/disable, intensity slider, weight inputs
- [x] T043 [US2] Connect route gradient data to incline controller during ride in src/app.rs
- [x] T044 [US2] Write unit tests for gradient calculations in tests/unit/incline.rs

**Checkpoint**: Trainer resistance changes based on route gradient ✅

---

## Phase 5: User Story 3 - Cycling Dynamics (Priority: P2)

**Goal**: Display L/R power balance and pedaling efficiency metrics from dual-sided power meters

**Independent Test**: Connect dual-sided power meter, verify L/R balance and torque effectiveness display

### Implementation for User Story 3

- [x] T045 [P] [US3] Create CyclingDynamicsData, LeftRightBalance, PedalSmoothness, TorqueEffectiveness types in src/sensors/dynamics.rs
- [x] T046 [US3] Implement extended Cycling Power Service parsing for L/R balance in src/sensors/dynamics.rs
- [x] T047 [US3] Implement CyclingDynamicsProvider trait with get_current_dynamics(), get_session_averages() in src/sensors/dynamics.rs
- [x] T048 [US3] Create cycling_dynamics_samples table and add to migrations in src/storage/schema.rs
- [x] T049 [US3] Implement cycling dynamics recording in ride sample collection in src/recording/recorder.rs
- [x] T050 [US3] Update FIT export to include cycling dynamics fields in src/recording/exporter_fit.rs
- [x] T051 [P] [US3] Create src/ui/widgets/dynamics_display.rs with L/R balance arc visualization
- [x] T052 [US3] Integrate dynamics display widget into ride screen in src/ui/screens/ride.rs
- [x] T053 [US3] Add cycling dynamics summary to ride summary screen in src/ui/screens/ride_summary.rs
- [x] T054 [US3] Write unit tests for cycling dynamics parsing in tests/unit/dynamics.rs

**Checkpoint**: L/R balance and pedaling metrics visible during and after rides

---

## Phase 6: User Story 4 - Audio Cues and Voice Alerts (Priority: P2)

**Goal**: Voice alerts for interval transitions, zone changes, and milestones during workouts

**Independent Test**: Start structured workout, verify voice announcements at interval transitions and zone crossings

### Implementation for User Story 4

- [x] T055 [P] [US4] Create AudioConfig, AlertType, AlertConfig types in src/audio/alerts.rs
- [x] T056 [P] [US4] Create CueTemplate type and default templates in src/audio/cues.rs
- [x] T057 [US4] Implement AudioEngine trait with play_sound(), speak(), set_volume() using rodio in src/audio/engine.rs
- [x] T058 [US4] Implement TtsProvider trait using tts crate with cross-platform voice selection in src/audio/tts.rs
- [x] T059 [US4] Implement AlertManager trait with trigger_alert(), configure() in src/audio/alerts.rs
- [x] T060 [US4] Implement audio queue to prevent alert overlap in src/audio/engine.rs
- [x] T061 [US4] Create CueBuilder with message template expansion in src/audio/cues.rs
- [x] T062 [US4] Integrate audio alerts with workout engine interval transitions in src/workouts/engine.rs
- [x] T063 [US4] Integrate audio alerts with zone change detection in src/metrics/zones.rs
- [x] T064 [US4] Update src/ui/screens/settings.rs with audio settings: enable/disable per alert type, volume slider, voice selection
- [x] T065 [US4] Write unit tests for cue template expansion in tests/unit/audio_cues.rs

**Checkpoint**: Voice alerts play during workouts for intervals and zone changes

---

## Phase 7: User Story 5 - Smart Home Fan Control (Priority: P3)

**Goal**: Smart fan automatically adjusts speed based on power or HR zone

**Independent Test**: Configure MQTT broker, verify fan speed changes correlate with power output

### Implementation for User Story 5

- [x] T066 [P] [US5] Create MqttConfig and FanProfile types in src/integrations/mqtt/mod.rs
- [x] T067 [US5] Implement MqttClient trait with connect(), publish(), subscribe() using rumqttc in src/integrations/mqtt/client.rs
- [x] T068 [US5] Implement automatic reconnection with exponential backoff in src/integrations/mqtt/client.rs
- [x] T069 [US5] Implement FanController trait with start(), stop(), update_metrics() in src/integrations/mqtt/fan.rs
- [x] T070 [US5] Implement zone-to-speed mapping logic in src/integrations/mqtt/fan.rs
- [x] T071 [US5] Integrate fan controller with ride metrics updates in src/app.rs
- [x] T072 [US5] Update src/ui/screens/settings.rs with MQTT settings: broker host/port, TLS, credentials, test connection button
- [x] T073 [US5] Add fan profile configuration UI with zone-to-speed mapping editor in src/ui/screens/settings.rs
- [x] T074 [US5] Write integration tests for MQTT fan control in tests/integration/mqtt_fan.rs

**Checkpoint**: Fan speed adjusts automatically based on training zone

---

## Phase 8: User Story 6 - External Display Streaming (Priority: P3)

**Goal**: Real-time metrics viewable on secondary devices via WebSocket

**Independent Test**: Start streaming, connect browser on secondary device, verify metrics update in real-time

### Implementation for User Story 6

- [x] T075 [P] [US6] Create StreamingConfig and StreamingSession types in src/integrations/streaming/mod.rs
- [x] T076 [US6] Implement StreamingServer trait with start(), stop(), broadcast_metrics() using tokio-tungstenite in src/integrations/streaming/server.rs
- [x] T077 [US6] Implement PIN-based authentication with 6-digit PIN generation and validation in src/integrations/streaming/pin.rs
- [x] T078 [US6] Create embedded HTML/CSS/JS dashboard as const string in src/integrations/streaming/dashboard.rs
- [x] T079 [US6] Implement QR code generation for connection URL using qrcode crate in src/integrations/streaming/server.rs
- [x] T080 [US6] Integrate streaming server with ride metrics broadcasts in src/app.rs
- [x] T081 [P] [US6] Create src/ui/screens/streaming.rs with streaming enable toggle, URL display, QR code, PIN display, connected clients list
- [x] T082 [US6] Add streaming screen to navigation in src/ui/screens/mod.rs
- [x] T083 [US6] Write integration tests for WebSocket streaming in tests/integration/websocket.rs

**Checkpoint**: Secondary devices can view real-time ride metrics via browser

---

## Phase 9: User Story 7 - Stream Deck / USB Button Integration (Priority: P3)

**Goal**: Physical buttons trigger ride actions (lap, pause, skip interval)

**Independent Test**: Connect USB HID device, map buttons, verify button presses trigger actions during ride

### Implementation for User Story 7

- [x] T084 [P] [US7] Create HidDevice and ButtonMapping types in src/hid/device.rs
- [x] T085 [P] [US7] Create ButtonAction enum with all available actions in src/hid/actions.rs
- [x] T086 [US7] Implement HidDeviceManager trait with scan_devices(), open_device() using hidapi in src/hid/device.rs
- [x] T087 [US7] Implement device hot-plug detection and monitoring in src/hid/device.rs
- [x] T088 [US7] Implement ButtonInputHandler trait with register_mappings(), learning mode in src/hid/mapping.rs
- [x] T089 [US7] Implement ActionExecutor trait with execute() for all ButtonAction variants in src/hid/actions.rs
- [x] T090 [US7] Add known device profiles (Stream Deck VID/PID) in src/hid/device.rs
- [x] T091 [US7] Integrate HID button events with ride/workout controls in src/app.rs
- [x] T092 [US7] Update src/ui/screens/settings.rs with HID device list and button mapping UI with learning mode
- [x] T093 [US7] Write integration tests for HID button actions in tests/integration/hid_buttons.rs

**Checkpoint**: Physical buttons control ride actions

---

## Phase 10: User Story 8 - Weather Integration (Priority: P4)

**Goal**: Display current local weather conditions during rides

**Independent Test**: Configure location, start ride, verify temperature and conditions display

### Implementation for User Story 8

- [x] T094 [P] [US8] Create WeatherConfig and WeatherData types in src/integrations/weather/mod.rs
- [x] T095 [US8] Implement WeatherProvider trait with get_weather(), refresh() using reqwest in src/integrations/weather/provider.rs
- [x] T096 [US8] Implement weather data caching with 30-minute expiry in src/integrations/weather/provider.rs
- [x] T097 [US8] Implement graceful fallback when API unavailable in src/integrations/weather/provider.rs
- [x] T098 [P] [US8] Create src/ui/widgets/weather_widget.rs with temperature, conditions icon, humidity display
- [x] T099 [US8] Integrate weather widget into ride screen in src/ui/screens/ride.rs
- [x] T100 [US8] Update src/ui/screens/settings.rs with weather settings: enable, API key input, location lat/lon, units

**Checkpoint**: Weather conditions display during rides

---

## Phase 11: User Story 9 - Fitness Watch Data Sync (Priority: P4)

**Goal**: Rides sync to Garmin Connect, Strava, or Apple Health

**Independent Test**: Complete ride, initiate sync, verify activity appears on external platform

### Implementation for User Story 9

- [x] T101 [P] [US9] Create PlatformSync, SyncRecord, TokenStatus types in src/integrations/sync/mod.rs
- [x] T102 [US9] Implement OAuthHandler trait with start_authorization(), handle_callback(), refresh_token() in src/integrations/sync/oauth.rs
- [x] T103 [US9] Implement CredentialStore trait using keyring crate in src/integrations/sync/oauth.rs
- [x] T104 [US9] Implement local HTTP callback server for OAuth redirect in src/integrations/sync/oauth.rs
- [x] T105 [P] [US9] Implement Garmin Connect API upload in src/integrations/sync/garmin.rs
- [x] T106 [P] [US9] Implement Strava API upload in src/integrations/sync/strava.rs
- [x] T107 [US9] Implement PlatformUploader trait with upload(), retry() in src/integrations/sync/mod.rs
- [x] T108 [US9] Add sync button to ride summary screen with platform selection in src/ui/screens/ride_summary.rs
- [x] T109 [US9] Update src/ui/screens/settings.rs with platform connection UI (connect/disconnect for each platform)
- [x] T110 [US9] Add sync status display with error messages and retry option

**Checkpoint**: Completed rides can be synced to external fitness platforms

---

## Phase 12: User Story 10 - Muscle Oxygen Monitoring (Priority: P4)

**Goal**: Display real-time SmO2 data from muscle oxygen sensors

**Independent Test**: Connect SmO2 sensor, verify percentage displays and updates in real-time

### Implementation for User Story 10

- [x] T111 [P] [US10] Create SmO2Reading and MuscleLocation types in src/sensors/smo2.rs
- [x] T112 [US10] Implement SmO2Provider trait with discover_smo2_sensors(), connect(), get_current_reading() in src/sensors/smo2.rs
- [x] T113 [US10] Implement SmO2 BLE GATT service UUID parsing (Moxy service) in src/sensors/smo2.rs
- [x] T114 [US10] Create smo2_samples table and add to migrations in src/storage/schema.rs
- [x] T115 [US10] Integrate SmO2 data recording with ride samples in src/recording/recorder.rs
- [x] T116 [P] [US10] Create src/ui/widgets/smo2_display.rs with SmO2 percentage gauge and trend line
- [x] T117 [US10] Integrate SmO2 widget into ride screen in src/ui/screens/ride.rs
- [x] T118 [US10] Add SmO2 trends to ride summary in src/ui/screens/ride_summary.rs

**Checkpoint**: Muscle oxygen data displays during and after rides

---

## Phase 13: User Story 11 - Video Course Sync (Priority: P5)

**Goal**: Scenic video playback synchronized to ride speed

**Independent Test**: Load video for route, verify playback speed correlates with rider speed

### Implementation for User Story 11

- [x] T119 [P] [US11] Create VideoSync and SyncPoint types in src/video/sync.rs
- [x] T120 [US11] Implement VideoPlayer trait with load(), play(), pause(), set_speed() using ffmpeg-next in src/video/player.rs
- [x] T121 [US11] Implement frame buffering and hardware acceleration detection in src/video/player.rs
- [x] T122 [US11] Implement VideoSyncController trait with speed-to-playback mapping in src/video/sync.rs
- [x] T123 [US11] Implement pause on zero speed (< 5 km/h threshold) in src/video/sync.rs
- [x] T124 [US11] Integrate video frame rendering with egui texture in src/video/player.rs
- [x] T125 [US11] Add video panel to ride screen (togglable) in src/ui/screens/ride.rs
- [x] T126 [US11] Create video sync configuration in route settings

**Checkpoint**: Video playback adjusts speed to match rider pace ✅

---

## Phase 14: User Story 12 - Pedal Sensor Integration (Priority: P5)

**Goal**: Display force vectors and advanced pedaling dynamics from compatible pedal sensors

**Independent Test**: Connect compatible pedal sensors, verify force vector data displays

### Implementation for User Story 12

- [x] T127 [US12] Extend CyclingDynamicsData with PowerPhase (force vectors) in src/sensors/dynamics.rs
- [x] T128 [US12] Implement extended pedal metrics parsing for Shimano/Assioma in src/sensors/dynamics.rs
- [x] T129 [US12] Add force vector visualization to dynamics display widget in src/ui/widgets/dynamics_display.rs
- [x] T130 [US12] Include extended pedal data in FIT export in src/recording/exporter_fit.rs

**Checkpoint**: Advanced pedaling metrics visible from compatible sensors ✅

---

## Phase 15: User Story 13 - Cadence Sensor Fusion (Priority: P5)

**Goal**: Combine multiple cadence sources for improved accuracy

**Independent Test**: Connect two cadence sources, verify fused value is smoother than either alone

### Implementation for User Story 13

- [x] T131 [P] [US13] Create SensorFusionConfig and FusionDiagnostics types in src/sensors/fusion.rs
- [x] T132 [US13] Implement SensorFusion trait with configure_fusion(), get_fused_value() in src/sensors/fusion.rs
- [x] T133 [US13] Implement complementary filter algorithm for cadence fusion in src/sensors/fusion.rs
- [x] T134 [US13] Implement sensor dropout detection and seamless fallback in src/sensors/fusion.rs
- [x] T135 [US13] Integrate sensor fusion with cadence display in src/app.rs
- [x] T136 [US13] Write unit tests for sensor fusion algorithms in tests/unit/fusion.rs

**Checkpoint**: Fused cadence value from multiple sensors with dropout handling

---

## Phase 16: User Story 14 - Motion Tracking / Rocker Plate (Priority: P5)

**Goal**: Capture and display motion data from IMU sensors

**Independent Test**: Connect IMU sensor, verify motion data captured and displayed

### Implementation for User Story 14

- [x] T137 [P] [US14] Create MotionSample, Vector3, Quaternion types in src/sensors/imu.rs
- [x] T138 [US14] Implement MotionProvider trait with discover_motion_sensors(), connect(), calibrate() in src/sensors/imu.rs
- [x] T139 [US14] Create motion_samples table and add to migrations in src/storage/schema.rs
- [x] T140 [US14] Integrate motion data recording with ride samples in src/recording/recorder.rs
- [x] T141 [US14] Add simple tilt indicator widget to ride screen (optional display) in src/ui/widgets/tilt_indicator.rs
- [x] T142 [US14] Add motion visualization to post-ride analysis in src/ui/screens/ride_detail.rs

**Checkpoint**: Motion data captured from IMU sensors for analysis

---

## Phase 17: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T143 [P] Add error handling and user-friendly error messages for all new hardware failures
- [x] T144 [P] Add tracing/logging for all new modules with appropriate log levels
- [x] T145 Review and update src/storage/database.rs with all new migration functions
- [x] T146 [P] Update Cargo.toml with feature flags for optional dependencies (video, healthkit)
- [x] T147 [P] Create conditional compilation for macOS HealthKit in src/integrations/sync/healthkit.rs
- [x] T148 Run cargo fmt and cargo clippy on all new code
- [ ] T149 Validate quickstart.md hardware setup instructions work on all platforms
- [ ] T150 Performance testing: verify sensor data latency <100ms, audio <500ms, button <100ms

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Stories (Phase 3-16)**: All depend on Foundational phase completion
  - User stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3 → P4 → P5)
- **Polish (Phase 17)**: Depends on all desired user stories being complete

### User Story Dependencies

| Story | Priority | Dependencies | Can Parallel With |
|-------|----------|--------------|-------------------|
| US1 (ANT+) | P1 | Foundational only | US2 |
| US2 (Incline) | P1 | Foundational only | US1 |
| US3 (Dynamics) | P2 | Foundational only | US4 |
| US4 (Audio) | P2 | Foundational only | US3 |
| US5 (Fan/MQTT) | P3 | Foundational only | US6, US7 |
| US6 (Streaming) | P3 | Foundational only | US5, US7 |
| US7 (HID) | P3 | Foundational only | US5, US6 |
| US8 (Weather) | P4 | Foundational only | US9, US10 |
| US9 (Sync) | P4 | Foundational only | US8, US10 |
| US10 (SmO2) | P4 | Foundational only | US8, US9 |
| US11 (Video) | P5 | Foundational only | US12, US13, US14 |
| US12 (Pedals) | P5 | US3 (extends dynamics) | US11, US13, US14 |
| US13 (Fusion) | P5 | Foundational only | US11, US12, US14 |
| US14 (Motion) | P5 | Foundational only | US11, US12, US13 |

### Within Each User Story

- Types/models before service implementations
- Services before UI integration
- Core implementation before settings UI
- All story tasks complete before checkpoint

### Parallel Opportunities

**Setup Phase**:
- T002, T003, T004, T005, T006, T007 can all run in parallel

**Foundational Phase**:
- T011, T012, T013, T014, T015, T016 can run in parallel (after T008-T010)

**User Story 1 (ANT+)**:
- T019, T020 (types) in parallel
- T024, T025, T026, T027 (profiles) in parallel after T023

**All User Stories**:
- Once Foundational completes, all P1 stories can start in parallel
- Within same priority tier, stories are independent

---

## Parallel Example: User Story 1

```bash
# Launch type definitions in parallel:
Task: "Create AntDongle and DongleStatus types in src/sensors/ant/dongle.rs"
Task: "Create AntChannel and ChannelStatus types in src/sensors/ant/channels.rs"

# After channel manager, launch profiles in parallel:
Task: "Implement ANT+ Power (PWR) profile parser in src/sensors/ant/profiles/power.rs"
Task: "Implement ANT+ Heart Rate (HRM) profile parser in src/sensors/ant/profiles/hr.rs"
Task: "Implement ANT+ FE-C profile parser in src/sensors/ant/profiles/fec.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1-2 Only)

1. Complete Phase 1: Setup (T001-T007)
2. Complete Phase 2: Foundational (T008-T018) - CRITICAL BLOCKING PHASE
3. Complete Phase 3: User Story 1 - ANT+ (T019-T034)
4. Complete Phase 4: User Story 2 - Incline (T035-T044)
5. **STOP and VALIDATE**: Test both stories independently
6. Deploy/demo if ready - this is a functional MVP

### Incremental Delivery by Priority

| Increment | Stories | Tasks | Cumulative Value |
|-----------|---------|-------|------------------|
| MVP | US1 + US2 | T001-T044 | ANT+ sensors + gradient simulation |
| +P2 | US3 + US4 | T045-T065 | Cycling dynamics + audio cues |
| +P3 | US5 + US6 + US7 | T066-T093 | Fan control + streaming + HID buttons |
| +P4 | US8 + US9 + US10 | T094-T118 | Weather + sync + SmO2 |
| +P5 | US11-US14 | T119-T142 | Video + pedals + fusion + motion |
| Polish | - | T143-T150 | Quality improvements |

### Parallel Team Strategy

With 3 developers after Foundational phase:
- Developer A: US1 (ANT+) → US3 (Dynamics) → US5 (Fan) → US8 (Weather) → US11 (Video)
- Developer B: US2 (Incline) → US4 (Audio) → US6 (Streaming) → US9 (Sync) → US12 (Pedals)
- Developer C: (wait for P3) → US7 (HID) → US10 (SmO2) → US13 (Fusion) → US14 (Motion)

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- US12 (Pedals) extends US3 (Dynamics) - minor dependency but can be parallelized with care
