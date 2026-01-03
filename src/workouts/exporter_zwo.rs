//! ZWO (Zwift workout) export functionality.
//!
//! Provides functions to export Workout structs to Zwift's ZWO XML format.

use crate::workouts::types::{Workout, WorkoutExportError};
use std::path::Path;

/// Export a workout to ZWO XML format.
///
/// Returns the workout as a ZWO-formatted XML string.
///
/// # Errors
/// Returns `WorkoutExportError::EmptyWorkout` if the workout has no segments.
pub fn export_zwo(workout: &Workout) -> Result<String, WorkoutExportError> {
    if workout.segments.is_empty() {
        return Err(WorkoutExportError::EmptyWorkout);
    }

    // TODO: Implement ZWO XML generation in phase 2
    todo!("ZWO export implementation")
}

/// Export a workout to ZWO format and write to a file.
///
/// # Errors
/// Returns `WorkoutExportError::IoError` if the file cannot be written.
/// Returns `WorkoutExportError::EmptyWorkout` if the workout has no segments.
pub fn export_zwo_to_file(workout: &Workout, path: &Path) -> Result<(), WorkoutExportError> {
    let content = export_zwo(workout)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a default filename for a workout ZWO export.
///
/// The filename is based on the workout name with invalid filesystem
/// characters removed and a `.zwo` extension added.
pub fn generate_zwo_filename(workout: &Workout) -> String {
    let sanitized = sanitize_filename(&workout.name);
    format!("{}.zwo", sanitized)
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_zwo_filename_simple() {
        let workout = Workout::new("Sweet Spot".to_string(), vec![]);
        let filename = generate_zwo_filename(&workout);
        assert_eq!(filename, "Sweet Spot.zwo");
    }

    #[test]
    fn test_generate_zwo_filename_sanitizes_invalid_chars() {
        let workout = Workout::new("Test/Workout:Name*Here".to_string(), vec![]);
        let filename = generate_zwo_filename(&workout);
        assert_eq!(filename, "Test_Workout_Name_Here.zwo");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Normal Name"), "Normal Name");
        assert_eq!(sanitize_filename("File/With\\Bad:Chars"), "File_With_Bad_Chars");
        assert_eq!(sanitize_filename("Has*Question?Mark"), "Has_Question_Mark");
        assert_eq!(sanitize_filename("Quotes\"and<brackets>"), "Quotes_and_brackets_");
    }

    #[test]
    fn test_export_zwo_empty_workout_error() {
        let workout = Workout::new("Empty".to_string(), vec![]);
        let result = export_zwo(&workout);
        assert!(matches!(result, Err(WorkoutExportError::EmptyWorkout)));
    }
}
