//! Stream Deck Protocol Support
//!
//! Implements input report parsing for Elgato Stream Deck devices.
//! Stream Deck devices use a custom HID protocol where each button's state
//! is reported as a separate byte in the input report.

use super::mapping::RawButtonEvent;
use std::time::Instant;
use uuid::Uuid;

/// Stream Deck product IDs
pub mod product_ids {
    /// Stream Deck Original (15 buttons, 3x5 grid)
    pub const STREAM_DECK_ORIGINAL: u16 = 0x0060;
    /// Stream Deck Mini (6 buttons, 2x3 grid)
    pub const STREAM_DECK_MINI: u16 = 0x006C;
    /// Stream Deck XL (32 buttons, 4x8 grid)
    pub const STREAM_DECK_XL: u16 = 0x006D;
    /// Stream Deck MK.2 (15 buttons, 3x5 grid)
    pub const STREAM_DECK_MK2: u16 = 0x0080;
    /// Stream Deck Pedal (3 foot pedals)
    pub const STREAM_DECK_PEDAL: u16 = 0x0086;
}

/// Elgato vendor ID
pub const ELGATO_VENDOR_ID: u16 = 0x0FD9;

/// Stream Deck model variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamDeckModel {
    /// Original Stream Deck (15 buttons, 3x5 grid)
    Original,
    /// Stream Deck Mini (6 buttons, 2x3 grid)
    Mini,
    /// Stream Deck XL (32 buttons, 4x8 grid)
    Xl,
    /// Stream Deck MK.2 (15 buttons, 3x5 grid)
    Mk2,
    /// Stream Deck Pedal (3 foot pedals)
    Pedal,
}

impl StreamDeckModel {
    /// Create from product ID
    pub fn from_product_id(product_id: u16) -> Option<Self> {
        match product_id {
            product_ids::STREAM_DECK_ORIGINAL => Some(StreamDeckModel::Original),
            product_ids::STREAM_DECK_MINI => Some(StreamDeckModel::Mini),
            product_ids::STREAM_DECK_XL => Some(StreamDeckModel::Xl),
            product_ids::STREAM_DECK_MK2 => Some(StreamDeckModel::Mk2),
            product_ids::STREAM_DECK_PEDAL => Some(StreamDeckModel::Pedal),
            _ => None,
        }
    }

    /// Check if a product ID is a Stream Deck device
    pub fn is_stream_deck(vendor_id: u16, product_id: u16) -> bool {
        vendor_id == ELGATO_VENDOR_ID && Self::from_product_id(product_id).is_some()
    }

    /// Get the number of buttons for this model
    pub fn button_count(&self) -> u8 {
        match self {
            StreamDeckModel::Original => 15,
            StreamDeckModel::Mini => 6,
            StreamDeckModel::Xl => 32,
            StreamDeckModel::Mk2 => 15,
            StreamDeckModel::Pedal => 3,
        }
    }

    /// Get the grid layout (rows, columns) for this model
    pub fn grid_layout(&self) -> (u8, u8) {
        match self {
            StreamDeckModel::Original => (3, 5),
            StreamDeckModel::Mini => (2, 3),
            StreamDeckModel::Xl => (4, 8),
            StreamDeckModel::Mk2 => (3, 5),
            StreamDeckModel::Pedal => (1, 3),
        }
    }

    /// Get the display name for this model
    pub fn name(&self) -> &'static str {
        match self {
            StreamDeckModel::Original => "Stream Deck",
            StreamDeckModel::Mini => "Stream Deck Mini",
            StreamDeckModel::Xl => "Stream Deck XL",
            StreamDeckModel::Mk2 => "Stream Deck MK.2",
            StreamDeckModel::Pedal => "Stream Deck Pedal",
        }
    }

    /// Get the expected input report size for this model
    ///
    /// Returns the number of bytes expected in a button state report,
    /// including the report ID byte.
    pub fn expected_report_size(&self) -> usize {
        match self {
            // Original and MK.2: 1 byte report ID + 15 button bytes
            StreamDeckModel::Original | StreamDeckModel::Mk2 => 17,
            // Mini: 1 byte report ID + 6 button bytes
            StreamDeckModel::Mini => 17, // Actually sends more bytes, but only first 7 matter
            // XL: 1 byte report ID + 32 button bytes + extra bytes
            StreamDeckModel::Xl => 36,
            // Pedal: 3 button bytes (no report ID in some firmware versions)
            StreamDeckModel::Pedal => 3,
        }
    }

    /// Check if this model uses the new key state report format
    ///
    /// Newer Stream Deck devices (MK.2 and later) may use a different
    /// report structure with additional header bytes.
    pub fn uses_new_report_format(&self) -> bool {
        matches!(self, StreamDeckModel::Mk2 | StreamDeckModel::Xl)
    }

    /// Get the byte offset where button data starts in the report
    ///
    /// Different models have different report structures:
    /// - Original/Mini: Report ID at byte 0, buttons start at byte 1
    /// - MK.2/XL: May have additional header bytes
    /// - Pedal: No report ID, buttons start at byte 0
    pub fn button_data_offset(&self) -> usize {
        match self {
            StreamDeckModel::Original | StreamDeckModel::Mini => 1,
            StreamDeckModel::Mk2 | StreamDeckModel::Xl => 4, // Skip additional header
            StreamDeckModel::Pedal => 0,
        }
    }
}

