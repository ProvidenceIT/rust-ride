# Research: Hardware Integration

**Feature Branch**: `007-hardware-integration`
**Date**: 2025-12-26

This document captures technology research and decisions for the Hardware Integration feature.

---

## 1. ANT+ Protocol Support

### Decision: Use `ant-rs` crate with libusb backend

### Rationale
- `ant-rs` provides pure Rust implementation of ANT+ protocol
- Cross-platform support via libusb (Windows, macOS, Linux)
- Supports required device profiles: PWR (power), HRM (heart rate), FE-C (fitness equipment)
- Active maintenance and cycling-specific examples
- No external dependencies beyond libusb

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| libant (C library) | Mature, well-tested | FFI complexity, build complexity | Rust-native preferred |
| openant (Python) | Feature-complete | Wrong language | Not applicable |
| Direct USB HID | No dependencies | Must implement full protocol | Too much work |

### Implementation Notes
- Requires libusb driver installation on Windows (Zadig)
- Dongle detection via USB VID/PID for common ANT+ sticks (Garmin, Suunto, Dynastream)
- Channel allocation: max 8 channels per dongle
- Dual-protocol detection: match by device serial number or manufacturer ID + device type

---

## 2. Audio Cues / Text-to-Speech

### Decision: Use `tts` crate for TTS, `rodio` for audio playback

### Rationale
- `tts` provides cross-platform TTS using native engines:
  - Windows: SAPI5
  - macOS: AVSpeechSynthesizer
  - Linux: speech-dispatcher
- `rodio` handles audio mixing and playback for pre-recorded sounds
- Both are pure Rust with minimal dependencies
- No cloud API required (privacy-friendly)

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| espeak-ng | Offline, cross-platform | Voice quality, requires binary | Native engines sound better |
| Cloud TTS (Google/AWS) | High quality | Requires internet, cost | Offline preferred |
| cpal only | Low-level control | Must implement TTS | Too much work |

### Implementation Notes
- Queue alerts to prevent overlap
- Configurable voice: system default or user-selected
- Volume control independent of system volume (rodio sink)
- Pre-record common phrases for faster playback

---

## 3. Smart Home Fan Control (MQTT)

### Decision: Use `rumqttc` for MQTT client

### Rationale
- Pure Rust async MQTT 3.1.1/5.0 client
- Tokio-native, integrates with existing runtime
- TLS support for secure brokers
- Automatic reconnection with backoff
- Well-documented, active development

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| paho-mqtt | Feature-complete | C library FFI | Prefer pure Rust |
| ntex-mqtt | High performance | Less documentation | rumqttc simpler |
| mqttrust | Minimal | Fewer features | Need TLS, auto-reconnect |

### Implementation Notes
- Default topic pattern: `home/fan/{device_id}/set`
- Payload: JSON `{"speed": 0-100}` or simple `0`, `1`, `2`, `3` for levels
- Zone-to-speed mapping stored in user config
- Graceful degradation if broker unavailable

---

## 4. External Display Streaming (WebSocket)

### Decision: Use `tokio-tungstenite` for WebSocket server, embedded HTML/JS dashboard

### Rationale
- `tokio-tungstenite` is async, production-ready WebSocket library
- Integrates seamlessly with tokio runtime
- Embedded dashboard avoids external file dependencies
- QR code generation with `qrcode` crate

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| axum | Full web framework | Overkill for simple streaming | Only need WebSocket |
| warp | Lightweight | Less WebSocket-focused | tungstenite more direct |
| actix-web | High performance | Different async ecosystem | Already using tokio |

### Implementation Notes
- Server binds to `0.0.0.0:8080` (configurable)
- PIN-based auth: 6-digit code displayed on main app, entered in browser
- Message format: JSON `{"power": 250, "hr": 145, "cadence": 90, ...}`
- Dashboard: single HTML file with inline CSS/JS, served at `/`
- WebSocket endpoint: `/ws`

---

## 5. USB HID Button Integration

### Decision: Use `hidapi` crate

### Rationale
- Cross-platform USB HID library
- Works with Stream Deck, USB keypads, foot pedals
- Device enumeration and hot-plug support
- Raw button input capture

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| rusb | Lower level | Must implement HID protocol | hidapi handles it |
| gilrs (gamepad) | Gamepad-focused | Not general HID | Need generic HID |
| raw keyboard | No extra deps | Can't distinguish devices | Need device-specific mapping |

### Implementation Notes
- Device identification by VID/PID
- Button mapping persisted in config file
- Available actions: `lap_marker`, `pause_resume`, `skip_interval`, `end_ride`, `volume_up`, `volume_down`
- Support multiple simultaneous devices

---

## 6. Weather Integration

### Decision: Use `reqwest` with OpenWeatherMap API (free tier)

### Rationale
- OpenWeatherMap free tier: 1000 calls/day, sufficient for 30-min refresh
- `reqwest` is the standard async HTTP client for Rust
- JSON response, easy to parse with serde
- No API key embedded; user provides their own

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| WeatherAPI | Good free tier | Less known | OpenWeatherMap more common |
| Tomorrow.io | Modern API | Lower free limits | Rate limits |
| Open-Meteo | No API key needed | Less comprehensive | OpenWeatherMap well-known |

