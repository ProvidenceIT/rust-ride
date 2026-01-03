//! QR code generation for companion app pairing.
//!
//! This module provides QR code generation for easy mobile app pairing.
//! The QR code contains the WebSocket connection URL and optional PIN
//! for authentication.
//!
//! ## Supported Formats
//!
//! - **ASCII**: Text-based representation for terminal display
//! - **SVG**: Scalable vector graphics for desktop UI rendering
//!
//! ## Connection Data Format
//!
//! The QR code encodes a JSON object with connection information:
//!
//! ```json
//! {
//!   "url": "ws://192.168.1.100:9876",
//!   "pin": "123456",
//!   "version": "1"
//! }
//! ```
//!
//! If PIN authentication is not required, the `pin` field is omitted.

use qrcode::render::svg;
use qrcode::render::unicode;
use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};

use super::discovery::COMPANION_PROTOCOL_VERSION;

/// QR code generation error types.
#[derive(Debug, thiserror::Error)]
pub enum QrCodeError {
    /// Failed to generate QR code from data.
    #[error("Failed to generate QR code: {0}")]
    GenerationFailed(String),

    /// Failed to serialize connection data.
    #[error("Failed to serialize connection data: {0}")]
    SerializationFailed(String),
}

/// Connection data encoded in the QR code.
///
/// This structure is serialized to JSON and encoded in the QR code
/// for the mobile app to scan and parse.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionConnectionData {
    /// WebSocket URL (e.g., "ws://192.168.1.100:9876")
    pub url: String,

    /// Optional PIN for authentication (6 digits)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,

    /// Protocol version for compatibility checking
    pub version: String,
}

impl CompanionConnectionData {
    /// Create new connection data with a PIN.
    pub fn with_pin(url: String, pin: String) -> Self {
        Self {
            url,
            pin: Some(pin),
            version: COMPANION_PROTOCOL_VERSION.to_string(),
        }
    }

    /// Create new connection data without a PIN.
    pub fn without_pin(url: String) -> Self {
        Self {
            url,
            pin: None,
            version: COMPANION_PROTOCOL_VERSION.to_string(),
        }
    }

    /// Convert to JSON string for QR code encoding.
    pub fn to_json(&self) -> Result<String, QrCodeError> {
        serde_json::to_string(self).map_err(|e| QrCodeError::SerializationFailed(e.to_string()))
    }

    /// Parse from JSON string (for mobile app use).
    pub fn from_json(json: &str) -> Result<Self, QrCodeError> {
        serde_json::from_str(json).map_err(|e| QrCodeError::SerializationFailed(e.to_string()))
    }
}

/// QR code generator for companion app pairing.
///
/// Generates QR codes in various formats containing the connection
/// URL and optional PIN for easy mobile app pairing.
#[derive(Debug, Clone)]
pub struct CompanionQrCode {
    /// The connection data to encode.
    connection_data: CompanionConnectionData,
    /// The generated QR code.
    qr_code: QrCode,
}

impl CompanionQrCode {
    /// Create a new QR code from connection data.
    ///
    /// # Arguments
    ///
    /// * `connection_data` - The connection information to encode
    ///
    /// # Returns
    ///
    /// A new `CompanionQrCode` or an error if generation fails.
    pub fn new(connection_data: CompanionConnectionData) -> Result<Self, QrCodeError> {
        let json = connection_data.to_json()?;
        let qr_code = QrCode::with_error_correction_level(&json, EcLevel::M)
            .map_err(|e| QrCodeError::GenerationFailed(e.to_string()))?;

        Ok(Self {
            connection_data,
            qr_code,
        })
    }

    /// Create a QR code from URL and optional PIN.
    ///
    /// This is a convenience constructor that creates the connection
    /// data internally.
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL (e.g., "ws://192.168.1.100:9876")
    /// * `pin` - Optional PIN for authentication
    ///
    /// # Returns
    ///
    /// A new `CompanionQrCode` or an error if generation fails.
    pub fn from_url_and_pin(url: String, pin: Option<String>) -> Result<Self, QrCodeError> {
        let connection_data = match pin {
            Some(p) => CompanionConnectionData::with_pin(url, p),
            None => CompanionConnectionData::without_pin(url),
        };
        Self::new(connection_data)
    }

    /// Get the connection data encoded in this QR code.
    pub fn connection_data(&self) -> &CompanionConnectionData {
        &self.connection_data
    }

