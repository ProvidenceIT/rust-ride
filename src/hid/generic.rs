//! Generic HID Button Controller Support
//!
//! Implements input report parsing for generic USB button controllers
//! that use standard HID gamepad or keyboard protocols.
//!
//! Many USB foot pedals, button boxes, and macro keypads use these
//! standard protocols rather than proprietary ones.

use super::mapping::RawButtonEvent;
use std::time::Instant;
use uuid::Uuid;

/// HID Usage Pages for device classification
pub mod usage_pages {
    /// Generic Desktop Controls (mice, keyboards, gamepads)
    pub const GENERIC_DESKTOP: u16 = 0x01;
    /// Keyboard/Keypad
    pub const KEYBOARD: u16 = 0x07;
    /// Button page
    pub const BUTTON: u16 = 0x09;
    /// Consumer Control (media keys)
    pub const CONSUMER: u16 = 0x0C;
}

/// HID Usages within the Generic Desktop page
pub mod desktop_usages {
    /// Gamepad device
    pub const GAMEPAD: u16 = 0x05;
    /// Joystick device
    pub const JOYSTICK: u16 = 0x04;
    /// Keyboard device
    pub const KEYBOARD: u16 = 0x06;
    /// Keypad device
    pub const KEYPAD: u16 = 0x07;
    /// Multi-axis controller
    pub const MULTI_AXIS: u16 = 0x08;
}

/// Report format types for generic HID devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericReportFormat {
    /// Bitmap encoding: each bit represents a button (0=released, 1=pressed)
    /// Common for gamepads and simple button controllers
    Bitmap,
    /// Byte-per-button: each byte represents a button (0=released, non-zero=pressed)
    /// Used by some button boxes and specialized controllers
    BytePerButton,
    /// Keyboard scan codes: reports contain scan codes of pressed keys
    /// Used by devices that emulate keyboard input
    KeyboardScanCode,
    /// Consumer control: media key style reports
    /// Used by devices with media controls
    ConsumerControl,
    /// Unknown format - will try to auto-detect
    Unknown,
}

impl GenericReportFormat {
    /// Get a human-readable name for this format
    pub fn name(&self) -> &'static str {
        match self {
            GenericReportFormat::Bitmap => "Bitmap",
            GenericReportFormat::BytePerButton => "Byte-per-button",
            GenericReportFormat::KeyboardScanCode => "Keyboard",
            GenericReportFormat::ConsumerControl => "Consumer Control",
            GenericReportFormat::Unknown => "Unknown",
        }
    }
}

/// Configuration for a generic HID device
#[derive(Debug, Clone)]
pub struct GenericDeviceConfig {
    /// Report format to use for parsing
    pub format: GenericReportFormat,
    /// Number of buttons to track
    pub button_count: u8,
    /// Byte offset where button data starts in the report
    pub button_data_offset: usize,
    /// Whether the first byte is a report ID to skip
    pub has_report_id: bool,
    /// Report ID to expect (if has_report_id is true)
    pub expected_report_id: Option<u8>,
}

impl Default for GenericDeviceConfig {
    fn default() -> Self {
        Self {
            format: GenericReportFormat::Bitmap,
            button_count: 32,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        }
    }
}

impl GenericDeviceConfig {
    /// Create a config for a gamepad-style device
    pub fn gamepad(button_count: u8) -> Self {
        Self {
            format: GenericReportFormat::Bitmap,
            button_count,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        }
    }

    /// Create a config for a gamepad with report ID prefix
    pub fn gamepad_with_report_id(button_count: u8, report_id: u8) -> Self {
        Self {
            format: GenericReportFormat::Bitmap,
            button_count,
            button_data_offset: 1,
            has_report_id: true,
            expected_report_id: Some(report_id),
        }
    }

    /// Create a config for a keyboard-style device
    pub fn keyboard() -> Self {
        Self {
            format: GenericReportFormat::KeyboardScanCode,
            button_count: 128, // Standard keyboard has ~104 keys, allow more
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        }
    }

