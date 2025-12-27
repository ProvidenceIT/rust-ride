# Feature Specification: Hardware Integration

**Feature Branch**: `007-hardware-integration`
**Created**: 2025-12-26
**Status**: Draft
**Input**: Expanded sensor and device support including ANT+ protocol, cycling dynamics, smart trainer incline mode, audio cues, smart home fan control, external display support, weather integration, Stream Deck buttons, fitness watch integration, muscle oxygen monitoring, video course sync, pedal sensor integration, cadence sensor fusion, and motion tracking.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - ANT+ Sensor Connection (Priority: P1)

A user with legacy ANT+ sensors (power meter, heart rate monitor, or smart trainer) connects their USB ANT+ dongle and expects to use their existing equipment with RustRide. They should be able to discover, pair, and receive data from ANT+ devices just as seamlessly as BLE devices.

**Why this priority**: ANT+ is still widely used in the cycling community, especially for legacy trainers and power meters. Many users have significant investments in ANT+ hardware. Without this support, a large segment of potential users cannot use RustRide.

**Independent Test**: Can be fully tested by connecting an ANT+ USB dongle, discovering sensors, pairing a power meter, and verifying power data streams correctly during a free ride.

**Acceptance Scenarios**:

1. **Given** an ANT+ USB dongle is connected to the computer, **When** the user opens the sensor discovery screen, **Then** the system detects the dongle and enables ANT+ scanning.
2. **Given** ANT+ scanning is active, **When** an ANT+ power meter broadcasts, **Then** the device appears in the discovery list with type and signal strength.
3. **Given** an ANT+ device is discovered, **When** the user selects it to pair, **Then** the system establishes a connection and begins receiving data within 5 seconds.
4. **Given** an ANT+ sensor is paired, **When** data is received, **Then** the metrics display updates in real-time identically to BLE sensors.

---

### User Story 2 - Smart Trainer Incline/Slope Mode (Priority: P1)

A user riding a virtual route or workout wants their smart trainer to simulate realistic gradients. When climbing a virtual hill, the trainer resistance increases proportionally to the gradient. This provides immersive gradient-based training.

**Why this priority**: Gradient simulation is a core feature for virtual riding and route simulation. It directly enables realistic outdoor ride replication and is expected by users of modern smart trainers.

**Independent Test**: Can be fully tested by loading a route with varying gradients and verifying the trainer resistance changes correspond to the gradient values shown on screen.

**Acceptance Scenarios**:

1. **Given** a smart trainer supporting FTMS slope mode is connected, **When** the user starts a route with gradient data, **Then** the system sends slope commands to the trainer.
2. **Given** the virtual route shows a 5% gradient, **When** the user pedals, **Then** the trainer resistance feels proportionally harder than flat sections.
3. **Given** the route transitions from uphill to downhill, **When** the gradient becomes negative, **Then** the trainer resistance decreases to simulate descending.
4. **Given** gradient simulation is active, **When** the user's weight is configured, **Then** the resistance calculation accounts for rider weight for realistic feel.

---

### User Story 3 - Cycling Dynamics and Left/Right Balance (Priority: P2)

A user with a dual-sided power meter wants to see their left/right power balance and pedaling dynamics metrics. This helps them identify muscle imbalances and improve pedaling efficiency.

**Why this priority**: Cycling dynamics provide valuable training insights for users with compatible power meters. It enhances training quality without requiring new hardware for those who already have dual-sided power meters.

**Independent Test**: Can be fully tested by connecting a dual-sided power meter, performing a ride, and verifying L/R balance percentages and torque effectiveness display correctly.

**Acceptance Scenarios**:

1. **Given** a power meter supporting cycling dynamics is connected, **When** the user views real-time metrics, **Then** left/right power balance percentage is displayed.
2. **Given** cycling dynamics data is available, **When** the user pedals, **Then** pedal smoothness and torque effectiveness metrics update in real-time.
3. **Given** a ride with cycling dynamics is completed, **When** the user views ride summary, **Then** average L/R balance and efficiency metrics are shown.
4. **Given** cycling dynamics data is recorded, **When** the user exports the ride, **Then** the data is included in compatible export formats.

---

### User Story 4 - Audio Cues and Voice Alerts (Priority: P2)

A user wants audio notifications during workouts to stay informed without constantly watching the screen. Voice alerts announce interval transitions, zone changes, and milestone achievements.

**Why this priority**: Audio cues significantly improve workout experience by reducing the need to visually monitor the screen. This is especially valuable during intense efforts when visual attention is limited.

**Independent Test**: Can be fully tested by starting a structured workout and verifying voice announcements occur at interval transitions and zone boundary crossings.