    /// Render the QR code as an ASCII string.
    ///
    /// Uses Unicode block characters for a compact representation
    /// suitable for terminal display.
    ///
    /// # Returns
    ///
    /// A string containing the ASCII art representation of the QR code.
    pub fn to_ascii(&self) -> String {
        self.qr_code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .quiet_zone(true)
            .build()
    }

    /// Render the QR code as an ASCII string with custom characters.
    ///
    /// # Arguments
    ///
    /// * `dark` - Character to use for dark modules
    /// * `light` - Character to use for light modules
    ///
    /// # Returns
    ///
    /// A string containing the ASCII art representation of the QR code.
    pub fn to_ascii_custom(&self, dark: char, light: char) -> String {
        self.qr_code
            .render::<char>()
            .dark_color(dark)
            .light_color(light)
            .quiet_zone(true)
            .build()
    }

    /// Render the QR code as an SVG string.
    ///
    /// The SVG is suitable for display in desktop app UI and
    /// scales cleanly to any size.
    ///
    /// # Returns
    ///
    /// A string containing the SVG representation of the QR code.
    pub fn to_svg(&self) -> String {
        self.qr_code
            .render()
            .min_dimensions(200, 200)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .quiet_zone(true)
            .build()
    }

    /// Render the QR code as an SVG string with custom colors.
    ///
    /// # Arguments
    ///
    /// * `dark_color` - Hex color for dark modules (e.g., "#000000")
    /// * `light_color` - Hex color for light modules (e.g., "#ffffff")
    /// * `min_size` - Minimum dimensions in pixels
    ///
    /// # Returns
    ///
    /// A string containing the SVG representation of the QR code.
    pub fn to_svg_custom(&self, dark_color: &str, light_color: &str, min_size: u32) -> String {
        self.qr_code
            .render()
            .min_dimensions(min_size, min_size)
            .dark_color(svg::Color(dark_color))
            .light_color(svg::Color(light_color))
            .quiet_zone(true)
            .build()
    }

    /// Get the size of the QR code in modules.
    ///
    /// A module is a single square in the QR code.
    pub fn module_count(&self) -> usize {
        self.qr_code.width()
    }

    /// Get the error correction level used.
    pub fn error_correction_level(&self) -> EcLevel {
        EcLevel::M
    }

    /// Get the raw module data for custom rendering.
    ///
    /// Returns a 2D vector where `true` represents a dark module
    /// and `false` represents a light module. This is useful for
    /// rendering the QR code in custom UI frameworks like egui.
    ///
    /// The outer vector contains rows, and each inner vector contains
    /// the columns for that row. The size is `module_count() x module_count()`.
    ///
    /// # Returns
    ///
    /// A 2D vector of booleans representing the QR code modules.
    pub fn to_module_data(&self) -> Vec<Vec<bool>> {
        let size = self.module_count();
        let mut data = Vec::with_capacity(size);

        for y in 0..size {
            let mut row = Vec::with_capacity(size);
            for x in 0..size {
                // qrcode crate: Dark = true/filled, Light = false/empty
                row.push(self.qr_code[(x, y)] == qrcode::Color::Dark);
            }
            data.push(row);
        }

        data
    }

    /// Get the WebSocket URL encoded in this QR code.
    pub fn url(&self) -> &str {
        &self.connection_data.url
    }
}

