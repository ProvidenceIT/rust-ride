# Tasks: Competitive Feature Gaps

**Input**: Design documents from `/specs/010-competitive-features/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/, quickstart.md

**Tests**: Not explicitly requested in spec. Core unit tests included for critical business logic.

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root (Rust project)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, module structure, and database migrations

- [X] T001 Create gradient module structure in src/gradient/mod.rs
- [X] T002 [P] Create achievements module structure in src/achievements/mod.rs
- [X] T003 [P] Create power_profile module structure in src/power_profile/mod.rs
- [X] T004 [P] Create training_plans module structure in src/training_plans/mod.rs
- [X] T005 [P] Create career module structure in src/career/mod.rs
- [X] T006 Add new module exports to src/lib.rs
- [X] T007 Add database migration for competitive features tables in src/storage/schema.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T008 Create GradientSettings struct with defaults in src/gradient/settings.rs
- [X] T009 [P] Create AchievementCategory and AchievementTier enums in src/achievements/types.rs
- [X] T010 [P] Create Discipline and DifficultyLevel enums in src/training_plans/disciplines.rs
- [X] T011 [P] Create RewardType enum and Reward struct in src/career/rewards.rs
- [X] T012 [P] Create ProfileType enum and PowerPoint struct in src/power_profile/types.rs
- [X] T013 Add storage module for user_xp table operations in src/storage/xp_store.rs
- [X] T014 Add storage module for user_achievements table operations in src/storage/achievements_store.rs
- [X] T015 [P] Add storage module for user_rewards table operations in src/storage/rewards_store.rs
- [X] T016 [P] Add storage module for power_profiles table operations in src/storage/power_profile_store.rs
- [X] T017 [P] Add storage module for gradient_settings table operations in src/storage/gradient_store.rs
- [X] T018 [P] Add storage module for plan_assignments table operations in src/storage/plan_store.rs
- [X] T019 Update src/storage/mod.rs to export new storage modules

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Gradient-Responsive Resistance (Priority: P1) 🎯 MVP

**Goal**: Auto-adjust trainer resistance based on GPX route elevation data

**Independent Test**: Load GPX file with elevation data, verify trainer resistance changes correspond to gradient changes during the ride.

### Implementation for User Story 1

- [X] T020 [US1] Create RoutePoint and GradientSegment structs in src/gradient/types.rs
- [X] T021 [US1] Create GPXRoute struct with gradient calculation in src/gradient/route.rs
- [X] T022 [US1] Implement gradient calculator with Haversine distance in src/gradient/calculator.rs
- [X] T023 [US1] Implement gradient smoothing with moving average in src/gradient/smoothing.rs
- [X] T024 [US1] Create GradientController trait implementation in src/gradient/controller.rs
- [X] T025 [US1] Integrate with existing FTMS build_set_simulation_with_crr in src/gradient/resistance.rs
- [X] T026 [US1] Create GradientError enum and error handling in src/gradient/error.rs
- [X] T027 [US1] Create gradient settings UI panel in src/ui/widgets/gradient_settings.rs
- [X] T028 [US1] Create gradient display widget showing current grade in src/ui/widgets/gradient_display.rs
- [X] T029 [US1] Integrate gradient controller into ride session in src/app.rs (GradientController field)
- [X] T030 [US1] Add gradient route loading to ride setup flow in src/ui/screens/ride_setup.rs
- [X] T031 [US1] Add unit tests for gradient calculator in tests/unit/gradient_tests.rs

**Checkpoint**: User Story 1 complete - GPX-based gradient-responsive resistance functional

---

## Phase 4: User Story 2 - Achievement Badges & XP System (Priority: P2)

**Goal**: Gamification with badges, XP, and level progression

**Independent Test**: Complete rides, verify badges are awarded and XP accumulates correctly.

### Implementation for User Story 2

- [X] T032 [P] [US2] Create Achievement struct with XP value in src/achievements/achievement.rs
- [X] T033 [P] [US2] Create EarnedAchievement and RideMetrics structs in src/achievements/earned.rs
- [X] T034 [US2] Create AchievementNotification queue system in src/achievements/notifications.rs
- [X] T035 [US2] Define initial 50+ achievement definitions with XP values in src/achievements/definitions.rs
- [X] T036 [US2] Create AchievementTracker trait implementation in src/achievements/tracker.rs
- [X] T037 [US2] Implement ride achievement checks (distance, power, duration) in src/achievements/checks/ride.rs
- [X] T038 [US2] Implement cumulative achievement checks (lifetime totals) in src/achievements/checks/cumulative.rs
- [X] T039 [US2] Implement consistency achievement checks (streaks, daily) in src/achievements/checks/consistency.rs
- [X] T040 [US2] Create XP calculation and level progression logic in src/achievements/xp.rs
- [X] T041 [US2] Create achievement notification widget in src/ui/widgets/achievement_notification.rs
- [X] T042 [US2] Create achievements gallery screen in src/ui/screens/achievements.rs
- [X] T043 [US2] Integrate achievement tracking into ride completion flow in src/app.rs
- [X] T044 [US2] Add XP/level display to user profile screen in src/ui/screens/profile.rs
- [X] T045 [US2] Add unit tests for XP curve and level calculations in tests/unit/achievements_tests.rs

**Checkpoint**: User Story 2 complete - Achievement system with XP and levels functional

---

## Phase 5: User Story 3 - 4D Power Profiling (Priority: P3)

**Goal**: Multi-duration power analysis with 90-day rolling window and lifetime bests

**Independent Test**: Process ride history, verify 90-day vs lifetime separation and strength/weakness identification.

### Implementation for User Story 3

- [X] T046 [P] [US3] Create PowerProfile and PowerProfilePoint structs in src/power_profile/profile.rs
- [X] T047 [P] [US3] Create ProfileAnalysis and DurationStrength structs in src/power_profile/analysis.rs
- [X] T048 [US3] Create RiderType enum and classification logic in src/power_profile/rider_type.rs
- [X] T049 [US3] Implement 90-day rolling window profile calculation in src/power_profile/rolling.rs
- [X] T050 [US3] Implement lifetime best tracking in src/power_profile/lifetime.rs
- [X] T051 [US3] Create PowerProfileManager trait implementation in src/power_profile/manager.rs
- [X] T052 [US3] Integrate with existing MmpCalculator from metrics/analytics/pdc.rs in src/power_profile/mmp_adapter.rs
- [X] T053 [US3] Implement reference curve comparison for analysis in src/power_profile/comparison.rs
- [X] T054 [US3] Create power profile visualization screen in src/ui/screens/power_profile.rs
- [X] T055 [US3] Create power curve chart widget in src/ui/widgets/power_curve_chart.rs
- [X] T056 [US3] Integrate power profile updates into ride save flow in src/power_profile/ride_integration.rs
- [X] T057 [US3] Add power profile achievements (new PRs) in src/achievements/checks/power.rs
- [X] T058 [US3] Add unit tests for profile calculations in tests/unit/power_profile_tests.rs

**Checkpoint**: User Story 3 complete - 4D power profiling with analysis functional

---

## Phase 6: User Story 4 - Multi-Discipline Training Plans (Priority: P4)

**Goal**: Pre-built training plans for road, gravel, triathlon, MTB, and general fitness

**Independent Test**: Select a discipline, receive a plan, verify workouts align with discipline demands.

### Implementation for User Story 4

- [X] T059 [P] [US4] Create TrainingPlan and PlanWeek structs in src/training_plans/plan.rs
- [X] T060 [P] [US4] Create ScheduledWorkout and UpcomingWorkout structs in src/training_plans/workout.rs
- [X] T061 [P] [US4] Create PlanAssignment and PlanStatus structs in src/training_plans/assignment.rs
- [X] T062 [US4] Create TrainingPlanManager trait implementation in src/training_plans/manager.rs
- [X] T063 [US4] Define initial training plan library (8 plans: 2 per discipline) in src/training_plans/library.rs
- [X] T064 [US4] Implement plan assignment and scheduling logic in src/training_plans/scheduler.rs
- [X] T065 [US4] Implement workout completion/skip tracking in src/training_plans/progress.rs
- [X] T066 [US4] Create training plans browse/filter screen in src/ui/screens/training_plans.rs
- [X] T067 [US4] Create plan detail view with weekly overview in src/ui/widgets/plan_detail.rs
- [X] T068 [US4] Create upcoming workouts widget in src/ui/widgets/upcoming_workouts.rs
- [X] T069 [US4] Integrate plan workouts with existing workout library in src/training_plans/workout_loader.rs
- [X] T070 [US4] Add plan-related achievements (plan completion, streak) in src/achievements/checks/training.rs

**Checkpoint**: User Story 4 complete - Multi-discipline training plans functional

---

## Phase 7: User Story 5 - Career Levels with Long-Term Progression (Priority: P5)

**Goal**: 50-level progression with cosmetic unlocks at milestone levels

**Independent Test**: Add XP, verify level ups trigger at correct thresholds with appropriate rewards.

**Dependency**: Requires User Story 2 (XP system) to be complete

### Implementation for User Story 5

- [X] T071 [P] [US5] Create CareerStatus struct in src/career/status.rs
- [X] T072 [P] [US5] Create LevelUpEvent and UnlockedReward structs in src/career/events.rs
- [X] T073 [US5] Implement XP curve calculations (xp_for_level, level_from_xp) in src/career/xp_curve.rs
- [X] T074 [US5] Define all 50 levels with reward unlocks in src/career/levels.rs
- [X] T075 [US5] Create CareerManager trait implementation in src/career/manager.rs
- [X] T076 [US5] Define cosmetic rewards (jerseys, frames, themes) in src/career/cosmetics.rs
- [X] T077 [US5] Create level up notification widget in src/ui/widgets/level_up_notification.rs
- [X] T078 [US5] Create career progress screen with level display in src/ui/screens/career.rs
- [X] T079 [US5] Create rewards gallery screen in src/ui/screens/rewards.rs
- [X] T080 [US5] Integrate career manager with achievement XP awards in src/app.rs
- [X] T081 [US5] Add career-related achievements (level milestones) in src/achievements/checks/career.rs
- [X] T082 [US5] Add unit tests for career levels in tests/unit/career_tests.rs

**Checkpoint**: User Story 5 complete - Career progression with unlocks functional

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T083 Add integration test for gradient ride simulation in tests/integration/gradient_ride_test.rs
- [X] T084 [P] Add integration test for achievement flow in tests/integration/achievement_flow_test.rs
- [X] T085 [P] Add integration test for power profile updates in tests/integration/power_profile_test.rs
- [X] T086 Code cleanup and ensure consistent error handling across new modules
- [X] T087 Performance optimization for power profile calculations (<5 seconds) - verified <1ms
- [X] T088 Performance optimization for achievement checks (<2 seconds notification) - verified <1ms
- [X] T089 Run quickstart.md validation scenarios (all modules tested: gradient, achievements, training_plans, career)
- [X] T090 Verify all clippy warnings addressed in new code (reduced from 32 to 8, remaining are MSRV/external)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - US1 (Gradient): Independent, can start first
  - US2 (Achievements): Independent, can start in parallel with US1
  - US3 (Power Profile): Independent, can start in parallel
  - US4 (Training Plans): Independent, can start in parallel
  - US5 (Career): **Depends on US2** (requires XP system)
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

```
        ┌── US1 (Gradient) ──┐
        │                    │