/// Stream Deck input report parser
#[derive(Debug)]
pub struct StreamDeckParser {
    /// The Stream Deck model
    model: StreamDeckModel,
    /// Previous button states for detecting press/release transitions
    previous_states: Vec<bool>,
}

impl StreamDeckParser {
    /// Create a new parser for the given model
    pub fn new(model: StreamDeckModel) -> Self {
        let button_count = model.button_count() as usize;
        Self {
            model,
            previous_states: vec![false; button_count],
        }
    }

    /// Create a parser from vendor/product IDs
    pub fn from_ids(vendor_id: u16, product_id: u16) -> Option<Self> {
        if vendor_id != ELGATO_VENDOR_ID {
            return None;
        }
        StreamDeckModel::from_product_id(product_id).map(Self::new)
    }

    /// Get the model
    pub fn model(&self) -> StreamDeckModel {
        self.model
    }

    /// Reset all button states to released
    pub fn reset_states(&mut self) {
        self.previous_states.fill(false);
    }

    /// Parse a button state input report
    ///
    /// Returns a vector of button events for any buttons that changed state.
    /// Only emits events for transitions (press or release).
    pub fn parse_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        // Dispatch to model-specific parser
        match self.model {
            StreamDeckModel::Pedal => self.parse_pedal_report(device_id, report, timestamp),
            _ => self.parse_standard_report(device_id, report, timestamp),
        }
    }

    /// Parse standard Stream Deck button report (Original, Mini, XL, MK.2)
    ///
    /// Report format varies by model:
    /// - Original/Mini: [report_id, button0, button1, ..., buttonN]
    /// - MK.2/XL: [report_id, extra1, extra2, extra3, button0, button1, ...]
    ///
    /// Each button byte is 0x00 (released) or 0x01 (pressed).
    fn parse_standard_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        let mut events = Vec::new();

        // Verify minimum report size
        if report.is_empty() {
            return events;
        }

        // Check for button state report ID (0x01)
        // Some models don't send the report ID in certain contexts
        let (has_report_id, button_data_start) = if report[0] == 0x01 {
            (true, self.model.button_data_offset())
        } else if self.model.uses_new_report_format() && report[0] == 0x00 {
            // New format devices may send 0x00 as first byte
            (true, self.model.button_data_offset())
        } else {
            // No recognizable report ID, treat all bytes as button data
            (false, 0)
        };

        // For MK.2 and XL, the report has extra header bytes after report ID
        // Report structure: [0x01, 0x00, 0x??, 0x??, button_states...]
        let button_data = if has_report_id && button_data_start < report.len() {
            &report[button_data_start..]
        } else if !has_report_id {
            report
        } else {
            return events;
        };

        let button_count = self.model.button_count() as usize;

        for (index, &button_byte) in button_data.iter().enumerate() {
            if index >= button_count {
                break;
            }

            let is_pressed = button_byte != 0;
            let was_pressed = self.previous_states.get(index).copied().unwrap_or(false);

            // Only emit event on state change
            if is_pressed != was_pressed {
                // Map the physical index to logical button code
                let button_code = self.map_button_index(index as u8);

                events.push(RawButtonEvent {
                    device_id: *device_id,
                    button_code,
                    pressed: is_pressed,
                    timestamp,
                });

                tracing::debug!(
                    "{} button {} (physical {}) {}",
                    self.model.name(),
                    button_code,
                    index,
                    if is_pressed { "pressed" } else { "released" }
                );
            }

            // Update state
            if index < self.previous_states.len() {
                self.previous_states[index] = is_pressed;
            }
        }

        events
    }

    /// Parse Stream Deck Pedal button report
    ///
    /// The pedal uses a simpler format:
    /// - 3 bytes, one per pedal (left, middle, right)
    /// - No report ID prefix
    /// - Each byte is 0x00 (released) or 0x01 (pressed)
    fn parse_pedal_report(
        &mut self,
        device_id: &Uuid,
        report: &[u8],
        timestamp: Instant,
    ) -> Vec<RawButtonEvent> {
        let mut events = Vec::new();

        // Pedal sends 3 bytes for 3 pedals
        for (index, &button_byte) in report.iter().take(3).enumerate() {
            let is_pressed = button_byte != 0;
            let was_pressed = self.previous_states.get(index).copied().unwrap_or(false);

            if is_pressed != was_pressed {
                // Pedals are labeled Left=0, Middle=1, Right=2
                let button_code = index as u8;

                events.push(RawButtonEvent {
                    device_id: *device_id,
                    button_code,
                    pressed: is_pressed,
                    timestamp,
                });

                let pedal_name = match index {
                    0 => "left",
                    1 => "middle",
                    2 => "right",
                    _ => "unknown",
                };

                tracing::debug!(
                    "Stream Deck Pedal {} pedal {}",
                    pedal_name,
                    if is_pressed { "pressed" } else { "released" }
                );
            }

            if index < self.previous_states.len() {
                self.previous_states[index] = is_pressed;
            }
        }

        events
    }

    /// Map a physical button index to a logical button code
    ///
    /// The Stream Deck button indices in the HID report are in a specific order
    /// that may not match the visual layout. This function normalizes the index
    /// to a consistent logical scheme (left-to-right, top-to-bottom).
    ///
    /// For most models, the physical index matches the logical index.
    /// The Original Stream Deck has buttons numbered 0-14 left-to-right,
    /// top-to-bottom which matches our logical scheme.
    fn map_button_index(&self, physical_index: u8) -> u8 {
        // Stream Deck button layout is already left-to-right, top-to-bottom
        // No remapping needed for current models
        //
        // Layout for 15-button models (3x5 grid):
        //  0  1  2  3  4
        //  5  6  7  8  9
        // 10 11 12 13 14
        //
        // Layout for 6-button Mini (2x3 grid):
        //  0  1  2
        //  3  4  5
        //
        // Layout for 32-button XL (4x8 grid):
        //  0  1  2  3  4  5  6  7
        //  8  9 10 11 12 13 14 15
        // 16 17 18 19 20 21 22 23
        // 24 25 26 27 28 29 30 31

        physical_index
    }

    /// Convert a button code to grid coordinates (row, col)
    ///
    /// Useful for UI display and button mapping interfaces.
    pub fn button_to_grid(&self, button_code: u8) -> Option<(u8, u8)> {
        let (rows, cols) = self.model.grid_layout();
        let total = rows * cols;

        if button_code >= total {
            return None;
        }

        let row = button_code / cols;
        let col = button_code % cols;
        Some((row, col))
    }

    /// Convert grid coordinates to button code
    pub fn grid_to_button(&self, row: u8, col: u8) -> Option<u8> {
        let (rows, cols) = self.model.grid_layout();

        if row >= rows || col >= cols {
            return None;
        }

        Some(row * cols + col)
    }

    /// Get a human-readable button label
    ///
    /// Returns a label like "Button 1" or "Left Pedal" for display purposes.
    pub fn button_label(&self, button_code: u8) -> String {
        match self.model {
            StreamDeckModel::Pedal => match button_code {
                0 => "Left Pedal".to_string(),
                1 => "Middle Pedal".to_string(),
                2 => "Right Pedal".to_string(),
                _ => format!("Pedal {}", button_code + 1),
            },
            _ => {
                if let Some((row, col)) = self.button_to_grid(button_code) {
                    format!("Button R{}C{}", row + 1, col + 1)
                } else {
                    format!("Button {}", button_code + 1)
                }
            }
        }
    }
}

