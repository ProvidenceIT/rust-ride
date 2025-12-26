//! Physics engine for power-to-speed calculation
//!
//! Implements a simplified cycling physics model to convert power output
//! to virtual speed based on rider weight and virtual gradient.
//!
//! T043: Add trainer resistance control based on route gradient
//! T100: Integrate difficulty modifier with trainer resistance control

use super::route::GradientScaler;

/// Physics constants
const AIR_DENSITY: f32 = 1.225; // kg/m³ at sea level
const GRAVITY: f32 = 9.81; // m/s²
const DEFAULT_CDA: f32 = 0.32; // m² (hoods position)
const DEFAULT_CRR: f32 = 0.004; // Rolling resistance for road tires
const BIKE_MASS: f32 = 8.0; // kg

/// Physics engine for calculating virtual speed from power
#[derive(Debug, Clone)]
pub struct PhysicsEngine {
    /// Rider mass in kilograms
    pub rider_mass_kg: f32,
    /// Bike mass in kilograms
    pub bike_mass_kg: f32,
    /// Drag coefficient times frontal area (CdA)
    pub cda: f32,
    /// Rolling resistance coefficient
    pub crr: f32,
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self {
            rider_mass_kg: 75.0,
            bike_mass_kg: BIKE_MASS,
            cda: DEFAULT_CDA,
            crr: DEFAULT_CRR,
        }
    }
}

impl PhysicsEngine {
    /// Create a new physics engine with specified rider mass
    pub fn new(rider_mass_kg: f32) -> Self {
        Self {
            rider_mass_kg,
            ..Default::default()
        }
    }

    /// Total system mass (rider + bike)
    fn total_mass(&self) -> f32 {
        self.rider_mass_kg + self.bike_mass_kg
    }

    /// Calculate speed from power and gradient using Newton-Raphson iteration
    ///
    /// # Arguments
    /// * `power_watts` - Current power output in watts
    /// * `gradient_percent` - Virtual road gradient as a percentage (-50 to +50)
    ///
    /// # Returns
    /// Speed in meters per second
    pub fn calculate_speed(&self, power_watts: u16, gradient_percent: f32) -> f32 {
        let power = power_watts as f32;
        let mass = self.total_mass();

        // Convert percentage to gradient ratio (rise/run)
        // For small angles: sin(atan(x)) ≈ x / sqrt(1 + x^2)
        let grade_ratio = gradient_percent / 100.0;
        let sin_g = grade_ratio / (1.0 + grade_ratio * grade_ratio).sqrt();
        let cos_g = 1.0 / (1.0 + grade_ratio * grade_ratio).sqrt();

        // Handle zero power case
        if power <= 0.0 {
            return 0.0;
        }

        // Newton-Raphson to solve: P = v * F_total
        // where F_total = F_gravity + F_rolling + F_air
        // F_air = 0.5 * rho * CdA * v^2
        // Note: F_gravity is negative for downhill, providing assistance

        // Better initial guess based on gradient
        let mut v = if gradient_percent < -3.0 {
            15.0 // Downhill, start higher
        } else if gradient_percent > 5.0 {
            3.0 // Steep uphill, start lower
        } else {
            8.0 // Default
        };

        for _ in 0..50 {
            // Forces
            // Gravity component: positive = resistance (uphill), negative = assistance (downhill)
            let f_gravity = mass * GRAVITY * sin_g;
            let f_rolling = self.crr * mass * GRAVITY * cos_g;
            let f_air = 0.5 * AIR_DENSITY * self.cda * v * v;

            // Total resistance force (can be negative on steep downhills)
            let f_total = f_gravity + f_rolling + f_air;

            // Power balance equation: P = v * F_total
            // We solve: f(v) = v * F_total - P = 0
            let f_v = v * f_total - power;

            // Derivative: f'(v) = F_total + v * dF_total/dv
            // dF_air/dv = rho * CdA * v
            let df_v = f_total + v * AIR_DENSITY * self.cda * v;

            // Newton-Raphson update
            if df_v.abs() < 1e-10 {
                // Avoid division by zero, use gradient-based adjustment
                v *= 1.1;
                continue;
            }

            let v_new = v - f_v / df_v;

            // Check convergence before clamping
            if (v_new - v).abs() < 0.001 {
                v = v_new;
                break;
            }

            // Clamp to reasonable range
            v = v_new.clamp(0.5, 50.0); // 0.5 to 180 km/h
        }

        v.max(0.0)
    }

