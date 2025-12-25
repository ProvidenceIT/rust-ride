# Feature Specification: 3D Virtual World & Complete Feature Implementation

**Feature Branch**: `002-3d-world-features`
**Created**: 2025-12-24
**Status**: Draft
**Input**: User description: "implement all coming soon features and create a zwift 3d environment"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Complete Sensor Control (Priority: P1)

A cyclist opens RustRide and wants to connect their smart trainer and additional sensors (heart rate, cadence) via Bluetooth. They click "Start Scanning" to discover nearby devices, see them appear in a list with signal strength indicators, tap to connect, and receive confirmation when connected. They can also disconnect sensors when done.

**Why this priority**: Without working sensor connectivity, no other features (training, recording, 3D world) can function. This is the foundation of the entire application.

**Independent Test**: Can be fully tested by launching app, scanning for BLE devices, connecting a trainer, verifying data flows, and disconnecting - all without any other features.

**Acceptance Scenarios**:

1. **Given** the app is open and Bluetooth is enabled, **When** user clicks "Start Scanning", **Then** the system initiates BLE discovery and shows discovered fitness devices within 10 seconds
2. **Given** sensors are discovered, **When** user taps on a sensor, **Then** a connection is established and the sensor status shows "Connected" with live data display
3. **Given** a sensor is connected, **When** user clicks "Disconnect", **Then** the connection is terminated and the sensor returns to the discovered list
4. **Given** sensors are connected, **When** the home screen is viewed, **Then** the sensor status bar shows all connected sensor names and their connection states

---

### User Story 2 - Ride Recording & Persistence (Priority: P1)

A cyclist completes a training session and wants their ride data saved permanently. When they end the ride, the system automatically saves all recorded samples (power, heart rate, cadence, speed, duration) to persistent storage. If the app crashes during a ride, recovery data allows them to resume or save what was recorded.

**Why this priority**: Users expect their training effort to be preserved. Data loss would severely impact trust in the application.

**Independent Test**: Start a ride, record data for several minutes, end ride, close app, reopen app, and verify ride appears in history with all metrics intact.

**Acceptance Scenarios**:

1. **Given** a ride is in progress with recorded samples, **When** user clicks "End Ride" and confirms, **Then** all ride data is persisted to the database and user sees the ride summary
2. **Given** ride data is saved, **When** user navigates to ride history, **Then** the completed ride appears with correct date, duration, and summary metrics
3. **Given** a ride is in progress, **When** the app crashes or is force-closed, **Then** upon restart the user is prompted to recover the interrupted ride
4. **Given** recovery data exists, **When** user chooses "Recover", **Then** the ride is restored and can be saved to history
5. **Given** recovery data exists, **When** user chooses "Discard", **Then** the recovery data is deleted and the user starts fresh

---

### User Story 3 - Ride History & Details (Priority: P2)

A cyclist wants to review their past training sessions. They navigate to ride history, see a chronological list of all rides with key metrics (date, duration, distance, average power), can filter or search, and tap any ride to see detailed analysis including power zones distribution, heart rate trends, and the ability to export or delete.

**Why this priority**: Reviewing past performance is essential for training progress. Without history, each ride exists in isolation.

**Independent Test**: Complete multiple rides, navigate to history, verify all rides listed with accurate metrics, tap to view details, and export one ride.

**Acceptance Scenarios**:

1. **Given** multiple rides exist in the database, **When** user opens ride history, **Then** all rides are displayed in reverse chronological order with date, duration, distance, and average power
2. **Given** ride history is displayed, **When** user taps on a ride, **Then** the detail view shows comprehensive metrics including power zones, HR zones, and lap data
3. **Given** ride detail is displayed, **When** user taps "Export", **Then** the ride is exported in the selected format (TCX, CSV) and saved to the designated location
4. **Given** ride detail is displayed, **When** user taps "Delete" and confirms, **Then** the ride is permanently removed from history

---

### User Story 4 - Workout Library & Execution (Priority: P2)