    /// Create a config for byte-per-button devices (like some foot pedals)
    pub fn byte_per_button(button_count: u8) -> Self {
        Self {
            format: GenericReportFormat::BytePerButton,
            button_count,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        }
    }
}

/// Generic HID device input parser
///
/// Handles various generic HID report formats including gamepad-style
/// bitmap encoding and keyboard scan codes.
#[derive(Debug)]
pub struct GenericHidParser {
    /// Device configuration
    config: GenericDeviceConfig,
    /// Previous button states for detecting press/release transitions
    previous_states: Vec<bool>,
    /// For keyboard mode: previously pressed key codes
    previous_keys: Vec<u8>,
}

impl GenericHidParser {
    /// Create a new parser with the given configuration
    pub fn new(config: GenericDeviceConfig) -> Self {
        let button_count = config.button_count as usize;
        Self {
            config,
            previous_states: vec![false; button_count],
            previous_keys: Vec::with_capacity(6), // Standard keyboard report has 6 key slots
        }
    }

    /// Create a parser with default gamepad configuration
    pub fn gamepad(button_count: u8) -> Self {
        Self::new(GenericDeviceConfig::gamepad(button_count))
    }

    /// Create a parser with keyboard configuration
    pub fn keyboard() -> Self {
        Self::new(GenericDeviceConfig::keyboard())
    }

    /// Get the current configuration
    pub fn config(&self) -> &GenericDeviceConfig {
        &self.config
    }

    /// Get the report format
    pub fn format(&self) -> GenericReportFormat {
        self.config.format
    }

    /// Reset all button states to released
    pub fn reset_states(&mut self) {
        self.previous_states.fill(false);
        self.previous_keys.clear();
    }

    /// Parse a HID input report and generate button events
    ///
    /// Returns a vector of button events for any buttons that changed state.
    pub fn parse_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        if report.is_empty() {
            return Vec::new();
        }

