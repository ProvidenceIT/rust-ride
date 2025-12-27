# Contract: HID Module

**Module**: `src/hid/`
**Feature**: Hardware Integration
**Date**: 2025-12-26

This contract defines the USB HID device integration API for Stream Deck and USB button devices.

---

## Device Manager (`src/hid/device.rs`)

```rust
/// USB HID device detection and management
pub trait HidDeviceManager: Send + Sync {
    /// Scan for connected HID devices
    fn scan_devices(&self) -> Vec<HidDevice>;

    /// Start monitoring for device connect/disconnect
    async fn start_monitoring(&self) -> Result<(), HidError>;

    /// Stop monitoring
    fn stop_monitoring(&self);

    /// Get device by ID
    fn get_device(&self, device_id: &Uuid) -> Option<HidDevice>;

    /// Open device for input
    async fn open_device(&self, device_id: &Uuid) -> Result<(), HidError>;

    /// Close device
    async fn close_device(&self, device_id: &Uuid) -> Result<(), HidError>;

    /// Check if device is open
    fn is_open(&self, device_id: &Uuid) -> bool;

    /// Enable/disable device
    fn set_enabled(&self, device_id: &Uuid, enabled: bool);

    /// Subscribe to device events
    fn subscribe_events(&self) -> broadcast::Receiver<HidDeviceEvent>;
}

pub enum HidDeviceEvent {
    DeviceConnected(HidDevice),
    DeviceDisconnected(Uuid),
    DeviceOpened(Uuid),
    DeviceClosed(Uuid),
    Error { device_id: Option<Uuid>, error: String },
}
```

---

## Button Input Handler (`src/hid/mapping.rs`)

```rust
/// Handle button input and map to actions
pub trait ButtonInputHandler: Send + Sync {
    /// Register button mappings for a device
    fn register_mappings(&self, device_id: &Uuid, mappings: Vec<ButtonMapping>);

    /// Get mappings for a device
    fn get_mappings(&self, device_id: &Uuid) -> Vec<ButtonMapping>;

    /// Add a single mapping
    fn add_mapping(&self, device_id: &Uuid, mapping: ButtonMapping);

    /// Remove a mapping
    fn remove_mapping(&self, mapping_id: &Uuid);

    /// Update a mapping
    fn update_mapping(&self, mapping_id: &Uuid, new_action: ButtonAction);

    /// Clear all mappings for a device
    fn clear_mappings(&self, device_id: &Uuid);

    /// Subscribe to button press events (after mapping)
    fn subscribe_actions(&self) -> broadcast::Receiver<ButtonActionEvent>;

    /// Subscribe to raw button events (for mapping UI)
    fn subscribe_raw(&self) -> broadcast::Receiver<RawButtonEvent>;

    /// Start listening mode (for mapping new buttons)
    fn start_learning_mode(&self, device_id: &Uuid);

    /// Stop learning mode
    fn stop_learning_mode(&self);

    /// Check if in learning mode
    fn is_learning(&self) -> bool;
}

pub struct RawButtonEvent {
    pub device_id: Uuid,
    pub button_code: u8,
    pub pressed: bool,
    pub timestamp: Instant,
}

pub struct ButtonActionEvent {
    pub device_id: Uuid,
    pub mapping_id: Uuid,
    pub action: ButtonAction,
    pub timestamp: Instant,
}
```

---

## Action Executor (`src/hid/actions.rs`)

```rust
/// Execute button-triggered actions
pub trait ActionExecutor: Send + Sync {
    /// Execute an action
    async fn execute(&self, action: &ButtonAction) -> Result<(), ActionError>;

    /// Get list of available actions
    fn available_actions() -> Vec<ActionInfo>;

    /// Check if action is available in current context
    fn is_available(&self, action: &ButtonAction) -> bool;

    /// Subscribe to action execution results
    fn subscribe_results(&self) -> broadcast::Receiver<ActionResult>;
}

pub struct ActionInfo {
    pub action: ButtonAction,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub available_during: ActionContext,
}

pub enum ActionContext {
    Always,
    DuringRide,
    DuringWorkout,
    NotDuringRide,
}

pub struct ActionResult {
    pub action: ButtonAction,
    pub success: bool,
    pub message: Option<String>,
    pub timestamp: Instant,
}
```

---

