//! Smart Trainer Incline/Slope Mode
//!
//! Provides gradient simulation for smart trainers during virtual rides.
//! Supports FTMS and FE-C protocols for controlling trainer resistance based on route gradients.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// =============================================================================
// Configuration Types (T035)
// =============================================================================

/// Configuration for incline/slope mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclineConfig {
    /// Whether incline mode is enabled.
    pub enabled: bool,
    /// Intensity scaling factor (0.5 = 50%, 1.0 = 100%, 1.5 = 150%).
    /// Adjusts the perceived gradient difficulty.
    pub intensity: f32,
    /// Rider weight in kilograms (used for resistance calculations).
    pub rider_weight_kg: f32,
    /// Bike weight in kilograms (used for resistance calculations).
    pub bike_weight_kg: f32,
    /// Maximum gradient to simulate (e.g., 20.0 for 20%).
    /// Gradients steeper than this will be clamped.
    pub max_gradient: f32,
    /// Minimum gradient to simulate (e.g., -15.0 for -15% downhill).
    /// Gradients steeper than this will be clamped.
    pub min_gradient: f32,
    /// Smoothing duration for gradient transitions.
    pub smoothing_duration_ms: u32,
    /// Whether to enable downhill coasting simulation.
    pub enable_downhill: bool,
}

impl Default for InclineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            rider_weight_kg: 75.0,
            bike_weight_kg: 10.0,
            max_gradient: 20.0,
            min_gradient: -15.0,
            smoothing_duration_ms: 2000,
            enable_downhill: true,
        }
    }
}

impl InclineConfig {
    /// Total system weight (rider + bike) in kilograms.
    pub fn total_weight_kg(&self) -> f32 {
        self.rider_weight_kg + self.bike_weight_kg
    }

    /// Apply intensity scaling and clamping to a gradient.
    pub fn apply_intensity(&self, gradient: f32) -> f32 {
        let scaled = gradient * self.intensity;
        scaled.clamp(self.min_gradient, self.max_gradient)
    }

    /// Check if the configuration is valid.
    pub fn is_valid(&self) -> bool {
        self.intensity >= 0.5
            && self.intensity <= 1.5
            && self.rider_weight_kg > 0.0
            && self.bike_weight_kg >= 0.0
            && self.max_gradient >= 0.0
            && self.min_gradient <= 0.0
    }
}

/// Current gradient state during a ride.
#[derive(Debug, Clone)]
pub struct GradientState {
    /// Raw gradient from route data (percentage, e.g., 5.0 = 5%).
    pub raw_gradient: f32,
    /// Effective gradient after intensity scaling and clamping.
    pub effective_gradient: f32,
    /// Smoothed gradient for display (transitioning between values).
    pub smoothed_gradient: f32,
    /// Target gradient we're transitioning toward.
    pub target_gradient: f32,
    /// When the last gradient change started.
    pub transition_start: Option<Instant>,
    /// Gradient at the start of the transition.
    pub transition_start_gradient: f32,
    /// Duration for the current transition.
    pub transition_duration: Duration,
    /// Whether the trainer is currently applying gradient.
    pub is_active: bool,
    /// Current resistance level being sent to trainer (0-100%).
    pub current_resistance: f32,
    /// Last update timestamp.
    pub last_update: Instant,
}

impl Default for GradientState {
    fn default() -> Self {
        Self {
            raw_gradient: 0.0,
            effective_gradient: 0.0,
            smoothed_gradient: 0.0,
            target_gradient: 0.0,
            transition_start: None,
            transition_start_gradient: 0.0,
            transition_duration: Duration::from_millis(2000),
            is_active: false,
            current_resistance: 0.0,
            last_update: Instant::now(),
        }
    }
}

impl GradientState {
    /// Create a new gradient state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a gradient transition is in progress.
    pub fn is_transitioning(&self) -> bool {
        if let Some(start) = self.transition_start {
            start.elapsed() < self.transition_duration
        } else {
            false
        }
    }

    /// Get the progress of the current transition (0.0 to 1.0).
    pub fn transition_progress(&self) -> f32 {
        if let Some(start) = self.transition_start {
            let elapsed = start.elapsed().as_secs_f32();
            let duration = self.transition_duration.as_secs_f32();
            (elapsed / duration).min(1.0)
        } else {
            1.0
        }
    }
}

// =============================================================================
// Incline Controller (T036)
// =============================================================================

