# Feature Specification: RustRide Indoor Cycling Application

**Feature Branch**: `001-indoor-cycling-app`
**Created**: 2025-12-24
**Status**: Draft
**Input**: User description: "RustRide - Open-source, self-hosted indoor cycling training application built in Rust"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect Smart Trainer and Start Free Ride (Priority: P1)

As a cyclist, I want to quickly connect my smart trainer and start riding so that I can begin my training session with minimal setup.

**Why this priority**: This is the foundational capability - without sensor connectivity and basic riding, no other features are usable. A cyclist needs to be able to connect their equipment and see their metrics immediately.

**Independent Test**: Can be fully tested by launching the application, discovering a BLE smart trainer, connecting to it, and seeing real-time power/cadence/speed metrics on screen. Delivers immediate value for unstructured training sessions.

**Acceptance Scenarios**:

1. **Given** the application is launched and a BLE smart trainer is powered on, **When** the user opens the sensor discovery screen, **Then** the trainer appears in the list of discovered devices within 10 seconds
2. **Given** a trainer is discovered, **When** the user selects it and confirms pairing, **Then** the connection is established and a success indicator is shown
3. **Given** a trainer is connected, **When** the user starts a free ride, **Then** real-time power, cadence, and speed values are displayed and updated at least once per second
4. **Given** the trainer connection is lost, **When** the signal is restored, **Then** the application automatically reconnects without user intervention

---

### User Story 2 - Execute Structured Workout with ERG Mode (Priority: P2)

As a cyclist following a training plan, I want to load and execute a structured workout file so that my trainer automatically adjusts resistance to hit my power targets.

**Why this priority**: Structured workouts with ERG mode control are the core differentiating feature that provides training value beyond a basic power display. This enables purposeful, effective training sessions.

**Independent Test**: Can be fully tested by importing a .zwo workout file, starting the workout, and verifying the trainer resistance adjusts to match the prescribed power targets as intervals change.

**Acceptance Scenarios**:

1. **Given** the user has a valid .zwo workout file, **When** they import it through the workout library, **Then** the workout appears in the library with name, duration, and estimated TSS displayed
2. **Given** a workout is loaded and trainer is connected, **When** the user starts the workout in ERG mode, **Then** the trainer resistance adjusts to achieve the target power for each interval
3. **Given** a workout is in progress, **When** an interval ends and a new one begins, **Then** the target power smoothly transitions over 3 seconds (configurable ramp rate)
4. **Given** a workout is in progress, **When** the user presses pause, **Then** the timer stops, the trainer enters free ride mode, and the workout can be resumed from the same point
5. **Given** a workout is in progress, **When** the user skips an interval, **Then** the next interval begins immediately with appropriate power target

---

### User Story 3 - Record and Export Ride Data (Priority: P3)

As a cyclist, I want my ride data to be automatically recorded and exportable so that I can upload it to Strava or Garmin Connect for tracking and analysis.

**Why this priority**: Ride recording captures the value of training sessions for long-term tracking. Export capability allows integration with the broader cycling ecosystem that users already rely on.

**Independent Test**: Can be fully tested by completing a ride, verifying all metrics were captured, exporting to .fit format, and successfully uploading to Strava or Garmin Connect.

**Acceptance Scenarios**:

1. **Given** a ride is started, **When** sensor data is received, **Then** the data is recorded with 1-second resolution including timestamp, power, cadence, heart rate, speed, and distance
2. **Given** a ride is in progress, **When** 30 seconds have elapsed since the last save, **Then** the ride data is automatically persisted to storage (crash recovery)
3. **Given** a completed ride, **When** the user selects export to .fit format, **Then** a valid .fit file is generated that can be uploaded to Strava
4. **Given** a completed ride, **When** the user views the ride summary, **Then** they see duration, distance, average/max power, normalized power, TSS, average/max heart rate, and calories

---

### User Story 4 - Display Real-Time Training Metrics (Priority: P4)

As a cyclist during a training session, I want to see my current metrics in a clear, readable display so that I can monitor my effort and stay in the correct training zones.