A cyclist has structured workout files (.zwo, .mrc) they want to use for interval training. They access the workout library, import their files using a file browser, see workout previews with intervals and duration, select one to execute, and follow guided intervals with ERG mode controlling their trainer's resistance automatically.

**Why this priority**: Structured training is the core differentiator from simple free-riding. ERG mode automation is what makes smart trainers valuable.

**Independent Test**: Import a workout file, view it in library, start workout, verify ERG targets change with intervals, complete workout successfully.

**Acceptance Scenarios**:

1. **Given** the workout library is open, **When** user clicks "Import" and selects a .zwo or .mrc file, **Then** the workout is parsed and added to the library with name, duration, and description
2. **Given** workouts exist in library, **When** user views the library, **Then** each workout shows name, duration, type, and a visual preview of the interval structure
3. **Given** a workout is selected, **When** user starts the workout with a connected trainer, **Then** ERG mode activates and power targets are sent to the trainer for each interval
4. **Given** a workout is in progress, **When** an interval changes, **Then** the user sees visual/audio notification and feels the resistance change within 3 seconds
5. **Given** a workout is in progress, **When** user pauses/resumes/skips, **Then** the workout state updates accordingly and ERG targets adjust

---

### User Story 5 - Settings & User Profile (Priority: P2)

A cyclist wants to configure their training zones, personal metrics (FTP, max HR, weight), display preferences (units, theme), and data storage locations. They access settings, modify values, and see changes reflected throughout the app.

**Why this priority**: Personalized zones and FTP are necessary for accurate training metrics. Without correct settings, all derived metrics (TSS, IF, zones) are meaningless.

**Independent Test**: Open settings, change FTP, save, start ride, verify power zone calculations use new FTP value.

**Acceptance Scenarios**:

1. **Given** settings screen is open, **When** user updates FTP value, **Then** all power zone calculations throughout the app use the new FTP
2. **Given** settings screen is open, **When** user changes display units (metric/imperial), **Then** all speed and distance displays update to the selected unit system
3. **Given** settings screen is open, **When** user toggles theme (light/dark), **Then** the entire UI updates to the selected theme immediately
4. **Given** settings have been modified, **When** user clicks "Save", **Then** changes persist across app restarts
5. **Given** settings screen is open, **When** user modifies max HR, **Then** heart rate zone calculations update accordingly

---

### User Story 6 - 3D Virtual World Riding (Priority: P3)

A cyclist wants an immersive experience while training. They enter a 3D virtual environment where an avatar represents them on a virtual road. As they pedal, their avatar moves through the world at a speed proportional to their power output. The world includes roads, scenery (trees, buildings, mountains), other ambient riders, and visual feedback for effort level.

**Why this priority**: While functional without it, the 3D world provides motivation and engagement that transforms indoor training from boring to enjoyable.

**Independent Test**: Connect trainer, start 3D ride mode, verify avatar moves when pedaling, scenery scrolls appropriately, and speed correlates to power output.

**Acceptance Scenarios**:

1. **Given** a trainer is connected, **When** user selects "Start Ride" in 3D mode, **Then** a virtual world loads with the user's avatar positioned on a road
2. **Given** user is in the 3D world, **When** user pedals, **Then** the avatar moves forward at a speed calculated from power output, weight, and virtual gradient
3. **Given** user is riding in 3D, **When** power output changes, **Then** avatar speed updates smoothly within 1 second
4. **Given** user is in the 3D world, **Then** scenery includes roads, terrain, vegetation, sky with dynamic lighting, and ambient visual elements
5. **Given** user is riding, **When** they approach a turn in the road, **Then** the avatar follows the road path automatically
6. **Given** user ends the ride, **Then** the 3D session concludes and ride data is saved normally

---

### User Story 7 - 3D World Selection & Routes (Priority: P3)

A cyclist wants variety in their virtual experience. They can choose from different virtual worlds (flat roads, mountainous terrain, urban, coastal), each with distinct visual themes and route characteristics. Routes have defined distances and elevation profiles.

**Why this priority**: Variety prevents monotony in repeated training sessions. Different routes provide different visual and mental experiences.

**Independent Test**: View world selection, choose different worlds, verify each loads with distinct visuals and route characteristics.