        match self.config.format {
            GenericReportFormat::Bitmap => {
                self.parse_bitmap_report(device_id, report, timestamp)
            }
            GenericReportFormat::BytePerButton => {
                self.parse_byte_per_button_report(device_id, report, timestamp)
            }
            GenericReportFormat::KeyboardScanCode => {
                self.parse_keyboard_report(device_id, report, timestamp)
            }
            GenericReportFormat::ConsumerControl => {
                self.parse_consumer_report(device_id, report, timestamp)
            }
            GenericReportFormat::Unknown => {
                // Try bitmap as a fallback
                self.parse_bitmap_report(device_id, report, timestamp)
            }
        }
    }

    /// Parse a bitmap-encoded button report
    ///
    /// Each bit in the report represents a button state.
    /// Bit 0 of byte 0 = button 0, bit 1 of byte 0 = button 1, etc.
    fn parse_bitmap_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        let mut events = Vec::new();

        // Validate report and apply offset
        let button_data = if self.config.has_report_id {
            // Check report ID if expected
            if let Some(expected_id) = self.config.expected_report_id {
                if report.first() != Some(&expected_id) {
                    return events;
                }
            }
            if report.len() <= self.config.button_data_offset {
                return events;
            }
            &report[self.config.button_data_offset..]
        } else {
            report
        };

        let mut button_index = 0usize;

        for &byte in button_data.iter() {
            for bit in 0..8 {
                if button_index >= self.config.button_count as usize {
                    break;
                }

                let is_pressed = (byte >> bit) & 1 != 0;
                let was_pressed = self.previous_states
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

                if button_index < self.previous_states.len() {
                    self.previous_states[button_index] = is_pressed;
                }

                button_index += 1;
            }
        }

        events
    }

    /// Parse a byte-per-button report
    ///
    /// Each byte represents a button: 0 = released, non-zero = pressed
    fn parse_byte_per_button_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        let mut events = Vec::new();

        // Apply offset
        let button_data = if self.config.button_data_offset < report.len() {
            &report[self.config.button_data_offset..]
        } else {
            return events;
        };

        for (index, &button_byte) in button_data.iter().enumerate() {
            if index >= self.config.button_count as usize {
                break;
            }

            let is_pressed = button_byte != 0;
            let was_pressed = self.previous_states
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

            if index < self.previous_states.len() {
                self.previous_states[index] = is_pressed;
            }
        }

        events
    }

    /// Parse a keyboard scan code report
    ///
    /// Standard HID keyboard reports have the format:
    /// [modifier_byte, reserved, key1, key2, key3, key4, key5, key6]
    ///
    /// We map each unique key code to a button for action mapping.
    fn parse_keyboard_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        let mut events = Vec::new();

        // Standard keyboard report is 8 bytes
        // Byte 0: Modifier keys (Ctrl, Shift, Alt, etc.)
        // Byte 1: Reserved
        // Bytes 2-7: Up to 6 key codes
        if report.len() < 3 {
            return events;
        }

        let modifier_byte = report[0];
        let key_codes = &report[2..report.len().min(8)];

        // Process modifier keys as buttons 128-135
        // Bit 0: Left Ctrl, Bit 1: Left Shift, Bit 2: Left Alt, Bit 3: Left GUI
        // Bit 4: Right Ctrl, Bit 5: Right Shift, Bit 6: Right Alt, Bit 7: Right GUI
        for bit in 0..8 {
            let modifier_index = 128 + bit;
            if modifier_index >= self.config.button_count as usize {
                break;
            }

            let is_pressed = (modifier_byte >> bit) & 1 != 0;
            let was_pressed = self.previous_states
                .get(modifier_index)
                .copied()
                .unwrap_or(false);

            if is_pressed != was_pressed {
                events.push(RawButtonEvent {
                    device_id: *device_id,
                    button_code: modifier_index as u8,
                    pressed: is_pressed,
                    timestamp,
                });

                let modifier_name = match bit {
                    0 => "Left Ctrl",
                    1 => "Left Shift",
                    2 => "Left Alt",
                    3 => "Left GUI",
                    4 => "Right Ctrl",
                    5 => "Right Shift",
                    6 => "Right Alt",
                    7 => "Right GUI",
                    _ => "Unknown",
                };

                tracing::debug!(
                    "Keyboard {} {}",
                    modifier_name,
                    if is_pressed { "pressed" } else { "released" }
                );

                if modifier_index < self.previous_states.len() {
                    self.previous_states[modifier_index] = is_pressed;
                }
            }
        }

        // Process regular key codes
        // Key codes are in the range 0x04 - 0x65 typically
        // We use them directly as button codes (0-127 range)
        let current_keys: Vec<u8> = key_codes
            .iter()
            .filter(|&&k| k != 0) // 0 means no key pressed in that slot
            .copied()
            .collect();

        // Detect newly pressed keys
        for &key in &current_keys {
            if !self.previous_keys.contains(&key) && key < 128 {
                let key_index = key as usize;

                events.push(RawButtonEvent {
                    device_id: *device_id,
                    button_code: key,
                    pressed: true,
                    timestamp,
                });

                tracing::debug!("Keyboard key 0x{:02X} pressed", key);

                if key_index < self.previous_states.len() {
                    self.previous_states[key_index] = true;
                }
            }
        }

        // Detect released keys
        for &key in &self.previous_keys {
            if !current_keys.contains(&key) && key < 128 {
                let key_index = key as usize;

                events.push(RawButtonEvent {
                    device_id: *device_id,
                    button_code: key,
                    pressed: false,
                    timestamp,
                });

                tracing::debug!("Keyboard key 0x{:02X} released", key);

                if key_index < self.previous_states.len() {
                    self.previous_states[key_index] = false;
                }
            }
        }

        self.previous_keys = current_keys;

        events
    }

    /// Parse a consumer control (media key) report
    ///
    /// Consumer control reports typically send a 16-bit usage code
    /// for the currently active control.
    fn parse_consumer_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        let mut events = Vec::new();

        if report.len() < 2 {
            return events;
        }

        // Consumer control reports are typically 2 bytes (16-bit usage code)
        // When a key is pressed, the code is sent. When released, 0x0000 is sent.
        let usage_code = u16::from_le_bytes([report[0], report[1]]);

        // Map common consumer control codes to button indices
        let button_code = match usage_code {
            0x00E2 => Some(0u8),  // Mute
            0x00E9 => Some(1u8),  // Volume Up
            0x00EA => Some(2u8),  // Volume Down
            0x00B5 => Some(3u8),  // Scan Next Track
            0x00B6 => Some(4u8),  // Scan Previous Track
            0x00B7 => Some(5u8),  // Stop
            0x00CD => Some(6u8),  // Play/Pause
            0x0183 => Some(7u8),  // Media Select
            0x018A => Some(8u8),  // Mail
            0x0192 => Some(9u8),  // Calculator
            0x0194 => Some(10u8), // My Computer
            0x0221 => Some(11u8), // Search
            0x0223 => Some(12u8), // Home
            0x0224 => Some(13u8), // Back
            0x0225 => Some(14u8), // Forward
            0x0226 => Some(15u8), // Stop (browser)
            0x0227 => Some(16u8), // Refresh
            0x022A => Some(17u8), // Favorites
            _ if usage_code != 0 => {
                // Unknown usage code - use lower byte as button code
                Some((usage_code & 0x7F) as u8)
            }
            _ => None,
        };

        if let Some(code) = button_code {
            if usage_code != 0 {
                // Key pressed
                let code_index = code as usize;
                let was_pressed = self.previous_states
                    .get(code_index)
                    .copied()
                    .unwrap_or(false);

                if !was_pressed {
                    events.push(RawButtonEvent {
                        device_id: *device_id,
                        button_code: code,
                        pressed: true,
                        timestamp,
                    });

                    tracing::debug!("Consumer control 0x{:04X} pressed (button {})", usage_code, code);

                    if code_index < self.previous_states.len() {
                        self.previous_states[code_index] = true;
                    }
                }
            }
        }

        // Handle release (usage_code == 0)
        if usage_code == 0 {
            // Find any buttons that were previously pressed and release them
            for (index, was_pressed) in self.previous_states.iter_mut().enumerate() {
                if *was_pressed && index < 32 { // Only check consumer control range
                    events.push(RawButtonEvent {
                        device_id: *device_id,
                        button_code: index as u8,
                        pressed: false,
                        timestamp,
                    });

                    tracing::debug!("Consumer control button {} released", index);

                    *was_pressed = false;
                }
            }
        }

        events
    }

    /// Get a human-readable label for a button
    pub fn button_label(&self, button_code: u8) -> String {
        match self.config.format {
            GenericReportFormat::KeyboardScanCode => {
                if button_code >= 128 {
                    // Modifier keys
                    match button_code - 128 {
                        0 => "Left Ctrl".to_string(),
                        1 => "Left Shift".to_string(),
                        2 => "Left Alt".to_string(),
                        3 => "Left GUI".to_string(),
                        4 => "Right Ctrl".to_string(),
                        5 => "Right Shift".to_string(),
                        6 => "Right Alt".to_string(),
                        7 => "Right GUI".to_string(),
                        _ => format!("Modifier {}", button_code),
                    }
                } else {
                    // Regular keys - convert scan code to key name
                    keyboard_scancode_name(button_code)
                }
            }
            GenericReportFormat::ConsumerControl => {
                match button_code {
                    0 => "Mute".to_string(),
                    1 => "Volume Up".to_string(),
                    2 => "Volume Down".to_string(),
                    3 => "Next Track".to_string(),
                    4 => "Previous Track".to_string(),
                    5 => "Stop".to_string(),
                    6 => "Play/Pause".to_string(),
                    7 => "Media Select".to_string(),
                    8 => "Mail".to_string(),
                    9 => "Calculator".to_string(),
                    10 => "My Computer".to_string(),
                    11 => "Search".to_string(),
                    12 => "Home".to_string(),
                    13 => "Back".to_string(),
                    14 => "Forward".to_string(),
                    15 => "Stop (Browser)".to_string(),
                    16 => "Refresh".to_string(),
                    17 => "Favorites".to_string(),
                    _ => format!("Media {}", button_code),
                }
            }
            _ => format!("Button {}", button_code + 1),
        }
    }
}

