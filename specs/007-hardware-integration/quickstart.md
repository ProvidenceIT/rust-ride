# Quickstart: Hardware Integration Development

**Feature Branch**: `007-hardware-integration`
**Date**: 2025-12-26

This guide covers development environment setup for the Hardware Integration feature.

---

## Prerequisites

### System Requirements

- **Rust**: 1.75+ stable
- **OS**: Windows 10+, macOS 12+, or Linux (Ubuntu 22.04+)
- **Memory**: 8GB+ RAM recommended
- **Storage**: 2GB+ for dependencies and test videos

### Required System Libraries

#### Windows

```powershell
# Install Visual Studio Build Tools (if not present)
winget install Microsoft.VisualStudio.2022.BuildTools

# Install libusb (for ANT+ dongle)
# Download from: https://github.com/libusb/libusb/releases
# Or use vcpkg:
vcpkg install libusb:x64-windows

# FFmpeg (for video sync)
winget install Gyan.FFmpeg
# Add to PATH: C:\Program Files\FFmpeg\bin
```

#### macOS

```bash
# Xcode command line tools
xcode-select --install

# Homebrew packages
brew install libusb ffmpeg hidapi

# For HealthKit (optional)
# Requires macOS 12+ and Xcode
```

#### Linux (Ubuntu/Debian)

```bash
# System packages
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libusb-1.0-0-dev \
    libasound2-dev \
    libspeechd-dev \
    libhidapi-dev \
    libffmpeg-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev

# For Bluetooth (existing BLE support)
sudo apt install -y libbluetooth-dev libdbus-1-dev
```

---

## Repository Setup

```bash
# Clone and checkout feature branch
git clone https://github.com/yourorg/rustride.git
cd rustride
git checkout 007-hardware-integration

# Install Rust dependencies
cargo build

# Verify setup
cargo test --no-run
```

---

## Hardware Setup

### ANT+ USB Dongle

#### Windows Driver Setup (Zadig)

1. Download Zadig: https://zadig.akeo.ie/
2. Insert ANT+ USB dongle
3. Run Zadig as Administrator
4. Options → List All Devices
5. Select your ANT+ dongle (e.g., "USB ANT Stick")
6. Select "WinUSB" driver
7. Click "Replace Driver"

#### Linux Permissions

```bash
# Create udev rule for ANT+ dongles
sudo tee /etc/udev/rules.d/99-antplus.rules << 'EOF'
# Garmin USB ANT Stick
SUBSYSTEM=="usb", ATTR{idVendor}=="0fcf", ATTR{idProduct}=="1008", MODE="0666", GROUP="plugdev"
# Dynastream USB ANT Stick
SUBSYSTEM=="usb", ATTR{idVendor}=="0fcf", ATTR{idProduct}=="1009", MODE="0666", GROUP="plugdev"
# Suunto Movestick
SUBSYSTEM=="usb", ATTR{idVendor}=="0fcf", ATTR{idProduct}=="1004", MODE="0666", GROUP="plugdev"
EOF

sudo udevadm control --reload-rules
sudo udevadm trigger

# Add user to plugdev group
sudo usermod -aG plugdev $USER
# Log out and back in
```

#### macOS

No special driver needed. Dongle should work automatically.

### USB HID Devices (Stream Deck)

#### Windows

Stream Deck works automatically with hidapi.

#### Linux

```bash
# Add udev rule for Elgato Stream Deck
sudo tee /etc/udev/rules.d/99-streamdeck.rules << 'EOF'
# Elgato Stream Deck
SUBSYSTEM=="usb", ATTR{idVendor}=="0fd9", ATTR{idProduct}=="0060", MODE="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTR{idVendor}=="0fd9", ATTR{idProduct}=="006c", MODE="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTR{idVendor}=="0fd9", ATTR{idProduct}=="006d", MODE="0666", GROUP="plugdev"
EOF

sudo udevadm control --reload-rules
```

---

## Development Configuration

### Environment Variables

```bash
# Create .env file (not committed)
cat > .env << 'EOF'
# Weather API (get free key from openweathermap.org)
OPENWEATHERMAP_API_KEY=your_api_key_here

# OAuth (for development/testing)
GARMIN_CLIENT_ID=your_garmin_client_id
GARMIN_CLIENT_SECRET=your_garmin_secret
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_secret

# MQTT Broker (for fan control testing)
MQTT_BROKER_HOST=localhost
MQTT_BROKER_PORT=1883

# Development flags
RUST_LOG=debug
RUSTRIDE_DEV_MODE=1
EOF
```

### Test MQTT Broker

