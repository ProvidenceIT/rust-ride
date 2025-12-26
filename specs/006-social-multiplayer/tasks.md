# Tasks: Social & Multiplayer

**Input**: Design documents from `/specs/006-social-multiplayer/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Not explicitly requested - implementation tasks only.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependencies, and module structure

- [X] T001 Add mdns-sd and bincode dependencies to Cargo.toml
- [X] T002 [P] Create src/social/mod.rs with module exports
- [X] T003 [P] Create src/networking/mod.rs with module exports
- [X] T004 [P] Create src/leaderboards/mod.rs with module exports
- [X] T005 [P] Create src/racing/mod.rs with module exports
- [X] T006 Update src/lib.rs to export new social, networking, leaderboards, racing modules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Database Schema

- [X] T007 Add social schema migration to src/storage/schema.rs (riders, clubs, badges, segments, challenges tables)
- [X] T008 Create src/storage/social_store.rs with SocialStore struct and database connection

### Core Types

- [X] T009 [P] Create src/social/types.rs with Rider, RiderProfile, BadgeCategory, GoalType enums
- [X] T010 [P] Create src/networking/protocol.rs with ProtocolMessage enum and RiderMetrics struct
- [X] T011 [P] Create src/leaderboards/segments.rs with Segment, SegmentCategory types
- [X] T012 [P] Create src/racing/events.rs with RaceEvent, RaceStatus, ParticipantStatus types

### Networking Foundation

- [X] T013 Create src/networking/discovery.rs with DiscoveryService struct using mdns-sd
- [X] T014 Implement DiscoveryService::start() for mDNS registration in src/networking/discovery.rs
- [X] T015 Implement DiscoveryService::peers() and subscribe() in src/networking/discovery.rs
- [X] T016 Create NetworkConfig struct in src/networking/mod.rs with default values

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Join LAN Group Ride (Priority: P1) 🎯 MVP

**Goal**: Enable real-time group rides on LAN with peer discovery, session management, and live metric sync

**Independent Test**: Two devices on same network - start group ride on one, join from another, verify both see each other's metrics and positions

### Core Networking

- [X] T017 [US1] Create src/networking/session.rs with SessionManager, Session, Participant structs
- [X] T018 [US1] Implement SessionManager::host_session() in src/networking/session.rs
- [X] T019 [US1] Implement SessionManager::join_session() in src/networking/session.rs
- [X] T020 [US1] Implement SessionManager::leave_session() in src/networking/session.rs
- [X] T021 [US1] Create src/networking/sync.rs with MetricSync struct for UDP communication
- [X] T022 [US1] Implement MetricSync::start() with tokio UDP socket in src/networking/sync.rs
- [X] T023 [US1] Implement MetricSync::broadcast_metrics() with bincode serialization in src/networking/sync.rs
- [X] T024 [US1] Implement MetricSync::subscribe() for receiving peer metrics in src/networking/sync.rs
- [X] T025 [US1] Add heartbeat/disconnect detection to MetricSync in src/networking/sync.rs

### Storage

- [X] T026 [US1] Add group_rides and group_ride_participants table operations to src/storage/social_store.rs
- [X] T027 [US1] Implement auto-save every 30 seconds for group ride data in src/networking/session.rs

### UI

- [X] T028 [US1] Create src/ui/screens/group_ride.rs with GroupRideScreen struct
- [X] T029 [US1] Implement peer discovery list UI in src/ui/screens/group_ride.rs
- [X] T030 [US1] Implement host/join session buttons in src/ui/screens/group_ride.rs
- [X] T031 [US1] Create src/ui/widgets/participant_list.rs for showing connected riders
- [X] T032 [US1] Display live metrics for all participants in src/ui/widgets/participant_list.rs
- [X] T033 [US1] Add group ride screen to navigation in src/ui/screens/mod.rs

### Integration

- [X] T034 [US1] Integrate networking with app state in src/app.rs
- [X] T035 [US1] Handle reconnection after temporary network loss in src/networking/session.rs

**Checkpoint**: User Story 1 complete - LAN group rides with real-time metrics should work

---

## Phase 4: User Story 2 - View Local Leaderboards (Priority: P2)

**Goal**: Track segment efforts and display ranked leaderboards with export capability

**Independent Test**: Complete multiple rides over same segment, verify leaderboard ranks correctly and export works

### Segment Management

- [X] T036 [US2] Create src/leaderboards/efforts.rs with SegmentEffort, EffortTracker structs
- [X] T037 [US2] Implement segment entry/exit detection in src/leaderboards/segments.rs
- [X] T038 [US2] Implement EffortTracker::update() for active segment tracking in src/leaderboards/efforts.rs
- [X] T039 [US2] Implement EffortTracker::record_effort() in src/leaderboards/efforts.rs

### Rankings

- [X] T040 [US2] Create src/leaderboards/rankings.rs with LeaderboardService struct
- [X] T041 [US2] Implement LeaderboardService::get_leaderboard() in src/leaderboards/rankings.rs
- [X] T042 [US2] Implement LeaderboardService::personal_bests() in src/leaderboards/rankings.rs
- [X] T043 [US2] Add segment_efforts table operations to src/storage/social_store.rs

### Export/Import

- [X] T044 [P] [US2] Create src/leaderboards/export.rs with LeaderboardExporter struct
- [X] T045 [US2] Implement export_json() in src/leaderboards/export.rs
- [X] T046 [US2] Implement export_csv() in src/leaderboards/export.rs
- [X] T047 [US2] Implement import_json() with name matching and merge prompt in src/leaderboards/export.rs

### UI

- [X] T048 [US2] Create src/ui/screens/leaderboard.rs with LeaderboardScreen struct
- [X] T049 [US2] Implement segment list view in src/ui/screens/leaderboard.rs
- [X] T050 [US2] Implement leaderboard table view in src/ui/screens/leaderboard.rs
- [X] T051 [US2] Add export/import buttons to src/ui/screens/leaderboard.rs
- [X] T052 [US2] Integrate leaderboard tracking with ride recording in src/recording/recorder.rs

**Checkpoint**: User Story 2 complete - Segment leaderboards with export/import working

---

## Phase 5: User Story 3 - Browse and Use Community Workouts (Priority: P3)

**Goal**: Provide 50-100 pre-curated workouts with filtering and library management

**Independent Test**: Browse repository, filter by criteria, add workout to library, execute successfully

### Repository

- [X] T053 [US3] Create src/workouts/repository.rs with WorkoutRepository struct
- [X] T054 [US3] Implement bundled workout loading from assets/workouts/ in src/workouts/repository.rs
- [X] T055 [US3] Create assets/workouts/ directory structure with 50+ TOML workout files
- [X] T056 [US3] Implement WorkoutRepository::search() with filtering in src/workouts/repository.rs
- [X] T057 [US3] Implement WorkoutRepository::add_to_library() in src/workouts/repository.rs
- [X] T058 [US3] Implement optional GitHub sync in src/workouts/repository.rs

### UI

- [X] T059 [US3] Create workout repository browser UI in src/ui/screens/workout_library.rs
- [X] T060 [US3] Add difficulty filter dropdown in src/ui/screens/workout_library.rs
- [X] T061 [US3] Add training focus filter in src/ui/screens/workout_library.rs
- [X] T062 [US3] Add duration range filter in src/ui/screens/workout_library.rs
- [X] T063 [US3] Implement add-to-library button in src/ui/screens/workout_library.rs

**Checkpoint**: User Story 3 complete - Workout repository browsing and filtering working

---

## Phase 6: User Story 4 - Create and Share Training Challenges (Priority: P4)

**Goal**: Create, export, import challenges with automatic progress tracking

**Independent Test**: Create challenge, export to file, import on another device, verify progress tracking

### Challenge Logic

- [X] T064 [US4] Create src/social/challenges.rs with ChallengeManager struct
- [X] T065 [US4] Implement ChallengeManager::create() in src/social/challenges.rs
- [X] T066 [US4] Implement ChallengeManager::update_progress() in src/social/challenges.rs
- [X] T067 [US4] Implement ChallengeManager::export() to TOML in src/social/challenges.rs
- [X] T068 [US4] Implement ChallengeManager::import() from TOML in src/social/challenges.rs
- [X] T069 [US4] Add challenges and challenge_progress table operations to src/storage/social_store.rs

### UI

- [X] T070 [US4] Create src/ui/screens/challenges.rs with ChallengeScreen struct
- [X] T071 [US4] Implement challenge creation form in src/ui/screens/challenges.rs
- [X] T072 [US4] Implement active challenges list with progress bars in src/ui/screens/challenges.rs
- [X] T073 [US4] Add export/import buttons in src/ui/screens/challenges.rs

### Integration

- [X] T074 [US4] Hook challenge progress update to ride completion in src/recording/recorder.rs

**Checkpoint**: User Story 4 complete - Challenge creation, sharing, and tracking working

---

## Phase 7: User Story 5 - Participate in Virtual Race Events (Priority: P5)

**Goal**: Scheduled races with synchronized countdown and real-time position tracking

**Independent Test**: Schedule race, participants join, synchronized start, complete race, verify results

**Dependency**: Requires US1 (Group Rides) networking foundation

### Race Management

- [X] T075 [US5] Create src/racing/countdown.rs with CountdownSync struct
- [X] T076 [US5] Implement clock synchronization in src/racing/countdown.rs
- [X] T077 [US5] Create src/racing/results.rs with RaceResults struct
- [X] T078 [US5] Implement RaceManager in src/racing/events.rs
- [X] T079 [US5] Implement RaceManager::create_race() in src/racing/events.rs
- [X] T080 [US5] Implement RaceManager::join_race() in src/racing/events.rs
- [X] T081 [US5] Implement 60-second grace period for disconnections in src/racing/events.rs
- [X] T082 [US5] Add race_events and race_participants table operations to src/storage/social_store.rs

### UI

- [X] T083 [US5] Create src/ui/screens/race_lobby.rs with RaceLobbyScreen struct
- [X] T084 [US5] Implement race creation form in src/ui/screens/race_lobby.rs
- [X] T085 [US5] Implement race discovery list in src/ui/screens/race_lobby.rs
- [X] T086 [US5] Implement countdown overlay in src/ui/screens/race_lobby.rs
- [X] T087 [US5] Implement real-time position display in src/ui/screens/race_lobby.rs
- [X] T088 [US5] Implement results screen in src/ui/screens/race_lobby.rs

**Checkpoint**: User Story 5 complete - Virtual racing with synchronized starts working

---

## Phase 8: User Story 6 - View Activity Feed (Priority: P6)

**Goal**: Discover and display ride summaries from LAN peers

**Independent Test**: Complete rides on multiple devices, verify all can see each other's summaries

### Activity Feed Logic

- [X] T089 [US6] Create src/social/feed.rs with ActivityFeed struct
- [X] T090 [US6] Implement ActivityFeed::record() for post-ride summary in src/social/feed.rs
- [X] T091 [US6] Implement ActivityFeed::peer_activities() via mDNS discovery in src/social/feed.rs
- [X] T092 [US6] Add activity_summaries table operations to src/storage/social_store.rs

### UI

- [X] T093 [US6] Create src/ui/screens/activity_feed.rs with ActivityFeedScreen struct
- [X] T094 [US6] Implement activity card layout in src/ui/screens/activity_feed.rs
- [X] T095 [US6] Implement activity detail view in src/ui/screens/activity_feed.rs
- [X] T096 [US6] Add sharing toggle in rider profile settings

**Checkpoint**: User Story 6 complete - Activity feed from LAN peers working

---

## Phase 9: User Story 7 - Rate and Review Workouts (Priority: P7)

**Goal**: Rate workouts 1-5 stars with optional reviews, filter by rating

**Independent Test**: Complete workout, rate and review, verify rating appears in browse

**Dependency**: Requires US3 (Workout Repository)

### Ratings Logic

- [X] T097 [US7] Create src/workouts/ratings.rs with WorkoutRatings struct
- [X] T098 [US7] Implement WorkoutRatings::rate() in src/workouts/ratings.rs
- [X] T099 [US7] Implement WorkoutRatings::get_ratings() in src/workouts/ratings.rs
- [X] T100 [US7] Add workout_ratings table operations to src/storage/social_store.rs

### UI

- [X] T101 [US7] Add post-workout rating prompt in src/ui/screens/ride_summary.rs
- [X] T102 [US7] Add rating display to workout cards in src/ui/screens/workout_library.rs
- [X] T103 [US7] Add rating filter to workout browser in src/ui/screens/workout_library.rs
- [X] T104 [US7] Implement reviews display in workout detail view

**Checkpoint**: User Story 7 complete - Workout ratings and reviews working

---

## Phase 10: User Story 8 - Manage Clubs (Priority: P8)

**Goal**: Create/join clubs with member roster and aggregate statistics

**Independent Test**: Create club, invite members via code, verify roster and stats

### Club Logic

- [X] T105 [US8] Create src/social/clubs.rs with ClubManager struct
- [X] T106 [US8] Implement ClubManager::create() with join code generation in src/social/clubs.rs
- [X] T107 [US8] Implement ClubManager::join() in src/social/clubs.rs
- [X] T108 [US8] Implement ClubManager::update_stats() in src/social/clubs.rs
- [X] T109 [US8] Add clubs and club_memberships table operations to src/storage/social_store.rs

### UI

- [X] T110 [US8] Create src/ui/screens/clubs.rs with ClubsScreen struct
- [X] T111 [US8] Implement club creation form in src/ui/screens/clubs.rs
- [X] T112 [US8] Implement join-by-code form in src/ui/screens/clubs.rs
- [X] T113 [US8] Implement club detail view with roster in src/ui/screens/clubs.rs
- [X] T114 [US8] Display aggregate club statistics in src/ui/screens/clubs.rs

**Checkpoint**: User Story 8 complete - Club management working

---

## Phase 11: User Story 9 - Earn Achievement Badges (Priority: P9)

**Goal**: Detect milestones and award badges with profile display

**Independent Test**: Complete activities triggering badge criteria, verify badges unlock

### Badge Logic

- [X] T115 [US9] Create src/social/badges.rs with BadgeSystem struct
- [X] T116 [US9] Implement badge definitions (distance, FTP, consistency) in src/social/badges.rs
- [X] T117 [US9] Implement BadgeSystem::check_progress() in src/social/badges.rs
- [X] T118 [US9] Implement BadgeSystem::initialize_badges() for seeding in src/social/badges.rs
- [X] T119 [US9] Add badges and rider_badges table operations to src/storage/social_store.rs

### UI

- [X] T120 [US9] Create src/ui/widgets/badge_display.rs for showing badges
- [X] T121 [US9] Add badge unlock notification popup
- [X] T122 [US9] Display earned badges on rider profile

### Integration

- [X] T123 [US9] Hook badge checking to ride completion in src/recording/recorder.rs

**Checkpoint**: User Story 9 complete - Achievement badges working

---

## Phase 12: User Story 10 - Chat During Group Rides (Priority: P10)

**Goal**: Real-time text chat during group rides with history preservation

**Independent Test**: Join group ride, send messages, verify delivery and post-ride history

**Dependency**: Requires US1 (Group Rides)

### Chat Logic

- [X] T124 [US10] Create src/networking/chat.rs with ChatService struct
- [X] T125 [US10] Implement ChatService::send() with delivery acknowledgment in src/networking/chat.rs
- [X] T126 [US10] Implement ChatService::subscribe() in src/networking/chat.rs
- [X] T127 [US10] Add chat_messages table operations to src/storage/social_store.rs

### UI

- [X] T128 [US10] Create src/ui/widgets/chat_panel.rs with ChatPanel widget
- [X] T129 [US10] Implement message input and send in src/ui/widgets/chat_panel.rs
- [X] T130 [US10] Implement message history display in src/ui/widgets/chat_panel.rs
- [X] T131 [US10] Integrate chat panel into group ride screen in src/ui/screens/group_ride.rs

**Checkpoint**: User Story 10 complete - Group ride chat working

---

## Phase 13: User Story 11 - Compare Rides (Priority: P11)

**Goal**: Overlay comparison of multiple rides with aligned charts

**Independent Test**: Complete multiple rides on same route, overlay charts, verify alignment

### Comparison Logic

- [X] T132 [US11] Create ride comparison service in src/metrics/comparison.rs
- [X] T133 [US11] Implement data alignment by distance or time in src/metrics/comparison.rs
- [X] T134 [US11] Implement comparison data preparation in src/metrics/comparison.rs

### UI

- [X] T135 [US11] Create src/ui/screens/ride_compare.rs with RideCompareScreen struct
- [X] T136 [US11] Implement ride selection UI in src/ui/screens/ride_compare.rs
- [X] T137 [US11] Implement overlay chart using egui_plot in src/ui/screens/ride_compare.rs
- [X] T138 [US11] Add power, HR, cadence trace toggles in src/ui/screens/ride_compare.rs

**Checkpoint**: User Story 11 complete - Ride comparison charts working

---

## Phase 14: User Story 12 - Manage Rider Profile (Priority: P12)

**Goal**: Set profile info (name, avatar, bio) embedded in shared activities

**Independent Test**: Create profile, share activity, verify profile visible to others

### Profile Logic

- [X] T139 [US12] Create src/social/profile.rs with ProfileManager struct
- [X] T140 [US12] Implement ProfileManager::current() and update() in src/social/profile.rs
- [X] T141 [US12] Implement ProfileManager::record_ride() for stats aggregation in src/social/profile.rs
- [X] T142 [US12] Add riders table operations to src/storage/social_store.rs

### UI

- [X] T143 [US12] Create src/ui/screens/rider_profile.rs with RiderProfileScreen struct
- [X] T144 [US12] Implement profile edit form in src/ui/screens/rider_profile.rs
- [X] T145 [US12] Implement avatar selector in src/ui/screens/rider_profile.rs
- [X] T146 [US12] Display stats summary (distance, time, badges) in src/ui/screens/rider_profile.rs

### Integration

- [X] T147 [US12] Embed profile info in activity summaries in src/social/feed.rs

**Checkpoint**: User Story 12 complete - Rider profile management working

---

## Phase 15: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T148 [P] Add social screens to main navigation in src/ui/screens/home.rs
- [X] T149 [P] Add network status indicator widget in src/ui/widgets/mod.rs
- [X] T150 Code cleanup and refactoring across all new modules
- [X] T151 Performance optimization for 10-participant group rides
- [X] T152 Validate all screens work at 60 fps during group rides
- [X] T153 Run quickstart.md manual testing checklist

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-14)**: All depend on Foundational phase completion
  - User stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → ... → P12)
- **Polish (Phase 15)**: Depends on all desired user stories being complete

### User Story Dependencies

| Story | Depends On | Can Start After |
|-------|------------|-----------------|
| US1 (Group Rides) | Foundational | Phase 2 |
| US2 (Leaderboards) | Foundational | Phase 2 |
| US3 (Workouts) | Foundational | Phase 2 |
| US4 (Challenges) | Foundational | Phase 2 |
| US5 (Racing) | US1 networking | Phase 3 |
| US6 (Activity Feed) | Foundational | Phase 2 |
| US7 (Workout Ratings) | US3 repository | Phase 5 |
| US8 (Clubs) | Foundational | Phase 2 |
| US9 (Badges) | Foundational | Phase 2 |
| US10 (Chat) | US1 networking | Phase 3 |
| US11 (Ride Compare) | Foundational | Phase 2 |
| US12 (Profile) | Foundational | Phase 2 |

### Within Each User Story

- Models/types before services
- Services before UI
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel
- After Foundational: US1, US2, US3, US4, US6, US8, US9, US11, US12 can start in parallel
- US5, US7, US10 must wait for their dependencies

---

## Parallel Example: Phase 2 Foundational

```bash
# Launch all core type definitions together:
Task: "Create src/social/types.rs with Rider, RiderProfile, BadgeCategory, GoalType enums"
Task: "Create src/networking/protocol.rs with ProtocolMessage enum and RiderMetrics struct"
Task: "Create src/leaderboards/segments.rs with Segment, SegmentCategory types"
Task: "Create src/racing/events.rs with RaceEvent, RaceStatus, ParticipantStatus types"
```

---

## Parallel Example: After Foundational

```bash
# Multiple developers can work on independent stories:
Developer A: User Story 1 (Group Rides) - T017-T035
Developer B: User Story 2 (Leaderboards) - T036-T052
Developer C: User Story 3 (Workouts) - T053-T063
Developer D: User Story 4 (Challenges) - T064-T074
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T006)
2. Complete Phase 2: Foundational (T007-T016)
3. Complete Phase 3: User Story 1 - Group Rides (T017-T035)
4. **STOP and VALIDATE**: Test with two devices on LAN
5. Deploy/demo if ready - you have working group rides!

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → Test → Deploy (MVP: Group Rides)
3. Add User Story 2 → Test → Deploy (Leaderboards)
4. Add User Story 3 → Test → Deploy (Workout Repository)
5. Add User Story 4 → Test → Deploy (Challenges)
6. Continue with remaining stories...