**Acceptance Scenarios**:

1. **Given** audio cues are enabled in settings, **When** a workout interval begins, **Then** a voice alert announces the interval name and target.
2. **Given** audio cues are enabled, **When** the user's power zone changes, **Then** a voice alert announces the new zone.
3. **Given** audio cues are enabled, **When** the user reaches a milestone (halfway, final minute), **Then** a voice alert provides encouragement.
4. **Given** a workout is in progress, **When** the user adjusts audio volume, **Then** subsequent alerts respect the new volume level.
5. **Given** the user prefers silent mode, **When** audio cues are disabled, **Then** no voice alerts are played.

---

### User Story 5 - Smart Home Fan Control (Priority: P3)

A user wants their smart fan to automatically adjust speed based on their effort level. When working harder (higher power or heart rate zone), the fan speeds up for better cooling.

**Why this priority**: Automatic fan control enhances comfort during intense training. While not essential for core functionality, it provides meaningful quality-of-life improvement for users with smart home equipment.

**Independent Test**: Can be fully tested by configuring an MQTT broker connection, linking a smart fan, and verifying fan speed changes correlate with power output during a ride.

**Acceptance Scenarios**:

1. **Given** MQTT broker credentials are configured, **When** the user tests the connection, **Then** the system confirms successful connection.
2. **Given** a smart fan device is configured, **When** the user's power exceeds zone thresholds, **Then** the fan speed increases automatically.
3. **Given** fan control is active, **When** the user stops pedaling or ends the ride, **Then** the fan returns to idle or off state.
4. **Given** multiple fan speed levels are supported, **When** power zones change, **Then** fan speed adjusts proportionally to the zone.

---

### User Story 6 - External Display Streaming (Priority: P3)

A user wants to view their metrics on a secondary display (tablet, TV, or phone) positioned for better visibility while riding. The external display shows real-time metrics synchronized with the main application.

**Why this priority**: External displays improve ergonomics and visibility. Many users prefer larger screens positioned at eye level. This enhances the training experience without requiring screen mirroring.

**Independent Test**: Can be fully tested by starting the WebSocket server, connecting a browser on a secondary device, and verifying metrics update in real-time during a ride.

**Acceptance Scenarios**:

1. **Given** external display streaming is enabled, **When** the user views the network settings, **Then** a URL/QR code for connecting is displayed.
2. **Given** a secondary device connects to the stream URL, **When** the user is riding, **Then** real-time metrics (power, HR, cadence, time) display on the secondary device.
3. **Given** multiple devices are connected, **When** metrics update, **Then** all connected displays refresh within 1 second.
4. **Given** the ride ends, **When** the user stops streaming, **Then** connected devices show a disconnection message.

---

### User Story 7 - Stream Deck / USB Button Integration (Priority: P3)

A user wants to use physical buttons (Stream Deck, USB keypad) for quick actions during rides. One-press buttons for lap markers, interval skipping, or pausing reduce interaction friction.

**Why this priority**: Physical buttons improve usability during intense efforts when touchscreen or mouse interaction is difficult. This caters to power users who want efficient control.

**Independent Test**: Can be fully tested by connecting a USB HID device, mapping buttons to actions in settings, and verifying button presses trigger the assigned actions during a ride.

**Acceptance Scenarios**:

1. **Given** a USB HID device is connected, **When** the user opens button configuration, **Then** detected devices are listed for mapping.
2. **Given** a button is mapped to "Add Lap Marker", **When** the user presses that button during a ride, **Then** a lap marker is added at the current timestamp.
3. **Given** a button is mapped to "Skip Interval", **When** pressed during a workout, **Then** the workout advances to the next interval.
4. **Given** button mappings are configured, **When** the user reconnects the device in a future session, **Then** the mappings persist and work immediately.

---

### User Story 8 - Weather Integration (Priority: P4)

A user wants to see current local weather conditions displayed during their ride. This provides context for indoor training decisions and conversation topics with fellow riders.

**Why this priority**: Weather display is a convenience feature that adds polish to the application. It has minimal impact on core training functionality but enhances the overall experience.

**Independent Test**: Can be fully tested by configuring location settings, starting a ride, and verifying current temperature and conditions display in the metrics area.

**Acceptance Scenarios**:

1. **Given** location is configured, **When** the user starts a ride, **Then** current weather conditions display on the ride screen.
2. **Given** weather is displayed, **When** conditions change during a long ride, **Then** the display updates periodically.
3. **Given** weather API is unavailable, **When** the user starts a ride, **Then** the weather section shows "unavailable" gracefully without affecting other features.

---