/// Get a human-readable name for a USB HID keyboard scan code
fn keyboard_scancode_name(scancode: u8) -> String {
    match scancode {
        0x04 => "A".to_string(),
        0x05 => "B".to_string(),
        0x06 => "C".to_string(),
        0x07 => "D".to_string(),
        0x08 => "E".to_string(),
        0x09 => "F".to_string(),
        0x0A => "G".to_string(),
        0x0B => "H".to_string(),
        0x0C => "I".to_string(),
        0x0D => "J".to_string(),
        0x0E => "K".to_string(),
        0x0F => "L".to_string(),
        0x10 => "M".to_string(),
        0x11 => "N".to_string(),
        0x12 => "O".to_string(),
        0x13 => "P".to_string(),
        0x14 => "Q".to_string(),
        0x15 => "R".to_string(),
        0x16 => "S".to_string(),
        0x17 => "T".to_string(),
        0x18 => "U".to_string(),
        0x19 => "V".to_string(),
        0x1A => "W".to_string(),
        0x1B => "X".to_string(),
        0x1C => "Y".to_string(),
        0x1D => "Z".to_string(),
        0x1E => "1".to_string(),
        0x1F => "2".to_string(),
        0x20 => "3".to_string(),
        0x21 => "4".to_string(),
        0x22 => "5".to_string(),
        0x23 => "6".to_string(),
        0x24 => "7".to_string(),
        0x25 => "8".to_string(),
        0x26 => "9".to_string(),
        0x27 => "0".to_string(),
        0x28 => "Enter".to_string(),
        0x29 => "Escape".to_string(),
        0x2A => "Backspace".to_string(),
        0x2B => "Tab".to_string(),
        0x2C => "Space".to_string(),
        0x2D => "-".to_string(),
        0x2E => "=".to_string(),
        0x2F => "[".to_string(),
        0x30 => "]".to_string(),
        0x31 => "\\".to_string(),
        0x33 => ";".to_string(),
        0x34 => "'".to_string(),
        0x35 => "`".to_string(),
        0x36 => ",".to_string(),
        0x37 => ".".to_string(),
        0x38 => "/".to_string(),
        0x39 => "Caps Lock".to_string(),
        0x3A => "F1".to_string(),
        0x3B => "F2".to_string(),
        0x3C => "F3".to_string(),
        0x3D => "F4".to_string(),
        0x3E => "F5".to_string(),
        0x3F => "F6".to_string(),
        0x40 => "F7".to_string(),
        0x41 => "F8".to_string(),
        0x42 => "F9".to_string(),
        0x43 => "F10".to_string(),
        0x44 => "F11".to_string(),
        0x45 => "F12".to_string(),
        0x46 => "Print Screen".to_string(),
        0x47 => "Scroll Lock".to_string(),
        0x48 => "Pause".to_string(),
        0x49 => "Insert".to_string(),
        0x4A => "Home".to_string(),
        0x4B => "Page Up".to_string(),
        0x4C => "Delete".to_string(),
        0x4D => "End".to_string(),
        0x4E => "Page Down".to_string(),
        0x4F => "Right Arrow".to_string(),
        0x50 => "Left Arrow".to_string(),
        0x51 => "Down Arrow".to_string(),
        0x52 => "Up Arrow".to_string(),
        0x53 => "Num Lock".to_string(),
        0x54 => "Keypad /".to_string(),
        0x55 => "Keypad *".to_string(),
        0x56 => "Keypad -".to_string(),
        0x57 => "Keypad +".to_string(),
        0x58 => "Keypad Enter".to_string(),
        0x59 => "Keypad 1".to_string(),
        0x5A => "Keypad 2".to_string(),
        0x5B => "Keypad 3".to_string(),
        0x5C => "Keypad 4".to_string(),
        0x5D => "Keypad 5".to_string(),
        0x5E => "Keypad 6".to_string(),
        0x5F => "Keypad 7".to_string(),
        0x60 => "Keypad 8".to_string(),
        0x61 => "Keypad 9".to_string(),
        0x62 => "Keypad 0".to_string(),
        0x63 => "Keypad .".to_string(),
        _ => format!("Key 0x{:02X}", scancode),
    }
}