**Why this priority**: While metrics display is essential, it builds on the connected sensor foundation (P1). The quality of the display directly impacts the training experience.

**Independent Test**: Can be fully tested by riding with connected sensors and verifying all metrics are displayed with correct values and zone color-coding.

**Acceptance Scenarios**:

1. **Given** sensors are connected and providing data, **When** viewing the ride screen, **Then** current power, 3-second average power, cadence, heart rate, speed, distance, and elapsed time are all visible
2. **Given** the user has configured their FTP, **When** current power is displayed, **Then** the power zone is indicated with color coding
3. **Given** the user has configured heart rate zones, **When** current heart rate is displayed, **Then** the HR zone is indicated with color coding
4. **Given** the ride screen is displayed, **When** the user toggles full-screen mode, **Then** the metrics remain clearly visible with large, readable numbers

---

### User Story 5 - Configure User Profile and Training Zones (Priority: P5)

As a cyclist, I want to set up my profile with FTP and other metrics so that training zones and TSS calculations are accurate for my fitness level.

**Why this priority**: Profile configuration enables accurate zone calculations and training metrics but isn't required for basic functionality. Users can start riding with defaults and configure later.

**Independent Test**: Can be fully tested by entering profile data (FTP, max HR, weight) and verifying that power zones, HR zones, and TSS calculations reflect the configured values.

**Acceptance Scenarios**:

1. **Given** the user opens profile settings, **When** they enter their FTP value, **Then** power zones are automatically calculated using the Coggan 7-zone model
2. **Given** the user enters max heart rate and resting heart rate, **When** they view HR zones, **Then** zones are calculated using standard formulas
3. **Given** the user has configured weight and FTP, **When** completing a ride, **Then** TSS and calorie calculations use these values
4. **Given** the user prefers custom zones, **When** they manually edit zone boundaries, **Then** the custom zones override the auto-calculated values

---

### User Story 6 - Browse Ride History and Analyze Past Rides (Priority: P6)

As a cyclist tracking my training, I want to view my past rides with summary statistics and charts so that I can monitor my progress over time.

**Why this priority**: History and analysis provide long-term value but aren't essential for individual training sessions. This is a "nice to have" for MVP that enhances the overall training experience.

**Independent Test**: Can be fully tested by completing multiple rides, browsing the history list, opening a past ride, and viewing the summary statistics and power/HR charts.

**Acceptance Scenarios**:

1. **Given** multiple rides have been recorded, **When** the user opens ride history, **Then** rides are listed with date, duration, distance, and average power
2. **Given** the user selects a past ride, **When** the detail view opens, **Then** they see full summary statistics and power/HR/cadence charts over time
3. **Given** the user is viewing ride history, **When** they filter by date range, **Then** only rides within that range are displayed
4. **Given** the user selects a past ride, **When** they choose to re-export it, **Then** the ride can be exported to any supported format

---

### User Story 7 - Connect Additional Sensors (Priority: P7)

As a cyclist with multiple sensors, I want to connect my heart rate monitor, cadence sensor, and power meter simultaneously so that I have complete data from all my equipment.

**Why this priority**: Multi-sensor support extends the core connectivity but many trainers already provide all necessary data. This is important for users with separate power meters or HR monitors.

**Independent Test**: Can be fully tested by pairing a heart rate monitor and cadence sensor alongside a smart trainer and verifying all data streams are received and displayed.

**Acceptance Scenarios**:

1. **Given** multiple BLE sensors are broadcasting, **When** the user opens sensor discovery, **Then** all discovered sensors are listed with their type (trainer, HR, cadence, power)
2. **Given** multiple sensors are paired, **When** a ride is started, **Then** data from all sensors is received and displayed simultaneously
3. **Given** a previously paired sensor is powered on, **When** the application starts, **Then** the sensor is automatically reconnected without manual intervention
4. **Given** sensor data conflicts (e.g., cadence from trainer and separate sensor), **When** both are connected, **Then** the user can select which source to use as primary