### User Story 9 - Fitness Watch Data Sync (Priority: P4)

A user wants their ride data to sync to their fitness watch ecosystem (Garmin Connect, Apple Health, Google Fit). This consolidates training data across platforms.

**Why this priority**: Platform sync extends the utility of RustRide data but is not essential for core functionality. Users can export files manually as an alternative.

**Independent Test**: Can be fully tested by completing a ride, initiating sync to a configured platform, and verifying the activity appears in the external platform.

**Acceptance Scenarios**:

1. **Given** Garmin Connect credentials are configured, **When** the user completes a ride, **Then** an option to sync to Garmin appears.
2. **Given** sync is initiated, **When** the upload completes, **Then** the user sees a success confirmation with a link to view the activity.
3. **Given** sync fails, **When** an error occurs, **Then** the user sees a clear error message and retry option.

---

### User Story 10 - Muscle Oxygen Monitoring (Priority: P4)

A user with a muscle oxygen sensor (SmO2) wants to see real-time muscle oxygenation data alongside power and heart rate. This provides insights into muscular fatigue and recovery.

**Why this priority**: SmO2 monitoring is an advanced feature for users with specialized sensors. The user base is smaller, but the data is valuable for serious athletes.

**Independent Test**: Can be fully tested by connecting a SmO2 sensor, performing intervals, and verifying SmO2 percentage displays and updates in real-time.

**Acceptance Scenarios**:

1. **Given** a SmO2 sensor is discovered, **When** the user pairs it, **Then** the sensor type is recognized and appropriate metrics are enabled.
2. **Given** SmO2 data is streaming, **When** the user views real-time metrics, **Then** muscle oxygen saturation percentage is displayed.
3. **Given** a ride with SmO2 data is completed, **When** the user views the ride summary, **Then** SmO2 trends are visualized alongside power and HR.

---

### User Story 11 - Video Course Sync (Priority: P5)

A user wants to watch scenic ride videos synchronized to their workout. As they pedal, the video playback speed adjusts to match their pace, creating an immersive experience.

**Why this priority**: Video sync is a significant undertaking that adds entertainment value. While engaging, it's not essential for training effectiveness and requires substantial video content.

**Independent Test**: Can be fully tested by loading a video file with a compatible route, starting a ride, and verifying video playback speed correlates with rider speed.

**Acceptance Scenarios**:

1. **Given** a video file is loaded for a route, **When** the user starts riding, **Then** video playback begins synchronized to the route start.
2. **Given** video sync is active, **When** the user pedals faster, **Then** video playback speed increases proportionally.
3. **Given** the user stops pedaling, **When** speed drops to zero, **Then** video playback pauses.
4. **Given** a workout ends, **When** the video reaches completion, **Then** playback stops gracefully.

---

### User Story 12 - Pedal Sensor Integration (Priority: P5)

A user with advanced pedal sensors (Shimano, Favero Assioma) wants to see detailed pedaling metrics including force vectors and pedaling dynamics beyond basic L/R balance.

**Why this priority**: Advanced pedal metrics serve a niche audience with specific high-end hardware. The feature builds on cycling dynamics (P2) with more detailed data.

**Independent Test**: Can be fully tested by connecting compatible pedal sensors and verifying force vector and advanced dynamics data displays correctly.

**Acceptance Scenarios**:

1. **Given** pedal sensors supporting extended metrics are connected, **When** the user views detailed metrics, **Then** force vector data is displayed.
2. **Given** extended pedal data is available, **When** recorded in a ride, **Then** the data can be exported for external analysis.

---

### User Story 13 - Cadence Sensor Fusion (Priority: P5)

A user with multiple cadence sources (crank sensor + pedal sensor) wants the system to intelligently combine data for improved accuracy using sensor fusion algorithms.

**Why this priority**: Sensor fusion is an optimization for edge cases with multiple redundant sensors. Most users have single cadence sources.

**Independent Test**: Can be fully tested by connecting two cadence sources and verifying the displayed cadence is a fused value that's smoother and more accurate than either source alone.

**Acceptance Scenarios**:

1. **Given** multiple cadence sensors are connected, **When** both report data, **Then** the system displays a fused cadence value.
2. **Given** one sensor drops out temporarily, **When** the other continues, **Then** cadence display remains stable without gaps.

---

### User Story 14 - Motion Tracking / Rocker Plate Detection (Priority: P5)

A user with a rocker plate or motion platform wants the system to detect and potentially use motion data for enhanced metrics or bike fit analysis.

**Why this priority**: Motion tracking is a specialized feature for users with advanced setups. It's technically complex with limited audience.

