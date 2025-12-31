# Bluetooth Trainer Detection Fix

## Problem
The application was unable to detect smart trainers via Bluetooth due to several critical bugs:

1. **SensorManager never initialized** - The `SensorManager::initialize()` method was never called, so the BLE adapter was never set up
2. **No async runtime** - BLE operations require a tokio runtime, but the app didn't have one
3. **UI not wired** - The SensorSetupScreen had `// TODO` comments where it should trigger scanning

## Solution

### Changes Made

#### 1. Added Tokio Runtime (`src/app.rs`)
- Added `tokio::runtime::Runtime` to the `RustRideApp` struct
- Created runtime during app initialization to handle async BLE operations
- Wrapped `SensorManager` in `Arc<TokioMutex<>>` for async access

#### 2. Initialize SensorManager (`src/app.rs` lines 178-206)
```rust
// Create tokio runtime for async operations (BLE, ANT+, etc.)
let tokio_runtime = Arc::new(
    tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime for async operations"),
);

// Initialize BLE adapter asynchronously
let rt = tokio_runtime.clone();
let init_result = rt.block_on(async {
    sensor_manager.initialize().await
});

if let Err(e) = init_result {
    tracing::error!("Failed to initialize BLE adapter: {}", e);
    tracing::info!("Please check that Bluetooth is enabled...");
} else {
    tracing::info!("BLE adapter initialized successfully");
    // Also initialize ANT+ support (optional)
    rt.block_on(async { sensor_manager.initialize_ant().await });
}
```

#### 3. Added Sensor Discovery Methods (`src/app.rs` lines 332-394)
- `start_sensor_discovery()` - Spawns async task to start BLE/ANT+ scanning
- `stop_sensor_discovery()` - Stops scanning
- `connect_to_sensor()` - Connects to a discovered sensor
- `disconnect_from_sensor()` - Disconnects from a sensor

#### 4. Wire UI to Sensor Manager (`src/app.rs` lines 931-948)
Connected the SensorSetupScreen to actually trigger scanning when the user clicks "Start Scanning":

```rust
Screen::SensorSetup => {
    let was_scanning = self.sensor_setup_screen.is_scanning;

    if let Some(next) = self.sensor_setup_screen.show(ui) {
        self.navigate(next);
    }

    // Detect scanning state changes and trigger actual BLE scan
    let is_scanning_now = self.sensor_setup_screen.is_scanning;
    if !was_scanning && is_scanning_now {
        self.start_sensor_discovery();  // NEW!
    } else if was_scanning && !is_scanning_now {
        self.stop_sensor_discovery();   // NEW!
    }
}
```

## Testing

### System Requirements
Before running the app, ensure Bluetooth development libraries are installed:

**Ubuntu/Debian:**
```bash
sudo apt install libdbus-1-dev pkg-config libudev-dev
```

**Fedora:**
```bash
sudo dnf install dbus-devel pkgconf-pkg-config systemd-devel
```

**macOS:**
```bash
# No additional dependencies needed - uses CoreBluetooth
```

**Windows:**
```bash
# No additional dependencies needed - uses WinRT
```

### Build and Run
```bash
cargo build --release
cargo run --release
```

### Verify Bluetooth Works
1. Launch the app
2. Navigate to "Sensor Setup" screen
3. Click "Start Scanning"
4. Check logs for `BLE adapter initialized successfully`
5. Check logs for `Sensor discovery started`
6. Discovered sensors should appear in the list

### Troubleshooting

#### "Failed to initialize BLE adapter: AdapterNotFound"
- **Cause**: No Bluetooth adapter detected
- **Fix**: Ensure Bluetooth is enabled on your system
  ```bash
  # Linux
  sudo systemctl status bluetooth
  sudo systemctl start bluetooth

  # Check adapter status
  hciconfig
  bluetoothctl power on
  ```

#### "Failed to initialize BLE adapter: PermissionDenied"
- **Cause**: App doesn't have permission to access Bluetooth
- **Fix Linux**: Add user to bluetooth group
  ```bash
  sudo usermod -a -G bluetooth $USER
  # Then log out and log back in
  ```

#### No sensors discovered
- **Cause**: Trainer not in pairing mode or too far away
- **Fix**:
  1. Put trainer in pairing mode (usually by waking it up)
  2. Ensure trainer is within 10 feet / 3 meters
  3. Disconnect trainer from other devices (Zwift, TrainerRoad, etc.)
  4. Check trainer battery level

## Architecture Notes

### Async/Sync Bridge
The app uses `egui` (synchronous) for UI and `btleplug` (async) for Bluetooth:

```
┌─────────────────────┐
│   egui UI (sync)    │
│  SensorSetupScreen  │
└──────────┬──────────┘
           │ Button clicks
           ▼
┌─────────────────────┐
│  RustRideApp        │
│  (sync methods)     │
└──────────┬──────────┘
           │ spawn async task
           ▼
┌─────────────────────┐
│  Tokio Runtime      │
│  (async executor)   │
└──────────┬──────────┘
           │ await
           ▼
┌─────────────────────┐
│  SensorManager      │
│  (async methods)    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  btleplug BLE API   │
│  (async)            │
└─────────────────────┘
```

### Event Flow

1. **Scanning**: `UI button click` → `start_sensor_discovery()` → `tokio::spawn` → `SensorManager::start_discovery()` → `btleplug::start_scan()`
2. **Discovery**: `BLE device found` → `SensorEvent::Discovered` → `crossbeam channel` → `process_sensor_events()` → `UI update`
3. **Connection**: `UI connect click` → `connect_to_sensor()` → `SensorManager::connect()` → `SensorEvent::ConnectionChanged` → `UI update`

## Related Files
- `src/app.rs` - Main application, sensor manager integration
- `src/sensors/manager.rs` - BLE/ANT+ sensor discovery and connection
- `src/sensors/ftms.rs` - FTMS protocol implementation
- `src/ui/screens/sensor_setup.rs` - Sensor setup UI screen

## References
- btleplug documentation: https://github.com/deviceplug/btleplug
- FTMS specification: https://www.bluetooth.com/specifications/specs/fitness-machine-service-1-0/
- BLE security: https://www.bluetooth.com/blog/bluetooth-pairing-part-1-pairing-feature-exchange/