/// Trait for controlling trainer gradient/incline.
pub trait InclineController: Send + Sync {
    /// Set the target gradient (percentage).
    fn set_gradient(&mut self, gradient: f32);

    /// Calculate the effective gradient after intensity scaling.
    fn calculate_effective_gradient(&self, raw_gradient: f32) -> f32;

    /// Update the smoothed gradient based on elapsed time.
    fn update_smoothing(&mut self);

    /// Get the current gradient state.
    fn get_state(&self) -> &GradientState;

    /// Get the current configuration.
    fn get_config(&self) -> &InclineConfig;

    /// Update configuration.
    fn set_config(&mut self, config: InclineConfig);

    /// Calculate resistance for the current gradient.
    fn calculate_resistance(&self) -> f32;

    /// Enable or disable incline control.
    fn set_enabled(&mut self, enabled: bool);

    /// Check if incline control is enabled.
    fn is_enabled(&self) -> bool;
}

/// Default implementation of incline controller.
#[derive(Debug)]
pub struct DefaultInclineController {
    config: InclineConfig,
    state: GradientState,
}

impl DefaultInclineController {
    /// Create a new incline controller with the given configuration.
    pub fn new(config: InclineConfig) -> Self {
        let mut state = GradientState::default();
        state.transition_duration = Duration::from_millis(config.smoothing_duration_ms as u64);

        Self { config, state }
    }

    /// Apply easing function for smooth transitions.
    fn ease_in_out(t: f32) -> f32 {
        // Cubic ease-in-out for natural feel
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    /// Calculate gravitational resistance component.
    fn calculate_gravity_resistance(&self, gradient_percent: f32) -> f32 {
        // F = m * g * sin(θ)
        // For small angles: sin(θ) ≈ gradient/100
        // Resistance as percentage of max trainer resistance

        const GRAVITY: f32 = 9.81; // m/s²
        const MAX_RESISTANCE_WATTS: f32 = 2000.0; // Typical max trainer resistance

        let gradient_decimal = gradient_percent / 100.0;
        let total_mass = self.config.total_weight_kg();

        // Force in Newtons
        let force = total_mass * GRAVITY * gradient_decimal;

        // At a typical speed of ~30 km/h (8.33 m/s), calculate power
        const REFERENCE_SPEED_MPS: f32 = 8.33;
        let power = force * REFERENCE_SPEED_MPS;

        // Convert to percentage of max resistance
        (power / MAX_RESISTANCE_WATTS * 100.0).clamp(0.0, 100.0)
    }
}

impl InclineController for DefaultInclineController {
    fn set_gradient(&mut self, gradient: f32) {
        self.state.raw_gradient = gradient;
        self.state.target_gradient = self.calculate_effective_gradient(gradient);

        // Start a new transition if gradient changed significantly
        let gradient_diff = (self.state.target_gradient - self.state.smoothed_gradient).abs();
        if gradient_diff > 0.1 {
            self.state.transition_start_gradient = self.state.smoothed_gradient;
            self.state.transition_start = Some(Instant::now());
        }

        self.state.last_update = Instant::now();
    }

    fn calculate_effective_gradient(&self, raw_gradient: f32) -> f32 {
        // Apply intensity scaling
        let scaled = raw_gradient * self.config.intensity;

        // Clamp to configured limits
        if !self.config.enable_downhill && scaled < 0.0 {
            0.0 // Treat downhill as flat if disabled
        } else {
            scaled.clamp(self.config.min_gradient, self.config.max_gradient)
        }
    }

    fn update_smoothing(&mut self) {
        if !self.state.is_transitioning() {
            // Transition complete, set smoothed to target
            self.state.smoothed_gradient = self.state.target_gradient;
            self.state.effective_gradient = self.state.target_gradient;
            return;
        }

        // Calculate smoothed value using easing
        let progress = self.state.transition_progress();
        let eased_progress = Self::ease_in_out(progress);

        let delta = self.state.target_gradient - self.state.transition_start_gradient;
        self.state.smoothed_gradient =
            self.state.transition_start_gradient + (delta * eased_progress);
        self.state.effective_gradient = self.state.smoothed_gradient;

        // Update resistance
        self.state.current_resistance = self.calculate_resistance();
    }

    fn get_state(&self) -> &GradientState {
        &self.state
    }

    fn get_config(&self) -> &InclineConfig {
        &self.config
    }

    fn set_config(&mut self, config: InclineConfig) {
        self.state.transition_duration = Duration::from_millis(config.smoothing_duration_ms as u64);
        self.config = config;
    }