### Implementation Notes
- Cache weather data for 30 minutes
- Location: latitude/longitude from user config
- Graceful fallback: show "unavailable" if API fails
- Display: temperature, conditions icon, humidity

---

## 7. Fitness Platform Sync (OAuth)

### Decision: Use `keyring` for credential storage, `reqwest` for API calls, `oauth2` crate for OAuth flow

### Rationale
- `keyring` provides secure OS credential storage:
  - Windows: Credential Manager
  - macOS: Keychain
  - Linux: Secret Service (GNOME Keyring, KWallet)
- `oauth2` handles OAuth 2.0 flow complexity
- Garmin Connect and Strava both use standard OAuth 2.0

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| Plain config file | Simple | Insecure | Tokens must be protected |
| SQLite encrypted | Single location | Custom encryption needed | OS keyring better |
| Master password | User-controlled | UX friction | OS keyring transparent |

### Implementation Notes
- OAuth redirect: local HTTP server on `http://localhost:9876/callback`
- Token refresh handled automatically
- Upload format: FIT file (already supported)
- Garmin API: `https://connect.garmin.com/modern/proxy/upload-service/upload`
- Strava API: `https://www.strava.com/api/v3/uploads`
- HealthKit: conditional compilation for macOS only

---

## 8. Video Course Sync

### Decision: Use `ffmpeg-next` for video playback with variable speed

### Rationale
- FFmpeg supports all common codecs (MP4, MKV, H.264, H.265)
- `ffmpeg-next` provides safe Rust bindings
- Variable playback speed via frame timing manipulation
- Hardware acceleration available on supported platforms

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| gstreamer | Powerful | Complex, large dependency | FFmpeg simpler |
| libvlc | Feature-rich | Heavy, licensing | FFmpeg lighter |
| Native video APIs | No deps | Platform-specific code | Cross-platform preferred |

### Implementation Notes
- Playback speed range: 0.5x - 2.0x (matching virtual speed)
- Frame rendering via egui texture
- Pause when rider stops (speed < 5 km/h)
- Video-to-route mapping via metadata or manual sync point

---

## 9. Sensor Fusion (Cadence)

### Decision: Implement complementary filter with fallback

### Rationale
- Simple complementary filter sufficient for cadence
- Kalman filter overkill for two sensors with similar accuracy
- Immediate fallback to single sensor on dropout

### Algorithm
```
fused_cadence = α * sensor1 + (1 - α) * sensor2
α adjusted based on sensor reliability (signal strength, dropout rate)
On sensor dropout: α → 1.0 for remaining sensor
```

### Alternatives Considered

| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| Kalman filter | Optimal | Complex, needs tuning | Overkill for simple fusion |
| Simple average | Easy | No weighting | Doesn't handle dropouts |
| Priority-based | Predictable | Ignores secondary | Wastes data |

---

## 10. Muscle Oxygen (SmO2) Sensors

### Decision: Extend existing BLE infrastructure with SmO2 GATT profile

### Rationale
- SmO2 sensors (Moxy, Train.Red) use standard BLE GATT
- Reuse btleplug infrastructure
- Custom GATT service UUID for SmO2 data

### Implementation Notes
- Moxy Service UUID: `C0310001-3F7D-0000-8000-00805F9B34FB`
- SmO2 characteristic: percentage (0-100)
- THb (total hemoglobin) if available
- Sample rate: 1 Hz typical

---

## 11. Motion Tracking / IMU

### Decision: Support generic BLE IMU sensors via standard characteristics

### Rationale
- Most consumer IMUs use BLE with standard-ish profiles
- Focus on tilt (pitch/roll) for rocker plate detection
- Vibration analysis for pedal stroke quality

### Implementation Notes
- Look for accelerometer/gyroscope BLE services
- Record raw data for post-ride analysis
- Real-time display: simplified tilt indicator
- Not attempting complex motion modeling in v1

---

## Dependency Summary

| Crate | Version | Purpose |
|-------|---------|---------|
| ant-rs | latest | ANT+ protocol |
| tts | ^0.26 | Text-to-speech |
| rodio | ^0.17 | Audio playback |
| rumqttc | ^0.24 | MQTT client |
| tokio-tungstenite | ^0.21 | WebSocket server |
| qrcode | ^0.13 | QR code generation |
| hidapi | ^2.5 | USB HID devices |
| reqwest | ^0.11 | HTTP client |
| keyring | ^2.0 | OS credential store |
| oauth2 | ^4.4 | OAuth 2.0 flow |
| ffmpeg-next | ^6.0 | Video playback |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| ANT+ dongle driver issues on Windows | Medium | High | Document Zadig setup, provide troubleshooting |
| TTS voice quality varies by OS | Low | Low | Allow user to select voice |
| MQTT broker compatibility | Low | Medium | Test with Mosquitto, Home Assistant |
| OAuth token expiration | Medium | Medium | Automatic refresh, clear error messages |
| FFmpeg licensing (LGPL/GPL) | Low | Medium | Dynamic linking, document compliance |
| HID device compatibility | Medium | Low | Document tested devices |
