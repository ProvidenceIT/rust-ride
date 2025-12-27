//! Leaderboard export and import.
//!
//! Provides JSON and CSV export, and import with name matching.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::storage::Database;

/// Export format for leaderboard data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardExport {
    pub segment: SegmentExport,
    pub efforts: Vec<EffortExport>,
    pub exported_at: DateTime<Utc>,
    pub export_version: String,
}

/// Segment data for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentExport {
    pub id: String,
    pub name: String,
    pub world_id: String,
    pub distance_m: f64,
    pub elevation_gain_m: f64,
}

/// Effort data for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortExport {
    pub rank: u32,
    pub rider_name: String,
    pub elapsed_time_ms: u32,
    pub avg_power_watts: Option<u16>,
    pub recorded_at: DateTime<Utc>,
}

/// Import result.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported_count: u32,
    pub duplicate_count: u32,
    pub name_conflicts: Vec<NameConflict>,
}

/// Name conflict during import.
#[derive(Debug, Clone)]
pub struct NameConflict {
    pub imported_name: String,
    pub existing_rider_id: Option<Uuid>,
    pub effort_count: u32,
}

/// Conflict resolution.
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    MergeWithExisting(Uuid),
    CreateNew,
    Skip,
}

/// Leaderboard exporter.
pub struct LeaderboardExporter {
    db: Arc<Database>,
}

impl LeaderboardExporter {
    /// Create a new exporter.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Export leaderboard to JSON.
    pub fn export_json(&self, segment_id: Uuid) -> Result<String, ExportError> {
        let export = self.build_export(segment_id)?;
        serde_json::to_string_pretty(&export)
            .map_err(|e| ExportError::SerializationFailed(e.to_string()))
    }

