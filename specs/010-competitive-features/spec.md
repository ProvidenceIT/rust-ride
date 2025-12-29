# Feature Specification: Competitive Feature Gaps

**Feature Branch**: `010-competitive-features`
**Created**: 2025-12-28
**Status**: Draft
**Input**: User description: "Competitive Feature Gaps - Features from Zwift, TrainerRoad, Wahoo SYSTM, Rouvy, and Fulgaz that RustRide doesn't have"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Gradient-Responsive Resistance (Priority: P1)

A cyclist loads a real-world route (GPX file or video-based course) and rides it on their smart trainer. As they virtually ascend hills, the trainer resistance automatically increases to simulate the gradient. During descents, resistance decreases. The user experiences a realistic simulation of outdoor terrain without manually adjusting settings.

**Why this priority**: Gradient-responsive resistance is the most technically feasible feature to implement with the existing FTMS trainer control infrastructure. It directly enhances the core indoor cycling experience and differentiates RustRide from basic trainer apps. Users with smart trainers expect this functionality.

**Independent Test**: Can be fully tested by loading a GPX file with elevation data and verifying trainer resistance changes correspond to gradient changes during the ride.

**Acceptance Scenarios**:

1. **Given** a user has loaded a GPX route with elevation data, **When** they start riding, **Then** the trainer resistance adjusts automatically based on the gradient at their current position
2. **Given** a user is riding a simulated uphill section (positive gradient), **When** the gradient increases, **Then** the resistance increases proportionally
3. **Given** a user is on a descent (negative gradient), **When** they pedal, **Then** the resistance is reduced to simulate easier spinning
4. **Given** a user's virtual weight is configured, **When** riding on gradients, **Then** the resistance calculation factors in their body weight plus bike weight

---

### User Story 2 - Achievement Badges & XP System (Priority: P2)

A cyclist completes various milestones during their training - first 100km, first interval workout, consecutive training days, personal power records, etc. The system awards badges and experience points, providing gamification that motivates continued training. Users can view their badge collection and track their level progression.

**Why this priority**: Gamification significantly increases user retention and motivation. Badges and XP require no external infrastructure (unlike multiplayer features) and can be implemented entirely client-side with the existing database. This addresses a key engagement gap versus Zwift.

**Independent Test**: Can be tested by completing specific achievements and verifying badges are awarded and XP accumulates correctly.

**Acceptance Scenarios**:

1. **Given** a user completes their first ride, **When** the ride ends, **Then** they receive a "First Ride" badge and corresponding XP
2. **Given** a user reaches a new distance milestone (e.g., 100km total), **When** the threshold is crossed, **Then** they receive the appropriate badge and notification
3. **Given** a user has accumulated enough XP to level up, **When** they reach the threshold, **Then** their level increases and they receive a level-up notification
4. **Given** a user wants to view their achievements, **When** they open the achievements screen, **Then** they see all earned badges and their current level/XP progress

---

### User Story 3 - 4D Power Profiling (Priority: P3)

A cyclist wants to understand their power capabilities beyond just FTP. The system analyzes their ride data to create a power profile across multiple durations: sprint (5-15 seconds), neuromuscular power (30 seconds), anaerobic capacity (1-3 minutes), sustained power (5-20 minutes), and aerobic endurance (60+ minutes). This multi-dimensional profile helps users identify strengths and weaknesses.

**Why this priority**: Provides actionable training insights beyond basic FTP that help users train smarter. Can be calculated from existing ride data without new hardware. Addresses the sophisticated training analytics that TrainerRoad and Wahoo SYSTM offer.

**Independent Test**: Can be tested by importing historical ride data and verifying the system generates an accurate multi-duration power profile.

**Acceptance Scenarios**:

1. **Given** a user has completed several rides with varied efforts, **When** they view their power profile, **Then** they see their best power outputs across multiple time durations
2. **Given** a user's power profile shows weakness in a specific duration, **When** displayed, **Then** the weakness is visually highlighted with training recommendations
3. **Given** a user completes a new ride with record power at a duration, **When** the ride is saved, **Then** their profile updates and they're notified of the improvement
4. **Given** a user views their profile history, **When** comparing over time, **Then** they can see how each power duration has improved or declined