```bash
# Install Mosquitto (local MQTT broker)
# macOS
brew install mosquitto
brew services start mosquitto

# Ubuntu
sudo apt install mosquitto mosquitto-clients
sudo systemctl start mosquitto

# Windows
# Download from: https://mosquitto.org/download/

# Test MQTT
mosquitto_sub -t 'home/fan/#' &
mosquitto_pub -t 'home/fan/test/set' -m '{"speed": 50}'
```

---

## Running Tests

### Unit Tests

```bash
# All unit tests
cargo test

# Specific modules
cargo test sensors::ant
cargo test audio
cargo test integrations::mqtt
cargo test hid
cargo test video
```

### Integration Tests

```bash
# Integration tests require hardware or mocks
cargo test --test ant_sensors -- --ignored
cargo test --test mqtt_fan -- --ignored
cargo test --test websocket -- --ignored
cargo test --test hid_buttons -- --ignored
```

### Hardware Simulation

For testing without physical hardware:

```bash
# ANT+ simulator (Python)
pip install openant
python scripts/ant_simulator.py

# Mock MQTT messages
mosquitto_pub -t 'home/fan/living_room/status' -m '{"speed": 75, "on": true}'
```

---

## Development Workflow

### Feature Modules

Each hardware feature is in a separate module for independent development:

```
src/
├── sensors/ant/       # P1: ANT+ support
├── sensors/incline.rs # P1: Slope mode
├── sensors/dynamics.rs# P2: Cycling dynamics
├── audio/             # P2: Audio cues
├── integrations/mqtt/ # P3: Fan control
├── integrations/streaming/ # P3: External display
├── hid/               # P3: USB buttons
├── integrations/weather/   # P4: Weather
├── integrations/sync/      # P4: Platform sync
├── sensors/smo2.rs    # P4: Muscle oxygen
├── video/             # P5: Video sync
├── sensors/fusion.rs  # P5: Sensor fusion
├── sensors/imu.rs     # P5: Motion tracking
```

### Testing Priorities

1. **P1 features**: Test with real hardware if available
2. **P2-P3 features**: Mocks acceptable
3. **P4-P5 features**: Mocks recommended (specialized hardware)

### Code Style

```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Check for unsafe code
cargo geiger
```

---

## Debugging

### Logging

```bash
# Enable debug logging
RUST_LOG=rustride=debug cargo run

# Specific module logging
RUST_LOG=rustride::sensors::ant=trace cargo run
RUST_LOG=rustride::audio=debug,rustride::integrations=debug cargo run
```

### ANT+ Debugging

```rust
// In code, enable ANT+ protocol tracing
tracing::debug!(channel = %channel, data = ?bytes, "ANT+ message received");
```

### WebSocket Debugging

```bash
# Connect to streaming server with wscat
npm install -g wscat
wscat -c ws://localhost:8080/ws

# Send test auth
{"type": "auth", "pin": "123456"}
```

### MQTT Debugging

```bash
# Monitor all MQTT traffic
mosquitto_sub -v -t '#'

# Publish test message
mosquitto_pub -t 'home/fan/test/set' -m '100'
```

---

## Common Issues

### ANT+ Dongle Not Detected

**Windows**: Ensure WinUSB driver is installed via Zadig
**Linux**: Check udev rules and user group membership
**macOS**: Try unplugging and replugging the dongle

### Audio Not Working

**Windows**: Check Windows audio settings
**Linux**: Ensure speech-dispatcher is running:
```bash
systemctl --user status speech-dispatcher
```
**macOS**: Check System Preferences → Security → Privacy → Microphone

### FFmpeg Errors

Ensure FFmpeg libraries are in PATH:
```bash
ffmpeg -version
```

If missing codecs, rebuild FFmpeg with required options or install full version.

### HID Device Not Found

Check device permissions and that no other application is using the device.

---

## API Keys and Secrets

For OAuth and weather integration, you'll need API credentials:

1. **OpenWeatherMap**: https://openweathermap.org/api (free tier)
2. **Garmin Connect**: https://developer.garmin.com/gc-developer-program/
3. **Strava**: https://www.strava.com/settings/api

Store credentials in OS keyring during development:
```bash
# macOS
security add-generic-password -a rustride -s openweathermap -w YOUR_API_KEY

# Linux (with secret-tool)
secret-tool store --label='RustRide OpenWeatherMap' service rustride key openweathermap

# Windows (PowerShell)
cmdkey /add:rustride-openweathermap /user:api /pass:YOUR_API_KEY
```

---

## Next Steps

1. Review `spec.md` for feature requirements
2. Review `data-model.md` for entity definitions
3. Review `contracts/*.md` for module APIs
4. Start with P1 features (ANT+, Incline Mode)
5. Run `/speckit.tasks` to generate task breakdown
