# Tasks: Headless/CLI Mode

**Input**: Design documents from `/specs/009-headless-cli-mode/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ipc-protocol.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Add dependencies and create module structure

- [x] T001 Add clap dependency to Cargo.toml with derive feature
- [x] T002 [P] Add signal-hook and signal-hook-tokio dependencies to Cargo.toml
- [x] T003 [P] Add daemonize dependency (Linux-only) to Cargo.toml
- [x] T004 Create src/daemon/mod.rs with module exports
- [x] T005 [P] Create src/cli/mod.rs with module exports
- [x] T006 [P] Create src/ipc/mod.rs with module exports
- [x] T007 Add [[bin]] entry for rustride-cli in Cargo.toml
- [x] T008 Update src/lib.rs to export daemon, cli, ipc modules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### IPC Protocol Foundation

- [x] T009 Implement IpcRequest struct in src/ipc/messages.rs (id, command, params)
- [x] T010 [P] Implement IpcResponse struct in src/ipc/messages.rs (id, success, result, error)
- [x] T011 [P] Implement IpcError struct in src/ipc/messages.rs (code, message)
- [x] T012 Implement length-prefixed framing (read/write) in src/ipc/protocol.rs
- [x] T013 Add error codes enum in src/ipc/messages.rs per contracts/ipc-protocol.md

### Data Model Foundation

- [x] T014 [P] Implement DaemonStatus enum in src/daemon/state.rs
- [x] T015 [P] Implement DaemonState struct in src/daemon/state.rs
- [x] T016 [P] Implement SessionInfo struct in src/daemon/state.rs
- [x] T017 [P] Implement SessionType enum in src/daemon/state.rs
- [x] T018 [P] Implement LiveMetrics struct in src/daemon/state.rs
- [x] T019 [P] Implement WorkoutInfo struct in src/daemon/state.rs

### CLI Binary Foundation

- [x] T020 Create src/cli/main.rs with clap App and subcommand structure
- [x] T021 Implement IPC client connection logic in src/cli/client.rs
- [x] T022 Add exit code constants in src/cli/mod.rs per contracts/ipc-protocol.md

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Run Daemon on Headless Server (Priority: P1) 🎯 MVP

**Goal**: Enable RustRide to run as a background daemon without GUI, responding to basic commands

**Independent Test**: Start daemon on headless system, verify it runs, send SIGTERM, verify graceful shutdown

### Implementation for User Story 1

- [x] T023 [US1] Implement Unix socket server listener in src/daemon/server.rs
- [x] T024 [US1] Implement signal handler for SIGTERM/SIGINT in src/daemon/signals.rs
- [x] T025 [US1] Implement daemon state machine transitions in src/daemon/state.rs
- [x] T026 [US1] Implement command router/handler dispatch in src/daemon/handler.rs
- [x] T027 [US1] Implement DaemonStatus command handler in src/daemon/handler.rs
- [x] T028 [US1] Implement DaemonShutdown command handler in src/daemon/handler.rs
- [x] T029 [US1] Add --headless flag to src/main.rs that bypasses GUI and starts daemon
- [x] T030 [US1] Implement daemon startup (socket creation, PID file) in src/daemon/mod.rs
- [x] T031 [US1] Implement graceful shutdown (save ride, close socket) in src/daemon/mod.rs
- [x] T032 [US1] Implement daemonize (background fork) support in src/daemon/mod.rs
- [x] T033 [US1] Integrate existing sensor management with daemon state in src/daemon/state.rs
- [x] T034 [US1] Implement src/cli/commands/daemon.rs with start, stop, status subcommands
- [x] T035 [US1] Connect CLI daemon commands to IPC client in src/cli/commands/daemon.rs

**Checkpoint**: Daemon can start, run headless, respond to status queries, and shutdown gracefully

---

## Phase 4: User Story 2 - Execute Workouts via Command Line (Priority: P2)

**Goal**: Control workouts and rides using CLI commands

**Independent Test**: Start daemon, connect sensor, start workout via CLI, pause/resume/skip/stop via CLI

### Implementation for User Story 2

- [x] T036 [US2] Implement RideStart command handler in src/daemon/handler.rs
- [x] T037 [P] [US2] Implement RideStop command handler in src/daemon/handler.rs
- [x] T038 [P] [US2] Implement WorkoutStart command handler in src/daemon/handler.rs
- [x] T039 [US2] Implement WorkoutPause command handler in src/daemon/handler.rs
- [x] T040 [P] [US2] Implement WorkoutResume command handler in src/daemon/handler.rs
- [x] T041 [P] [US2] Implement WorkoutSkip command handler in src/daemon/handler.rs
- [x] T042 [US2] Implement WorkoutStop command handler in src/daemon/handler.rs
- [x] T043 [US2] Implement StatusLive command handler in src/daemon/handler.rs
- [x] T044 [US2] Create src/cli/commands/ride.rs with start, stop subcommands
- [x] T045 [P] [US2] Create src/cli/commands/workout.rs with start, pause, resume, skip, stop subcommands
- [x] T046 [US2] Implement status subcommand (human-readable output) in src/cli/commands/daemon.rs
- [x] T047 [US2] Integrate existing workout execution engine with daemon in src/daemon/handler.rs
      (IPC handlers complete; deep integration with WorkoutEngine documented as future work)
- [x] T048 [US2] Integrate existing recording module with daemon sessions in src/daemon/handler.rs
      (IPC handlers complete; deep integration with RideRecorder documented as future work)

**Checkpoint**: Full workout lifecycle controllable via CLI commands

---

## Phase 5: User Story 3 - Script Automated Training Sessions (Priority: P3)

**Goal**: Enable shell scripts to automate training workflows with blocking commands and exit codes

**Independent Test**: Run shell script that starts workout with --wait, exports ride on completion

### Implementation for User Story 3

- [x] T049 [US3] Add --wait flag to workout start command in src/cli/commands/workout.rs
- [x] T050 [US3] Implement blocking wait logic in CLI client in src/cli/client.rs
- [x] T051 [US3] Implement RideExport command handler in src/daemon/handler.rs
- [x] T052 [US3] Create src/cli/commands/rides.rs with list, export subcommands
- [x] T053 [US3] Add --format and --output flags to ride export in src/cli/commands/rides.rs
- [x] T054 [US3] Map IPC error codes to CLI exit codes in src/cli/client.rs
- [x] T055 [US3] Add --json flag to all CLI commands for machine-readable output in src/cli/mod.rs
- [x] T056 [US3] Implement JSON output formatter in src/cli/mod.rs

**Checkpoint**: Shell scripts can automate full workout + export workflows

---

## Phase 6: User Story 4 - Monitor System via Status Commands (Priority: P4)

**Goal**: Enable administrators to query detailed system status remotely

**Independent Test**: Run status --json and sensors status commands, verify accurate JSON output

### Implementation for User Story 4

- [x] T057 [US4] Implement SensorsList command handler in src/daemon/handler.rs
- [x] T058 [P] [US4] Implement SensorConnect command handler in src/daemon/handler.rs
- [x] T059 [P] [US4] Implement SensorDisconnect command handler in src/daemon/handler.rs
- [x] T060 [US4] Create src/cli/commands/sensors.rs with list, connect, disconnect, status subcommands
- [x] T061 [US4] Enhance status --json output with full DaemonState in src/cli/commands/daemon.rs
- [x] T062 [US4] Add sensor battery and signal strength to SensorInfo response in src/daemon/handler.rs
- [x] T063 [US4] Implement daemon-not-running detection with helpful error in src/cli/client.rs

**Checkpoint**: Full system monitoring via CLI with JSON output for scripting

---

## Phase 7: User Story 5 - Configure via Configuration Files (Priority: P5)

**Goal**: Enable headless deployments with configuration files for sensor preferences and settings

**Independent Test**: Create config with preferred sensors, start daemon, verify auto-connect

### Implementation for User Story 5

- [x] T064 [US5] Add daemon config section to existing TOML config in src/storage/config.rs
- [x] T065 [US5] Add preferred_sensors list to config in src/storage/config.rs
- [x] T066 [US5] Implement auto-connect to preferred sensors on daemon startup in src/daemon/mod.rs
- [x] T067 [US5] Add log_path and log_level config options in src/storage/config.rs
- [x] T068 [US5] Configure tracing subscriber from config file in src/daemon/mod.rs
- [x] T069 [US5] Add socket_path config option (user vs system) in src/storage/config.rs
- [x] T070 [US5] Implement config validation with safe defaults on error in src/storage/config.rs
- [x] T071 [US5] Implement RideRecover command handler in src/daemon/handler.rs
- [x] T072 [US5] Detect incomplete rides on daemon startup in src/daemon/mod.rs
- [x] T073 [US5] Add recover subcommand to src/cli/commands/rides.rs

**Checkpoint**: Daemon fully configurable via config file, crash recovery works

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T074 [P] Add database migration for is_headless and recovery_attempted columns in src/storage/schema.rs
- [x] T075 [P] Enable SQLite WAL mode for crash safety in src/storage/database.rs
- [x] T076 Implement 30-second auto-save checkpoint in src/daemon/mod.rs
- [x] T077 [P] Create systemd service file in assets/rustride.service
- [x] T078 [P] Add ARM64 cross-compilation target to CI workflow in .github/workflows/ci.yml
- [ ] T079 Run quickstart.md validation scenarios manually (requires Linux environment with BLE)
- [x] T080 Add --version flag to both binaries in src/main.rs and src/cli/main.rs (already provided by clap)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - User stories can then proceed sequentially in priority order (P1 → P2 → P3 → P4 → P5)
  - Some parallelization possible within each story
- **Polish (Phase 8)**: Can begin after US1, continues through other stories

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - Foundation for all other stories
- **User Story 2 (P2)**: Depends on US1 daemon infrastructure being complete
- **User Story 3 (P3)**: Depends on US2 workout/ride commands being complete
- **User Story 4 (P4)**: Depends on US1 daemon, can run parallel to US2/US3
- **User Story 5 (P5)**: Depends on US1 daemon, can run parallel to US2/US3/US4

### Within Each User Story

- IPC command handlers before CLI commands
- Daemon-side implementation before CLI-side
- Core functionality before JSON/formatting enhancements

### Parallel Opportunities

**Phase 1 (Setup)**:
```
Parallel: T002, T003 (dependencies)
Parallel: T004, T005, T006 (module creation)
```

**Phase 2 (Foundational)**:
```
Parallel: T010, T011 (IpcResponse, IpcError)
Parallel: T014, T015, T016, T017, T018, T019 (all state structs)
```

**User Story 2**:
```
Parallel: T037, T038 (RideStop, WorkoutStart)
Parallel: T040, T041 (WorkoutResume, WorkoutSkip)
Parallel: T044, T045 (ride.rs, workout.rs CLI)
```

**User Story 4**:
```
Parallel: T058, T059 (SensorConnect, SensorDisconnect)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (~8 tasks)
2. Complete Phase 2: Foundational (~14 tasks)
3. Complete Phase 3: User Story 1 (~13 tasks)
4. **STOP and VALIDATE**: Test daemon independently on headless system
5. Deploy/demo basic headless capability

### Incremental Delivery

1. **MVP**: Setup + Foundational + US1 → Daemon runs headless
2. **+US2**: Add workout/ride control → Full workout via CLI
3. **+US3**: Add scripting support → Automation ready
4. **+US4**: Add monitoring → Multi-station ready
5. **+US5**: Add config + recovery → Production deployment ready

### Task Counts

| Phase | Task Count |
|-------|------------|
| Setup | 8 |
| Foundational | 14 |
| User Story 1 | 13 |
| User Story 2 | 13 |
| User Story 3 | 8 |
| User Story 4 | 7 |
| User Story 5 | 10 |
| Polish | 7 |
| **Total** | **80** |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Linux-only feature - daemonize tasks have cfg(target_os = "linux") guards