**Acceptance Scenarios**:

1. **Given** user opens world selection, **When** they view available worlds, **Then** at least 3 distinct world options are displayed with preview images and descriptions
2. **Given** worlds are displayed, **When** user selects a world, **Then** routes available in that world are shown with distance and elevation profile
3. **Given** a route is selected, **When** user starts riding, **Then** the 3D environment matches the selected world's visual theme
4. **Given** user is riding a route, **When** they view the HUD, **Then** current position on route, remaining distance, and elevation are displayed

---

### User Story 8 - Avatar Customization (Priority: P4)

A cyclist wants their virtual representation to feel personal. They can customize their avatar's appearance including jersey color, bike style, and basic body characteristics to match their preference.

**Why this priority**: Personalization enhances engagement but is not essential for core functionality.

**Independent Test**: Open avatar customization, change jersey color, save, start ride, verify avatar displays with chosen customization.

**Acceptance Scenarios**:

1. **Given** avatar customization is open, **When** user selects jersey color, **Then** the preview updates to show the selected color
2. **Given** customization is complete, **When** user saves and starts a ride, **Then** the in-world avatar reflects all customization choices
3. **Given** avatar is customized, **When** user restarts the app, **Then** customization persists and is applied to future rides

---

### Edge Cases

- What happens when Bluetooth is disabled or unavailable? Display clear error message with instructions to enable Bluetooth.
- What happens when a sensor disconnects mid-ride? Show reconnection attempt notification, continue recording with gaps marked, auto-reconnect when sensor is available.
- What happens when storage is full? Warn user before ride starts if space is critically low, prevent new rides if no space available.
- What happens when an imported workout file is malformed? Display specific parse error and reject the file without crashing.
- What happens when the user's system cannot render 3D graphics? Detect capability on startup, offer 2D fallback mode, display system requirements.
- What happens when 3D world fails to load? Timeout after 30 seconds, offer retry or fallback to standard ride mode.
- What happens when trainer loses power during ERG mode? Detect loss of control, notify user, continue recording without ERG.

## Requirements *(mandatory)*

### Functional Requirements

**Sensor Connectivity**
- **FR-001**: System MUST initiate BLE scanning when user requests discovery
- **FR-002**: System MUST display discovered fitness sensors (FTMS, Cycling Power, Heart Rate services) within 10 seconds of scan start
- **FR-003**: System MUST establish BLE connections to selected sensors with visual feedback
- **FR-004**: System MUST terminate BLE connections on user request
- **FR-005**: System MUST display real-time sensor status on home screen

**Data Persistence**
- **FR-006**: System MUST save all ride samples (power, HR, cadence, speed, timestamp) when ride ends
- **FR-007**: System MUST auto-save ride progress every 30 seconds during active recording
- **FR-008**: System MUST detect crash recovery data on startup and prompt user
- **FR-009**: System MUST allow recovery or discard of interrupted ride data
- **FR-010**: System MUST calculate and store derived metrics (NP, TSS, IF, distance) with each ride

**Ride History**
- **FR-011**: System MUST display all saved rides in chronological order
- **FR-012**: System MUST show summary metrics (date, duration, distance, avg power, avg HR) for each ride
- **FR-013**: System MUST provide detailed view with power zones distribution and HR zones distribution
- **FR-014**: System MUST export rides to TCX format with all available data
- **FR-015**: System MUST export rides to CSV format
- **FR-016**: System MUST allow permanent deletion of rides with confirmation
- **FR-017**: System MUST calculate maximum speed from ride samples for TCX export

**Workout System**
- **FR-018**: System MUST provide file browser for importing .zwo and .mrc workout files
- **FR-019**: System MUST parse and validate imported workout files
- **FR-020**: System MUST display workout library with name, duration, and interval preview
- **FR-021**: System MUST execute workouts with automatic ERG mode power targets
- **FR-022**: System MUST allow pause, resume, and skip operations during workout execution
- **FR-023**: System MUST send power targets to connected trainer within 1 second of interval change