    /// Update rider mass (e.g., from settings change)
    pub fn set_rider_mass(&mut self, mass_kg: f32) {
        self.rider_mass_kg = mass_kg.clamp(30.0, 200.0);
    }
}

/// Trainer resistance controller for gradient-based simulation (T043)
///
/// This controller manages sending gradient/simulation commands to
/// smart trainers that support FTMS simulation mode.
#[derive(Debug, Clone)]
pub struct GradientController {
    /// Current gradient being sent to trainer
    current_gradient: f32,
    /// Minimum gradient change before sending update (to reduce noise)
    gradient_threshold: f32,
    /// Maximum gradient to send to trainer (trainer safety limit)
    max_gradient: f32,
    /// Minimum gradient to send to trainer
    min_gradient: f32,
    /// Smoothing factor for gradient changes (0.0-1.0)
    smoothing: f32,
    /// Whether the controller is enabled
    enabled: bool,
    /// Time since last update
    last_update_time: f32,
    /// Minimum time between updates (seconds)
    update_interval: f32,
    /// T100: Optional gradient scaler for difficulty adjustment
    gradient_scaler: Option<GradientScaler>,
}

impl Default for GradientController {
    fn default() -> Self {
        Self {
            current_gradient: 0.0,
            gradient_threshold: 0.3, // Only update if change > 0.3%
            max_gradient: 20.0,      // Cap at 20% (trainer limit)
            min_gradient: -10.0,     // Cap at -10% (trainer limit)
            smoothing: 0.3,          // Moderate smoothing
            enabled: true,
            last_update_time: 0.0,
            update_interval: 0.5, // Update at most every 500ms
            gradient_scaler: None,
        }
    }
}

impl GradientController {
    /// Create a new gradient controller
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom settings
    pub fn with_settings(
        max_gradient: f32,
        min_gradient: f32,
        smoothing: f32,
        update_interval: f32,
    ) -> Self {
        Self {
            max_gradient: max_gradient.clamp(0.0, 25.0),
            min_gradient: min_gradient.clamp(-15.0, 0.0),
            smoothing: smoothing.clamp(0.0, 1.0),
            update_interval,
            ..Default::default()
        }
    }

    /// Enable or disable the controller
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if controller is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the current gradient being sent to trainer
    pub fn current_gradient(&self) -> f32 {
        self.current_gradient
    }

    /// T100: Set the gradient scaler for difficulty adjustment
    ///
    /// When set, route gradients will be scaled according to the scaler's
    /// settings before being sent to the trainer.
    pub fn set_gradient_scaler(&mut self, scaler: Option<GradientScaler>) {
        self.gradient_scaler = scaler;
    }

    /// T100: Get the current gradient scaler
    pub fn gradient_scaler(&self) -> Option<&GradientScaler> {
        self.gradient_scaler.as_ref()
    }

    /// T100: Apply difficulty scaling to a gradient
    fn apply_difficulty_scaling(&self, gradient: f32) -> f32 {
        if let Some(ref scaler) = self.gradient_scaler {
            scaler.scale_gradient(gradient)
        } else {
            gradient
        }
    }

    /// Update with new route gradient and return command if needed
    ///
    /// Returns Some(gradient) if an update should be sent to the trainer,
    /// None if no update is needed.
    ///
    /// Note: If a gradient scaler is set, the route gradient will be scaled
    /// according to the difficulty settings before being sent to the trainer.
    pub fn update(&mut self, route_gradient: f32, delta_time: f32) -> Option<f32> {
        if !self.enabled {
            return None;
        }

        self.last_update_time += delta_time;

        // Rate limit updates
        if self.last_update_time < self.update_interval {
            return None;
        }

        // T100: Apply difficulty scaling if configured
        let scaled_gradient = self.apply_difficulty_scaling(route_gradient);

        // Clamp the route gradient to trainer limits
        let clamped_gradient = scaled_gradient.clamp(self.min_gradient, self.max_gradient);

        // Apply smoothing
        let smoothed_gradient =
            self.current_gradient * self.smoothing + clamped_gradient * (1.0 - self.smoothing);

        // Check if change exceeds threshold
        let gradient_change = (smoothed_gradient - self.current_gradient).abs();
        if gradient_change < self.gradient_threshold {
            return None;
        }

        // Update and return new gradient
        self.current_gradient = smoothed_gradient;
        self.last_update_time = 0.0;

        Some(self.current_gradient)
    }

