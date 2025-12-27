# Implementation Plan: Hardware Integration

**Branch**: `007-hardware-integration` | **Date**: 2025-12-26 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/007-hardware-integration/spec.md`

## Summary

Expand RustRide's hardware connectivity beyond BLE to include ANT+ protocol support, smart trainer incline/slope mode, cycling dynamics (L/R balance), audio cues/TTS, smart home fan control via MQTT, external display streaming via WebSocket, USB HID button integration, weather API integration, fitness platform sync (Garmin/Strava), muscle oxygen (SmO2) monitoring, video course sync, advanced pedal sensor metrics, cadence sensor fusion, and motion/IMU tracking. This feature adds 14 major capabilities organized into 5 priority tiers.

## Technical Context

**Language/Version**: Rust stable (1.75+)
**Primary Dependencies**:
- Existing: btleplug (BLE), egui/eframe (GUI), tokio (async), rusqlite (storage), serde, tracing
- New: ant-rs or libant (ANT+), tts/rodio (audio), rumqttc (MQTT), tokio-tungstenite (WebSocket), hidapi (USB HID), reqwest (HTTP APIs), keyring (OS credential store), ffmpeg-next or gstreamer (video)
**Storage**: SQLite via rusqlite (existing), extended schema for new entities
**Testing**: cargo test, integration tests with mock sensors
**Target Platform**: Windows, macOS, Linux desktop
**Project Type**: Single desktop application with embedded web dashboard
**Performance Goals**:
- Sensor data latency <100ms
- Audio cue delivery <500ms
- WebSocket streaming at 1Hz
- Button response <100ms
**Constraints**:
- Local-only operation (no cloud dependency except optional platform sync)
- Cross-platform compatibility
- Memory-efficient for long rides
**Scale/Scope**: Single-user desktop application, ~5 concurrent sensor connections, ~3 WebSocket clients

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project constitution is a template without specific gates defined. Proceeding with standard software engineering practices:

| Gate | Status | Notes |
|------|--------|-------|
| Library-first design | PASS | Each feature module (ant, audio, mqtt, streaming, hid, weather, sync, video) is independently testable |
| Test coverage | PASS | Each module will have unit tests; integration tests for sensor communication |
| Observability | PASS | Using existing tracing infrastructure |
| Simplicity | PASS | Features organized by priority; lower priority features can be deferred |

## Project Structure

### Documentation (this feature)

```text
specs/007-hardware-integration/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output - module API contracts
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── sensors/
│   ├── mod.rs           # Existing - sensor abstractions
│   ├── manager.rs       # Existing - sensor lifecycle
│   ├── ftms.rs          # Existing - FTMS protocol
│   ├── types.rs         # Existing - sensor types
│   ├── ant/             # NEW - ANT+ protocol support
│   │   ├── mod.rs       # ANT+ module exports
│   │   ├── dongle.rs    # USB dongle detection/management
│   │   ├── channels.rs  # ANT+ channel management
│   │   ├── profiles/    # Device profiles
│   │   │   ├── mod.rs
│   │   │   ├── power.rs # ANT+ PWR profile
│   │   │   ├── hr.rs    # ANT+ HRM profile
│   │   │   └── fec.rs   # ANT+ FE-C profile
│   │   └── duplex.rs    # Dual-protocol duplicate detection
│   ├── incline.rs       # NEW - FTMS slope/grade control
│   ├── dynamics.rs      # NEW - Cycling dynamics parsing
│   ├── smo2.rs          # NEW - Muscle oxygen sensors
│   ├── imu.rs           # NEW - Motion/IMU sensors
│   └── fusion.rs        # NEW - Multi-sensor data fusion
├── audio/               # NEW - Audio cues module
│   ├── mod.rs
│   ├── engine.rs        # Audio playback engine
│   ├── tts.rs           # Text-to-speech
│   ├── alerts.rs        # Alert configuration and triggers
│   └── cues.rs          # Predefined cue library
├── integrations/        # NEW - External integrations
│   ├── mod.rs
│   ├── mqtt/            # Smart home/fan control
│   │   ├── mod.rs
│   │   ├── client.rs    # MQTT connection management
│   │   └── fan.rs       # Fan speed control logic
│   ├── weather/         # Weather API
│   │   ├── mod.rs
│   │   └── provider.rs  # Weather data fetching
│   ├── sync/            # Platform sync
│   │   ├── mod.rs
│   │   ├── oauth.rs     # OAuth flow handling
│   │   ├── garmin.rs    # Garmin Connect API
│   │   ├── strava.rs    # Strava API
│   │   └── healthkit.rs # macOS HealthKit (conditional)
│   └── streaming/       # External display
│       ├── mod.rs
│       ├── server.rs    # WebSocket server
│       ├── dashboard.rs # Embedded web dashboard
│       └── pin.rs       # PIN-based authentication
├── hid/                 # NEW - USB HID integration
│   ├── mod.rs
│   ├── device.rs        # HID device detection
│   ├── mapping.rs       # Button-to-action mapping
│   └── actions.rs       # Available actions
├── video/               # NEW - Video sync
│   ├── mod.rs
│   ├── player.rs        # Video playback
│   └── sync.rs          # Speed-based sync logic
├── storage/
│   ├── mod.rs           # Existing
│   ├── database.rs      # Existing - extend migrations
│   ├── schema.rs        # Existing - extend schema
│   └── hardware_store.rs # NEW - Hardware config persistence
└── ui/
    ├── screens/
    │   ├── sensor_setup.rs  # Existing - extend for ANT+
    │   ├── settings.rs      # Existing - extend for new settings
    │   └── streaming.rs     # NEW - External display config
    └── widgets/
        ├── dynamics_display.rs  # NEW - L/R balance widget
        ├── smo2_display.rs      # NEW - Muscle oxygen widget
        └── weather_widget.rs    # NEW - Weather display

tests/
├── integration/
│   ├── ant_sensors.rs   # ANT+ integration tests
│   ├── mqtt_fan.rs      # MQTT integration tests
│   ├── websocket.rs     # Streaming tests
│   └── hid_buttons.rs   # HID integration tests
└── unit/
    ├── incline.rs       # Slope calculation tests
    ├── dynamics.rs      # Cycling dynamics parsing tests
    ├── fusion.rs        # Sensor fusion tests
    └── audio_cues.rs    # Audio alert tests
```

**Structure Decision**: Extends existing single-project structure with new modules under `src/sensors/`, `src/audio/`, `src/integrations/`, `src/hid/`, and `src/video/`. Each hardware feature area is self-contained for independent testing and optional compilation.

## Complexity Tracking

No constitution violations require justification. The feature scope is large but organized into clear priority tiers allowing incremental delivery.

## Phase Outputs

The following artifacts will be generated:

1. **research.md** - Technology decisions for ANT+, audio, MQTT, WebSocket, HID, OAuth, video playback
2. **data-model.md** - Extended entity definitions for all new sensor types and configurations
3. **contracts/** - Module API contracts for each new subsystem
4. **quickstart.md** - Developer setup for hardware testing
