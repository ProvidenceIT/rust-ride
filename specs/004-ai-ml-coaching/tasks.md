# Tasks: AI & Machine Learning Coaching

**Input**: Design documents from `/specs/004-ai-ml-coaching/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependencies, and module structure

- [x] T001 Add reqwest dependency with json feature to Cargo.toml
- [x] T002 [P] Create ml module directory structure at src/ml/
- [x] T003 [P] Create goals module directory structure at src/goals/
- [x] T004 [P] Create test fixtures directory at tests/fixtures/ml/
- [x] T005 Create src/ml/mod.rs with module exports for all ML submodules
- [x] T006 [P] Create src/goals/mod.rs with module exports for types and manager

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

### Database Schema Migration

- [ ] T007 Add MIGRATION_V2_TO_V3 constant to src/storage/schema.rs with new ML tables (training_goals, ml_predictions, workout_recommendations, performance_projections, builtin_workouts, fatigue_states)
- [ ] T008 Update CURRENT_VERSION to 3 and add migration logic in src/storage/schema.rs
- [ ] T009 Create src/storage/ml_store.rs with MlStore struct for ML prediction CRUD operations

### Cloud API Client

- [ ] T010 Create src/ml/client.rs with MlClient struct implementing cloud API calls via reqwest
- [ ] T011 [P] Implement offline queue with crossbeam channel in src/ml/client.rs for request queuing
- [ ] T012 [P] Implement exponential backoff retry logic (30s, 60s, 2m, 5m) in src/ml/client.rs

### Prediction Cache

- [ ] T013 Create src/ml/cache.rs with MlCache struct for SQLite-backed prediction storage
- [ ] T014 Implement cache expiry logic (7 days FTP, 24h fatigue, 1h difficulty) in src/ml/cache.rs

### Core Types

- [ ] T015 Create src/ml/types.rs with shared types: MlError enum, PredictionType enum, PredictionSource enum, Confidence levels
- [ ] T016 [P] Create src/goals/types.rs with TrainingGoal struct, GoalType enum, GoalStatus enum, TargetMetric struct

### Training Goals Management

- [ ] T017 Create src/goals/manager.rs with GoalManager struct for goal CRUD operations
- [ ] T018 Implement goal priority management (unique priority per user) in src/goals/manager.rs

### Built-In Workout Library

- [ ] T019 Create src/workouts/library.rs with BuiltInWorkout struct and WorkoutLibrary struct
- [ ] T020 Implement workout seeding logic for 80 initial workouts in src/workouts/library.rs
- [ ] T021 Add search/filter methods to WorkoutLibrary (by energy system, duration, difficulty) in src/workouts/library.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - AI FTP Prediction (Priority: P1)

**Goal**: Automatically predict FTP from workout history without formal tests

**Independent Test**: Record 2-3 weeks of varied workouts with power data, verify system predicts FTP within 5% of formal test result

### Implementation for User Story 1

- [ ] T022 [P] [US1] Create FtpPredictionPayload struct in src/ml/ftp_prediction.rs with predicted_ftp, confidence, method, supporting_efforts
- [ ] T023 [P] [US1] Create SupportingEffort struct in src/ml/ftp_prediction.rs with ride_id, duration, power, date
- [ ] T024 [US1] Implement FtpPredictor struct with predict() method wrapping cloud API in src/ml/ftp_prediction.rs
- [ ] T025 [US1] Implement local fallback prediction using existing FtpDetector in src/ml/ftp_prediction.rs
- [ ] T026 [US1] Implement should_notify() method checking >3% difference from current FTP in src/ml/ftp_prediction.rs
- [ ] T027 [US1] Add post-ride trigger integration calling FtpPredictor in src/metrics/analytics/triggers.rs
- [ ] T028 [US1] Create FTP prediction notification widget in src/ui/widgets/ftp_notification.rs
- [ ] T029 [US1] Add FTP prediction display to AI Insights screen in src/ui/screens/insights.rs
- [ ] T030 [US1] Handle insufficient data case with clear user messaging in src/ml/ftp_prediction.rs

**Checkpoint**: FTP prediction functional and testable independently

---

## Phase 4: User Story 2 - Real-Time Fatigue Detection (Priority: P1)

**Goal**: Detect fatigue during rides and alert rider to adjust effort

**Independent Test**: During workout, simulate fatigue patterns (HR drift, power decline) and verify appropriate alerts

### Implementation for User Story 2

- [ ] T031 [P] [US2] Create FatigueAnalysis struct in src/ml/fatigue_detection.rs with aerobic_decoupling_score, power_variability_index, hrv_indicator, severity
- [ ] T032 [P] [US2] Create AthleteBaseline struct in src/ml/fatigue_detection.rs with resting_hr, max_hr, typical thresholds
- [ ] T033 [P] [US2] Create FatigueState entity in src/ml/fatigue_detection.rs with alert state and cooldown tracking
- [ ] T034 [US2] Implement FatigueDetector struct with analyze() method in src/ml/fatigue_detection.rs
- [ ] T035 [US2] Implement aerobic_decoupling() calculation (HR drift vs constant power) in src/ml/fatigue_detection.rs
- [ ] T036 [US2] Implement power_variability_index() calculation in src/ml/fatigue_detection.rs
- [ ] T037 [US2] Implement should_alert() method with configurable thresholds in src/ml/fatigue_detection.rs
- [ ] T038 [US2] Implement alert cooldown logic (dismiss_alert, is_in_cooldown) for 5-10 min cooldown in src/ml/fatigue_detection.rs
- [ ] T039 [US2] Create fatigue alert widget in src/ui/widgets/fatigue_alert.rs
- [ ] T040 [US2] Integrate fatigue detection into ride screen with 5-min rolling analysis in src/ui/screens/ride.rs
- [ ] T041 [US2] Add fatigue state persistence to fatigue_states table via src/storage/ml_store.rs

**Checkpoint**: Fatigue detection functional and testable independently

---

## Phase 5: User Story 3 - Adaptive Workout Recommendations (Priority: P2)

**Goal**: Recommend workouts based on fitness, load, and goals

**Independent Test**: Set training goal, complete workouts, verify system recommends appropriate next workouts considering fatigue and progression

### Implementation for User Story 3

- [ ] T042 [P] [US3] Create WorkoutRecommendation struct in src/ml/workout_recommend.rs with workout_id, source, suitability_score, reasoning, energy_systems
- [ ] T043 [P] [US3] Create EnergySystem enum in src/ml/workout_recommend.rs (Neuromuscular, Anaerobic, Vo2max, Threshold, SweetSpot, Endurance, Recovery)
- [ ] T044 [US3] Implement WorkoutRecommender struct with recommend() method in src/ml/workout_recommend.rs
- [ ] T045 [US3] Implement ACWR-aware recommendation logic prioritizing recovery when ACWR > 1.3 in src/ml/workout_recommend.rs
- [ ] T046 [US3] Implement energy system gap detection (days since last training of each system) in src/ml/workout_recommend.rs
- [ ] T047 [US3] Implement suitability scoring algorithm combining goal alignment, load, and variety in src/ml/workout_recommend.rs
- [ ] T048 [US3] Implement recommend_for_goal() method targeting specific goal in src/ml/workout_recommend.rs
- [ ] T049 [US3] Add recommendation status tracking (pending, accepted, declined, completed) in src/ml/workout_recommend.rs
- [ ] T050 [US3] Create workout recommendation card widget in src/ui/widgets/recommendation_card.rs
- [ ] T051 [US3] Create training goals screen with goal list and add/edit/delete in src/ui/screens/goals.rs
- [ ] T052 [US3] Add recommendations section to AI Insights screen in src/ui/screens/insights.rs

**Checkpoint**: Workout recommendations functional and testable independently

---

## Phase 6: User Story 4 - Performance Trend Forecasting (Priority: P2)

**Goal**: Project fitness trajectory for planning around target events

**Independent Test**: With 4+ weeks training data, verify system projects fitness trends for next 4-12 weeks with reasonable accuracy

### Implementation for User Story 4

- [ ] T053 [P] [US4] Create PerformanceProjection struct in src/ml/performance_forecast.rs with forecast points, plateau flag, detraining risk
- [ ] T054 [P] [US4] Create ProjectedCtl struct in src/ml/performance_forecast.rs with date, projected_ctl, confidence_low, confidence_high
- [ ] T055 [P] [US4] Create DetrainingRisk enum in src/ml/performance_forecast.rs (None, Low, Medium, High)
- [ ] T056 [P] [US4] Create EventReadiness struct in src/ml/performance_forecast.rs with goal_id, target_ctl, projected_ctl, gap, recommendation
- [ ] T057 [US4] Implement PerformanceForecaster struct with forecast() method in src/ml/performance_forecast.rs
- [ ] T058 [US4] Implement local EWMA projection (linear trend extrapolation) as offline fallback in src/ml/performance_forecast.rs
- [ ] T059 [US4] Implement detect_plateau() method checking slope near zero in src/ml/performance_forecast.rs
- [ ] T060 [US4] Implement assess_detraining_risk() based on training frequency in src/ml/performance_forecast.rs
- [ ] T061 [US4] Implement event_gap() calculating CTL gap to target in src/ml/performance_forecast.rs
- [ ] T062 [US4] Create forecast chart widget in src/ui/widgets/forecast_chart.rs
- [ ] T063 [US4] Add forecast visualization to AI Insights screen in src/ui/screens/insights.rs
- [ ] T064 [US4] Add target event date entry to training goals screen in src/ui/screens/goals.rs

**Checkpoint**: Performance forecasting functional and testable independently

---

## Phase 7: User Story 5 - Workout Difficulty Estimation (Priority: P3)

**Goal**: Show estimated difficulty for workouts before starting

**Independent Test**: View workout difficulty prediction, complete workout, verify prediction matched perceived exertion

### Implementation for User Story 5

- [ ] T065 [P] [US5] Create DifficultyEstimate struct in src/ml/difficulty.rs with base_difficulty, fatigue_adjustment, final_difficulty, factors
- [ ] T066 [P] [US5] Create DifficultyFactors struct in src/ml/difficulty.rs with user_ftp, workout_if, workout_tss, current_atl
- [ ] T067 [US5] Implement DifficultyEstimator struct with estimate() method in src/ml/difficulty.rs
- [ ] T068 [US5] Implement personalized difficulty calculation based on FTP and workout intensity in src/ml/difficulty.rs
- [ ] T069 [US5] Implement apply_fatigue_adjustment() modifying difficulty based on current ATL in src/ml/difficulty.rs
- [ ] T070 [US5] Add difficulty display to workout list screen in src/ui/screens/workouts.rs
- [ ] T071 [US5] Add difficulty display to workout detail/preview screen in src/ui/screens/workout_detail.rs

**Checkpoint**: Difficulty estimation functional and testable independently

---

## Phase 8: User Story 6 - Power Curve Profiling & Rider Classification (Priority: P3)

**Goal**: Classify rider type (sprinter, climber, TT, all-rounder) and track evolution

**Independent Test**: Accumulate sufficient power data across durations, verify system classifies rider type with actionable insights

### Implementation for User Story 6

- [ ] T072 [P] [US6] Extend existing RiderProfile in src/metrics/analytics/rider_classification.rs with classification explanation
- [ ] T073 [US6] Implement weakness identification relative to strengths in src/metrics/analytics/rider_classification.rs
- [ ] T074 [US6] Implement rider type evolution tracking over time in src/metrics/analytics/rider_classification.rs
- [ ] T075 [US6] Create rider profile insights section in src/ui/screens/insights.rs
- [ ] T076 [US6] Add historical rider type chart showing evolution in src/ui/screens/insights.rs

**Checkpoint**: Rider profiling enhanced and testable independently

---

## Phase 9: User Story 7 - Cadence & Technique Analysis (Priority: P4)

**Goal**: Provide feedback on optimal cadence and technique degradation

**Independent Test**: Record workouts with varying cadences, verify system identifies optimal cadence zones and patterns

### Implementation for User Story 7

- [ ] T077 [P] [US7] Create CadenceAnalysis struct in src/ml/cadence_analysis.rs with optimal_range, efficiency_by_zone, degradation_pattern
- [ ] T078 [P] [US7] Create CadenceEfficiency struct in src/ml/cadence_analysis.rs with cadence_band, efficiency_score, sample_count
- [ ] T079 [P] [US7] Create DegradationPattern struct in src/ml/cadence_analysis.rs with onset_minutes, variability_increase
- [ ] T080 [US7] Implement CadenceAnalyzer struct with analyze() method in src/ml/cadence_analysis.rs
- [ ] T081 [US7] Implement detect_degradation() for technique breakdown under fatigue in src/ml/cadence_analysis.rs
- [ ] T082 [US7] Calculate efficiency metrics by cadence band in src/ml/cadence_analysis.rs
- [ ] T083 [US7] Add cadence analysis section to AI Insights screen in src/ui/screens/insights.rs

**Checkpoint**: Cadence analysis functional and testable independently

---

## Phase 10: User Story 8 - Training Load Adaptation Engine (Priority: P4)

**Goal**: Personalized load recommendations based on individual recovery capacity

**Independent Test**: Train consistently for 6+ weeks, verify system learns recovery patterns and adjusts recommendations

### Implementation for User Story 8

- [ ] T084 [P] [US8] Create AdaptationModel struct in src/ml/adaptation.rs with recovery_rate, optimal_ctl_range, optimal_acwr_range, confidence
- [ ] T085 [P] [US8] Create TrainingResponse struct in src/ml/adaptation.rs with ctl_sensitivity, fatigue_sensitivity, recovery_days
- [ ] T086 [P] [US8] Create LoadRecommendation struct in src/ml/adaptation.rs with suggested_tss, intensity_focus, recovery_needed, reasoning
- [ ] T087 [P] [US8] Create ModelConfidence enum in src/ml/adaptation.rs (Insufficient, Low, Medium, High based on weeks of data)
- [ ] T088 [US8] Implement AdaptationEngine struct with learn_patterns() method in src/ml/adaptation.rs
- [ ] T089 [US8] Implement recommend_load() using learned model in src/ml/adaptation.rs
- [ ] T090 [US8] Implement has_sufficient_data() checking 6+ weeks of history in src/ml/adaptation.rs
- [ ] T091 [US8] Add load adaptation insights to AI Insights screen in src/ui/screens/insights.rs

**Checkpoint**: Training load adaptation functional and testable independently

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

### AI Insights Screen Integration

- [ ] T092 Create unified AI Insights screen combining all ML outputs in src/ui/screens/insights.rs
- [ ] T093 Add navigation to AI Insights from main menu in src/ui/screens/mod.rs

### Configuration

- [ ] T094 [P] Add ML config section to config.toml (cloud_enabled, api_url, cache_cleanup_days)
- [ ] T095 [P] Add fatigue alert config (enabled, cooldown_minutes, thresholds) to config.toml
- [ ] T096 [P] Add recommendation config (auto_refresh, default_duration) to config.toml

### Offline Handling

- [ ] T097 Implement cached prediction display with "last updated" timestamps in src/ml/cache.rs
- [ ] T098 Implement queue flush on connectivity restoration in src/ml/client.rs

### Error Handling

- [ ] T099 Add user-friendly error messages for all MlError variants in src/ml/types.rs
- [ ] T100 Add insufficient data guidance (what workout types needed) across all ML modules

### Performance

- [ ] T101 Ensure all ML operations complete within 2 seconds (SC-009) via async execution
- [ ] T102 Add cache cleanup job for expired predictions in src/ml/cache.rs

### Validation

- [ ] T103 Run quickstart.md validation scenarios for all user stories
- [ ] T104 Validate all success criteria (SC-001 through SC-011) can be measured

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-10)**: All depend on Foundational phase completion
  - User Story 1 (FTP Prediction) and User Story 2 (Fatigue Detection) can run in parallel
  - User Story 3 (Recommendations) can start after Foundational, benefits from US1
  - User Story 4 (Forecasting) can start after Foundational
  - User Story 5-8 can proceed independently after Foundational
- **Polish (Phase 11)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundational only - can start immediately after Phase 2
- **User Story 2 (P1)**: Foundational only - can run parallel with US1
- **User Story 3 (P2)**: Foundational + benefits from US1 FTP predictions for workout intensity matching
- **User Story 4 (P2)**: Foundational only - can run parallel with US1/US2
- **User Story 5 (P3)**: Foundational + benefits from US1 FTP for difficulty calculation
- **User Story 6 (P3)**: Foundational only - extends existing analytics
- **User Story 7 (P4)**: Foundational only - independent cadence analysis
- **User Story 8 (P4)**: Foundational only - independent adaptation engine

### Within Each User Story

- Types/structs before implementation logic
- Core implementation before UI integration
- Local functionality before cloud integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational completes, US1 and US2 can start in parallel
- All struct definitions within a story marked [P] can run in parallel
- Different user stories can be worked on by different team members

---

## Parallel Example: User Story 1 (FTP Prediction)

```bash
# Launch all struct definitions for User Story 1 together:
Task: T022 "Create FtpPredictionPayload struct in src/ml/ftp_prediction.rs"
Task: T023 "Create SupportingEffort struct in src/ml/ftp_prediction.rs"