---

### User Story 4 - Multi-Discipline Training Plans (Priority: P4)

A cyclist selects a training plan tailored to their specific discipline - road racing, gravel events, triathlon, mountain biking, or general fitness. The plan provides structured workouts designed for the unique demands of each discipline, with appropriate intensity distribution and skill-specific intervals.

**Why this priority**: Offers personalized training that goes beyond generic workouts. Requires content creation (workout libraries per discipline) but no new technical infrastructure. Addresses TrainerRoad's discipline-specific training advantage.

**Independent Test**: Can be tested by selecting a discipline, receiving a plan, and verifying workouts align with that discipline's typical demands.

**Acceptance Scenarios**:

1. **Given** a user selects "Road Racing" as their goal, **When** they browse plans, **Then** they see road-specific plans with appropriate interval types and volumes
2. **Given** a user is following a triathlon plan, **When** they view upcoming workouts, **Then** the workouts balance cycling with brick workout suggestions
3. **Given** a user switches disciplines mid-plan, **When** they change their goal, **Then** the system offers to adapt their remaining plan or switch to a new one
4. **Given** a user's schedule constraints are set, **When** a plan is generated, **Then** workout frequency and duration fit within their available time

---

### User Story 5 - Career Levels with Long-Term Progression (Priority: P5)

A cyclist engages with RustRide over months and years, accumulating experience that unlocks new features, cosmetic rewards, and potentially partner discounts. The system provides a long-term progression path that rewards consistent training and app engagement beyond individual achievements.

**Why this priority**: Drives long-term user retention through meaningful progression. Relatively lower implementation complexity once the XP system exists. Addresses Rouvy's 80+ level career system.

**Independent Test**: Can be tested by simulating long-term usage and verifying levels unlock progressively with appropriate rewards.

**Acceptance Scenarios**:

1. **Given** a user reaches career level 10, **When** they level up, **Then** they unlock a new UI theme or avatar customization option
2. **Given** a user has achieved level 25, **When** they view their profile, **Then** their level badge is prominently displayed
3. **Given** a user reaches a significant milestone level, **When** they level up, **Then** they receive a special "milestone" notification and reward
4. **Given** the system supports partner rewards, **When** a user reaches the required level, **Then** they can access partner discount codes

---

### Edge Cases

- What happens when a GPX file has missing or corrupted elevation data? System gracefully falls back to flat resistance or notifies user.
- What happens when the user disconnects mid-ride during a gradient section? Resistance returns to baseline and recovers when reconnected.
- How does the achievement system handle imported historical data? Badges can be retroactively awarded for past accomplishments.
- What if a user's power profile has insufficient data for a duration? That duration shows as "incomplete" with guidance on what rides would provide data.
- How are XP and levels handled if the user reinstalls the app? Profile data syncs from local database; cloud sync is optional.

## Requirements *(mandatory)*

### Functional Requirements

**Gradient-Responsive Resistance**:
- **FR-001**: System MUST parse GPX files to extract distance and elevation data
- **FR-002**: System MUST calculate instantaneous gradient from elevation profile during ride playback
- **FR-003**: System MUST convert gradient percentage to trainer resistance using physics model (user weight + bike weight + gradient)
- **FR-004**: System MUST send resistance commands to FTMS-compatible trainers at minimum 1Hz update rate
- **FR-005**: System MUST allow users to configure a "trainer difficulty" multiplier (0-100%) to scale gradient effect
- **FR-006**: System MUST display current virtual gradient to user during ride
- **FR-006a**: System MUST allow users to configure maximum gradient cap (default: -15% to +15%) with smooth transitions at limits

**Achievement Badges & XP System**:
- **FR-007**: System MUST track achievement progress across defined categories (distance, time, power, consistency, milestones)
- **FR-008**: System MUST award badges immediately upon achievement completion
- **FR-009**: System MUST display visual notifications when badges are earned
- **FR-009a**: During active rides, system MUST queue achievement notifications and display at natural break points (interval rest periods, ride pause, or ride end)
- **FR-010**: System MUST persist achievement state to local database
- **FR-011**: System MUST calculate and display user level based on cumulative XP
- **FR-012**: System MUST provide an achievements gallery screen showing earned and locked badges

