# Quickstart: Headless/CLI Mode

**Feature**: 009-headless-cli-mode
**Date**: 2025-12-28

## Prerequisites

- Linux x86_64 or ARM64 (Raspberry Pi 4/5)
- Bluetooth adapter (USB or built-in)
- BlueZ installed (`sudo apt install bluez`)

## Installation

### From Release Binary

```bash
# Download for your architecture
wget https://github.com/ProvidenceIT/rust-ride/releases/latest/download/rustride-linux-amd64.tar.gz
# or for Raspberry Pi
wget https://github.com/ProvidenceIT/rust-ride/releases/latest/download/rustride-linux-arm64.tar.gz

# Extract and install
tar xzf rustride-linux-*.tar.gz
sudo mv rustride rustride-cli /usr/local/bin/
```

### From Source

```bash
git clone https://github.com/ProvidenceIT/rust-ride.git
cd rust-ride
cargo build --release
sudo cp target/release/rustride target/release/rustride-cli /usr/local/bin/
```

## Quick Start

### 1. Start the Daemon

```bash
# Start in foreground (for testing)
rustride --headless

# Or start as background daemon
rustride daemon start

# Check daemon status
rustride-cli status
```

### 2. Connect Sensors

```bash
# Scan for sensors
rustride-cli sensors list --scan

# Connect to a smart trainer
rustride-cli sensors connect AA:BB:CC:DD:EE:FF

# Verify connection
rustride-cli sensors status
```

### 3. Start a Free Ride

```bash
# Start riding
rustride-cli ride start

# Monitor live metrics
rustride-cli status --live

# Stop and save
rustride-cli ride stop
```

### 4. Execute a Workout

```bash
# Start a workout
rustride-cli workout start ~/workouts/sweetspot.zwo

# Check progress
rustride-cli status

# Control the workout
rustride-cli workout pause
rustride-cli workout resume
rustride-cli workout skip    # Skip to next interval

# End the workout
rustride-cli workout stop
```

### 5. Export Rides

```bash
# List recent rides
rustride-cli rides list

# Export to FIT format
rustride-cli ride export --format fit --output ~/rides/ride.fit

# Export to TCX
rustride-cli ride export --format tcx --output ~/rides/ride.tcx
```

## Configuration

Edit `~/.config/rustride/config.toml`:

```toml
[daemon]
log_level = "info"
log_path = "~/.local/share/rustride/daemon.log"
auto_connect_sensors = true

[sensors]
# Auto-connect to these sensors on daemon start
preferred = [
    "AA:BB:CC:DD:EE:FF",  # Wahoo KICKR
    "11:22:33:44:55:66",  # Garmin HRM
]

[user]
ftp = 250
weight_kg = 75
max_hr = 185

[zones.power]
z1 = 0.55   # 55% of FTP
z2 = 0.75
z3 = 0.90
z4 = 1.05
z5 = 1.20
z6 = 1.50
```

## Systemd Service

For automatic startup on boot:

```bash
# Copy service file
sudo cp /usr/share/rustride/rustride.service /etc/systemd/system/

# Enable and start
sudo systemctl enable rustride
sudo systemctl start rustride

# Check status
sudo systemctl status rustride
```

Service file (`/etc/systemd/system/rustride.service`):

```ini
[Unit]
Description=RustRide Cycling Daemon
After=bluetooth.target

[Service]
Type=simple
ExecStart=/usr/local/bin/rustride --headless
Restart=on-failure
RestartSec=5
User=pi

[Install]
WantedBy=multi-user.target
```

## Scripting Example

Automated morning workout:

```bash
#!/bin/bash
# morning-workout.sh

set -e

# Ensure daemon is running
if ! rustride-cli status >/dev/null 2>&1; then
    echo "Starting daemon..."
    rustride daemon start
    sleep 5
fi

# Connect sensors
echo "Connecting sensors..."
rustride-cli sensors connect AA:BB:CC:DD:EE:FF  # Trainer
rustride-cli sensors connect 11:22:33:44:55:66  # HRM

# Wait for connections
sleep 3

# Start workout and wait for completion
echo "Starting workout..."
rustride-cli workout start ~/workouts/morning.zwo --wait

# Export ride
RIDE_ID=$(rustride-cli rides list --limit 1 --json | jq -r '.[0].id')
rustride-cli ride export --ride-id "$RIDE_ID" --format fit \
    --output ~/rides/$(date +%Y%m%d)-morning.fit

echo "Workout complete!"
```

## JSON Output

All commands support `--json` for machine-readable output:

```bash
# Get status as JSON
rustride-cli status --json | jq .

# List sensors as JSON
rustride-cli sensors list --json

# Pipe to other tools
rustride-cli status --json | jq '.metrics.power_watts'
```

## Troubleshooting

### Daemon won't start

```bash
# Check if already running
rustride-cli status

# Check logs
tail -f ~/.local/share/rustride/daemon.log

# Check BLE adapter
bluetoothctl show
```

### Can't find sensors

```bash
# Ensure BLE adapter is up
sudo hciconfig hci0 up

# Check BlueZ is running
systemctl status bluetooth

# Scan manually
bluetoothctl scan on
```

### Permission denied

```bash
# Add user to bluetooth group
sudo usermod -a -G bluetooth $USER
# Log out and back in
```

## CLI Reference

```bash
rustride --help
rustride-cli --help
rustride-cli <command> --help
```

| Command | Description |
|---------|-------------|
| `rustride --headless` | Start daemon in foreground |
| `rustride daemon start` | Start daemon in background |
| `rustride daemon stop` | Stop the daemon |
| `rustride-cli status` | Show daemon and session status |
| `rustride-cli sensors list` | List discovered sensors |
| `rustride-cli sensors connect <id>` | Connect to a sensor |
| `rustride-cli ride start` | Start free ride |
| `rustride-cli ride stop` | Stop and save ride |
| `rustride-cli workout start <file>` | Start workout |
| `rustride-cli workout pause/resume/skip/stop` | Control workout |
| `rustride-cli ride export` | Export ride to file |