**Independent Test**: Can be fully tested by connecting an IMU sensor, riding on a rocker plate, and verifying motion data is captured and displayed.

**Acceptance Scenarios**:

1. **Given** an IMU/motion sensor is connected, **When** the user rides, **Then** motion data (tilt, vibration) is captured.
2. **Given** motion data is recorded, **When** viewing ride analysis, **Then** motion patterns can be visualized.

---

### Edge Cases

- What happens when an ANT+ dongle is disconnected during a ride?
- How does the system handle conflicting data from BLE and ANT+ versions of the same sensor? → System detects duplicate sensors and prompts user to choose preferred protocol.
- What happens when MQTT broker connection is lost during fan control?
- How does the system behave when weather API rate limits are exceeded?
- What happens when video file duration doesn't match route distance?
- How does incline mode behave with extremely steep gradients (>20%)?
- What happens when multiple Stream Deck devices are connected simultaneously?
- How does the system handle SmO2 sensors with different data formats?

## Requirements *(mandatory)*

### Functional Requirements

**ANT+ Protocol Support**
- **FR-001**: System MUST detect connected ANT+ USB dongles on startup and during runtime.
- **FR-002**: System MUST support ANT+ device profiles for power meters (ANT+ PWR), heart rate monitors (ANT+ HRM), and fitness equipment (ANT+ FE-C).
- **FR-003**: System MUST allow simultaneous connections to both ANT+ and BLE devices.
- **FR-004**: System MUST handle ANT+ channel management for multi-device scenarios.
- **FR-004a**: System MUST detect when a sensor broadcasts on both BLE and ANT+ protocols and prompt the user to choose their preferred protocol, preventing duplicate data streams.

**Smart Trainer Incline Mode**
- **FR-005**: System MUST send FTMS slope/grade commands to compatible smart trainers.
- **FR-006**: System MUST calculate resistance based on gradient, rider weight, and bike weight.
- **FR-007**: System MUST support gradient values from -20% to +20%.
- **FR-008**: System MUST allow users to adjust gradient feel intensity (50%-150% scaling).

**Cycling Dynamics**
- **FR-009**: System MUST parse extended Cycling Power Service data for L/R balance.
- **FR-010**: System MUST display pedal smoothness and torque effectiveness when available.
- **FR-011**: System MUST record cycling dynamics data for post-ride analysis.
- **FR-012**: System MUST include cycling dynamics in compatible export formats (FIT).

**Audio Cues**
- **FR-013**: System MUST provide voice alerts for workout interval transitions.
- **FR-014**: System MUST announce power zone changes during rides.
- **FR-015**: System MUST allow users to configure which audio cues are enabled.
- **FR-016**: System MUST support adjustable alert volume independent of system volume.
- **FR-017**: System MUST provide text-to-speech capability for dynamic content.

**Smart Home Fan Control**
- **FR-018**: System MUST connect to MQTT brokers with configurable credentials.
- **FR-019**: System MUST publish fan speed commands based on power or HR zone thresholds.
- **FR-020**: System MUST allow users to configure zone-to-speed mappings.
- **FR-021**: System MUST handle broker disconnection gracefully with automatic reconnection.

**External Display Streaming**
- **FR-022**: System MUST provide a WebSocket server for real-time metrics streaming.
- **FR-023**: System MUST serve a web-based dashboard accessible from local network devices.
- **FR-024**: System MUST stream metrics updates at configurable intervals (default 1 second).
- **FR-025**: System MUST display connection URL and QR code for easy device pairing.
- **FR-025a**: System MUST require PIN-based pairing for external display connections; a PIN is displayed on the main application and entered on the secondary device to authorize access.

**Stream Deck / USB Buttons**
- **FR-026**: System MUST detect USB HID devices capable of button input.
- **FR-027**: System MUST allow mapping of buttons to application actions (lap, pause, skip, etc.).
- **FR-028**: System MUST persist button mappings across sessions.
- **FR-029**: System MUST support multiple simultaneous HID devices.

**Weather Integration**
- **FR-030**: System MUST fetch current weather data from a weather API based on configured location.
- **FR-031**: System MUST display temperature, conditions, and humidity on the ride screen.
- **FR-032**: System MUST cache weather data to minimize API calls (refresh every 30 minutes).
- **FR-033**: System MUST handle API unavailability gracefully without affecting core features.

**Fitness Watch Integration**
- **FR-034**: System MUST support OAuth authentication for fitness platforms (Garmin, Strava).
- **FR-034a**: System MUST store OAuth tokens securely using the OS credential store (Windows Credential Manager, macOS Keychain, Linux Secret Service).
- **FR-034b**: System MUST handle token refresh automatically when tokens expire, re-prompting for authentication only if refresh fails.
- **FR-035**: System MUST upload completed rides to connected platforms.
- **FR-036**: System MUST support HealthKit integration on macOS for Apple Health sync.
- **FR-037**: System MUST provide sync status and error feedback to users.