    fn calculate_resistance(&self) -> f32 {
        self.calculate_gravity_resistance(self.state.smoothed_gradient)
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        self.state.is_active = enabled;

        if !enabled {
            // Reset to flat when disabled
            self.state.target_gradient = 0.0;
            self.state.smoothed_gradient = 0.0;
            self.state.effective_gradient = 0.0;
            self.state.current_resistance = 0.0;
        }
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

// =============================================================================
// FTMS Slope Commands (T037)
// =============================================================================

/// FTMS Control Point commands for slope/incline mode.
pub mod ftms_commands {
    /// Build FTMS Set Indoor Bike Simulation Parameters command.
    ///
    /// # Arguments
    /// * `grade_percent` - Grade/slope in percentage (-100.0 to +100.0)
    /// * `rolling_resistance` - Rolling resistance coefficient (typically 0.004)
    /// * `wind_resistance` - Wind resistance coefficient (kg/m)
    ///
    /// # Returns
    /// 8-byte command for FTMS Control Point characteristic.
    pub fn set_simulation_parameters(
        grade_percent: f32,
        rolling_resistance: f32,
        wind_resistance: f32,
    ) -> [u8; 8] {
        // FTMS Control Point Op Code 0x11: Set Indoor Bike Simulation Parameters
        const OP_CODE: u8 = 0x11;

        // Grade: signed 16-bit, 0.01% resolution
        // Range: -100.00% to +100.00% (-10000 to +10000)
        let grade_encoded = (grade_percent * 100.0).round() as i16;
        let grade_bytes = grade_encoded.to_le_bytes();

        // Rolling resistance: unsigned 8-bit, 0.0001 resolution
        let rr_encoded = ((rolling_resistance / 0.0001).round() as u16).min(255) as u8;

        // Wind resistance: unsigned 8-bit, 0.01 kg/m resolution
        let wr_encoded = ((wind_resistance / 0.01).round() as u16).min(255) as u8;

        // Wind speed: signed 8-bit, 1 km/h resolution (0 = no wind)
        let wind_speed: i8 = 0;

        [
            OP_CODE,
            wind_speed as u8,
            wr_encoded,
            rr_encoded,
            grade_bytes[0],
            grade_bytes[1],
            0x00, // Reserved
            0x00, // Reserved
        ]
    }

    /// Build FTMS Set Target Inclination command.
    ///
    /// # Arguments
    /// * `inclination_percent` - Target inclination in percentage (-100.0 to +100.0)
    ///
    /// # Returns
    /// 3-byte command for FTMS Control Point characteristic.
    pub fn set_target_inclination(inclination_percent: f32) -> [u8; 3] {
        // FTMS Control Point Op Code 0x03: Set Target Inclination
        const OP_CODE: u8 = 0x03;

        // Inclination: signed 16-bit, 0.1% resolution
        // Range: -100.0% to +100.0% (-1000 to +1000)
        let inclination_encoded = (inclination_percent * 10.0).round() as i16;
        let inclination_bytes = inclination_encoded.to_le_bytes();

        [OP_CODE, inclination_bytes[0], inclination_bytes[1]]
    }

    /// Build FTMS Set Target Resistance Level command.
    ///
    /// # Arguments
    /// * `resistance_level` - Target resistance (0.0 to 100.0 percent)
    ///
    /// # Returns
    /// 2-byte command for FTMS Control Point characteristic.
    pub fn set_target_resistance(resistance_level: f32) -> [u8; 2] {
        // FTMS Control Point Op Code 0x04: Set Target Resistance Level
        const OP_CODE: u8 = 0x04;

        // Resistance: unsigned 8-bit, 0.1 resolution (0-254 = 0-25.4)
        // We map 0-100% to 0-254
        let resistance_encoded = ((resistance_level / 100.0 * 254.0).round() as u16).min(254) as u8;

        [OP_CODE, resistance_encoded]
    }

    /// Parse FTMS Control Point Response.
    ///
    /// # Arguments
    /// * `data` - Response data from FTMS Control Point
    ///
    /// # Returns
    /// Tuple of (op_code_responded_to, result_code, is_success)
    pub fn parse_response(data: &[u8]) -> Option<(u8, u8, bool)> {
        if data.len() < 3 {
            return None;
        }

        // Response format: 0x80, requested_op_code, result_code
        if data[0] != 0x80 {
            return None;
        }

        let op_code = data[1];
        let result = data[2];
        let success = result == 0x01; // 0x01 = Success

        Some((op_code, result, success))
    }
}

// =============================================================================
// Gradient Calculations (T038)
// =============================================================================

/// Physics constants for gradient calculations.
pub mod physics {
    /// Gravitational acceleration (m/s²).
    pub const GRAVITY: f32 = 9.81;

    /// Air density at sea level, 20°C (kg/m³).
    pub const AIR_DENSITY: f32 = 1.204;

    /// Typical coefficient of rolling resistance for road tires.
    pub const ROLLING_RESISTANCE_ROAD: f32 = 0.004;

    /// Typical coefficient of rolling resistance for trainer tires.
    pub const ROLLING_RESISTANCE_TRAINER: f32 = 0.003;

    /// Typical drag coefficient for cyclist in drops.
    pub const DRAG_COEFFICIENT_DROPS: f32 = 0.88;

    /// Typical frontal area for cyclist (m²).
    pub const FRONTAL_AREA: f32 = 0.40;

    /// Calculate power required to maintain speed on a gradient.
    ///
    /// # Arguments
    /// * `speed_mps` - Speed in meters per second
    /// * `gradient_percent` - Gradient as percentage (5.0 = 5%)
    /// * `total_mass_kg` - Combined mass of rider and bike
    /// * `rolling_resistance` - Coefficient of rolling resistance
    ///
    /// # Returns
    /// Power in watts required to maintain the given speed.
    pub fn calculate_power_for_gradient(
        speed_mps: f32,
        gradient_percent: f32,
        total_mass_kg: f32,
        rolling_resistance: f32,
    ) -> f32 {
        let gradient_decimal = gradient_percent / 100.0;

        // Gravitational component: m * g * sin(θ) * v
        // For small angles: sin(θ) ≈ θ (in radians) ≈ gradient_decimal
        let power_gravity = total_mass_kg * GRAVITY * gradient_decimal * speed_mps;

        // Rolling resistance component: Crr * m * g * cos(θ) * v
        // For small angles: cos(θ) ≈ 1
        let power_rolling = rolling_resistance * total_mass_kg * GRAVITY * speed_mps;

        // Total power (excluding aerodynamic drag which trainer can't simulate)
        power_gravity + power_rolling
    }

    /// Calculate resistance percentage for trainer based on gradient and speed.
    ///
    /// # Arguments
    /// * `gradient_percent` - Gradient as percentage
    /// * `speed_kmh` - Current speed in km/h
    /// * `total_mass_kg` - Combined mass of rider and bike
    /// * `max_trainer_watts` - Maximum trainer resistance in watts
    ///
    /// # Returns
    /// Resistance level as percentage (0-100).
    pub fn calculate_resistance_for_gradient(
        gradient_percent: f32,
        speed_kmh: f32,
        total_mass_kg: f32,
        max_trainer_watts: f32,
    ) -> f32 {
        let speed_mps = speed_kmh / 3.6;
        let power = calculate_power_for_gradient(
            speed_mps,
            gradient_percent,
            total_mass_kg,
            ROLLING_RESISTANCE_TRAINER,
        );

        // Clamp power to valid range
        let clamped_power = power.max(0.0);

        // Convert to percentage
        ((clamped_power / max_trainer_watts) * 100.0).clamp(0.0, 100.0)
    }

    /// Calculate virtual speed based on power and gradient.
    ///
    /// # Arguments
    /// * `power_watts` - Current power output
    /// * `gradient_percent` - Current gradient
    /// * `total_mass_kg` - Combined mass of rider and bike
    ///
    /// # Returns
    /// Virtual speed in km/h.
    pub fn calculate_virtual_speed(
        power_watts: f32,
        gradient_percent: f32,
        total_mass_kg: f32,
    ) -> f32 {
        if power_watts <= 0.0 {
            return 0.0;
        }

        let gradient_decimal = gradient_percent / 100.0;

        // Simplified model: P = v * (m*g*sin(θ) + Crr*m*g)
        // Solving for v: v = P / (m*g*(sin(θ) + Crr))

        let resistance_factor = (gradient_decimal + ROLLING_RESISTANCE_TRAINER).abs();
        if resistance_factor < 0.001 {
            // Nearly flat, use simpler calculation
            return (power_watts / 10.0).min(80.0); // Rough estimate
        }

        let speed_mps = power_watts / (total_mass_kg * GRAVITY * resistance_factor);
        let speed_kmh = speed_mps * 3.6;

        // Clamp to reasonable range
        speed_kmh.clamp(0.0, 100.0)
    }
}

// =============================================================================
// Gradient Smoothing (T039)
// =============================================================================

/// Gradient smoother for natural transitions.
#[derive(Debug)]
pub struct GradientSmoother {
    /// Current smoothed value.
    current: f32,
    /// Target value.
    target: f32,
    /// Smoothing time constant in seconds.
    time_constant: f32,
    /// Last update time.
    last_update: Instant,
    /// Whether currently transitioning.
    transitioning: bool,
}

impl GradientSmoother {
    /// Create a new gradient smoother.
    ///
    /// # Arguments
    /// * `time_constant` - Smoothing time constant in seconds (higher = slower).
    pub fn new(time_constant: f32) -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            time_constant,
            last_update: Instant::now(),
            transitioning: false,
        }
    }