/// Check if a report is a valid Stream Deck button state report
pub fn is_button_report(report: &[u8], model: StreamDeckModel) -> bool {
    if report.is_empty() {
        return false;
    }

    match model {
        StreamDeckModel::Pedal => {
            // Pedal just sends 3 bytes
            report.len() >= 3
        }
        StreamDeckModel::Original | StreamDeckModel::Mini => {
            // Standard format: starts with report ID 0x01
            report[0] == 0x01 && report.len() >= 2
        }
        StreamDeckModel::Mk2 | StreamDeckModel::Xl => {
            // New format: report ID 0x01 or 0x00 with extra header
            (report[0] == 0x01 || report[0] == 0x00) && report.len() >= 5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_from_product_id() {
        assert_eq!(
            StreamDeckModel::from_product_id(0x0060),
            Some(StreamDeckModel::Original)
        );
        assert_eq!(
            StreamDeckModel::from_product_id(0x006C),
            Some(StreamDeckModel::Mini)
        );
        assert_eq!(
            StreamDeckModel::from_product_id(0x006D),
            Some(StreamDeckModel::Xl)
        );
        assert_eq!(
            StreamDeckModel::from_product_id(0x0080),
            Some(StreamDeckModel::Mk2)
        );
        assert_eq!(
            StreamDeckModel::from_product_id(0x0086),
            Some(StreamDeckModel::Pedal)
        );
        assert_eq!(StreamDeckModel::from_product_id(0x9999), None);
    }

    #[test]
    fn test_model_button_count() {
        assert_eq!(StreamDeckModel::Original.button_count(), 15);
        assert_eq!(StreamDeckModel::Mini.button_count(), 6);
        assert_eq!(StreamDeckModel::Xl.button_count(), 32);
        assert_eq!(StreamDeckModel::Mk2.button_count(), 15);
        assert_eq!(StreamDeckModel::Pedal.button_count(), 3);
    }

    #[test]
    fn test_model_grid_layout() {
        assert_eq!(StreamDeckModel::Original.grid_layout(), (3, 5));
        assert_eq!(StreamDeckModel::Mini.grid_layout(), (2, 3));
        assert_eq!(StreamDeckModel::Xl.grid_layout(), (4, 8));
        assert_eq!(StreamDeckModel::Mk2.grid_layout(), (3, 5));
        assert_eq!(StreamDeckModel::Pedal.grid_layout(), (1, 3));
    }

    #[test]
    fn test_is_stream_deck() {
        assert!(StreamDeckModel::is_stream_deck(0x0FD9, 0x0060));
        assert!(StreamDeckModel::is_stream_deck(0x0FD9, 0x0086));
        assert!(!StreamDeckModel::is_stream_deck(0x1234, 0x0060));
        assert!(!StreamDeckModel::is_stream_deck(0x0FD9, 0x9999));
    }

    #[test]
    fn test_parser_creation() {
        let parser = StreamDeckParser::new(StreamDeckModel::Original);
        assert_eq!(parser.model(), StreamDeckModel::Original);

        let parser = StreamDeckParser::from_ids(0x0FD9, 0x006C);
        assert!(parser.is_some());
        assert_eq!(parser.unwrap().model(), StreamDeckModel::Mini);

        let parser = StreamDeckParser::from_ids(0x1234, 0x5678);
        assert!(parser.is_none());
    }

    #[test]
    fn test_parse_original_button_press() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Original);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Report: button 0 pressed
        let report = [0x01, 0x01, 0x00, 0x00, 0x00, 0x00];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_parse_original_button_release() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Original);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // First press button 0
        let report1 = [0x01, 0x01, 0x00, 0x00, 0x00, 0x00];
        let _ = parser.parse_report(&device_id, &report1, timestamp);

        // Then release button 0
        let report2 = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        let events = parser.parse_report(&device_id, &report2, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(!events[0].pressed);
    }

    #[test]
    fn test_parse_multiple_buttons() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Original);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Buttons 0, 2, and 4 pressed
        let report = [0x01, 0x01, 0x00, 0x01, 0x00, 0x01];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 3);

        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&2));
        assert!(codes.contains(&4));
    }

    #[test]
    fn test_no_event_when_no_change() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Original);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press button 0
        let report = [0x01, 0x01, 0x00, 0x00];
        let _ = parser.parse_report(&device_id, &report, timestamp);

        // Same state again
        let events = parser.parse_report(&device_id, &report, timestamp);
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_pedal() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Pedal);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Middle pedal pressed
        let report = [0x00, 0x01, 0x00];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 1);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_parse_all_pedals_pressed() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Pedal);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // All pedals pressed
        let report = [0x01, 0x01, 0x01];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_parse_mk2_format() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Mk2);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // MK.2 format: [report_id, 0x00, extra, extra, button_states...]
        let report = [0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].button_code, 0);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_parse_xl_multiple_buttons() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Xl);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // XL format with buttons 0, 7, and 31 pressed
        let mut report = vec![0x01, 0x00, 0x00, 0x00]; // Header
        report.resize(4 + 32, 0x00); // Add 32 button bytes
        report[4] = 0x01; // Button 0
        report[11] = 0x01; // Button 7
        report[35] = 0x01; // Button 31

        let events = parser.parse_report(&device_id, &report, timestamp);

        assert_eq!(events.len(), 3);
        let codes: Vec<u8> = events.iter().map(|e| e.button_code).collect();
        assert!(codes.contains(&0));
        assert!(codes.contains(&7));
        assert!(codes.contains(&31));
    }

    #[test]
    fn test_button_to_grid() {
        let parser = StreamDeckParser::new(StreamDeckModel::Original);

        // First button is top-left (0, 0)
        assert_eq!(parser.button_to_grid(0), Some((0, 0)));
        // Button 4 is top-right
        assert_eq!(parser.button_to_grid(4), Some((0, 4)));
        // Button 5 is second row, first column
        assert_eq!(parser.button_to_grid(5), Some((1, 0)));
        // Button 14 is bottom-right
        assert_eq!(parser.button_to_grid(14), Some((2, 4)));
        // Button 15 is out of range
        assert_eq!(parser.button_to_grid(15), None);
    }

    #[test]
    fn test_grid_to_button() {
        let parser = StreamDeckParser::new(StreamDeckModel::Original);

        assert_eq!(parser.grid_to_button(0, 0), Some(0));
        assert_eq!(parser.grid_to_button(0, 4), Some(4));
        assert_eq!(parser.grid_to_button(1, 0), Some(5));
        assert_eq!(parser.grid_to_button(2, 4), Some(14));
        assert_eq!(parser.grid_to_button(3, 0), None); // Row out of range
        assert_eq!(parser.grid_to_button(0, 5), None); // Column out of range
    }

    #[test]
    fn test_button_label() {
        let parser = StreamDeckParser::new(StreamDeckModel::Original);
        assert_eq!(parser.button_label(0), "Button R1C1");
        assert_eq!(parser.button_label(4), "Button R1C5");
        assert_eq!(parser.button_label(5), "Button R2C1");

        let parser = StreamDeckParser::new(StreamDeckModel::Pedal);
        assert_eq!(parser.button_label(0), "Left Pedal");
        assert_eq!(parser.button_label(1), "Middle Pedal");
        assert_eq!(parser.button_label(2), "Right Pedal");
    }

    #[test]
    fn test_reset_states() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Mini);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press button 0
        let report = [0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        let _ = parser.parse_report(&device_id, &report, timestamp);

        // Reset states
        parser.reset_states();

        // Same report should now generate a press event again
        let events = parser.parse_report(&device_id, &report, timestamp);
        assert_eq!(events.len(), 1);
        assert!(events[0].pressed);
    }

    #[test]
    fn test_is_button_report() {
        // Original format
        assert!(is_button_report(&[0x01, 0x00], StreamDeckModel::Original));
        assert!(!is_button_report(&[0x02, 0x00], StreamDeckModel::Original));
        assert!(!is_button_report(&[], StreamDeckModel::Original));

        // Pedal format
        assert!(is_button_report(&[0x00, 0x00, 0x00], StreamDeckModel::Pedal));
        assert!(is_button_report(&[0x01, 0x00, 0x00], StreamDeckModel::Pedal));
        assert!(!is_button_report(&[0x00, 0x00], StreamDeckModel::Pedal));

        // MK.2 format
        assert!(is_button_report(
            &[0x01, 0x00, 0x00, 0x00, 0x00],
            StreamDeckModel::Mk2
        ));
        assert!(is_button_report(
            &[0x00, 0x00, 0x00, 0x00, 0x00],
            StreamDeckModel::Mk2
        ));
        assert!(!is_button_report(&[0x01, 0x00, 0x00], StreamDeckModel::Mk2));
    }

    #[test]
    fn test_empty_report() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Original);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        let events = parser.parse_report(&device_id, &[], timestamp);
        assert!(events.is_empty());
    }

    #[test]
    fn test_mini_button_count() {
        let mut parser = StreamDeckParser::new(StreamDeckModel::Mini);
        let device_id = Uuid::new_v4();
        let timestamp = Instant::now();

        // Press all 6 buttons
        let report = [0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01];
        let events = parser.parse_report(&device_id, &report, timestamp);

        // Should only get 6 events (not 7 even though report has 7 bytes)
        assert_eq!(events.len(), 6);
    }
}