### Parallel Team Strategy

With 4 developers after Foundational:

| Developer | Stories | Tasks |
|-----------|---------|-------|
| A | US1 (Group Rides) | T017-T035 |
| B | US2 (Leaderboards) | T036-T052 |
| C | US3 (Workouts) | T053-T063 |
| D | US4 (Challenges) | T064-T074 |

Then:
- A continues to US5 (Racing) after US1
- C continues to US7 (Ratings) after US3
- Others pick up US6, US8, US9, US10, US11, US12

---

## Summary

| Metric | Count |
|--------|-------|
| **Total Tasks** | 153 |
| **Setup Tasks** | 6 |
| **Foundational Tasks** | 10 |
| **User Story Tasks** | 131 |
| **Polish Tasks** | 6 |

### Tasks Per User Story

| Story | Title | Tasks | Can Parallel Start |
|-------|-------|-------|-------------------|
| US1 | Group Rides | 19 | Yes (after Phase 2) |
| US2 | Leaderboards | 17 | Yes (after Phase 2) |
| US3 | Workouts | 11 | Yes (after Phase 2) |
| US4 | Challenges | 11 | Yes (after Phase 2) |
| US5 | Racing | 14 | No (needs US1) |
| US6 | Activity Feed | 8 | Yes (after Phase 2) |
| US7 | Ratings | 8 | No (needs US3) |
| US8 | Clubs | 10 | Yes (after Phase 2) |
| US9 | Badges | 9 | Yes (after Phase 2) |
| US10 | Chat | 8 | No (needs US1) |
| US11 | Ride Compare | 7 | Yes (after Phase 2) |
| US12 | Profile | 9 | Yes (after Phase 2) |

### Suggested MVP Scope

**User Story 1 (Group Rides)** - 35 tasks total including setup/foundational

This delivers the core social feature: LAN-based group rides with peer discovery, session management, and real-time metric synchronization.

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
