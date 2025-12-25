//! Physics engine for power-to-speed calculation
//!
//! Implements a simplified cycling physics model to convert power output
//! to virtual speed based on rider weight and virtual gradient.

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
}
