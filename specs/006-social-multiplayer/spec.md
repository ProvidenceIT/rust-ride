# Feature Specification: Social & Multiplayer

**Feature Branch**: `006-social-multiplayer`
**Created**: 2025-12-26
**Status**: Draft
**Input**: User description: "Social & Multiplayer community features designed for self-hosted, offline-first architecture including LAN-based group rides, local leaderboards, community workout repository, training challenges, virtual race events, activity feed, workout ratings, club management, achievement badges, rider profiles, LAN chat, ride comparison, and async group workouts."

## Clarifications

### Session 2025-12-26

- Q: What trust model should apply to shared data between LAN peers? → A: Trust all discovered peers (share freely, no verification required)
- Q: How are segments (for leaderboards) created and defined? → A: Pre-defined segments bundled with routes/world content, not user-editable
- Q: What happens when a race participant disconnects mid-race? → A: 60-second grace period to reconnect, then DNF if not back
- Q: What happens to ride data when app crashes during group ride? → A: Auto-save every 30 seconds, recoverable on restart
- Q: How is rider identity handled when importing leaderboard data? → A: Display name matching with prompt to confirm/merge duplicates

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Join LAN Group Ride (Priority: P1)

A cyclist wants to ride together with friends or family members on the same local network. They launch the application, which automatically discovers other riders on the LAN. They join an available group ride session and can see other riders' real-time metrics (power, cadence, heart rate) and avatar positions during the ride.

**Why this priority**: Group riding is the core social feature that enables real-time interaction. It provides immediate value by letting users train together without requiring internet connectivity or external services, aligning with the self-hosted philosophy.

**Independent Test**: Can be tested with two devices on the same network - start a group ride on one device, join from another, verify both see each other's metrics and positions.

**Acceptance Scenarios**:

1. **Given** two users on the same LAN, **When** one starts a group ride session, **Then** the other user sees the session in their discovery list within 5 seconds
2. **Given** a user has joined a group ride, **When** another participant's metrics change, **Then** the user sees the updated metrics within 1 second
3. **Given** a user is in a group ride, **When** they lose network connectivity temporarily, **Then** their session continues locally and reconnects automatically when network is restored
4. **Given** a group ride is in progress, **When** a new rider joins, **Then** all existing participants see the new rider appear without disrupting their session

---

### User Story 2 - View Local Leaderboards (Priority: P2)

A cyclist wants to compare their performance on specific segments or routes against their own previous efforts and other local riders. They select a segment and view a ranked leaderboard showing times, power output, and other relevant metrics. They can export leaderboard data to share with others.

**Why this priority**: Leaderboards provide motivation and gamification that enhances solo and group training. They work entirely offline and build on the existing ride recording functionality.

**Independent Test**: Complete multiple rides over the same segment, verify leaderboard ranks efforts correctly and displays comparison metrics.

**Acceptance Scenarios**:

1. **Given** a user has completed rides on a segment, **When** they view the segment leaderboard, **Then** they see their efforts ranked by time with relevant metrics
2. **Given** multiple local users have ridden a segment, **When** any user views the leaderboard, **Then** all users' efforts appear with proper attribution
3. **Given** a user is viewing a leaderboard, **When** they export the data, **Then** they receive a valid CSV or JSON file that can be shared
4. **Given** no rides exist for a segment, **When** a user views the segment, **Then** they see an empty leaderboard with guidance on how to add their first effort

---

### User Story 3 - Browse and Use Community Workouts (Priority: P3)

A cyclist wants to find structured workouts created by the community. They browse a repository of pre-curated workouts, filter by difficulty, duration, or training focus (endurance, threshold, VO2max), and add selected workouts to their personal library for execution.

**Why this priority**: Access to quality workouts reduces friction for new users and provides variety for experienced cyclists. The repository ships with the application and can be updated via optional GitHub sync.

**Independent Test**: Browse the workout repository, filter by criteria, add a workout to personal library, execute the workout successfully.

**Acceptance Scenarios**:

1. **Given** a user opens the workout repository, **When** they browse workouts, **Then** they see at least 50 pre-curated workouts with descriptions, difficulty ratings, and duration
2. **Given** a user is viewing the repository, **When** they filter by "threshold" focus and "intermediate" difficulty, **Then** only matching workouts appear
3. **Given** a user selects a workout, **When** they add it to their library, **Then** the workout appears in their personal collection and is executable
4. **Given** the user has internet access, **When** they trigger a repository sync, **Then** new community workouts are downloaded and available locally

---

### User Story 4 - Create and Share Training Challenges (Priority: P4)