---

### Edge Cases

- What happens when the trainer disconnects mid-workout? System should pause the workout, display a reconnection message, and auto-resume when reconnected.
- What happens when no sensors are discovered? System should display troubleshooting tips (check Bluetooth is enabled, trainer is awake, etc.).
- What happens when a workout file is corrupted or invalid? System should display a clear error message identifying the issue and not crash.
- What happens when storage is full during ride recording? System should warn the user and continue recording to memory, then prompt for storage cleanup post-ride.
- What happens when the application crashes during a ride? On restart, the user should be prompted to recover the last ride from auto-save data.
- What happens when the user's FTP is set to zero or unrealistic values? System should validate FTP is within reasonable range (50-600W) and prompt for correction.
- What happens when power readings spike unrealistically (>2000W)? System should filter obvious noise/errors from displayed and recorded values.

## Requirements *(mandatory)*

### Functional Requirements

**Sensor Connectivity**

- **FR-001**: System MUST discover BLE devices advertising Fitness Machine Service (FTMS), Cycling Power Service, and Heart Rate Service
- **FR-002**: System MUST allow users to pair with discovered sensors and remember pairings across sessions
- **FR-003**: System MUST support simultaneous connections to multiple sensors (trainer + HR monitor + separate cadence sensor)
- **FR-004**: System MUST display connection status for all paired sensors
- **FR-005**: System MUST automatically attempt to reconnect when a paired sensor signal is lost
- **FR-006**: System MUST display sensor battery level where the sensor protocol supports it

**ERG Mode and Trainer Control**

- **FR-007**: System MUST be able to set target power on connected FTMS-compatible trainers
- **FR-008**: System MUST support configurable power ramp rate for smooth transitions (default: 3 seconds)
- **FR-009**: System MUST allow manual power adjustment during rides (+/- 5W, +/- 10W increments)
- **FR-010**: System MUST allow toggling between ERG mode and free ride mode during a session
- **FR-011**: System MUST support trainer spindown calibration

**Structured Workouts**

- **FR-012**: System MUST import .zwo (Zwift) workout files
- **FR-013**: System MUST import .mrc/.erg (TrainerRoad/generic) workout files
- **FR-014**: System MUST display workout library with name, duration, and estimated TSS
- **FR-015**: System MUST execute workout segments in sequence, setting appropriate ERG targets
- **FR-016**: System MUST support pause, resume, skip interval, and extend interval (+30s, +1min) during workout execution
- **FR-017**: System MUST display workout progress including current interval, time remaining in interval, and overall workout progress

**Real-Time Metrics**

- **FR-018**: System MUST display current power (watts) and 3-second average power
- **FR-019**: System MUST display cadence (rpm), heart rate (bpm), speed (km/h or mph), and distance
- **FR-020**: System MUST display elapsed time and estimated calories burned
- **FR-021**: System MUST calculate and display Normalized Power, Intensity Factor, and TSS when FTP is configured
- **FR-022**: System MUST indicate current power zone and heart rate zone with color coding
- **FR-023**: System MUST support configurable dashboard layout

**Ride Recording**

- **FR-024**: System MUST record ride data at 1-second resolution including all available metrics
- **FR-025**: System MUST auto-save ride data at configurable intervals (default: 30 seconds) for crash recovery
- **FR-026**: System MUST export rides to .fit format compatible with Strava and Garmin Connect
- **FR-027**: System MUST export rides to .tcx format
- **FR-028**: System MUST calculate and store ride summary statistics (avg/max power, NP, TSS, avg/max HR, distance, calories)

**User Profile**

- **FR-029**: System MUST allow users to configure FTP, max heart rate, resting heart rate, weight, and height
- **FR-030**: System MUST auto-calculate power zones using Coggan 7-zone model based on FTP
- **FR-031**: System MUST auto-calculate heart rate zones based on max and resting heart rate
- **FR-032**: System MUST allow custom zone boundaries to override calculated values
- **FR-033**: System MUST support metric and imperial unit preferences