    /// Force an immediate update with the given gradient
    pub fn force_update(&mut self, gradient: f32) -> f32 {
        let clamped = gradient.clamp(self.min_gradient, self.max_gradient);
        self.current_gradient = clamped;
        self.last_update_time = 0.0;
        clamped
    }

    /// Reset the controller to zero gradient
    pub fn reset(&mut self) {
        self.current_gradient = 0.0;
        self.last_update_time = 0.0;
    }

    /// Build FTMS simulation command for current gradient
    pub fn build_ftms_command(&self) -> Vec<u8> {
        crate::sensors::ftms::build_set_simulation_grade(self.current_gradient)
    }
}

/// Simulation mode for trainer control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrainerSimulationMode {
    /// ERG mode - trainer maintains target power
    Erg,
    /// Simulation mode - trainer simulates gradient
    #[default]
    Simulation,
    /// Resistance mode - manual resistance level
    Resistance,
    /// Free ride - no resistance control
    FreeRide,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_power_gives_zero_speed() {
        let engine = PhysicsEngine::default();
        let speed = engine.calculate_speed(0, 0.0);
        assert_eq!(speed, 0.0);
    }

    #[test]
    fn test_flat_road_speed() {
        let engine = PhysicsEngine::new(75.0);
        let speed = engine.calculate_speed(200, 0.0);
        // At 200W on flat, should be roughly 30-35 km/h (8-10 m/s)
        assert!(speed > 7.0 && speed < 12.0, "Speed was {} m/s", speed);
    }

    #[test]
    fn test_uphill_slower() {
        let engine = PhysicsEngine::new(75.0);
        let flat_speed = engine.calculate_speed(200, 0.0);
        let uphill_speed = engine.calculate_speed(200, 5.0);
        assert!(uphill_speed < flat_speed);
    }

    #[test]
    fn test_downhill_faster() {
        let engine = PhysicsEngine::new(75.0);
        let flat_speed = engine.calculate_speed(200, 0.0);
        let downhill_speed = engine.calculate_speed(200, -5.0);
        assert!(downhill_speed > flat_speed);
    }

    #[test]
    fn test_gradient_controller_default() {
        let controller = GradientController::new();
        assert!(controller.is_enabled());
        assert_eq!(controller.current_gradient(), 0.0);
    }

    #[test]
    fn test_gradient_controller_update_below_threshold() {
        let mut controller = GradientController::new();
        // Small change should not trigger update
        let result = controller.update(0.2, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_gradient_controller_update_above_threshold() {
        let mut controller = GradientController::new();
        // Large change should trigger update after interval
        let result = controller.update(5.0, 1.0);
        assert!(result.is_some());
        assert!(result.unwrap() > 0.0);
    }

    #[test]
    fn test_gradient_controller_rate_limiting() {
        let mut controller = GradientController::new();
        // First update after interval should work
        let _ = controller.update(5.0, 1.0);
        // Immediate second update should be blocked (too soon)
        let result = controller.update(10.0, 0.1);
        assert!(result.is_none());
    }

    #[test]
    fn test_gradient_controller_clamping() {
        let mut controller = GradientController::new();
        // Extreme gradient should be clamped
        let result = controller.force_update(50.0);
        assert!(result <= 20.0);

        let result = controller.force_update(-30.0);
        assert!(result >= -10.0);
    }

    #[test]
    fn test_gradient_controller_disabled() {
        let mut controller = GradientController::new();
        controller.set_enabled(false);
        let result = controller.update(10.0, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_gradient_controller_reset() {
        let mut controller = GradientController::new();
        controller.force_update(5.0);
        assert!(controller.current_gradient() > 0.0);

        controller.reset();
        assert_eq!(controller.current_gradient(), 0.0);
    }

    #[test]
    fn test_gradient_controller_custom_settings() {
        let controller = GradientController::with_settings(
            15.0, // max gradient
            -5.0, // min gradient
            0.5,  // smoothing
            0.25, // update interval
        );
        assert!(controller.is_enabled());
    }

    #[test]
    fn test_trainer_simulation_mode_default() {
        let mode = TrainerSimulationMode::default();
        assert_eq!(mode, TrainerSimulationMode::Simulation);
    }
}
