//! HID Input Reading
//!
//! Background task for reading input reports from HID devices
//! and translating them to button events.

use super::generic::{
    detect_report_format, find_generic_device_profile, GenericDeviceConfig, GenericHidParser,
    GenericReportFormat,
};
use super::mapping::{DefaultButtonInputHandler, RawButtonEvent};
use super::streamdeck::{StreamDeckModel, StreamDeckParser};
use super::{find_known_device, HidDeviceEvent, HidError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Read timeout for HID devices in milliseconds
/// Short timeout allows responsive checking of multiple devices
const READ_TIMEOUT_MS: i32 = 50;

/// Polling interval between read cycles
const POLL_INTERVAL_MS: u64 = 10;

/// Device type for protocol selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidDeviceType {
    /// Elgato Stream Deck (various models)
    StreamDeck,
    /// Stream Deck Pedal (foot controller)
    StreamDeckPedal,
    /// Generic HID gamepad/button device
    Generic,
}

impl HidDeviceType {
    /// Determine device type from vendor/product IDs
    pub fn from_ids(vendor_id: u16, product_id: u16) -> Self {
        // Elgato vendor ID
        if vendor_id == 0x0FD9 {
            match product_id {
                // Stream Deck Pedal
                0x0086 => HidDeviceType::StreamDeckPedal,
                // All other Elgato Stream Deck variants
                0x0060 | 0x006C | 0x006D | 0x0080 => HidDeviceType::StreamDeck,
                _ => HidDeviceType::Generic,
            }
        } else {
            HidDeviceType::Generic
        }
    }
}

/// Information about an open device for input reading
#[derive(Debug, Clone)]
pub struct OpenDeviceInfo {
    /// Device UUID
    pub id: Uuid,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device type for protocol selection
    pub device_type: HidDeviceType,
    /// Number of buttons
    pub button_count: u8,
    /// Previous button states for press/release detection (for generic devices)
    pub previous_states: Vec<bool>,
    /// Stream Deck model (if this is a Stream Deck device)
    pub stream_deck_model: Option<StreamDeckModel>,
    /// Generic device configuration (if this is a generic device)
    pub generic_config: Option<GenericDeviceConfig>,
    /// Cached report format for auto-detection
    pub detected_format: Option<GenericReportFormat>,
}

impl OpenDeviceInfo {
    /// Create new device info
    pub fn new(id: Uuid, vendor_id: u16, product_id: u16) -> Self {
        let device_type = HidDeviceType::from_ids(vendor_id, product_id);

        // Try to detect Stream Deck model
        let stream_deck_model = StreamDeckModel::from_product_id(product_id)
            .filter(|_| vendor_id == super::streamdeck::ELGATO_VENDOR_ID);

        // Try to find known generic device profile
        let generic_profile = find_generic_device_profile(vendor_id, product_id);

        // Get button count from Stream Deck model, known generic profile, or known devices
        let button_count = stream_deck_model
            .map(|m| m.button_count())
            .or_else(|| generic_profile.map(|p| p.config.button_count))
            .or_else(|| find_known_device(vendor_id, product_id).map(|d| d.button_count))
            .unwrap_or(32); // Default to 32 buttons for unknown devices

        // Get generic device configuration if available
        let generic_config = if device_type == HidDeviceType::Generic {
            generic_profile
                .map(|p| p.config.clone())
                .or_else(|| Some(GenericDeviceConfig::gamepad(button_count)))
        } else {
            None
        };

        Self {
            id,
            vendor_id,
            product_id,
            device_type,
            button_count,
            previous_states: vec![false; button_count as usize],
            stream_deck_model,
            generic_config,
            detected_format: None,
        }
    }

    /// Set a custom generic device configuration
    pub fn with_generic_config(mut self, config: GenericDeviceConfig) -> Self {
        self.generic_config = Some(config);
        self.button_count = config.button_count;
        self.previous_states = vec![false; config.button_count as usize];
        self
    }

    /// Set the detected report format
    pub fn with_detected_format(mut self, format: GenericReportFormat) -> Self {
        self.detected_format = Some(format);
        if let Some(ref mut config) = self.generic_config {
            config.format = format;
        }
        self
    }
}