/// Try to auto-detect the report format from a sample report
///
/// This is a best-effort heuristic based on report structure.
pub fn detect_report_format(sample_report: &[u8]) -> GenericReportFormat {
    if sample_report.is_empty() {
        return GenericReportFormat::Unknown;
    }

    // Check for keyboard report pattern (8 bytes with modifier byte)
    if sample_report.len() == 8 {
        let modifier = sample_report[0];
        let reserved = sample_report[1];

        // Keyboards have reserved byte as 0 and modifiers in valid range
        if reserved == 0 && modifier <= 0xFF {
            let key_codes = &sample_report[2..8];
            let valid_key_range = key_codes.iter().all(|&k| k == 0 || (k >= 0x04 && k <= 0x73));
            if valid_key_range {
                return GenericReportFormat::KeyboardScanCode;
            }
        }
    }

    // Check for consumer control (2-byte usage code)
    if sample_report.len() == 2 || sample_report.len() == 3 {
        let usage = if sample_report.len() == 2 {
            u16::from_le_bytes([sample_report[0], sample_report[1]])
        } else {
            // Some have report ID prefix
            u16::from_le_bytes([sample_report[1], sample_report[2]])
        };

        // Check for common consumer control codes
        match usage {
            0x0000 | 0x00CD | 0x00E2 | 0x00E9 | 0x00EA | 0x00B5 | 0x00B6 => {
                return GenericReportFormat::ConsumerControl;
            }
            _ => {}
        }
    }

    // Check for byte-per-button (all bytes are 0 or 1)
    let all_binary = sample_report.iter().all(|&b| b == 0 || b == 1);
    if all_binary && sample_report.len() <= 16 {
        return GenericReportFormat::BytePerButton;
    }

    // Default to bitmap for short reports, unknown for longer ones
    if sample_report.len() <= 8 {
        GenericReportFormat::Bitmap
    } else {
        GenericReportFormat::Unknown
    }
}