    /// Set a new target gradient.
    pub fn set_target(&mut self, target: f32) {
        if (target - self.target).abs() > 0.01 {
            self.target = target;
            self.transitioning = true;
        }
    }

    /// Update the smoothed value based on elapsed time.
    ///
    /// # Returns
    /// Current smoothed gradient value.
    pub fn update(&mut self) -> f32 {
        if !self.transitioning {
            return self.current;
        }

        let dt = self.last_update.elapsed().as_secs_f32();
        self.last_update = Instant::now();

        // Exponential smoothing
        let alpha = 1.0 - (-dt / self.time_constant).exp();
        self.current = self.current + alpha * (self.target - self.current);

        // Check if we've reached the target
        if (self.current - self.target).abs() < 0.01 {
            self.current = self.target;
            self.transitioning = false;
        }

        self.current
    }

    /// Get the current smoothed value without updating.
    pub fn get_current(&self) -> f32 {
        self.current
    }

    /// Get the target value.
    pub fn get_target(&self) -> f32 {
        self.target
    }

    /// Check if currently transitioning.
    pub fn is_transitioning(&self) -> bool {
        self.transitioning
    }

    /// Reset to a specific value (no transition).
    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.transitioning = false;
        self.last_update = Instant::now();
    }
}

impl Default for GradientSmoother {
    fn default() -> Self {
        Self::new(2.0) // 2-second time constant
    }
}