    /// Export leaderboard to CSV.
    pub fn export_csv(&self, segment_id: Uuid) -> Result<String, ExportError> {
        let export = self.build_export(segment_id)?;

        let mut csv = String::new();
        csv.push_str("rank,rider_name,elapsed_time_ms,avg_power_watts,recorded_at\n");

        for effort in &export.efforts {
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                effort.rank,
                escape_csv(&effort.rider_name),
                effort.elapsed_time_ms,
                effort
                    .avg_power_watts
                    .map_or(String::new(), |p| p.to_string()),
                effort.recorded_at.to_rfc3339(),
            ));
        }

        Ok(csv)
    }

    /// Import efforts from JSON.
    pub fn import_json(&self, json_content: &str) -> Result<ImportResult, ExportError> {
        let export: LeaderboardExport = serde_json::from_str(json_content)
            .map_err(|e| ExportError::ParseError(e.to_string()))?;

        let conn = self.db.connection();

        // Find or create segment
        let segment_id = self.find_or_create_segment(&export.segment)?;

        let mut imported_count = 0u32;
        let mut duplicate_count = 0u32;
        let mut name_conflicts = Vec::new();

        // Group efforts by rider name
        let mut efforts_by_name: std::collections::HashMap<String, Vec<&EffortExport>> =
            std::collections::HashMap::new();
        for effort in &export.efforts {
            efforts_by_name
                .entry(effort.rider_name.clone())
                .or_default()
                .push(effort);
        }

        // Process each rider
        for (rider_name, efforts) in efforts_by_name {
            // Check for existing rider with same name
            let existing_rider_id = self.find_rider_by_name(&rider_name)?;

            if let Some(rider_id) = existing_rider_id {
                // Check if this is a name conflict (might be different person)
                name_conflicts.push(NameConflict {
                    imported_name: rider_name.clone(),
                    existing_rider_id: Some(rider_id),
                    effort_count: efforts.len() as u32,
                });
            }

            // For now, import with a new rider ID (actual merge would require UI confirmation)
            let import_rider_id = Uuid::new_v4();

            for effort in efforts {
                // Check for duplicate
                if self.is_duplicate_effort(segment_id, &rider_name, effort.elapsed_time_ms)? {
                    duplicate_count += 1;
                    continue;
                }

                // Insert effort
                let effort_id = Uuid::new_v4();
                conn.execute(
                    "INSERT INTO social_segment_efforts (id, segment_id, rider_id, elapsed_time_ms, avg_power_watts, recorded_at, imported, import_source_name)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)",
                    rusqlite::params![
                        effort_id.to_string(),
                        segment_id.to_string(),
                        import_rider_id.to_string(),
                        effort.elapsed_time_ms,
                        effort.avg_power_watts,
                        effort.recorded_at.to_rfc3339(),
                        rider_name,
                    ],
                )
                .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

                imported_count += 1;
            }
        }

        Ok(ImportResult {
            imported_count,
            duplicate_count,
            name_conflicts,
        })
    }

    fn build_export(&self, segment_id: Uuid) -> Result<LeaderboardExport, ExportError> {
        let conn = self.db.connection();

        // Get segment
        let mut stmt = conn
            .prepare(
                "SELECT id, world_id, name, start_distance_m, end_distance_m, elevation_gain_m
                 FROM social_segments WHERE id = ?1",
            )
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        let segment_data = stmt
            .query_row([segment_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            })
            .map_err(|_| ExportError::SegmentNotFound(segment_id))?;

        let segment = SegmentExport {
            id: segment_data.0,
            name: segment_data.2,
            world_id: segment_data.1,
            distance_m: segment_data.4 - segment_data.3,
            elevation_gain_m: segment_data.5,
        };

        // Get efforts
        let mut stmt = conn
            .prepare(
                "SELECT e.rider_id, r.display_name, e.import_source_name, MIN(e.elapsed_time_ms) as best_time, e.avg_power_watts, e.recorded_at
                 FROM social_segment_efforts e
                 LEFT JOIN riders r ON e.rider_id = r.id
                 WHERE e.segment_id = ?1
                 GROUP BY e.rider_id
                 ORDER BY best_time ASC",
            )
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([segment_id.to_string()], |row| {
                Ok((
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, Option<u16>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        let mut efforts = Vec::new();

        for (rank, row) in rows.enumerate() {
            let rank = (rank + 1) as u32;
            let (display_name, import_name, elapsed_time_ms, avg_power, recorded_at_str) =
                row.map_err(|e| ExportError::DatabaseError(e.to_string()))?;

            let rider_name = display_name
                .or(import_name)
                .unwrap_or_else(|| "Unknown".to_string());

            efforts.push(EffortExport {
                rank,
                rider_name,
                elapsed_time_ms,
                avg_power_watts: avg_power,
                recorded_at: DateTime::parse_from_rfc3339(&recorded_at_str)
                    .map_err(|e| ExportError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        Ok(LeaderboardExport {
            segment,
            efforts,
            exported_at: Utc::now(),
            export_version: "1.0".to_string(),
        })
    }

    fn find_or_create_segment(&self, segment: &SegmentExport) -> Result<Uuid, ExportError> {
        let conn = self.db.connection();

        // Try to find existing segment
        let mut stmt = conn
            .prepare("SELECT id FROM social_segments WHERE world_id = ?1 AND name = ?2")
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        let existing: Option<String> = stmt
            .query_row(rusqlite::params![segment.world_id, segment.name], |row| {
                row.get(0)
            })
            .ok();

        if let Some(id_str) = existing {
            return Uuid::parse_str(&id_str).map_err(|e| ExportError::DatabaseError(e.to_string()));
        }

        // Create new segment
        let id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO social_segments (id, world_id, name, start_distance_m, end_distance_m, category, elevation_gain_m)
             VALUES (?1, ?2, ?3, 0, ?4, 'mixed', ?5)",
            rusqlite::params![
                id.to_string(),
                segment.world_id,
                segment.name,
                segment.distance_m,
                segment.elevation_gain_m,
            ],
        )
        .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        Ok(id)
    }

    fn find_rider_by_name(&self, name: &str) -> Result<Option<Uuid>, ExportError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare("SELECT id FROM riders WHERE display_name = ?1")
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        let id_str: Option<String> = stmt.query_row([name], |row| row.get(0)).ok();

        if let Some(id_str) = id_str {
            Ok(Some(
                Uuid::parse_str(&id_str).map_err(|e| ExportError::DatabaseError(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    fn is_duplicate_effort(
        &self,
        segment_id: Uuid,
        rider_name: &str,
        elapsed_time_ms: u32,
    ) -> Result<bool, ExportError> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id FROM social_segment_efforts
                 WHERE segment_id = ?1 AND import_source_name = ?2 AND elapsed_time_ms = ?3",
            )
            .map_err(|e| ExportError::DatabaseError(e.to_string()))?;

        stmt.exists(rusqlite::params![
            segment_id.to_string(),
            rider_name,
            elapsed_time_ms
        ])
        .map_err(|e| ExportError::DatabaseError(e.to_string()))
    }
}

/// Escape a string for CSV.
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export errors.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Segment not found: {0}")]
    SegmentNotFound(Uuid),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
