//! Serialization for custom routes and world data.

use super::CustomRoute;
#[cfg(test)]
use super::RoutePoint;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

/// File format version
const FORMAT_VERSION: u32 = 1;

/// File header for custom route files
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileHeader {
    /// Magic identifier
    magic: [u8; 4],
    /// Format version
    version: u32,
    /// Route name
    name: String,
    /// Point count
    point_count: u32,
    /// Object count
    object_count: u32,
}

impl FileHeader {
    fn new(route: &CustomRoute) -> Self {
        Self {
            magic: *b"RRTE", // RustRide routE
            version: FORMAT_VERSION,
            name: route.name.clone(),
            point_count: route.points.len() as u32,
            object_count: route.objects.len() as u32,
        }
    }

    fn validate(&self) -> Result<(), SerializationError> {
        if &self.magic != b"RRTE" {
            return Err(SerializationError::InvalidMagic);
        }
        if self.version > FORMAT_VERSION {
            return Err(SerializationError::UnsupportedVersion(self.version));
        }
        Ok(())
    }
}

/// Serialization errors
#[derive(Debug, Clone)]
pub enum SerializationError {
    /// Invalid magic bytes
    InvalidMagic,
    /// Unsupported format version
    UnsupportedVersion(u32),
    /// IO error
    IoError(String),
    /// JSON/serialization error
    SerdeError(String),
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid file format"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported version: {}", v),
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::SerdeError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for SerializationError {}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Native binary format
    Binary,
    /// JSON format
    Json,
    /// GPX format (route points only)
    Gpx,
}

impl ExportFormat {
    /// Get file extension for format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Binary => "rrt",
            Self::Json => "json",
            Self::Gpx => "gpx",
        }
    }
}

/// Save route to binary format
pub fn save_binary<W: Write>(
    route: &CustomRoute,
    writer: &mut W,
) -> Result<(), SerializationError> {
    let header = FileHeader::new(route);

    // Serialize header
    let header_bytes = bincode_serialize(&header)?;
    writer
        .write_all(&(header_bytes.len() as u32).to_le_bytes())
        .map_err(|e| SerializationError::IoError(e.to_string()))?;
    writer
        .write_all(&header_bytes)
        .map_err(|e| SerializationError::IoError(e.to_string()))?;

    // Serialize route data
    let route_bytes = bincode_serialize(route)?;
    writer
        .write_all(&route_bytes)
        .map_err(|e| SerializationError::IoError(e.to_string()))?;

    Ok(())
}

/// Load route from binary format
pub fn load_binary<R: Read>(reader: &mut R) -> Result<CustomRoute, SerializationError> {
    // Read header length
    let mut len_bytes = [0u8; 4];
    reader
        .read_exact(&mut len_bytes)
        .map_err(|e| SerializationError::IoError(e.to_string()))?;
    let header_len = u32::from_le_bytes(len_bytes) as usize;

    // Read and validate header
    let mut header_bytes = vec![0u8; header_len];
    reader
        .read_exact(&mut header_bytes)
        .map_err(|e| SerializationError::IoError(e.to_string()))?;
    let header: FileHeader = bincode_deserialize(&header_bytes)?;
    header.validate()?;

    // Read route data
    let mut route_bytes = Vec::new();
    reader
        .read_to_end(&mut route_bytes)
        .map_err(|e| SerializationError::IoError(e.to_string()))?;
    let route: CustomRoute = bincode_deserialize(&route_bytes)?;

    Ok(route)
}

/// Save route to JSON format
pub fn save_json<W: Write>(route: &CustomRoute, writer: &mut W) -> Result<(), SerializationError> {
    serde_json::to_writer_pretty(writer, route)
        .map_err(|e| SerializationError::SerdeError(e.to_string()))
}

/// Load route from JSON format
pub fn load_json<R: Read>(reader: &mut R) -> Result<CustomRoute, SerializationError> {
    serde_json::from_reader(reader).map_err(|e| SerializationError::SerdeError(e.to_string()))
}