# Then implement core logic sequentially:
Task: T024 "Implement FtpPredictor struct with predict() method"
Task: T025 "Implement local fallback prediction"
# etc.
```

## Parallel Example: User Story 2 (Fatigue Detection)

```bash
# Launch all struct definitions for User Story 2 together:
Task: T031 "Create FatigueAnalysis struct"
Task: T032 "Create AthleteBaseline struct"
Task: T033 "Create FatigueState entity"

# Then implement core logic sequentially:
Task: T034 "Implement FatigueDetector struct"
Task: T035 "Implement aerobic_decoupling()"
# etc.
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (FTP Prediction)
4. Complete Phase 4: User Story 2 (Fatigue Detection)
5. **STOP and VALIDATE**: Test both P1 stories independently
6. Deploy/demo if ready - this is the core ML coaching MVP

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 + 2 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 3 (Recommendations) → Test independently → Deploy/Demo
4. Add User Story 4 (Forecasting) → Test independently → Deploy/Demo
5. Add User Stories 5-8 → Test independently → Deploy/Demo
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (FTP Prediction)
   - Developer B: User Story 2 (Fatigue Detection)
3. After P1 stories complete:
   - Developer A: User Story 3 (Recommendations)
   - Developer B: User Story 4 (Forecasting)
4. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Cloud API is optional - all features have local fallbacks for offline use
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