// =============================================================================
// Intensity Scaling (T040)
// =============================================================================

/// Apply intensity scaling to gradients.
pub struct IntensityScaler {
    /// Minimum intensity factor (e.g., 0.5 = 50%).
    pub min_intensity: f32,
    /// Maximum intensity factor (e.g., 1.5 = 150%).
    pub max_intensity: f32,
    /// Current intensity setting.
    pub current_intensity: f32,
}

impl IntensityScaler {
    /// Create a new intensity scaler.
    pub fn new(min_intensity: f32, max_intensity: f32) -> Self {
        Self {
            min_intensity,
            max_intensity,
            current_intensity: 1.0,
        }
    }

    /// Set the intensity level (clamped to valid range).
    pub fn set_intensity(&mut self, intensity: f32) {
        self.current_intensity = intensity.clamp(self.min_intensity, self.max_intensity);
    }

    /// Apply intensity scaling to a gradient.
    pub fn apply(&self, gradient: f32) -> f32 {
        gradient * self.current_intensity
    }

    /// Get the current intensity as a percentage string.
    pub fn intensity_percent(&self) -> String {
        format!("{}%", (self.current_intensity * 100.0).round() as u32)
    }
}

impl Default for IntensityScaler {
    fn default() -> Self {
        Self::new(0.5, 1.5)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incline_config_default() {
        let config = InclineConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.intensity, 1.0);
        assert_eq!(config.rider_weight_kg, 75.0);
        assert_eq!(config.bike_weight_kg, 10.0);
        assert!(config.is_valid());
    }

    #[test]
    fn test_incline_config_total_weight() {
        let config = InclineConfig {
            rider_weight_kg: 80.0,
            bike_weight_kg: 12.0,
            ..Default::default()
        };
        assert_eq!(config.total_weight_kg(), 92.0);
    }

    #[test]
    fn test_incline_config_apply_intensity() {
        let config = InclineConfig {
            intensity: 0.5,
            max_gradient: 20.0,
            min_gradient: -15.0,
            ..Default::default()
        };

        // 10% gradient scaled to 50% = 5%
        assert_eq!(config.apply_intensity(10.0), 5.0);

        // Clamping test
        assert_eq!(config.apply_intensity(50.0), 20.0); // Clamped to max
    }

