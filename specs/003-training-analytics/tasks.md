# Tasks: Training Science & Analytics

**Input**: Design documents from `/specs/003-training-analytics/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Unit tests included (constitution specifies Test-First approach)

**Organization**: Tasks grouped by user story to enable independent implementation and testing

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Exact file paths included in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and analytics module structure

- [x] T001 Create analytics module directory structure at src/metrics/analytics/
- [x] T002 Create analytics module root with exports in src/metrics/analytics/mod.rs
- [x] T003 Update metrics module to export analytics in src/metrics/mod.rs
- [x] T004 [P] Create AnalyticsError types with thiserror in src/metrics/analytics/error.rs
- [x] T005 [P] Create test fixtures directory at tests/fixtures/analytics/
- [x] T006 [P] Create sample ride power data fixture at tests/fixtures/analytics/sample_rides.json

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Database schema and storage layer that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T007 Add analytics tables to schema (PDC, CP, FTP, TrainingLoad, VO2max, RiderProfile) in src/storage/schema.rs
- [x] T008 Increment schema version from 1 to 2 and add migration logic in src/storage/schema.rs
- [x] T009 Create AnalyticsStore struct with connection handling in src/storage/analytics_store.rs
- [x] T010 [P] Implement PDC storage methods (load_pdc, save_pdc_points) in src/storage/analytics_store.rs
- [x] T011 [P] Implement CP model storage methods (load_current_cp_model, save_cp_model) in src/storage/analytics_store.rs
- [x] T012 [P] Implement FTP estimate storage methods (load_accepted_ftp, save_ftp_estimate, accept_ftp) in src/storage/analytics_store.rs
- [x] T013 [P] Implement training load storage methods (load_daily_load, save_daily_load) in src/storage/analytics_store.rs
- [x] T014 [P] Implement VO2max storage methods (load_current_vo2max, save_vo2max) in src/storage/analytics_store.rs
- [x] T015 [P] Implement rider profile storage methods (load_rider_profile, save_rider_profile) in src/storage/analytics_store.rs
- [x] T016 Implement aggregate_daily_tss query for TSS aggregation in src/storage/analytics_store.rs
- [x] T017 Implement load_ride_power_samples query for MMP extraction in src/storage/analytics_store.rs
- [x] T018 Add database initialization call for analytics tables in src/storage/database.rs

**Checkpoint**: Storage layer ready - user story implementation can now begin

---

## Phase 3: User Story 1 - View Power Duration Curve (Priority: P1) MVP

**Goal**: Display Power Duration Curve showing max power at each duration from ride history

**Independent Test**: Record several rides with varying efforts, view PDC chart showing power profile

### Tests for User Story 1

- [x] T019 [P] [US1] Unit test for MmpCalculator with constant power in src/metrics/analytics/pdc.rs
- [x] T020 [P] [US1] Unit test for MmpCalculator with variable power (interval efforts) in src/metrics/analytics/pdc.rs
- [x] T021 [P] [US1] Unit test for PowerDurationCurve update logic in src/metrics/analytics/pdc.rs
- [x] T022 [P] [US1] Unit test for PDC monotonicity validation in src/metrics/analytics/pdc.rs

### Implementation for User Story 1

- [x] T023 [US1] Implement PdcPoint struct with duration_secs and power_watts in src/metrics/analytics/pdc.rs
- [x] T024 [US1] Implement PowerDurationCurve struct with points vector in src/metrics/analytics/pdc.rs
- [x] T025 [US1] Implement PowerDurationCurve::new() and from_points() in src/metrics/analytics/pdc.rs
- [x] T026 [US1] Implement PowerDurationCurve::power_at() with interpolation in src/metrics/analytics/pdc.rs
- [x] T027 [US1] Implement PowerDurationCurve::update() returning changed points in src/metrics/analytics/pdc.rs
- [x] T028 [US1] Implement PowerDurationCurve::has_sufficient_data_for_cp() in src/metrics/analytics/pdc.rs
- [x] T029 [US1] Implement MmpCalculator struct with duration buckets in src/metrics/analytics/pdc.rs
- [x] T030 [US1] Implement MmpCalculator::standard() with default durations (1s-5h) in src/metrics/analytics/pdc.rs
- [x] T031 [US1] Implement MmpCalculator::calculate() using monotonic deque algorithm in src/metrics/analytics/pdc.rs
- [x] T032 [US1] Implement MmpCalculator::calculate_single() for focused queries in src/metrics/analytics/pdc.rs
- [x] T033 [US1] Create PDC chart widget using egui_plot in src/ui/widgets/pdc_chart.rs
- [x] T034 [US1] Implement date range filter for PDC (30/60/90 days, all time) in src/ui/widgets/pdc_chart.rs
- [x] T035 [US1] Implement hover tooltip showing power/duration on PDC chart in src/ui/widgets/pdc_chart.rs
- [x] T036 [US1] Create analytics screen with PDC display in src/ui/screens/analytics_screen.rs
- [x] T037 [US1] Add analytics screen to app navigation in src/app.rs
- [x] T038 [US1] Trigger PDC update after ride save in src/metrics/analytics/triggers.rs

**Checkpoint**: User Story 1 complete - PDC is fully functional and testable independently

---

## Phase 4: User Story 2 - Real-Time Metrics Dashboard (Priority: P1)

**Goal**: Display NP, TSS, IF in real-time during rides and persist in ride summary

**Independent Test**: Start a ride with power meter, observe NP/TSS/IF updating live

### Tests for User Story 2

- [x] T039 [P] [US2] Unit test for TSS calculation formula in src/metrics/calculator.rs
- [x] T040 [P] [US2] Unit test for IF calculation at various power levels in src/metrics/calculator.rs

### Implementation for User Story 2

- [x] T041 [US2] Verify NP calculation exists (already implemented in smoothing.rs) in src/metrics/smoothing.rs
- [x] T042 [US2] Verify TSS/IF calculation exists in MetricsCalculator in src/metrics/calculator.rs
- [x] T043 [US2] Add NP/TSS/IF display widget to ride screen in src/ui/screens/ride_screen.rs
- [x] T044 [US2] Ensure ride summary displays final NP/TSS/IF values in src/ui/screens/ride_summary.rs
- [x] T045 [US2] Verify NP/TSS/IF persisted to rides table on save in src/recording/mod.rs

**Checkpoint**: User Stories 1 AND 2 complete - both work independently

---

## Phase 5: User Story 3 - Critical Power & W' Model (Priority: P2)

**Goal**: Calculate CP and W' from PDC data, provide time-to-exhaustion predictions

**Independent Test**: With 3+ varied max efforts, view CP/W' values and test TTE predictions

**Dependency**: Requires US1 (PDC) to be complete

### Tests for User Story 3

- [x] T046 [P] [US3] Unit test for CP model fitting with known reference values in src/metrics/analytics/critical_power.rs
- [x] T047 [P] [US3] Unit test for time_to_exhaustion calculation in src/metrics/analytics/critical_power.rs
- [x] T048 [P] [US3] Unit test for power_at_duration calculation in src/metrics/analytics/critical_power.rs
- [x] T049 [P] [US3] Unit test for CpFitError on insufficient data in src/metrics/analytics/critical_power.rs

### Implementation for User Story 3

- [x] T050 [US3] Implement CpModel struct (cp, w_prime, r_squared) in src/metrics/analytics/critical_power.rs
- [x] T051 [US3] Implement CpFitError enum with thiserror in src/metrics/analytics/critical_power.rs
- [x] T052 [US3] Implement CpFitter struct with min/max duration config in src/metrics/analytics/critical_power.rs
- [x] T053 [US3] Implement CpFitter::fit() using linear regression on work vs time in src/metrics/analytics/critical_power.rs
- [x] T054 [US3] Implement linear regression helper function in src/metrics/analytics/critical_power.rs
- [x] T055 [US3] Implement CpModel::time_to_exhaustion() in src/metrics/analytics/critical_power.rs
- [x] T056 [US3] Implement CpModel::power_at_duration() in src/metrics/analytics/critical_power.rs
- [x] T057 [US3] Implement CpModel::w_prime_remaining() in src/metrics/analytics/critical_power.rs
- [x] T058 [US3] Add CP/W' display section to analytics screen in src/ui/screens/analytics_screen.rs
- [x] T059 [US3] Add TTE prediction input/display to analytics screen in src/ui/screens/analytics_screen.rs
- [x] T060 [US3] Trigger CP recalculation when PDC updates in 2-20min range in src/metrics/analytics/triggers.rs

**Checkpoint**: User Story 3 complete - CP/W' model works with PDC data

---

## Phase 6: User Story 4 - Automatic FTP Detection (Priority: P2)

**Goal**: Auto-detect FTP from ride history, notify user of changes, update zones on acceptance

**Independent Test**: After 2+ weeks of riding, view auto-detected FTP and accept to update zones

**Dependency**: Requires US1 (PDC) to be complete

### Tests for User Story 4

- [x] T061 [P] [US4] Unit test for FTP detection from 20-min power (95% rule) in src/metrics/analytics/ftp_detection.rs
- [x] T062 [P] [US4] Unit test for FTP detection from extended duration in src/metrics/analytics/ftp_detection.rs
- [x] T063 [P] [US4] Unit test for confidence level calculation in src/metrics/analytics/ftp_detection.rs
- [x] T064 [P] [US4] Unit test for significant change detection (>5%) in src/metrics/analytics/ftp_detection.rs

### Implementation for User Story 4

- [x] T065 [US4] Implement FtpConfidence enum (High, Medium, Low) in src/metrics/analytics/ftp_detection.rs
- [x] T066 [US4] Implement FtpMethod enum (TwentyMinute, ExtendedDuration, CriticalPower) in src/metrics/analytics/ftp_detection.rs
- [x] T067 [US4] Implement FtpEstimate struct in src/metrics/analytics/ftp_detection.rs
- [x] T068 [US4] Implement FtpDetector struct with configuration in src/metrics/analytics/ftp_detection.rs
- [x] T069 [US4] Implement FtpDetector::detect() from PDC in src/metrics/analytics/ftp_detection.rs
- [x] T070 [US4] Implement FtpDetector::detect_from_cp() in src/metrics/analytics/ftp_detection.rs
- [x] T071 [US4] Implement FtpDetector::is_significant_change() in src/metrics/analytics/ftp_detection.rs
- [x] T072 [US4] Implement FtpEstimate::should_notify() in src/metrics/analytics/ftp_detection.rs
- [x] T073 [US4] Add FTP notification UI component in src/ui/widgets/ftp_notification.rs
- [x] T074 [US4] Add FTP accept/dismiss dialog in src/ui/widgets/ftp_notification.rs
- [x] T075 [US4] Trigger zone recalculation on FTP acceptance in src/metrics/zones.rs
- [x] T076 [US4] Display current FTP with confidence on profile screen in src/ui/screens/settings.rs

**Checkpoint**: User Story 4 complete - FTP auto-detection works independently

---

## Phase 7: User Story 5 - Training Load Tracking (ACWR) (Priority: P3)

**Goal**: Display ATL/CTL/ACWR with color-coded status and injury risk warnings

**Independent Test**: After 28+ days of rides, view ACWR value and status color

**Dependency**: Requires US2 (TSS calculation) to be complete

### Tests for User Story 5

- [x] T077 [P] [US5] Unit test for EWMA ATL calculation (7-day) in src/metrics/analytics/training_load.rs
- [x] T078 [P] [US5] Unit test for EWMA CTL calculation (42-day) in src/metrics/analytics/training_load.rs
- [x] T079 [P] [US5] Unit test for ACWR calculation and status thresholds in src/metrics/analytics/training_load.rs
- [x] T080 [P] [US5] Unit test for cold start handling (no historical data) in src/metrics/analytics/training_load.rs

### Implementation for User Story 5

- [x] T081 [US5] Implement DailyLoad struct (tss, atl, ctl, tsb) in src/metrics/analytics/training_load.rs
- [x] T082 [US5] Implement AcwrStatus enum (Undertrained, Optimal, Caution, HighRisk) in src/metrics/analytics/training_load.rs
- [x] T083 [US5] Implement Acwr struct with ratio and status in src/metrics/analytics/training_load.rs
- [x] T084 [US5] Implement TrainingLoadCalculator struct in src/metrics/analytics/training_load.rs
- [x] T085 [US5] Implement TrainingLoadCalculator::calculate_day() with EWMA in src/metrics/analytics/training_load.rs
- [x] T086 [US5] Implement TrainingLoadCalculator::calculate_history() in src/metrics/analytics/training_load.rs
- [x] T087 [US5] Implement TrainingLoadCalculator::acwr() in src/metrics/analytics/training_load.rs
- [x] T088 [US5] Implement Acwr::color() for UI display in src/metrics/analytics/training_load.rs
- [x] T089 [US5] Implement Acwr::recommendation() for guidance text in src/metrics/analytics/training_load.rs
- [x] T090 [US5] Create training load widget with ACWR gauge in src/ui/widgets/training_load_widget.rs
- [x] T091 [US5] Add ATL/CTL/TSB history chart to analytics screen in src/ui/screens/analytics_screen.rs
- [x] T092 [US5] Display "Building baseline" message when <28 days data in src/ui/widgets/training_load_widget.rs
- [x] T093 [US5] Trigger daily training load update after ride save in src/metrics/analytics/triggers.rs

**Checkpoint**: User Story 5 complete - ACWR tracking works independently

---

## Phase 8: User Story 6 - VO2max Estimation (Priority: P3)

**Goal**: Estimate and display VO2max from 5-minute power, show population percentile

**Independent Test**: After recording a 5-min max effort, view VO2max value and percentile

**Dependency**: Requires US1 (PDC for 5-min power) to be complete

### Tests for User Story 6

- [x] T094 [P] [US6] Unit test for VO2max calculation (Hawley-Noakes formula) in src/metrics/analytics/vo2max.rs
- [x] T095 [P] [US6] Unit test for percentile lookup by age/gender in src/metrics/analytics/vo2max.rs

### Implementation for User Story 6

- [x] T096 [US6] Implement Vo2maxResult struct in src/metrics/analytics/vo2max.rs
- [x] T097 [US6] Implement Vo2maxCalculator struct with reference tables in src/metrics/analytics/vo2max.rs
- [x] T098 [US6] Implement Vo2maxCalculator::calculate() using Hawley-Noakes in src/metrics/analytics/vo2max.rs
- [x] T099 [US6] Implement Vo2maxCalculator::calculate_with_percentile() in src/metrics/analytics/vo2max.rs
- [x] T100 [US6] Add population percentile reference tables (ACSM) in src/metrics/analytics/vo2max.rs
- [x] T101 [US6] Add VO2max display section to analytics screen in src/ui/screens/analytics_screen.rs
- [x] T102 [US6] Add VO2max trend chart showing history in src/ui/screens/analytics_screen.rs
- [x] T103 [US6] Trigger VO2max recalculation when 5-min PDC updates in src/metrics/analytics/triggers.rs

**Checkpoint**: User Story 6 complete - VO2max estimation works independently

---

## Phase 9: User Story 7 - Sweet Spot Training Recommendations (Priority: P4)

**Goal**: Generate Sweet Spot workout recommendations based on FTP and training load

**Independent Test**: With valid FTP, request recommendation and verify 88-93% target range

**Dependency**: Requires US4 (FTP) and US5 (training load) to be complete

### Tests for User Story 7

- [x] T104 [P] [US7] Unit test for sweet spot power range calculation in src/metrics/analytics/sweet_spot.rs
- [x] T105 [P] [US7] Unit test for recommendation adjustment based on ACWR in src/metrics/analytics/sweet_spot.rs

### Implementation for User Story 7

- [x] T106 [US7] Implement WorkoutRecommendation struct in src/metrics/analytics/sweet_spot.rs
- [x] T107 [US7] Implement SweetSpotRecommender struct in src/metrics/analytics/sweet_spot.rs
- [x] T108 [US7] Implement SweetSpotRecommender::zone() for power range in src/metrics/analytics/sweet_spot.rs
- [x] T109 [US7] Implement SweetSpotRecommender::recommend() with CTL/ACWR adjustment in src/metrics/analytics/sweet_spot.rs
- [x] T110 [US7] Implement SweetSpotRecommender::is_in_zone() for live feedback in src/metrics/analytics/sweet_spot.rs
- [x] T111 [US7] Add Sweet Spot recommendations section to analytics screen in src/ui/screens/analytics_screen.rs
- [x] T112 [US7] Add live Sweet Spot zone indicator to ride screen in src/ui/screens/ride_screen.rs

**Checkpoint**: User Story 7 complete - Sweet Spot recommendations work

---

## Phase 10: User Story 8 - Rider Type Classification (Priority: P4)

**Goal**: Classify rider type based on PDC power profile, explain strengths/weaknesses

**Independent Test**: With 3+ months of PDC data, view rider type classification and explanation

**Dependency**: Requires US1 (PDC) with sufficient history

### Tests for User Story 8

- [x] T113 [P] [US8] Unit test for power profile normalization in src/metrics/analytics/rider_type.rs
- [x] T114 [P] [US8] Unit test for rider classification logic in src/metrics/analytics/rider_type.rs
- [x] T115 [P] [US8] Unit test for classification edge cases (all-rounder) in src/metrics/analytics/rider_type.rs

### Implementation for User Story 8

- [x] T116 [US8] Implement RiderType enum in src/metrics/analytics/rider_type.rs
- [x] T117 [US8] Implement PowerProfile struct (neuromuscular, anaerobic, vo2max, threshold) in src/metrics/analytics/rider_type.rs
- [x] T118 [US8] Implement RiderClassifier struct with reference tables in src/metrics/analytics/rider_type.rs
- [x] T119 [US8] Add Coggan reference power values by category in src/metrics/analytics/rider_type.rs
- [x] T120 [US8] Implement RiderClassifier::profile() for power profile calculation in src/metrics/analytics/rider_type.rs
- [x] T121 [US8] Implement RiderClassifier::classify() from profile in src/metrics/analytics/rider_type.rs
- [x] T122 [US8] Implement RiderClassifier::explain() for explanation text in src/metrics/analytics/rider_type.rs
- [x] T123 [US8] Implement RiderType::training_focus() in src/metrics/analytics/rider_type.rs
- [x] T124 [US8] Add rider type display to profile screen in src/ui/screens/settings.rs
- [x] T125 [US8] Add power profile radar chart to analytics screen in src/ui/screens/analytics_screen.rs

**Checkpoint**: User Story 8 complete - Rider classification works

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T126 [P] Add insufficient data messaging throughout analytics screens in src/ui/screens/analytics_screen.rs
- [x] T127 [P] Add edge case handling for power spikes (>2000W filter) in src/metrics/analytics/pdc.rs
- [x] T128 [P] Add interpolation handling for short sensor gaps (<10s) in src/metrics/analytics/pdc.rs
- [x] T129 Run cargo fmt and cargo clippy across all new files
- [x] T130 Update lib.rs exports for analytics module in src/lib.rs
- [x] T131 Create integration test for full analytics pipeline in tests/integration/analytics_integration.rs
- [x] T132 Validate quickstart.md scenarios work end-to-end
- [x] T133 Performance optimization for PDC calculation with large ride history

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **US1 PDC (Phase 3)**: Depends on Foundational
- **US2 Real-Time (Phase 4)**: Depends on Foundational (can parallel with US1)
- **US3 CP/W' (Phase 5)**: Depends on US1 (PDC)
- **US4 FTP (Phase 6)**: Depends on US1 (PDC)
- **US5 ACWR (Phase 7)**: Depends on US2 (TSS)
- **US6 VO2max (Phase 8)**: Depends on US1 (PDC)
- **US7 Sweet Spot (Phase 9)**: Depends on US4 + US5
- **US8 Rider Type (Phase 10)**: Depends on US1 (PDC)
- **Polish (Phase 11)**: Depends on all desired user stories

### User Story Dependencies Graph

```
              ┌──────────────┐
              │ Foundational │
              │   (Phase 2)  │
              └──────┬───────┘
                     │
         ┌──────────┼──────────┐
         ▼          ▼          ▼
      ┌─────┐    ┌─────┐    ┌─────┐
      │ US1 │    │ US2 │    │     │
      │ PDC │    │ NP  │    │     │
      │ P1  │    │ P1  │    │     │
      └──┬──┘    └──┬──┘    │     │
         │          │       │     │
    ┌────┼────┬─────┘       │     │
    ▼    ▼    ▼             ▼     │
  ┌───┐┌───┐┌───┐       ┌─────┐   │
  │US3││US4││US6│       │ US5 │   │
  │CP ││FTP││VO2│       │ACWR │   │
  │P2 ││P2 ││P3 │       │ P3  │   │
  └───┘└─┬─┘└───┘       └──┬──┘   │
         │                 │      │
         └────────┬────────┘      │
                  ▼               │
               ┌─────┐            │
               │ US7 │            │
               │Sweet│            │
               │ P4  │            │
               └─────┘            │
                                  │
         ┌────────────────────────┘
         ▼
      ┌─────┐
      │ US8 │
      │Rider│
      │ P4  │
      └─────┘