**Muscle Oxygen Monitoring**
- **FR-038**: System MUST discover and connect to SmO2 sensors via BLE.
- **FR-039**: System MUST display muscle oxygen saturation percentage in real-time.
- **FR-040**: System MUST record SmO2 data synchronized with other metrics.

**Video Course Sync**
- **FR-041**: System MUST load and play video files (MP4, MKV) synchronized to ride progress.
- **FR-042**: System MUST adjust video playback speed based on rider virtual speed.
- **FR-043**: System MUST pause video when rider stops.

**Pedal Sensor Integration**
- **FR-044**: System MUST parse extended pedal metrics from compatible sensors.
- **FR-045**: System MUST display force vector visualizations when data is available.

**Cadence Sensor Fusion**
- **FR-046**: System MUST combine data from multiple cadence sources using filtering algorithms.
- **FR-047**: System MUST provide seamless fallback when one sensor drops.

**Motion Tracking**
- **FR-048**: System MUST connect to IMU sensors for motion data capture.
- **FR-049**: System MUST record motion data for post-ride analysis.

### Key Entities

- **Sensor**: Represents any connected device (ANT+, BLE, USB HID) with type, protocol, connection status, and data channels.
- **AudioAlert**: Configuration for voice notifications including trigger conditions, message templates, and volume settings.
- **FanProfile**: Mapping of power/HR zones to fan speed percentages with MQTT topic configuration.
- **ExternalDisplay**: Connected streaming client with session ID, connection time, and streaming preferences.
- **ButtonMapping**: Association between HID device button and application action with optional modifier keys.
- **WeatherData**: Current conditions including temperature, humidity, conditions description, and last update time.
- **PlatformSync**: OAuth credentials and sync status for external fitness platforms.
- **SmO2Reading**: Muscle oxygen saturation value with timestamp and sensor source.
- **VideoSync**: Video file reference with route mapping and current playback position.
- **MotionSample**: IMU data including acceleration, rotation, and derived metrics.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users with ANT+ sensors can discover and connect devices within 10 seconds of enabling scanning.
- **SC-002**: Smart trainer gradient changes are imperceptible to users (transition smoothness rated satisfactory by 90% of testers).
- **SC-003**: Left/right balance accuracy matches manufacturer-reported values within 1% margin.
- **SC-004**: Audio cues are delivered within 500ms of the triggering event.
- **SC-005**: Fan speed adjustments occur within 2 seconds of zone threshold crossing.
- **SC-006**: External display metrics update within 1 second of main application.
- **SC-007**: Button press to action execution takes less than 100ms.
- **SC-008**: Weather data displays correctly for 95% of configured locations.
- **SC-009**: Platform sync completes successfully for 98% of upload attempts.
- **SC-010**: SmO2 readings display update at sensor broadcast frequency (typically 1Hz).
- **SC-011**: Video playback maintains sync within 2 seconds of actual ride progress.
- **SC-012**: Sensor fusion eliminates 90% of cadence dropouts compared to single-sensor mode.

## Assumptions

- Users have appropriate hardware (ANT+ dongle, compatible sensors, smart home devices) for the features they wish to use.
- Weather API free tier provides sufficient quota for typical usage (assumed ~50 calls/day per user).
- MQTT-compatible smart fans use standard on/off and speed level topics.
- Video files are pre-processed or compatible with standard media codecs.
- OAuth flows for fitness platforms follow standard patterns and platform APIs remain stable.
- SmO2 sensors broadcast via standard BLE GATT profiles.
- HID devices report button presses via standard USB HID protocols.

## Clarifications

### Session 2025-12-26

- Q: Should external display streaming require authentication? → A: PIN-based pairing - user enters a displayed PIN on the secondary device to connect.
- Q: How should the system handle sensors broadcasting on both BLE and ANT+? → A: Detect duplicate sensors and prompt user to choose preferred protocol.
- Q: How should OAuth tokens for fitness platform sync be stored securely? → A: Use OS credential store (Keychain/Credential Manager/Secret Service).

## Out of Scope

- Creating or hosting video content for video sync feature.
- Supporting proprietary/closed protocols without public documentation.
- Real-time coaching based on SmO2 data (recording only).
- Integration with home automation systems beyond MQTT fan control.
- Support for ANT+ dongles without standard drivers (chip-specific implementations).
- Bidirectional sync with fitness platforms (upload only, no download).