A cyclist creates a training challenge with specific goals (e.g., "Ride 100km this week", "Complete 5 threshold workouts"). They share the challenge definition with friends who can import it and track their own progress against the challenge goals.

**Why this priority**: Challenges add structured goals and social accountability. They build on existing ride tracking and work entirely offline with simple file-based sharing.

**Independent Test**: Create a challenge, export it, import on another device, verify both users can track progress toward the same goals.

**Acceptance Scenarios**:

1. **Given** a user wants to create a challenge, **When** they define goals and duration, **Then** a challenge is created and added to their active challenges
2. **Given** a user has created a challenge, **When** they export it, **Then** they receive a TOML or JSON file they can share
3. **Given** a user receives a challenge file, **When** they import it, **Then** the challenge appears in their list with accurate goal definitions
4. **Given** a user is tracking a challenge, **When** they complete a qualifying activity, **Then** their progress updates automatically

---

### User Story 5 - Participate in Virtual Race Events (Priority: P5)

A cyclist joins a scheduled virtual race event with other LAN participants. The race has a defined start time, synchronized countdown, and real-time position tracking. After completion, results are calculated and displayed as a ranked finish list.

**Why this priority**: Racing provides structured competitive events that enhance group ride engagement. It requires the group ride foundation (P1) to be in place.

**Independent Test**: Schedule a race, have participants join, start with synchronized countdown, complete race, verify results ranking.

**Acceptance Scenarios**:

1. **Given** a race organizer creates an event, **When** they set time and course, **Then** the event appears in the LAN event discovery list
2. **Given** participants have joined a race, **When** the start time arrives, **Then** all participants see a synchronized countdown and start simultaneously
3. **Given** a race is in progress, **When** participants ride, **Then** all participants see real-time positions of others
4. **Given** a race completes, **When** all participants finish, **Then** a results screen shows ranked finish order with times

---

### User Story 6 - View Activity Feed (Priority: P6)

A cyclist views a feed of recent ride summaries from other riders on the LAN. The feed shows completed rides with key metrics, allowing users to see what others in their local community have been doing.

**Why this priority**: Activity feeds create social connection and motivation without requiring real-time participation. They leverage existing ride recording functionality.

**Independent Test**: Complete rides on multiple devices, verify all devices can discover and display each other's ride summaries.

**Acceptance Scenarios**:

1. **Given** multiple users on a LAN have completed rides, **When** a user views the activity feed, **Then** they see recent ride summaries from discoverable riders
2. **Given** a ride summary is displayed, **When** a user views details, **Then** they see key metrics (distance, duration, power, elevation)
3. **Given** a user wants to share their activities, **When** they enable sharing, **Then** their ride summaries become discoverable on the LAN

---

### User Story 7 - Rate and Review Workouts (Priority: P7)

A cyclist rates a workout after completion and optionally writes a review. When browsing workouts, ratings and reviews from the local community help inform workout selection.

**Why this priority**: Ratings improve workout discovery and help users find quality content. They require the workout repository (P3) as a foundation.

**Independent Test**: Complete a workout, rate and review it, verify ratings appear when browsing that workout.

**Acceptance Scenarios**:

1. **Given** a user completes a workout, **When** prompted, **Then** they can rate it 1-5 stars and optionally add a text review
2. **Given** a workout has ratings, **When** displayed in the repository, **Then** the average rating and review count are shown
3. **Given** a user is browsing workouts, **When** they filter by rating, **Then** only workouts meeting the minimum rating appear

---

### User Story 8 - Manage Clubs (Priority: P8)

A cyclist creates or joins a club to organize a group of riders. The club has a roster, aggregate statistics (total distance, hours ridden), and provides a context for shared challenges and leaderboards.

**Why this priority**: Clubs provide organizational structure for groups but are not essential for core social features. They enhance rather than enable social functionality.

**Independent Test**: Create a club, invite members, verify roster and aggregate stats update as members ride.

**Acceptance Scenarios**:

1. **Given** a user creates a club, **When** they set name and description, **Then** the club is created and they are the administrator
2. **Given** a club exists, **When** a user joins via invitation or club code, **Then** they appear on the roster
3. **Given** a club has active members, **When** members complete rides, **Then** club aggregate statistics update accordingly

---

### User Story 9 - Earn Achievement Badges (Priority: P9)

A cyclist earns badges for completing milestones (first 100km, FTP improvement, consistency streaks). Badges are displayed on their profile and can be viewed by other riders.

**Why this priority**: Badges provide long-term motivation and recognition but are not essential for core training or social features.

**Independent Test**: Complete activities that trigger badge criteria, verify badges unlock and display correctly.

**Acceptance Scenarios**:

1. **Given** a user completes their first 100km total, **When** the ride saves, **Then** they receive a "Century Rider" badge notification
2. **Given** a user achieves an FTP improvement, **When** detected during testing, **Then** they receive an "FTP Breakthrough" badge
3. **Given** a user has earned badges, **When** viewing their profile, **Then** earned badges are displayed with unlock dates

---

### User Story 10 - Chat During Group Rides (Priority: P10)

Cyclists in a group ride can send text messages to each other during the ride. Post-ride, chat history is preserved and viewable.

**Why this priority**: Chat enhances real-time social interaction but is not essential for group ride functionality. It's a convenience feature.

**Independent Test**: Join a group ride, send messages between participants, verify messages appear and are saved post-ride.

**Acceptance Scenarios**:

1. **Given** users are in a group ride, **When** one sends a message, **Then** all participants see it within 2 seconds
2. **Given** a group ride ends, **When** users view ride history, **Then** chat messages from the ride are viewable
3. **Given** a user sends a message, **When** another participant has temporarily disconnected, **Then** the message is delivered upon reconnection

---

### User Story 11 - Compare Rides (Priority: P11)

A cyclist compares two or more rides side-by-side using overlay charts. They can compare against their personal best or against friends' efforts on the same route.

**Why this priority**: Comparison provides analytical value but depends on having sufficient ride history and is not essential for core functionality.

**Independent Test**: Complete multiple rides on the same route, overlay charts, verify visual comparison is accurate.

**Acceptance Scenarios**:

1. **Given** a user selects two of their own rides, **When** they compare, **Then** power, heart rate, and cadence are displayed as overlaid charts
2. **Given** a user and a friend have ridden the same segment, **When** comparing, **Then** both efforts appear on the same chart with clear differentiation
3. **Given** comparison data differs in length, **When** displayed, **Then** charts align by distance or time with clear indicators

---

### User Story 12 - Manage Rider Profile (Priority: P12)

A cyclist sets up their profile with name, avatar, bio, and stats summary. This profile information is embedded in shared activities and visible to other riders.

**Why this priority**: Profiles enhance social identity but are a supporting feature that can use sensible defaults until configured.

**Independent Test**: Create a profile, share an activity, verify profile info is visible to other riders.

**Acceptance Scenarios**:

1. **Given** a user opens profile settings, **When** they edit name, avatar, and bio, **Then** changes save and display correctly
2. **Given** a user has a configured profile, **When** their activities are shared, **Then** profile info is embedded in shared data
3. **Given** a user views another's activity, **When** examining rider details, **Then** the originating rider's profile info is displayed

---

### Edge Cases

- What happens when a group ride participant has a significantly different network latency than others?
- How does the system handle conflicting segment definitions when importing leaderboard data? → N/A, segments are pre-defined and consistent across instances
- What happens when a challenge goal requires data the user hasn't enabled tracking for?
- How does avatar synchronization handle when riders are at very different positions in a virtual course?
- What happens when a race participant disconnects mid-race? → 60-second grace period, then DNF
- How does the system handle workout ratings from users who imported the same workout from different sources?

## Requirements *(mandatory)*

### Functional Requirements

**LAN Discovery & Networking**
- **FR-001**: System MUST discover other RustRide instances on the same LAN using mDNS within 5 seconds
- **FR-002**: System MUST synchronize rider metrics via UDP with latency under 100ms on local networks
- **FR-003**: System MUST handle temporary network disconnections gracefully with automatic reconnection
- **FR-004**: System MUST work entirely offline without requiring internet connectivity for core features
- **FR-004a**: System MUST trust all discovered LAN peers automatically without requiring manual approval or cryptographic verification

**Group Rides**
- **FR-005**: System MUST support group rides with up to 10 concurrent participants on a LAN
- **FR-006**: System MUST synchronize avatar positions in real-time during group rides
- **FR-007**: System MUST display live metrics (power, cadence, heart rate) from all group ride participants
- **FR-008**: System MUST allow joining or leaving group rides without disrupting other participants
- **FR-008a**: System MUST auto-save group ride data every 30 seconds to enable recovery after crash or unexpected closure

**Leaderboards**
- **FR-009**: System MUST store segment/route rankings in local SQLite database
- **FR-010**: System MUST support exporting leaderboard data to CSV and JSON formats
- **FR-011**: System MUST automatically record segment efforts when riding over pre-defined segments bundled with route content
- **FR-012**: System MUST rank efforts by elapsed time with tiebreakers based on average power
- **FR-012a**: System MUST match imported leaderboard efforts by rider display name and prompt user to confirm or merge when duplicate names are detected