    #[test]
    fn test_gradient_state_default() {
        let state = GradientState::default();
        assert_eq!(state.raw_gradient, 0.0);
        assert!(!state.is_active);
        assert!(!state.is_transitioning());
    }

    #[test]
    fn test_ftms_set_simulation_parameters() {
        let cmd = ftms_commands::set_simulation_parameters(5.0, 0.004, 0.0);
        assert_eq!(cmd[0], 0x11); // Op code

        // Grade should be 500 (5.0 * 100)
        let grade = i16::from_le_bytes([cmd[4], cmd[5]]);
        assert_eq!(grade, 500);
    }

    #[test]
    fn test_ftms_set_target_inclination() {
        let cmd = ftms_commands::set_target_inclination(7.5);
        assert_eq!(cmd[0], 0x03); // Op code

        // Inclination should be 75 (7.5 * 10)
        let inclination = i16::from_le_bytes([cmd[1], cmd[2]]);
        assert_eq!(inclination, 75);
    }

    #[test]
    fn test_ftms_set_target_resistance() {
        let cmd = ftms_commands::set_target_resistance(50.0);
        assert_eq!(cmd[0], 0x04); // Op code
        assert_eq!(cmd[1], 127); // 50% of 254
    }

    #[test]
    fn test_ftms_parse_response_success() {
        let data = [0x80, 0x11, 0x01]; // Success response to 0x11
        let result = ftms_commands::parse_response(&data);
        assert!(result.is_some());
        let (op, code, success) = result.unwrap();
        assert_eq!(op, 0x11);
        assert_eq!(code, 0x01);
        assert!(success);
    }

    #[test]
    fn test_physics_power_calculation() {
        let power = physics::calculate_power_for_gradient(
            8.33, // ~30 km/h
            5.0,  // 5% gradient
            85.0, // 85 kg total
            0.004,
        );

        // Approximate: 85 * 9.81 * 0.05 * 8.33 ≈ 347W + rolling ≈ 375W
        assert!(power > 300.0 && power < 450.0);
    }

    #[test]
    fn test_physics_virtual_speed() {
        let speed = physics::calculate_virtual_speed(
            200.0, // 200W
            5.0,   // 5% gradient
            85.0,  // 85 kg total
        );

        // At 200W and 5%, should be around 10-15 km/h
        assert!(speed > 5.0 && speed < 25.0);
    }

    #[test]
    fn test_gradient_smoother() {
        let mut smoother = GradientSmoother::new(0.1); // Faster time constant for testing
        smoother.set_target(10.0);

        assert!(smoother.is_transitioning());
        assert_eq!(smoother.get_target(), 10.0);

        // After some updates, should approach target
        for _ in 0..50 {
            smoother.update();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let current = smoother.get_current();
        // Should be close to target after ~1 second with 0.1s time constant
        assert!(
            (current - 10.0).abs() < 2.0,
            "Expected ~10.0, got {}",
            current
        );
    }

    #[test]
    fn test_intensity_scaler() {
        let mut scaler = IntensityScaler::default();
        assert_eq!(scaler.apply(10.0), 10.0);

        scaler.set_intensity(0.5);
        assert_eq!(scaler.apply(10.0), 5.0);

        scaler.set_intensity(1.5);
        assert_eq!(scaler.apply(10.0), 15.0);

        // Test clamping
        scaler.set_intensity(2.0);
        assert_eq!(scaler.current_intensity, 1.5);
    }

    #[test]
    fn test_incline_controller() {
        let config = InclineConfig::default();
        let mut controller = DefaultInclineController::new(config);

        assert!(!controller.is_enabled());

        controller.set_enabled(true);
        assert!(controller.is_enabled());

        controller.set_gradient(5.0);
        assert_eq!(controller.get_state().raw_gradient, 5.0);

        // With default intensity of 1.0, effective should equal raw
        controller.update_smoothing();
        let state = controller.get_state();
        assert!(state.effective_gradient <= 5.0);
    }

    #[test]
    fn test_incline_controller_intensity_scaling() {
        let config = InclineConfig {
            intensity: 0.5,
            ..Default::default()
        };
        let controller = DefaultInclineController::new(config);

        // 10% gradient scaled by 0.5 = 5%
        let effective = controller.calculate_effective_gradient(10.0);
        assert_eq!(effective, 5.0);
    }
}