**Ride History**

- **FR-034**: System MUST display list of past rides with summary information
- **FR-035**: System MUST allow viewing detailed ride information including charts
- **FR-036**: System MUST allow filtering ride history by date
- **FR-037**: System MUST allow re-exporting past rides
- **FR-038**: System MUST allow deleting rides from history

**User Interface**

- **FR-039**: System MUST support keyboard shortcuts for common actions (Space for pause, +/- for power adjustment)
- **FR-040**: System MUST support dark and light themes
- **FR-041**: System MUST support full-screen mode for the ride display
- **FR-042**: System MUST work on displays with 1080p resolution and above

**Platform Support**

- **FR-043**: System MUST run on Windows 10/11
- **FR-044**: System MUST run on macOS 11+
- **FR-045**: System MUST run on Linux (Ubuntu 20.04+, Fedora 35+)
- **FR-046**: System MUST operate fully offline without internet connectivity

### Key Entities

- **User Profile**: Represents the cyclist with their physiological data (FTP, max HR, weight, height) and preferences (units, theme, zones)
- **Sensor**: Represents a discovered or paired device (trainer, power meter, HR monitor, cadence/speed sensor) with its connection status and capabilities
- **Workout**: Represents a structured training session definition with name, segments (warmup, intervals, cooldown), and metadata
- **Workout Segment**: Represents a portion of a workout with duration, target power (absolute or % FTP), and optional cadence target
- **Ride**: Represents a completed or in-progress training session with start time, duration, summary statistics, and associated workout (if applicable)
- **Ride Sample**: Represents a single data point (1-second) during a ride with all captured metrics
- **Power Zone**: Represents a training intensity range defined by % of FTP with color and name
- **Heart Rate Zone**: Represents a training intensity range defined by heart rate values with color and name

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can discover and connect to a BLE smart trainer within 30 seconds of opening sensor discovery
- **SC-002**: Users can go from application launch to actively riding in 3 clicks or fewer
- **SC-003**: Ride data loss in case of application crash is limited to 30 seconds or less (auto-save interval)
- **SC-004**: Application uses less than 200 MB of memory during an active ride
- **SC-005**: Application uses less than 10% CPU during an active ride on a modern processor
- **SC-006**: Exported .fit files are accepted by Strava and Garmin Connect without errors
- **SC-007**: Real-time metrics update at least once per second with no visible lag
- **SC-008**: Workout ERG targets achieve actual power within +/- 5% of target within 10 seconds of interval start
- **SC-009**: Application starts up and is ready for use in under 5 seconds on standard hardware
- **SC-010**: All primary user flows can be completed using only keyboard navigation for accessibility
- **SC-011**: 95% of .zwo workout files from popular workout libraries import successfully without errors
- **SC-012**: Users report the metrics display is readable from 1 meter away (typical trainer setup distance)

## Assumptions

- Users have compatible BLE smart trainers that support the FTMS protocol (Wahoo, Tacx, Elite, Saris, and similar modern trainers)
- Users have Bluetooth capability on their computer (built-in or USB adapter)
- Users understand basic cycling training concepts (FTP, power zones, intervals)
- Workout files from popular sources (TrainerRoad, Zwift) follow published format specifications
- Users will configure their FTP before expecting accurate TSS/IF calculations
- SQLite is sufficient for local storage of ride history without requiring a full database server
- 1-second data resolution is sufficient for training analysis purposes
- Users prefer a simple, functional interface over visual complexity

## Out of Scope

- 3D virtual worlds, avatars, or gamification elements
- Multiplayer or group ride functionality
- Social features (following, kudos, leaderboards, challenges)
- Route simulation with gradient changes (SIM mode)
- Mobile companion applications
- Cloud synchronization or online accounts
- ANT+ protocol support (deferred to post-MVP)
- Video integration or third-party video sync
- Automatic upload to Strava/Garmin (manual export only for MVP)
- Training plan generation or multi-week program management
- FTP testing protocols or power curve analysis