**Workout Repository**
- **FR-013**: System MUST include 50-100 pre-curated workouts covering various training focuses
- **FR-014**: System MUST allow filtering workouts by difficulty (beginner, intermediate, advanced)
- **FR-015**: System MUST allow filtering workouts by training focus (endurance, tempo, threshold, VO2max, sprint)
- **FR-016**: System MUST support optional GitHub sync for repository updates when internet is available
- **FR-017**: System MUST allow users to add repository workouts to their personal library

**Training Challenges**
- **FR-018**: System MUST allow creating challenges with volume goals (distance, time, TSS)
- **FR-019**: System MUST allow creating challenges with completion goals (number of workouts, specific workout types)
- **FR-020**: System MUST export challenges to TOML or JSON format for sharing
- **FR-021**: System MUST import challenge files and track progress automatically

**Virtual Racing**
- **FR-022**: System MUST support scheduling race events with defined start times
- **FR-023**: System MUST synchronize race start countdown across all participants
- **FR-024**: System MUST track and display real-time positions during races
- **FR-025**: System MUST calculate and display final race results with rankings
- **FR-025a**: System MUST provide a 60-second grace period for disconnected race participants to reconnect; after grace period expires, participant is marked DNF (Did Not Finish)

**Activity Feed**
- **FR-026**: System MUST share ride summaries via JSON files on LAN when enabled
- **FR-027**: System MUST discover and display activity feeds from other riders via mDNS
- **FR-028**: System MUST allow users to control which activities they share

**Ratings & Reviews**
- **FR-029**: System MUST allow rating workouts on a 1-5 star scale after completion
- **FR-030**: System MUST allow optional text reviews for workouts
- **FR-031**: System MUST display average ratings and review counts when browsing workouts

**Clubs**
- **FR-032**: System MUST allow creating clubs with name, description, and administrator role
- **FR-033**: System MUST track club member roster with join/leave history
- **FR-034**: System MUST calculate and display aggregate club statistics

**Achievements**
- **FR-035**: System MUST detect and award badges for predefined milestones offline
- **FR-036**: System MUST display earned badges on rider profiles with unlock dates
- **FR-037**: System MUST include milestone badges: distance totals, FTP improvements, consistency streaks

**Chat**
- **FR-038**: System MUST support text messaging between group ride participants
- **FR-039**: System MUST preserve chat history and associate it with ride records
- **FR-040**: System MUST deliver messages within 2 seconds on local networks

**Ride Comparison**
- **FR-041**: System MUST support overlay comparison of two or more rides
- **FR-042**: System MUST display power, heart rate, and cadence data on comparison charts
- **FR-043**: System MUST align comparison data by distance or time

**Rider Profiles**
- **FR-044**: System MUST allow users to set name, avatar, and bio
- **FR-045**: System MUST embed profile information in shared activity files
- **FR-046**: System MUST display profile information when viewing other riders' activities

### Key Entities

- **Rider**: Individual user with profile, preferences, and ride history. Has earned badges and club memberships.
- **GroupRide**: Active LAN session with multiple participants, shared state, and chat messages.
- **Segment**: Pre-defined section of a route bundled with world/route content (not user-editable). Has start/end coordinates and associated efforts.
- **SegmentEffort**: Single attempt at a segment with metrics (time, power, HR). Linked to rider and segment.
- **Workout**: Structured training session with intervals and targets. Has ratings, reviews, and difficulty metadata.
- **Challenge**: Goal-based activity with duration, targets, and participant progress tracking.
- **RaceEvent**: Scheduled competitive event with participant list, start time, and results.
- **Club**: Organization of riders with roster, aggregate stats, and administrator.
- **Badge**: Achievement milestone with criteria, unlock status, and timestamp.
- **ActivitySummary**: Shareable ride summary with key metrics and rider profile info.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can discover and join a LAN group ride within 10 seconds of launching the application
- **SC-002**: Real-time metrics from other riders update within 1 second during group rides
- **SC-003**: System supports 10 concurrent group ride participants without noticeable performance degradation
- **SC-004**: Users can browse and filter 100+ workouts and find relevant options within 30 seconds
- **SC-005**: Leaderboard data exports are compatible with spreadsheet applications and can be imported by other RustRide instances
- **SC-006**: Challenges can be created, exported, and imported between users with no data loss
- **SC-007**: 80% of users can successfully join their first group ride without documentation assistance
- **SC-008**: Race event synchronization achieves start time accuracy within 500ms across all participants
- **SC-009**: Activity feed discovery finds other riders on the same network within 5 seconds
- **SC-010**: All social features function completely offline without internet connectivity