## Button Actions

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonAction {
    // Ride control
    AddLapMarker,
    PauseResume,
    EndRide,

    // Workout control
    SkipInterval,
    ExtendInterval { seconds: u32 },
    RestartInterval,

    // Audio control
    VolumeUp,
    VolumeDown,
    MuteToggle,

    // Fan control (if MQTT enabled)
    FanSpeedUp,
    FanSpeedDown,
    FanToggle,

    // UI navigation
    ShowMetrics,
    ShowMap,
    ShowWorkout,
    ToggleFullscreen,

    // Camera (if 3D world enabled)
    CameraZoomIn,
    CameraZoomOut,
    CameraRotate { degrees: i16 },

    // Custom action
    Custom { command: String },
}

impl ButtonAction {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AddLapMarker => "Add Lap Marker",
            Self::PauseResume => "Pause/Resume",
            Self::EndRide => "End Ride",
            Self::SkipInterval => "Skip Interval",
            Self::ExtendInterval { .. } => "Extend Interval",
            Self::RestartInterval => "Restart Interval",
            Self::VolumeUp => "Volume Up",
            Self::VolumeDown => "Volume Down",
            Self::MuteToggle => "Mute/Unmute",
            Self::FanSpeedUp => "Fan Speed Up",
            Self::FanSpeedDown => "Fan Speed Down",
            Self::FanToggle => "Fan On/Off",
            Self::ShowMetrics => "Show Metrics",
            Self::ShowMap => "Show Map",
            Self::ShowWorkout => "Show Workout",
            Self::ToggleFullscreen => "Toggle Fullscreen",
            Self::CameraZoomIn => "Zoom In",
            Self::CameraZoomOut => "Zoom Out",
            Self::CameraRotate { .. } => "Rotate Camera",
            Self::Custom { .. } => "Custom Action",
        }
    }

    /// Get action category
    pub fn category(&self) -> ActionCategory {
        match self {
            Self::AddLapMarker | Self::PauseResume | Self::EndRide => ActionCategory::RideControl,
            Self::SkipInterval | Self::ExtendInterval { .. } | Self::RestartInterval => ActionCategory::WorkoutControl,
            Self::VolumeUp | Self::VolumeDown | Self::MuteToggle => ActionCategory::Audio,
            Self::FanSpeedUp | Self::FanSpeedDown | Self::FanToggle => ActionCategory::Fan,
            Self::ShowMetrics | Self::ShowMap | Self::ShowWorkout | Self::ToggleFullscreen => ActionCategory::Navigation,
            Self::CameraZoomIn | Self::CameraZoomOut | Self::CameraRotate { .. } => ActionCategory::Camera,
            Self::Custom { .. } => ActionCategory::Custom,
        }
    }
}

pub enum ActionCategory {
    RideControl,
    WorkoutControl,
    Audio,
    Fan,
    Navigation,
    Camera,
    Custom,
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum HidError {
    #[error("Device not found: {0}")]
    DeviceNotFound(Uuid),

    #[error("Device not open")]
    DeviceNotOpen,

    #[error("Failed to open device: {0}")]
    OpenFailed(String),

    #[error("Read error: {0}")]
    ReadError(String),

    #[error("HID API error: {0}")]
    HidApiError(#[from] hidapi::HidError),

    #[error("Device already in use")]
    DeviceInUse,
}

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("Action not available: {0}")]
    NotAvailable(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("No active ride")]
    NoActiveRide,

    #[error("No active workout")]
    NoActiveWorkout,
}
```

---

## Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidConfig {
    pub enabled: bool,
    pub devices: Vec<HidDeviceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidDeviceConfig {
    pub device_id: Uuid,
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: String,
    pub enabled: bool,
    pub mappings: Vec<ButtonMappingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMappingConfig {
    pub button_code: u8,
    pub action: ButtonAction,
    pub label: Option<String>,
}
```

---

## Supported Devices

Known device profiles for automatic configuration:

```rust
pub struct KnownDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: &'static str,
    pub button_count: u8,
    pub default_mappings: &'static [(u8, ButtonAction)],
}

pub const KNOWN_DEVICES: &[KnownDevice] = &[
    KnownDevice {
        vendor_id: 0x0FD9,  // Elgato
        product_id: 0x0060, // Stream Deck
        name: "Elgato Stream Deck",
        button_count: 15,
        default_mappings: &[],
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x006C, // Stream Deck Mini
        name: "Elgato Stream Deck Mini",
        button_count: 6,
        default_mappings: &[],
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x006D, // Stream Deck XL
        name: "Elgato Stream Deck XL",
        button_count: 32,
        default_mappings: &[],
    },
    // Generic USB keypad/footswitch support
];
```