Setup → Foundational ─┼── US2 (Achievements) ─┬── US5 (Career)
        │                    │                 │
        ├── US3 (Power) ─────┤                 │
        │                    │                 │
        └── US4 (Plans) ─────┴─────────────────┴── Polish
```

- **User Story 1 (P1)**: Can start after Foundational - No story dependencies
- **User Story 2 (P2)**: Can start after Foundational - No story dependencies
- **User Story 3 (P3)**: Can start after Foundational - No story dependencies
- **User Story 4 (P4)**: Can start after Foundational - No story dependencies
- **User Story 5 (P5)**: Must wait for **US2 completion** (uses XP system)

### Within Each User Story

- Types/structs before implementations
- Core logic before UI integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks T002-T005 marked [P] can run in parallel
- All Foundational tasks T009-T018 marked [P] can run in parallel (within Phase 2)
- US1-US4 can start in parallel after Foundational phase completes
- Within each story, tasks marked [P] can run in parallel

---

## Parallel Example: User Story 2

```bash
# Launch type definitions in parallel:
Task: "Create Achievement struct with XP value in src/achievements/achievement.rs"
Task: "Create EarnedAchievement and RideMetrics structs in src/achievements/earned.rs"

# After types, launch check modules in parallel:
Task: "Implement ride achievement checks in src/achievements/checks/ride.rs"
Task: "Implement cumulative achievement checks in src/achievements/checks/cumulative.rs"
Task: "Implement consistency achievement checks in src/achievements/checks/consistency.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Gradient-Responsive Resistance)
4. **STOP and VALIDATE**: Test gradient simulation independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (Achievement system)
4. Add User Story 3 → Test independently → Deploy/Demo (Power profiling)
5. Add User Story 4 → Test independently → Deploy/Demo (Training plans)
6. Add User Story 5 → Test independently → Deploy/Demo (Career levels)
7. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Gradient)
   - Developer B: User Story 2 (Achievements)
   - Developer C: User Story 3 (Power Profile) OR User Story 4 (Plans)
3. After US2 completes: Developer B → User Story 5 (Career)
4. Stories complete and integrate independently

---

## Summary

| Phase | Tasks | Parallel Tasks | Estimated Effort |
|-------|-------|----------------|------------------|
| Setup | 7 | 5 | Small |
| Foundational | 12 | 9 | Medium |
| US1 (Gradient) | 12 | 0 | Medium |
| US2 (Achievements) | 14 | 2 | Large |
| US3 (Power Profile) | 13 | 2 | Medium |
| US4 (Training Plans) | 12 | 3 | Medium |
| US5 (Career) | 12 | 2 | Medium |
| Polish | 8 | 2 | Small |
| **Total** | **90** | **25** | |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Existing infrastructure reused: GPX parser, FTMS control, PDC module, achievement definitions