/// Known generic device profiles
#[derive(Debug, Clone)]
pub struct GenericDeviceProfile {
    /// USB vendor ID
    pub vendor_id: u16,
    /// USB product ID
    pub product_id: u16,
    /// Device name
    pub name: &'static str,
    /// Device configuration
    pub config: GenericDeviceConfig,
}

/// Well-known generic USB button devices
pub const KNOWN_GENERIC_DEVICES: &[GenericDeviceProfile] = &[
    // Common USB foot pedals (these typically use keyboard emulation)
    GenericDeviceProfile {
        vendor_id: 0x0C45,  // Microdia
        product_id: 0x7403,
        name: "Generic USB Foot Pedal",
        config: GenericDeviceConfig {
            format: GenericReportFormat::KeyboardScanCode,
            button_count: 3,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        },
    },
    // VEC USB Foot Pedal (common medical transcription pedal)
    GenericDeviceProfile {
        vendor_id: 0x05F3,  // VEC
        product_id: 0x00FF,
        name: "VEC USB Foot Pedal",
        config: GenericDeviceConfig {
            format: GenericReportFormat::KeyboardScanCode,
            button_count: 3,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        },
    },
    // Olympus RS foot control
    GenericDeviceProfile {
        vendor_id: 0x07B4,  // Olympus
        product_id: 0x0218,
        name: "Olympus RS Foot Control",
        config: GenericDeviceConfig {
            format: GenericReportFormat::BytePerButton,
            button_count: 4,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        },
    },
];

