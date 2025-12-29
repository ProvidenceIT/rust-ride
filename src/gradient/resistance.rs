//! Resistance control for FTMS trainers based on gradient.

use crate::gradient::settings::GradientSettings;

/// FTMS Indoor Bike Simulation command opcode.
const FTMS_SET_SIMULATION: u8 = 0x11;

/// Build an FTMS Indoor Bike Simulation command.
///
/// This command sets the trainer to simulate a specific grade, wind speed,
/// and rolling resistance.
///
/// # Arguments
/// * `grade_percent` - Grade in percent (-100.0 to +100.0)
/// * `crr` - Coefficient of rolling resistance (typical: 0.004 for road)
/// * `cw` - Wind coefficient (kg/m, typical: 0.51 for road position)
///
/// # Returns
/// FTMS Control Point command bytes
pub fn build_simulation_command(grade_percent: f32, crr: f32, cw: f32) -> Vec<u8> {
    // Grade is sent as signed 16-bit value in 0.01% resolution
    // Range: -10000 to +10000 (representing -100% to +100%)
    let grade_raw = (grade_percent * 100.0).clamp(-10000.0, 10000.0) as i16;

    // CRR is sent as unsigned 8-bit value in 0.0001 resolution
    // Typical range: 0 to 255 (representing 0 to 0.0255)
    let crr_raw = (crr * 10000.0).clamp(0.0, 255.0) as u8;

    // CW is sent as unsigned 8-bit value in 0.01 kg/m resolution
    // Typical range: 0 to 255 (representing 0 to 2.55 kg/m)
    let cw_raw = (cw * 100.0).clamp(0.0, 255.0) as u8;

    vec![
        FTMS_SET_SIMULATION,
        0x00, // Wind speed low byte (0)
        0x00, // Wind speed high byte (0)
        (grade_raw & 0xFF) as u8,         // Grade low byte
        ((grade_raw >> 8) & 0xFF) as u8,  // Grade high byte
        crr_raw,                           // Rolling resistance
        cw_raw,                            // Wind coefficient
    ]
}

/// Build a simplified simulation command with just grade and rolling resistance.
pub fn build_simulation_with_crr(grade_percent: f32, crr: f32) -> Vec<u8> {
    build_simulation_command(grade_percent, crr, 0.51) // Default CW
}

/// Build a simulation command using gradient settings.
#[allow(dead_code)]
pub fn build_simulation_from_settings(grade_percent: f32, settings: &GradientSettings) -> Vec<u8> {
    let effective_grade = settings.effective_gradient(grade_percent);
    build_simulation_with_crr(effective_grade, settings.rolling_resistance)
}

/// Resistance controller for managing trainer resistance based on gradient.
pub struct ResistanceController {
    /// Current gradient being sent to trainer
    current_gradient: f32,
    /// Last command sent
    last_command: Option<Vec<u8>>,
    /// Minimum change threshold to send new command (percent)
    change_threshold: f32,
}

impl ResistanceController {
    /// Create a new resistance controller.
    pub fn new() -> Self {
        Self {
            current_gradient: 0.0,
            last_command: None,
            change_threshold: 0.1, // Only update if change > 0.1%
        }
    }

    /// Set the minimum change threshold for sending commands.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.change_threshold = threshold;
        self
    }

    /// Update the gradient and get the FTMS command if needed.
    ///
    /// Returns `Some(command)` if the gradient has changed enough to warrant
    /// sending a new command, `None` otherwise.
    pub fn update(
        &mut self,
        gradient: f32,
        settings: &GradientSettings,
    ) -> Option<Vec<u8>> {
        let effective = settings.effective_gradient(gradient);

        // Check if change is significant enough
        if (effective - self.current_gradient).abs() < self.change_threshold {
            return None;
        }

        self.current_gradient = effective;
        let command = build_simulation_with_crr(effective, settings.rolling_resistance);
        self.last_command = Some(command.clone());
        Some(command)
    }

    /// Force send a command regardless of change threshold.
    pub fn force_update(&mut self, gradient: f32, settings: &GradientSettings) -> Vec<u8> {
        let effective = settings.effective_gradient(gradient);
        self.current_gradient = effective;
        let command = build_simulation_with_crr(effective, settings.rolling_resistance);
        self.last_command = Some(command.clone());
        command
    }

    /// Get the current gradient being sent to the trainer.
    pub fn current_gradient(&self) -> f32 {
        self.current_gradient
    }

    /// Reset to flat (0% gradient).
    pub fn reset(&mut self) -> Vec<u8> {
        self.current_gradient = 0.0;
        let command = build_simulation_with_crr(0.0, 0.004);
        self.last_command = Some(command.clone());
        command
    }

    /// Get the last command sent.
    pub fn last_command(&self) -> Option<&[u8]> {
        self.last_command.as_deref()
    }
}

impl Default for ResistanceController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_command_format() {
        let cmd = build_simulation_command(5.0, 0.004, 0.51);

        assert_eq!(cmd.len(), 7);
        assert_eq!(cmd[0], FTMS_SET_SIMULATION);

        // Check grade encoding (5% = 500 in 0.01% units)
        let grade = i16::from_le_bytes([cmd[3], cmd[4]]);
        assert_eq!(grade, 500);
    }

    #[test]
    fn test_negative_gradient() {
        let cmd = build_simulation_command(-10.0, 0.004, 0.51);

        let grade = i16::from_le_bytes([cmd[3], cmd[4]]);
        assert_eq!(grade, -1000);
    }

    #[test]
    fn test_gradient_clamping() {
        // Test extreme values are clamped
        let cmd_max = build_simulation_command(150.0, 0.004, 0.51);
        let grade_max = i16::from_le_bytes([cmd_max[3], cmd_max[4]]);
        assert_eq!(grade_max, 10000); // Clamped to 100%

        let cmd_min = build_simulation_command(-150.0, 0.004, 0.51);
        let grade_min = i16::from_le_bytes([cmd_min[3], cmd_min[4]]);
        assert_eq!(grade_min, -10000); // Clamped to -100%
    }

    #[test]
    fn test_resistance_controller() {
        let settings = GradientSettings::default();
        let mut controller = ResistanceController::new();

        // First update should return a command
        let cmd = controller.update(5.0, &settings);
        assert!(cmd.is_some());
        assert_eq!(controller.current_gradient(), 5.0);

        // Small change should not trigger new command
        let cmd = controller.update(5.05, &settings);
        assert!(cmd.is_none());

        // Large change should trigger
        let cmd = controller.update(10.0, &settings);
        assert!(cmd.is_some());
    }

    #[test]
    fn test_with_difficulty_scaling() {
        let settings = GradientSettings {
            difficulty: 0.5,
            ..Default::default()
        };
        let mut controller = ResistanceController::new();

        // 10% at 50% difficulty = 5% effective
        let _ = controller.update(10.0, &settings);
        assert_eq!(controller.current_gradient(), 5.0);
    }
}