**4D Power Profiling**:
- **FR-013**: System MUST calculate best power outputs for durations: 5s, 15s, 30s, 1min, 3min, 5min, 10min, 20min, 60min
- **FR-014**: System MUST store power profile history to track changes over time
- **FR-014a**: System MUST maintain a "current" power profile using a rolling 90-day window
- **FR-014b**: System MUST separately track and display lifetime best power values
- **FR-015**: System MUST visualize power profile using a power curve graph (showing both current and lifetime bests)
- **FR-016**: System MUST identify relative strengths and weaknesses by comparing user's curve shape to reference curves
- **FR-017**: System MUST update power profile automatically after each ride with sufficient data

**Multi-Discipline Training Plans**:
- **FR-018**: System MUST provide pre-built training plans for at least 4 disciplines (road, gravel, triathlon, MTB)
- **FR-019**: System MUST allow users to filter and browse plans by discipline
- **FR-020**: System MUST schedule workouts based on user's available training days
- **FR-021**: System MUST support plan customization (swap workouts, adjust dates)

**Career Levels & Progression**:
- **FR-022**: System MUST define a level progression system with at least 50 levels
- **FR-023**: System MUST unlock cosmetic rewards (themes, avatars) at specific level milestones
- **FR-024**: System MUST display user's career level prominently in profile
- **FR-025**: System MUST calculate level from cumulative lifetime XP

### Key Entities

- **GPXRoute**: Represents an imported route with distance, elevation, and gradient data points
- **Achievement**: Defines a badge with criteria, XP value, icon, and category
- **UserAchievement**: Links a user to earned achievements with timestamp
- **PowerProfile**: Stores user's best power values across standard durations with date achieved
- **TrainingPlan**: A structured workout schedule with discipline type, duration, and weekly structure
- **UserLevel**: Tracks cumulative XP and current career level

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users riding gradient-responsive routes report resistance changes feel realistic (80%+ satisfaction in user feedback)
- **SC-002**: Users with achievement systems enabled ride 30%+ more frequently than baseline
- **SC-003**: 70% of users who view their power profile take action on identified weaknesses within 30 days
- **SC-004**: Users can load and start a gradient-responsive ride within 60 seconds
- **SC-005**: Achievement notifications appear within 2 seconds of completion
- **SC-006**: Power profile calculations complete within 5 seconds of ride save
- **SC-007**: Users following discipline-specific plans report higher relevance than generic plans (4.0+ average rating out of 5)
- **SC-008**: 50% of users who reach level 10 remain active after 6 months

## Clarifications

### Session 2025-12-28

- Q: How should extreme gradients be handled (trainer limits, safety)? → A: User-configurable gradient cap with recommended defaults
- Q: How long should power profile records remain valid for "current" fitness? → A: Rolling 90-day window for current profile, lifetime bests shown separately
- Q: When should achievement notifications appear during active rides? → A: Queue and show at natural break points (interval rest, pause, ride end)

## Assumptions

- Users have FTMS-compatible smart trainers capable of receiving resistance commands (no SIM mode required)
- GPX files follow standard schema with elevation data in meters
- Achievement definitions and XP values can be tuned post-launch without data migration
- Training plan content (workout definitions) will be created as a separate content effort
- Career level thresholds follow a standard exponential XP curve
- Power profile durations align with established sports science standards (e.g., Coggan power zones)

## Scope Boundaries

**In Scope**:
- Gradient-responsive resistance for GPX-based routes
- Local achievement and XP system
- Multi-duration power profile analysis
- Pre-built discipline-specific training plans
- Career level progression with cosmetic rewards

**Out of Scope (for this specification)**:
- Real-world route video integration (Rouvy/Fulgaz style)
- Real-time multiplayer/group rides (requires server infrastructure)
- AI-adaptive training plans (separate ML feature)
- Racing events and leaderboards (requires online services)
- In-app messaging
- Outdoor workout sync to bike computers
- AR overlays
- Turn physics simulation
- 3D virtual worlds

## Dependencies

- Existing FTMS trainer control implementation (sensors module)
- Existing ride recording and database infrastructure
- Existing workout parsing and execution engine