/// Generate a QR code for companion app pairing.
///
/// This is a convenience function that creates a QR code from the
/// server's URL and optional PIN.
///
/// # Arguments
///
/// * `url` - The WebSocket URL (e.g., "ws://192.168.1.100:9876")
/// * `pin` - Optional PIN for authentication
///
/// # Returns
///
/// A `CompanionQrCode` that can be rendered in various formats.
///
/// # Example
///
/// ```ignore
/// use rustride::companion::qr::generate_pairing_qr_code;
///
/// let qr = generate_pairing_qr_code(
///     "ws://192.168.1.100:9876".to_string(),
///     Some("123456".to_string()),
/// ).unwrap();
///
/// // Print ASCII art
/// println!("{}", qr.to_ascii());
///
/// // Get SVG for UI rendering
/// let svg = qr.to_svg();
/// ```
pub fn generate_pairing_qr_code(
    url: String,
    pin: Option<String>,
) -> Result<CompanionQrCode, QrCodeError> {
    CompanionQrCode::from_url_and_pin(url, pin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_data_with_pin() {
        let data = CompanionConnectionData::with_pin(
            "ws://192.168.1.100:9876".to_string(),
            "123456".to_string(),
        );
        assert_eq!(data.url, "ws://192.168.1.100:9876");
        assert_eq!(data.pin, Some("123456".to_string()));
        assert_eq!(data.version, COMPANION_PROTOCOL_VERSION);
    }

    #[test]
    fn test_connection_data_without_pin() {
        let data = CompanionConnectionData::without_pin("ws://192.168.1.100:9876".to_string());
        assert_eq!(data.url, "ws://192.168.1.100:9876");
        assert!(data.pin.is_none());
    }

    #[test]
    fn test_connection_data_json_roundtrip() {
        let original = CompanionConnectionData::with_pin(
            "ws://192.168.1.100:9876".to_string(),
            "654321".to_string(),
        );
        let json = original.to_json().unwrap();
        let parsed = CompanionConnectionData::from_json(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_connection_data_json_without_pin() {
        let data = CompanionConnectionData::without_pin("ws://10.0.0.5:9876".to_string());
        let json = data.to_json().unwrap();
        // PIN field should be omitted, not null
        assert!(!json.contains("pin"));
        assert!(json.contains("url"));
        assert!(json.contains("version"));
    }

    #[test]
    fn test_qr_code_generation() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            Some("123456".to_string()),
        )
        .unwrap();

        // Should have a reasonable size
        assert!(qr.module_count() > 0);
        assert!(qr.module_count() < 200); // QR codes are typically under 177x177
    }

    #[test]
    fn test_qr_code_ascii_output() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            None,
        )
        .unwrap();

        let ascii = qr.to_ascii();
        assert!(!ascii.is_empty());
        // ASCII output should contain multiple lines
        assert!(ascii.lines().count() > 5);
    }

    #[test]
    fn test_qr_code_ascii_custom() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            None,
        )
        .unwrap();

        let ascii = qr.to_ascii_custom('#', ' ');
        assert!(!ascii.is_empty());
        assert!(ascii.contains('#') || ascii.contains(' '));
    }

    #[test]
    fn test_qr_code_svg_output() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            Some("999999".to_string()),
        )
        .unwrap();

        let svg = qr.to_svg();
        assert!(!svg.is_empty());
        // SVG should have proper structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("viewBox"));
    }

    #[test]
    fn test_qr_code_svg_custom_colors() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            None,
        )
        .unwrap();

        let svg = qr.to_svg_custom("#1a1a1a", "#f0f0f0", 300);
        assert!(svg.contains("#1a1a1a"));
        assert!(svg.contains("#f0f0f0"));
    }

    #[test]
    fn test_generate_pairing_qr_code() {
        let qr = generate_pairing_qr_code(
            "ws://10.0.0.1:9876".to_string(),
            Some("555555".to_string()),
        )
        .unwrap();

        let data = qr.connection_data();
        assert_eq!(data.url, "ws://10.0.0.1:9876");
        assert_eq!(data.pin, Some("555555".to_string()));
    }

    #[test]
    fn test_connection_data_accessible() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            Some("111111".to_string()),
        )
        .unwrap();

        let data = qr.connection_data();
        assert_eq!(data.url, "ws://192.168.1.100:9876");
        assert_eq!(data.pin.as_deref(), Some("111111"));
    }

    #[test]
    fn test_error_correction_level() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://localhost:9876".to_string(),
            None,
        )
        .unwrap();

        assert_eq!(qr.error_correction_level(), EcLevel::M);
    }

    #[test]
    fn test_to_module_data() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://192.168.1.100:9876".to_string(),
            Some("123456".to_string()),
        )
        .unwrap();

        let module_data = qr.to_module_data();

        // Should be a square 2D array
        let size = qr.module_count();
        assert_eq!(module_data.len(), size);
        for row in &module_data {
            assert_eq!(row.len(), size);
        }

        // Should contain both dark (true) and light (false) modules
        let has_dark = module_data.iter().any(|row| row.iter().any(|&m| m));
        let has_light = module_data.iter().any(|row| row.iter().any(|&m| !m));
        assert!(has_dark);
        assert!(has_light);
    }

    #[test]
    fn test_url_accessor() {
        let qr = CompanionQrCode::from_url_and_pin(
            "ws://10.0.0.5:9876".to_string(),
            None,
        )
        .unwrap();

        assert_eq!(qr.url(), "ws://10.0.0.5:9876");
    }
}