/// Export route to GPX format
pub fn export_gpx<W: Write>(route: &CustomRoute, writer: &mut W) -> Result<(), SerializationError> {
    let mut gpx = String::new();
    gpx.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    gpx.push('\n');
    gpx.push_str(r#"<gpx version="1.1" creator="RustRide">"#);
    gpx.push('\n');
    gpx.push_str("  <trk>\n");
    gpx.push_str(&format!("    <name>{}</name>\n", escape_xml(&route.name)));
    gpx.push_str("    <trkseg>\n");

    for point in &route.points {
        gpx.push_str(&format!(
            "      <trkpt lat=\"{}\" lon=\"{}\">\n",
            point.latitude, point.longitude
        ));
        gpx.push_str(&format!("        <ele>{}</ele>\n", point.elevation));
        gpx.push_str("      </trkpt>\n");
    }

    gpx.push_str("    </trkseg>\n");
    gpx.push_str("  </trk>\n");
    gpx.push_str("</gpx>\n");

    writer
        .write_all(gpx.as_bytes())
        .map_err(|e| SerializationError::IoError(e.to_string()))
}

/// Save route to file with format detection
pub fn save_to_file(route: &CustomRoute, path: &Path) -> Result<(), SerializationError> {
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| match e.to_lowercase().as_str() {
            "json" => ExportFormat::Json,
            "gpx" => ExportFormat::Gpx,
            _ => ExportFormat::Binary,
        })
        .unwrap_or(ExportFormat::Binary);

    let mut file =
        std::fs::File::create(path).map_err(|e| SerializationError::IoError(e.to_string()))?;

    match format {
        ExportFormat::Binary => save_binary(route, &mut file),
        ExportFormat::Json => save_json(route, &mut file),
        ExportFormat::Gpx => export_gpx(route, &mut file),
    }
}

/// Load route from file with format detection
pub fn load_from_file(path: &Path) -> Result<CustomRoute, SerializationError> {
    let mut file =
        std::fs::File::open(path).map_err(|e| SerializationError::IoError(e.to_string()))?;

    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| match e.to_lowercase().as_str() {
            "json" => ExportFormat::Json,
            _ => ExportFormat::Binary,
        })
        .unwrap_or(ExportFormat::Binary);

    match format {
        ExportFormat::Binary => load_binary(&mut file),
        ExportFormat::Json => load_json(&mut file),
        ExportFormat::Gpx => Err(SerializationError::IoError(
            "GPX import not supported here - use import module".to_string(),
        )),
    }
}

/// Simple bincode-like serialization (would use bincode crate in full impl)
fn bincode_serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, SerializationError> {
    // Using JSON as placeholder for bincode
    serde_json::to_vec(value).map_err(|e| SerializationError::SerdeError(e.to_string()))
}

/// Simple bincode-like deserialization
fn bincode_deserialize<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
) -> Result<T, SerializationError> {
    serde_json::from_slice(bytes).map_err(|e| SerializationError::SerdeError(e.to_string()))
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_roundtrip() {
        let mut route = CustomRoute::new("Test Route".to_string());
        route.add_point(RoutePoint::new(45.0, 6.0, 100.0));
        route.add_point(RoutePoint::new(45.001, 6.001, 150.0));

        let mut buffer = Vec::new();
        save_json(&route, &mut buffer).unwrap();

        let loaded = load_json(&mut buffer.as_slice()).unwrap();
        assert_eq!(loaded.name, "Test Route");
        assert_eq!(loaded.points.len(), 2);
    }

    #[test]
    fn test_gpx_export() {
        let mut route = CustomRoute::new("GPX Test".to_string());
        route.add_point(RoutePoint::new(45.0, 6.0, 100.0));

        let mut buffer = Vec::new();
        export_gpx(&route, &mut buffer).unwrap();

        let gpx = String::from_utf8(buffer).unwrap();
        assert!(gpx.contains("GPX Test"));
        assert!(gpx.contains("45"));
        assert!(gpx.contains("100"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Test & Route"), "Test &amp; Route");
        assert_eq!(escape_xml("<route>"), "&lt;route&gt;");
    }
}