**Settings & Profile**
- **FR-024**: System MUST allow configuration of FTP (Functional Threshold Power)
- **FR-025**: System MUST allow configuration of maximum heart rate
- **FR-026**: System MUST allow configuration of weight
- **FR-027**: System MUST calculate power zones from FTP automatically
- **FR-028**: System MUST calculate HR zones from max HR automatically
- **FR-029**: System MUST allow selection of unit system (metric/imperial)
- **FR-030**: System MUST allow selection of display theme (light/dark)
- **FR-031**: System MUST implement light theme colors
- **FR-032**: System MUST persist all settings across sessions

**3D Virtual World**
- **FR-033**: System MUST render a 3D environment with roads, terrain, and scenery
- **FR-034**: System MUST display user avatar on virtual bicycle
- **FR-035**: System MUST calculate avatar speed from power output, weight, and virtual gradient
- **FR-036**: System MUST update avatar position smoothly based on speed
- **FR-037**: System MUST render scenery elements (vegetation, buildings, terrain features)
- **FR-038**: System MUST implement dynamic sky and lighting
- **FR-039**: System MUST provide minimum 3 distinct virtual worlds with different themes
- **FR-040**: System MUST define routes with distance and elevation profiles
- **FR-041**: System MUST display HUD with current metrics overlaid on 3D view
- **FR-042**: System MUST follow road paths automatically (no steering required)

**Avatar Customization**
- **FR-043**: System MUST allow customization of avatar jersey color
- **FR-044**: System MUST allow selection of bike style
- **FR-045**: System MUST persist avatar customization across sessions
- **FR-046**: System MUST apply customization to in-world avatar rendering

### Key Entities

- **Ride**: Represents a completed training session with start time, duration, samples, and calculated metrics
- **RideSample**: Individual data point captured during ride (timestamp, power, HR, cadence, speed)
- **Workout**: Structured training plan with intervals, power targets, and cadence targets
- **WorkoutSegment**: Individual interval within a workout (type, duration, power target)
- **UserProfile**: User's personal settings including FTP, max HR, weight, and preferences
- **VirtualWorld**: 3D environment definition with theme, terrain data, and available routes
- **Route**: Path through a virtual world with distance, elevation profile, and waypoints
- **Avatar**: User's virtual representation with customizable appearance

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can discover and connect BLE sensors within 30 seconds of opening the app
- **SC-002**: 100% of completed rides are successfully saved with zero data loss
- **SC-003**: Crash recovery successfully restores at least 95% of recorded data from interrupted sessions
- **SC-004**: Users can import a workout file and begin executing it within 60 seconds
- **SC-005**: ERG mode power targets are applied to trainer within 2 seconds of interval change
- **SC-006**: Users can locate and review any past ride within 10 seconds of opening history
- **SC-007**: Exported files are valid and can be imported by major platforms (Strava, TrainingPeaks, Garmin Connect)
- **SC-008**: 3D world loads and becomes interactive within 10 seconds on systems meeting minimum requirements
- **SC-009**: 3D rendering maintains minimum 30 frames per second during normal operation
- **SC-010**: Avatar speed updates reflect power changes within 1 second
- **SC-011**: Settings changes take effect immediately without requiring app restart
- **SC-012**: Users can complete a full training session (connect, ride in 3D world, save, review) in a single seamless workflow

## Assumptions

- User's computer has Bluetooth Low Energy capability
- User has a compatible smart trainer supporting FTMS protocol
- User's GPU supports hardware-accelerated 3D rendering (OpenGL 3.3+ or equivalent)
- Minimum 4GB RAM available for 3D world rendering
- User has write access to the designated data storage location
- Network connectivity is NOT required (all features work offline)
- Workouts will be imported from files, not created in-app (workout builder is out of scope)

## Out of Scope

- ANT+ protocol support (BLE only for this phase)
- Multiplayer or group ride functionality
- Social features (following, kudos, leaderboards)
- Cloud synchronization or online accounts
- Video integration or third-party video sync
- Route simulation with dynamic gradient control (SIM mode)
- Training plan generation or multi-week programs
- FTP testing protocols
- Mobile companion applications
- User-generated world content