```

### Parallel Opportunities

**Phase 2 (Foundational)**:
```
T010, T011, T012, T013, T014, T015 - all storage methods in parallel
```

**Phase 3 (US1)**:
```
T019, T020, T021, T022 - all tests in parallel
```

**After Foundational, these story starts can parallel**:
- US1 (PDC) and US2 (Real-Time) can run in parallel
- US3, US4, US6, US8 can start once US1 complete
- US5 can start once US2 complete
- US7 waits for US4 + US5

---

## Parallel Example: Phase 2 Foundational

```bash
# Launch all storage method implementations together:
Task: "Implement PDC storage methods in src/storage/analytics_store.rs"
Task: "Implement CP model storage methods in src/storage/analytics_store.rs"
Task: "Implement FTP estimate storage methods in src/storage/analytics_store.rs"
Task: "Implement training load storage methods in src/storage/analytics_store.rs"
Task: "Implement VO2max storage methods in src/storage/analytics_store.rs"
Task: "Implement rider profile storage methods in src/storage/analytics_store.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (PDC)
4. Complete Phase 4: User Story 2 (Real-Time NP/TSS/IF)
5. **STOP and VALIDATE**: Test PDC and real-time metrics independently
6. Deploy/demo MVP

### Incremental Delivery

1. MVP: Setup + Foundational + US1 + US2 → Core analytics visible
2. Add US3 (CP/W') → Pacing predictions available
3. Add US4 (FTP Auto) → No more manual FTP tests
4. Add US5 (ACWR) → Training load monitoring
5. Add US6 (VO2max) → Fitness benchmarking
6. Add US7 + US8 → Training recommendations and classification

### Parallel Team Strategy

With 2 developers after Foundational:
- Developer A: US1 (PDC) → US3 (CP) → US4 (FTP) → US7 (Sweet Spot)
- Developer B: US2 (Real-Time) → US5 (ACWR) → US6 (VO2max) → US8 (Rider Type)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story
- Each user story is independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- US1+US2 form the MVP - minimum viable analytics