/// Find a known generic device profile by VID/PID
pub fn find_generic_device_profile(vendor_id: u16, product_id: u16) -> Option<&'static GenericDeviceProfile> {
    KNOWN_GENERIC_DEVICES
        .iter()
        .find(|p| p.vendor_id == vendor_id && p.product_id == product_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_config() {
        let config = GenericDeviceConfig::gamepad(8);
        assert_eq!(config.format, GenericReportFormat::Bitmap);
        assert_eq!(config.button_count, 8);
        assert_eq!(config.button_data_offset, 0);
        assert!(!config.has_report_id);
    }

    #[test]
    fn test_gamepad_with_report_id() {
        let config = GenericDeviceConfig::gamepad_with_report_id(16, 0x01);
        assert_eq!(config.format, GenericReportFormat::Bitmap);
        assert_eq!(config.button_count, 16);
        assert_eq!(config.button_data_offset, 1);
        assert!(config.has_report_id);
        assert_eq!(config.expected_report_id, Some(0x01));
    }

    #[test]
    fn test_bitmap_parsing() {
        let mut parser = GenericHidParser::gamepad(8);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Buttons 0 and 2 pressed (binary: 00000101 = 0x05)
        let report = [0x05];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 2);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&2));
        assert!(events.iter().all(|e| e.pressed));
    }

    #[test]
    fn test_bitmap_release() {
        let mut parser = GenericHidParser::gamepad(8);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press button 0
        let report1 = [0x01];
        let _ = parser.parse_report(&device_id, &report1, timestamp);

        // Release button 0
        let report2 = [0x00];
        let events = parser.parse_report(&device_id, &report2, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(!events[0].pressed);
    }

    #[test]
    fn test_bitmap_no_change() {
        let mut parser = GenericHidParser::gamepad(8);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press button 0
        let report = [0x01];
        let _ = parser.parse_report(&device_id, &report, timestamp);

        // Same state - no events
        let events = parser.parse_report(&device_id, &report, timestamp);
        assert!(events.is_empty());
    }

    #[test]
    fn test_bitmap_with_report_id() {
        let config = GenericDeviceConfig::gamepad_with_report_id(8, 0x01);
        let mut parser = GenericHidParser::new(config);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Report with ID prefix: [report_id, button_data]
        let report = [0x01, 0x05]; // Buttons 0 and 2
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_bitmap_wrong_report_id() {
        let config = GenericDeviceConfig::gamepad_with_report_id(8, 0x01);
        let mut parser = GenericHidParser::new(config);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Wrong report ID - should be ignored
        let report = [0x02, 0x05];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert!(events.is_empty());
    }

    #[test]
    fn test_byte_per_button_parsing() {
        let config = GenericDeviceConfig::byte_per_button(3);
        let mut parser = GenericHidParser::new(config);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Buttons 0 and 2 pressed
        let report = [0x01, 0x00, 0x01];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 2);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&2));
    }

    #[test]
    fn test_keyboard_parsing() {
        let mut parser = GenericHidParser::keyboard();
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Standard keyboard report: [modifiers, reserved, key1, key2, key3, key4, key5, key6]
        // Key 'A' pressed (scancode 0x04), Left Shift held
        let report = [0x02, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];
        let events = parser.parse_report(&device_id, &report, timestamp);

        // Should have 2 events: Left Shift (modifier) and 'A' key
        assert_eq!(events.len(), 2);

        // Check for Left Shift modifier (128 + 1 = 129)
        assert!(events.iter().any(|e| e.button_code == 129 && e.pressed));
        // Check for 'A' key (scancode 0x04)
        assert!(events.iter().any(|e| e.button_code == 0x04 && e.pressed));
    }

    #[test]
    fn test_keyboard_key_release() {
        let mut parser = GenericHidParser::keyboard();
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press 'A'
        let report1 = [0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];
        let _ = parser.parse_report(&device_id, &report1, timestamp);

        // Release 'A'
        let report2 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let events = parser.parse_report(&device_id, &report2, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0x04);
        assert!(!events[0].pressed);
    }

    #[test]
    fn test_consumer_control_parsing() {
        let config = GenericDeviceConfig {
            format: GenericReportFormat::ConsumerControl,
            button_count: 32,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        };
        let mut parser = GenericHidParser::new(config);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Play/Pause pressed (usage code 0x00CD)
        let report = [0xCD, 0x00];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 6); // Play/Pause maps to button 6
        assert!(events[0].pressed);
    }

    #[test]
    fn test_consumer_control_release() {
        let config = GenericDeviceConfig {
            format: GenericReportFormat::ConsumerControl,
            button_count: 32,
            button_data_offset: 0,
            has_report_id: false,
            expected_report_id: None,
        };
        let mut parser = GenericHidParser::new(config);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press Play/Pause
        let report1 = [0xCD, 0x00];
        let _ = parser.parse_report(&device_id, &report1, timestamp);

        // Release (0x0000)
        let report2 = [0x00, 0x00];
        let events = parser.parse_report(&device_id, &report2, timestamp);

        assert_eq!(events.len(), 1);
        assert!(!events[0].pressed);
    }

    #[test]
    fn test_reset_states() {
        let mut parser = GenericHidParser::gamepad(8);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press button
        let report = [0x01];
        let _ = parser.parse_report(&device_id, &report, timestamp);

        // Reset
        parser.reset_states();

        // Same report should now generate a press event
        let events = parser.parse_report(&device_id, &report, timestamp);
        assert_eq!(events.len(), 1);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_button_labels() {
        let gamepad = GenericHidParser::gamepad(8);
        assert_eq!(gamepad.button_label(0), "Button 1");
        assert_eq!(gamepad.button_label(5), "Button 6");

        let keyboard = GenericHidParser::keyboard();
        assert_eq!(keyboard.button_label(0x04), "A");
        assert_eq!(keyboard.button_label(0x28), "Enter");
        assert_eq!(keyboard.button_label(129), "Left Shift");

        let config = GenericDeviceConfig {
            format: GenericReportFormat::ConsumerControl,
            ..Default::default()
        };
        let consumer = GenericHidParser::new(config);
        assert_eq!(consumer.button_label(0), "Mute");
        assert_eq!(consumer.button_label(6), "Play/Pause");
    }

    #[test]
    fn test_detect_report_format_keyboard() {
        // Standard keyboard report
        let report = [0x02, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_report_format(&report), GenericReportFormat::KeyboardScanCode);
    }

    #[test]
    fn test_detect_report_format_consumer() {
        // Play/Pause consumer control
        let report = [0xCD, 0x00];
        assert_eq!(detect_report_format(&report), GenericReportFormat::ConsumerControl);

        // Mute
        let report = [0xE2, 0x00];
        assert_eq!(detect_report_format(&report), GenericReportFormat::ConsumerControl);
    }

    #[test]
    fn test_detect_report_format_byte_per_button() {
        // All zeros and ones
        let report = [0x01, 0x00, 0x01, 0x00];
        assert_eq!(detect_report_format(&report), GenericReportFormat::BytePerButton);
    }

    #[test]
    fn test_detect_report_format_bitmap() {
        // Short report with mixed values
        let report = [0x05, 0x00];
        assert_eq!(detect_report_format(&report), GenericReportFormat::Bitmap);
    }

    #[test]
    fn test_empty_report() {
        let mut parser = GenericHidParser::gamepad(8);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        let events = parser.parse_report(&device_id, &[], timestamp);
        assert!(events.is_empty());
    }

    #[test]
    fn test_find_generic_profile() {
        // Known device
        let profile = find_generic_device_profile(0x05F3, 0x00FF);
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().name, "VEC USB Foot Pedal");

        // Unknown device
        let profile = find_generic_device_profile(0x1234, 0x5678);
        assert!(profile.is_none());
    }

    #[test]
    fn test_keyboard_scancode_names() {
        assert_eq!(keyboard_scancode_name(0x04), "A");
        assert_eq!(keyboard_scancode_name(0x1D), "Z");
        assert_eq!(keyboard_scancode_name(0x28), "Enter");
        assert_eq!(keyboard_scancode_name(0x29), "Escape");
        assert_eq!(keyboard_scancode_name(0x3A), "F1");
        assert_eq!(keyboard_scancode_name(0x4F), "Right Arrow");
    }

    #[test]
    fn test_multi_byte_bitmap() {
        let mut parser = GenericHidParser::gamepad(16);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Buttons 0, 7 (first byte) and 8, 15 (second byte)
        let report = [0x81, 0x81]; // 10000001, 10000001
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 4);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&7));
        assert!(codes.contains(&8));
        assert!(codes.contains(&15));
    }
}