/// HID Input Reader
///
/// Manages background reading of input reports from open HID devices.
pub struct HidInputReader {
    /// Reference to device handles (shared with device manager)
    device_handles: Arc<Mutex<HashMap<Uuid, hidapi::HidDevice>>>,
    /// Information about open devices
    open_devices: Arc<RwLock<HashMap<Uuid, OpenDeviceInfo>>>,
    /// Sender for raw button events
    event_tx: broadcast::Sender<RawButtonEvent>,
    /// Whether the reader is running
    is_running: Arc<RwLock<bool>>,
    /// Receiver for device events
    device_event_rx: Option<broadcast::Receiver<HidDeviceEvent>>,
    /// Button input handler for processing events and mapping to actions
    button_handler: Option<Arc<DefaultButtonInputHandler>>,
}

impl HidInputReader {
    /// Create a new HID input reader
    ///
    /// # Arguments
    /// * `device_handles` - Shared reference to device handles from device manager
    pub fn new(device_handles: Arc<Mutex<HashMap<Uuid, hidapi::HidDevice>>>) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            device_handles,
            open_devices: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            is_running: Arc::new(RwLock::new(false)),
            device_event_rx: None,
            button_handler: None,
        }
    }

    /// Set the device event receiver for tracking device opens/closes
    pub fn with_device_events(mut self, rx: broadcast::Receiver<HidDeviceEvent>) -> Self {
        self.device_event_rx = Some(rx);
        self
    }

    /// Set the button input handler for processing events and mapping to actions
    ///
    /// When set, all raw button events will be forwarded to the handler for:
    /// - Button mapping lookup and action emission
    /// - Learning mode button capture
    pub fn with_button_handler(mut self, handler: Arc<DefaultButtonInputHandler>) -> Self {
        self.button_handler = Some(handler);
        self
    }

    /// Get a reference to the button handler if set
    pub fn button_handler(&self) -> Option<&Arc<DefaultButtonInputHandler>> {
        self.button_handler.as_ref()
    }

    /// Subscribe to raw button events
    pub fn subscribe(&self) -> broadcast::Receiver<RawButtonEvent> {
        self.event_tx.subscribe()
    }

    /// Register a device for input reading
    pub async fn register_device(&self, id: Uuid, vendor_id: u16, product_id: u16) {
        let info = OpenDeviceInfo::new(id, vendor_id, product_id);
        tracing::info!(
            "Registered device {} for input reading (type: {:?}, {} buttons)",
            id,
            info.device_type,
            info.button_count
        );
        self.open_devices.write().await.insert(id, info);
    }

    /// Unregister a device from input reading
    pub async fn unregister_device(&self, id: &Uuid) {
        if self.open_devices.write().await.remove(id).is_some() {
            tracing::info!("Unregistered device {} from input reading", id);
        }
    }

    /// Check if the reader is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Start the input reading background task
    pub async fn start(&self) -> Result<(), HidError> {
        // Check if already running
        {
            let is_running = self.is_running.read().await;
            if *is_running {
                tracing::warn!("HID input reader already running");
                return Ok(());
            }
        }

        *self.is_running.write().await = true;
        tracing::info!("Starting HID input reader");

        // Clone references for the background task
        let is_running = Arc::clone(&self.is_running);
        let device_handles = Arc::clone(&self.device_handles);
        let open_devices = Arc::clone(&self.open_devices);
        let event_tx = self.event_tx.clone();
        let button_handler = self.button_handler.clone();

        // Spawn the input reading background task
        tokio::spawn(async move {
            let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);
            let mut interval = tokio::time::interval(poll_interval);

            // Buffer for reading HID reports
            let read_buffer_size = 64; // Most HID reports are under 64 bytes

            loop {
                interval.tick().await;

                // Check if we should stop
                {
                    if let Ok(running) = is_running.try_read() {
                        if !*running {
                            tracing::info!("HID input reader stopped");
                            break;
                        }
                    }
                }

                // Get list of device IDs to read from
                let device_ids: Vec<Uuid> = {
                    if let Ok(devices) = open_devices.try_read() {
                        devices.keys().cloned().collect()
                    } else {
                        continue;
                    }
                };

                // Read from each device
                for device_id in device_ids {
                    // Clone values needed for the blocking task
                    let handles = Arc::clone(&device_handles);
                    let devices = Arc::clone(&open_devices);
                    let tx = event_tx.clone();

                    // Perform the read in a blocking task since hidapi is sync
                    let read_result = tokio::task::spawn_blocking(move || {
                        let mut events = Vec::new();

                        // Lock handles and read
                        if let Ok(mut handles_guard) = handles.lock() {
                            if let Some(handle) = handles_guard.get_mut(&device_id) {
                                let mut buffer = vec![0u8; read_buffer_size];

                                match handle.read_timeout(&mut buffer, READ_TIMEOUT_MS) {
                                    Ok(0) => {
                                        // No data available (timeout)
                                    }
                                    Ok(bytes_read) => {
                                        // Got data - parse it
                                        events = parse_hid_report(
                                            &device_id,
                                            &buffer[..bytes_read],
                                            &devices,
                                        );
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Read error on device {}: {}",
                                            device_id,
                                            e
                                        );
                                        // Don't spam logs - device may have disconnected
                                    }
                                }
                            }
                        }

                        events
                    })
                    .await;

                    // Send any events that were generated
                    if let Ok(events) = read_result {
                        for event in events {
                            // Send to broadcast channel for other subscribers
                            let _ = tx.send(event.clone());

                            // Forward to button handler for mapping and action emission
                            if let Some(ref handler) = button_handler {
                                handler.process_event(event).await;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the input reading background task
    pub async fn stop(&self) {
        *self.is_running.write().await = false;
        tracing::info!("Signaled HID input reader to stop");
    }
}

/// Parse a HID report and generate button events
///
/// This is a basic implementation that handles the most common case.
/// Device-specific parsing (Stream Deck, etc.) is handled in subtask 2.2/2.3.
fn parse_hid_report(
    device_id: &Uuid,
    report: &[u8],
    open_devices: &Arc<RwLock<HashMap<Uuid, OpenDeviceInfo>>>,
) -> Vec<RawButtonEvent> {
    let mut events = Vec::new();
    let timestamp = Instant::now();

    // Try to get device info for state tracking
    let device_info = {
        if let Ok(mut devices) = open_devices.try_write() {
            devices.get_mut(device_id).cloned()
        } else {
            return events;
        }
    };

    let Some(mut device_info) = device_info else {
        return events;
    };

    // Determine parsing based on device type
    match device_info.device_type {
        HidDeviceType::StreamDeck => {
            // Stream Deck button report format (to be fully implemented in 2.2)
            // First byte is report ID (0x01 for button state)
            if report.first() == Some(&0x01) && report.len() > 1 {
                events = parse_streamdeck_report(device_id, report, &mut device_info, timestamp);
            }
        }
        HidDeviceType::StreamDeckPedal => {
            // Stream Deck Pedal - 3 buttons (to be fully implemented in 2.2)
            if !report.is_empty() {
                events = parse_pedal_report(device_id, report, &mut device_info, timestamp);
            }
        }
        HidDeviceType::Generic => {
            // Generic HID button device (to be fully implemented in 2.3)
            events = parse_generic_report(device_id, report, &mut device_info, timestamp);
        }
    }

    // Update stored state
    if !events.is_empty() {
        if let Ok(mut devices) = open_devices.try_write() {
            if let Some(info) = devices.get_mut(device_id) {
                info.previous_states = device_info.previous_states;
            }
        }
    }

    events
}

/// Parse Stream Deck button report
///
/// Stream Deck reports button states as a byte array where each byte
/// represents a button (1 = pressed, 0 = not pressed).
///
/// Report format varies by model:
/// - Original/Mini: [report_id, button0, button1, ..., buttonN]
/// - MK.2/XL: [report_id, extra1, extra2, extra3, button0, button1, ...]
fn parse_streamdeck_report(
    device_id: &Uuid,
    report: &[u8],
    device_info: &mut OpenDeviceInfo,
    timestamp: Instant,
) -> Vec<RawButtonEvent> {
    let mut events = Vec::new();

    // Verify minimum report size
    if report.is_empty() {
        return events;
    }

    // Determine button data offset based on model
    let button_data_offset = match device_info.stream_deck_model {
        Some(StreamDeckModel::Mk2) | Some(StreamDeckModel::Xl) => {
            // New format: [report_id, 0x00, extra, extra, button_states...]
            4
        }
        _ => {
            // Standard format: [report_id, button_states...]
            1
        }
    };

    // Ensure report has enough data
    if report.len() <= button_data_offset {
        return events;
    }

    let button_data = &report[button_data_offset..];

    for (index, &button_byte) in button_data.iter().enumerate() {
        if index >= device_info.button_count as usize {
            break;
        }

        let is_pressed = button_byte != 0;
        let was_pressed = device_info
            .previous_states
            .get(index)
            .copied()
            .unwrap_or(false);

        // Generate event on state change
        if is_pressed != was_pressed {
            events.push(RawButtonEvent {
                device_id: *device_id,
                button_code: index as u8,
                pressed: is_pressed,
                timestamp,
            });

            let model_name = device_info
                .stream_deck_model
                .map(|m| m.name())
                .unwrap_or("Stream Deck");

            tracing::debug!(
                "{} button {} {}",
                model_name,
                index,
                if is_pressed { "pressed" } else { "released" }
            );
        }

        // Update state
        if index < device_info.previous_states.len() {
            device_info.previous_states[index] = is_pressed;
        }
    }

    events
}

/// Parse Stream Deck Pedal report
///
/// The pedal has 3 buttons and uses a simple bitmap format.
fn parse_pedal_report(
    device_id: &Uuid,
    report: &[u8],
    device_info: &mut OpenDeviceInfo,
    timestamp: Instant,
) -> Vec<RawButtonEvent> {
    let mut events = Vec::new();

    // Pedal uses first few bytes for 3 button states
    for (index, &button_byte) in report.iter().take(3).enumerate() {
        if index >= device_info.button_count as usize {
            break;
        }

        let is_pressed = button_byte != 0;
        let was_pressed = device_info
            .previous_states
            .get(index)
            .copied()
            .unwrap_or(false);

        if is_pressed != was_pressed {
            events.push(RawButtonEvent {
                device_id: *device_id,
                button_code: index as u8,
                pressed: is_pressed,
                timestamp,
            });

            tracing::debug!(
                "Stream Deck Pedal button {} {}",
                index,
                if is_pressed { "pressed" } else { "released" }
            );
        }

        if index < device_info.previous_states.len() {
            device_info.previous_states[index] = is_pressed;
        }
    }

    events
}

/// Parse generic HID button report
///
/// Handles common HID gamepad/button formats including:
/// - Bitmap encoding (gamepad style)
/// - Byte-per-button encoding
/// - Keyboard scan codes
/// - Consumer control (media keys)
fn parse_generic_report(
    device_id: &Uuid,
    report: &[u8],
    device_info: &mut OpenDeviceInfo,
    timestamp: Instant,
) -> Vec<RawButtonEvent> {
    // Get or create parser configuration
    let config = device_info
        .generic_config
        .clone()
        .unwrap_or_else(|| GenericDeviceConfig::gamepad(device_info.button_count));

    // Auto-detect format if not yet determined and first report received
    let format = if device_info.detected_format.is_none() && !report.is_empty() {
        let detected = detect_report_format(report);
        if detected != GenericReportFormat::Unknown {
            tracing::debug!(
                "Auto-detected report format for device {:04X}:{:04X}: {:?}",
                device_info.vendor_id,
                device_info.product_id,
                detected
            );
            device_info.detected_format = Some(detected);
            detected
        } else {
            config.format
        }
    } else {
        device_info.detected_format.unwrap_or(config.format)
    };

    // Use the detected format for parsing
    let effective_config = GenericDeviceConfig {
        format,
        button_count: device_info.button_count,
        button_data_offset: config.button_data_offset,
        has_report_id: config.has_report_id,
        expected_report_id: config.expected_report_id,
    };

    // Create a temporary parser with the device's previous state
    let mut parser = GenericHidParser::new(effective_config);

    // Restore previous key states for keyboard mode
    // For other modes, we track state via device_info.previous_states

    // Parse the report
    let events = match format {
        GenericReportFormat::Bitmap => {
            parse_generic_bitmap(device_id, report, device_info, timestamp)
        }
        GenericReportFormat::BytePerButton => {
            parse_generic_byte_per_button(device_id, report, device_info, timestamp)
        }
        GenericReportFormat::KeyboardScanCode => {
            parser.parse_report(device_id, report, timestamp)
        }
        GenericReportFormat::ConsumerControl => {
            parser.parse_report(device_id, report, timestamp)
        }
        GenericReportFormat::Unknown => {
            // Fall back to bitmap parsing
            parse_generic_bitmap(device_id, report, device_info, timestamp)
        }
    };

    events
}

/// Parse bitmap-encoded button report
///
/// Each bit represents a button state.
fn parse_generic_bitmap(
    device_id: &Uuid,
    report: &[u8],
    device_info: &mut OpenDeviceInfo,
    timestamp: Instant,
) -> Vec<RawButtonEvent> {
    let mut events = Vec::new();
    let mut button_index = 0;

    for &byte in report.iter() {
        for bit in 0..8 {
            if button_index >= device_info.button_count as usize {
                break;
            }

            let is_pressed = (byte >> bit) & 1 != 0;
            let was_pressed = device_info
                .previous_states
                .get(button_index)
                .copied()
                .unwrap_or(false);

            if is_pressed != was_pressed {
                events.push(RawButtonEvent {
                    device_id: *device_id,
                    button_code: button_index as u8,
                    pressed: is_pressed,
                    timestamp,
                });

                tracing::debug!(
                    "Generic button {} {}",
                    button_index,
                    if is_pressed { "pressed" } else { "released" }
                );
            }

            if button_index < device_info.previous_states.len() {
                device_info.previous_states[button_index] = is_pressed;
            }

            button_index += 1;
        }
    }

    events
}

/// Parse byte-per-button report
///
/// Each byte represents a button: 0 = released, non-zero = pressed
fn parse_generic_byte_per_button(
    device_id: &Uuid,
    report: &[u8],
    device_info: &mut OpenDeviceInfo,
    timestamp: Instant,
) -> Vec<RawButtonEvent> {
    let mut events = Vec::new();

    for (index, &button_byte) in report.iter().enumerate() {
        if index >= device_info.button_count as usize {
            break;
        }

        let is_pressed = button_byte != 0;
        let was_pressed = device_info
            .previous_states
            .get(index)
            .copied()
            .unwrap_or(false);

        if is_pressed != was_pressed {
            events.push(RawButtonEvent {
                device_id: *device_id,
                button_code: index as u8,
                pressed: is_pressed,
                timestamp,
            });

            tracing::debug!(
                "Generic button {} {} (value: {})",
                index,
                if is_pressed { "pressed" } else { "released" },
                button_byte
            );
        }

        if index < device_info.previous_states.len() {
            device_info.previous_states[index] = is_pressed;
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_detection() {
        // Stream Deck Original
        assert_eq!(
            HidDeviceType::from_ids(0x0FD9, 0x0060),
            HidDeviceType::StreamDeck
        );

        // Stream Deck Mini
        assert_eq!(
            HidDeviceType::from_ids(0x0FD9, 0x006C),
            HidDeviceType::StreamDeck
        );

        // Stream Deck XL
        assert_eq!(
            HidDeviceType::from_ids(0x0FD9, 0x006D),
            HidDeviceType::StreamDeck
        );

        // Stream Deck MK.2
        assert_eq!(
            HidDeviceType::from_ids(0x0FD9, 0x0080),
            HidDeviceType::StreamDeck
        );

        // Stream Deck Pedal
        assert_eq!(
            HidDeviceType::from_ids(0x0FD9, 0x0086),
            HidDeviceType::StreamDeckPedal
        );

        // Unknown Elgato device
        assert_eq!(
            HidDeviceType::from_ids(0x0FD9, 0x9999),
            HidDeviceType::Generic
        );

        // Completely unknown device
        assert_eq!(
            HidDeviceType::from_ids(0x1234, 0x5678),
            HidDeviceType::Generic
        );
    }

    #[test]
    fn test_open_device_info_creation() {
        let id = Uuid::new_v4();
        let info = OpenDeviceInfo::new(id, 0x0FD9, 0x0060);

        assert_eq!(info.id, id);
        assert_eq!(info.vendor_id, 0x0FD9);
        assert_eq!(info.product_id, 0x0060);
        assert_eq!(info.device_type, HidDeviceType::StreamDeck);
        assert_eq!(info.button_count, 15);
        assert_eq!(info.previous_states.len(), 15);
        assert!(info.previous_states.iter().all(|&s| !s));
        assert_eq!(info.stream_deck_model, Some(StreamDeckModel::Original));
    }

    #[test]
    fn test_pedal_device_info() {
        let id = Uuid::new_v4();
        let info = OpenDeviceInfo::new(id, 0x0FD9, 0x0086);

        assert_eq!(info.device_type, HidDeviceType::StreamDeckPedal);
        assert_eq!(info.button_count, 3);
        assert_eq!(info.previous_states.len(), 3);
        assert_eq!(info.stream_deck_model, Some(StreamDeckModel::Pedal));
    }

    #[test]
    fn test_unknown_device_defaults() {
        let id = Uuid::new_v4();
        let info = OpenDeviceInfo::new(id, 0x1234, 0x5678);

        assert_eq!(info.device_type, HidDeviceType::Generic);
        assert_eq!(info.button_count, 32); // Default for unknown
        assert_eq!(info.previous_states.len(), 32);
        assert_eq!(info.stream_deck_model, None);
    }

    #[test]
    fn test_stream_deck_model_detection() {
        // Stream Deck Mini
        let info = OpenDeviceInfo::new(Uuid::new_v4(), 0x0FD9, 0x006C);
        assert_eq!(info.stream_deck_model, Some(StreamDeckModel::Mini));
        assert_eq!(info.button_count, 6);

        // Stream Deck XL
        let info = OpenDeviceInfo::new(Uuid::new_v4(), 0x0FD9, 0x006D);
        assert_eq!(info.stream_deck_model, Some(StreamDeckModel::Xl));
        assert_eq!(info.button_count, 32);

        // Stream Deck MK.2
        let info = OpenDeviceInfo::new(Uuid::new_v4(), 0x0FD9, 0x0080);
        assert_eq!(info.stream_deck_model, Some(StreamDeckModel::Mk2));
        assert_eq!(info.button_count, 15);
    }

    #[tokio::test]
    async fn test_input_reader_creation() {
        let handles = Arc::new(Mutex::new(HashMap::new()));
        let reader = HidInputReader::new(handles);

        assert!(!reader.is_running().await);
    }

    #[tokio::test]
    async fn test_device_registration() {
        let handles = Arc::new(Mutex::new(HashMap::new()));
        let reader = HidInputReader::new(handles);

        let device_id = Uuid::new_v4();
        reader.register_device(device_id, 0x0FD9, 0x0060).await;

        // Verify device is registered
        let devices = reader.open_devices.read().await;
        assert!(devices.contains_key(&device_id));
        assert_eq!(devices.get(&device_id).unwrap().device_type, HidDeviceType::StreamDeck);
    }

    #[tokio::test]
    async fn test_device_unregistration() {
        let handles = Arc::new(Mutex::new(HashMap::new()));
        let reader = HidInputReader::new(handles);

        let device_id = Uuid::new_v4();
        reader.register_device(device_id, 0x0FD9, 0x0060).await;
        reader.unregister_device(&device_id).await;

        let devices = reader.open_devices.read().await;
        assert!(!devices.contains_key(&device_id));
    }

    #[test]
    fn test_streamdeck_report_parsing() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x0060);
        let timestamp = Instant::now();

        // Simulate button 0 pressed (report format: [report_id, button_states...])
        let report = [0x01, 0x01, 0x00, 0x00, 0x00, 0x00]; // Button 0 pressed

        let events = parse_streamdeck_report(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(events[0].pressed);

        // Simulate button 0 released
        let report2 = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        let events2 = parse_streamdeck_report(&device_id, &report2, &mut device_info, timestamp);

        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].button_code, 0);
        assert!(!events2[0].pressed);
    }

    #[test]
    fn test_streamdeck_multiple_buttons() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x0060);
        let timestamp = Instant::now();

        // Simulate buttons 0, 2, and 4 pressed
        let report = [0x01, 0x01, 0x00, 0x01, 0x00, 0x01];

        let events = parse_streamdeck_report(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 3);

        let button_codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(button_codes.contains(&0));
        assert!(button_codes.contains(&2));
        assert!(button_codes.contains(&4));
    }

    #[test]
    fn test_pedal_report_parsing() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x0086);
        let timestamp = Instant::now();

        // Simulate left pedal pressed
        let report = [0x01, 0x00, 0x00];

        let events = parse_pedal_report(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_generic_bitmap_parsing() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x1234, 0x5678);
        device_info.button_count = 8; // Limit to 8 buttons for this test
        device_info.previous_states = vec![false; 8];
        let timestamp = Instant::now();

        // Simulate buttons 0, 2, and 7 pressed (binary: 10000101 = 0x85)
        let report = [0x85];

        let events = parse_generic_report(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 3);

        let button_codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(button_codes.contains(&0));
        assert!(button_codes.contains(&2));
        assert!(button_codes.contains(&7));
    }

    #[test]
    fn test_no_event_when_no_change() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x0060);
        let timestamp = Instant::now();

        // First press
        let report = [0x01, 0x01, 0x00, 0x00];
        let _ = parse_streamdeck_report(&device_id, &report, &mut device_info, timestamp);

        // Same state again - should produce no events
        let events = parse_streamdeck_report(&device_id, &report, &mut device_info, timestamp);
        assert!(events.is_empty());
    }

    #[test]
    fn test_mk2_format_parsing() {
        let device_id = Uuid::new_v4();
        // Create MK.2 device info (uses 4-byte header)
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x0080);
        let timestamp = Instant::now();

        assert_eq!(device_info.stream_deck_model, Some(StreamDeckModel::Mk2));

        // MK.2 format: [report_id, 0x00, extra, extra, button_states...]
        // Button 0 pressed
        let report = [0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
        let events = parse_streamdeck_report(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_xl_format_parsing() {
        let device_id = Uuid::new_v4();
        // Create XL device info (uses 4-byte header, 32 buttons)
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x006D);
        let timestamp = Instant::now();

        assert_eq!(device_info.stream_deck_model, Some(StreamDeckModel::Xl));
        assert_eq!(device_info.button_count, 32);

        // XL format with buttons 0 and 31 pressed
        let mut report = vec![0x01, 0x00, 0x00, 0x00]; // Header
        report.resize(4 + 32, 0x00); // Add 32 button bytes
        report[4] = 0x01; // Button 0
        report[35] = 0x01; // Button 31

        let events = parse_streamdeck_report(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 2);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&31));
    }

    #[test]
    fn test_empty_report_handling() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x0FD9, 0x0060);
        let timestamp = Instant::now();

        // Empty report should return no events
        let events = parse_streamdeck_report(&device_id, &[], &mut device_info, timestamp);
        assert!(events.is_empty());

        // Report too short for button data
        let short_report = [0x01]; // Only report ID
        let events = parse_streamdeck_report(&device_id, &short_report, &mut device_info, timestamp);
        assert!(events.is_empty());
    }

    #[test]
    fn test_generic_device_info_creation() {
        let id = Uuid::new_v4();
        let info = OpenDeviceInfo::new(id, 0x1234, 0x5678);

        assert_eq!(info.device_type, HidDeviceType::Generic);
        assert_eq!(info.button_count, 32); // Default
        assert!(info.generic_config.is_some());
        assert!(info.stream_deck_model.is_none());
        assert!(info.detected_format.is_none());
    }

    #[test]
    fn test_known_generic_device_profile() {
        let id = Uuid::new_v4();
        // VEC USB Foot Pedal (known generic device)
        let info = OpenDeviceInfo::new(id, 0x05F3, 0x00FF);

        assert_eq!(info.device_type, HidDeviceType::Generic);
        assert_eq!(info.button_count, 3);
        assert!(info.generic_config.is_some());
        assert_eq!(
            info.generic_config.as_ref().unwrap().format,
            GenericReportFormat::KeyboardScanCode
        );
    }

    #[test]
    fn test_with_generic_config() {
        let id = Uuid::new_v4();
        let info = OpenDeviceInfo::new(id, 0x1234, 0x5678)
            .with_generic_config(GenericDeviceConfig::keyboard());

        assert_eq!(info.button_count, 128);
        assert_eq!(
            info.generic_config.as_ref().unwrap().format,
            GenericReportFormat::KeyboardScanCode
        );
    }

    #[test]
    fn test_with_detected_format() {
        let id = Uuid::new_v4();
        let info = OpenDeviceInfo::new(id, 0x1234, 0x5678)
            .with_detected_format(GenericReportFormat::BytePerButton);

        assert_eq!(info.detected_format, Some(GenericReportFormat::BytePerButton));
        assert_eq!(
            info.generic_config.as_ref().unwrap().format,
            GenericReportFormat::BytePerButton
        );
    }

    #[test]
    fn test_generic_bitmap_parsing_direct() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x1234, 0x5678);
        device_info.button_count = 8; // Limit to 8 buttons for this test
        device_info.previous_states = vec![false; 8];
        let timestamp = Instant::now();

        // Buttons 0 and 2 pressed (binary: 00000101 = 0x05)
        let report = [0x05];

        let events = parse_generic_bitmap(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 2);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&2));
    }

    #[test]
    fn test_generic_byte_per_button_parsing() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x1234, 0x5678);
        device_info.button_count = 4;
        device_info.previous_states = vec![false; 4];
        let timestamp = Instant::now();

        // Buttons 0 and 2 pressed
        let report = [0x01, 0x00, 0x01, 0x00];

        let events = parse_generic_byte_per_button(&device_id, &report, &mut device_info, timestamp);

        assert_eq!(events.len(), 2);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&2));
    }

    #[test]
    fn test_generic_report_auto_detect() {
        let device_id = Uuid::new_v4();
        let mut device_info = OpenDeviceInfo::new(device_id, 0x1234, 0x5678);
        let timestamp = Instant::now();

        // Start with no detected format
        assert!(device_info.detected_format.is_none());

        // Send a bitmap-style report
        let report = [0x05, 0x00];
        let _ = parse_generic_report(&device_id, &report, &mut device_info, timestamp);

        // Format should now be detected
        assert!(device_info.detected_format.is_some());
    }

    #[tokio::test]
    async fn test_input_reader_with_button_handler() {
        let handles = Arc::new(Mutex::new(HashMap::new()));
        let handler = Arc::new(DefaultButtonInputHandler::new());
        let reader = HidInputReader::new(handles).with_button_handler(handler.clone());

        // Verify handler is set
        assert!(reader.button_handler().is_some());

        // Verify we can get a reference to the same handler
        let handler_ref = reader.button_handler().unwrap();
        assert!(Arc::ptr_eq(handler_ref, &handler));
    }

    #[tokio::test]
    async fn test_input_reader_builder_pattern() {
        let handles = Arc::new(Mutex::new(HashMap::new()));
        let handler = Arc::new(DefaultButtonInputHandler::new());

        // Test that builder pattern methods work correctly
        let reader = HidInputReader::new(handles)
            .with_button_handler(handler);

        assert!(reader.button_handler().is_some());
        assert!(!reader.is_running().await);
    }

    #[tokio::test]
    async fn test_button_handler_receives_events() {
        use super::actions::ButtonAction;
        use super::mapping::ButtonMapping;

        let handler = Arc::new(DefaultButtonInputHandler::new());
        let device_id = Uuid::new_v4();

        // Register a mapping for button 0
        let mapping = ButtonMapping::new(device_id, 0, ButtonAction::AddLapMarker);
        handler.register_mappings(&device_id, vec![mapping]);

        // Subscribe to action events before processing
        let mut action_rx = handler.subscribe_actions();

        // Create a raw button event
        let event = RawButtonEvent {
            device_id,
            button_code: 0,
            pressed: true,
            timestamp: Instant::now(),
        };

        // Process the event
        handler.process_event(event).await;

        // Should receive the mapped action
        match tokio::time::timeout(Duration::from_millis(100), action_rx.recv()).await {
            Ok(Ok(action_event)) => {
                assert_eq!(action_event.device_id, device_id);
                assert_eq!(action_event.action, ButtonAction::AddLapMarker);
            }
            Ok(Err(e)) => panic!("Failed to receive action event: {:?}", e),
            Err(_) => panic!("Timed out waiting for action event"),
        }
    }

    #[tokio::test]
    async fn test_learning_mode_captures_button() {
        let handler = Arc::new(DefaultButtonInputHandler::new());
        let device_id = Uuid::new_v4();

        // Start learning mode
        handler.start_learning_mode(&device_id);
        assert!(handler.is_learning());

        // Create a raw button event
        let event = RawButtonEvent {
            device_id,
            button_code: 5,
            pressed: true,
            timestamp: Instant::now(),
        };

        // Process the event
        handler.process_event(event).await;

        // Should have captured the button code
        assert_eq!(handler.get_learned_button(), Some(5));

        // Stop learning mode
        handler.stop_learning_mode();
        assert!(!handler.is_learning());
    }

    #[tokio::test]
    async fn test_learning_mode_ignores_mappings() {
        use super::actions::ButtonAction;
        use super::mapping::ButtonMapping;

        let handler = Arc::new(DefaultButtonInputHandler::new());
        let device_id = Uuid::new_v4();

        // Register a mapping for button 0
        let mapping = ButtonMapping::new(device_id, 0, ButtonAction::AddLapMarker);
        handler.register_mappings(&device_id, vec![mapping]);

        // Subscribe to action events
        let mut action_rx = handler.subscribe_actions();

        // Start learning mode
        handler.start_learning_mode(&device_id);

        // Create a raw button event for the mapped button
        let event = RawButtonEvent {
            device_id,
            button_code: 0,
            pressed: true,
            timestamp: Instant::now(),
        };

        // Process the event
        handler.process_event(event).await;

        // Should NOT receive any action event (learning mode takes priority)
        match tokio::time::timeout(Duration::from_millis(50), action_rx.recv()).await {
            Ok(_) => panic!("Should not receive action event during learning mode"),
            Err(_) => {} // Expected timeout
        }

        // Button should be learned instead
        assert_eq!(handler.get_learned_button(), Some(0));
    }
}
