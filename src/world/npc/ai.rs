//! NPC AI behavior for route following and speed calculation.

use super::NpcCyclist;

/// AI behavior configuration
#[derive(Debug, Clone)]
pub struct AiBehavior {
    /// Power variation range (±%)
    pub power_variation_percent: f32,
    /// How often to vary power (seconds)
    pub variation_interval: f32,
    /// Whether NPC responds to gradient
    pub gradient_response: bool,
    /// Time since last variation
    variation_timer: f32,
}

impl Default for AiBehavior {
    fn default() -> Self {
        Self {
            power_variation_percent: 10.0,
            variation_interval: 5.0,
            gradient_response: true,
            variation_timer: 0.0,
        }
    }
}

impl AiBehavior {
    /// Update AI behavior
    pub fn update(&mut self, npc: &mut NpcCyclist, delta_time: f32, gradient: f32) {
        self.variation_timer += delta_time;

        // Vary power periodically
        if self.variation_timer >= self.variation_interval {
            self.variation_timer = 0.0;
            self.apply_power_variation(npc);
        }

        // Respond to gradient (push harder on climbs, recover on descents)
        if self.gradient_response {
            self.apply_gradient_response(npc, gradient);
        }
    }

    fn apply_power_variation(&self, npc: &mut NpcCyclist) {
        let variation = (rand_simple() - 0.5) * 2.0 * self.power_variation_percent / 100.0;
        npc.current_power_watts = (npc.target_power_watts as f32 * (1.0 + variation)) as u16;
    }

    fn apply_gradient_response(&self, npc: &mut NpcCyclist, gradient: f32) {
        // Increase effort on climbs, decrease on descents
        let gradient_factor = 1.0 + gradient / 100.0 * 0.5;
        npc.current_power_watts =
            (npc.current_power_watts as f32 * gradient_factor.clamp(0.7, 1.3)) as u16;
    }
}

/// Calculate NPC speed based on power and gradient
pub fn calculate_speed(power_watts: u16, gradient_percent: f32, mass_kg: f32) -> f32 {
    // Simplified model: speed = f(power, gradient)
    // Based on empirical cycling power equations

    let power = power_watts as f32;
    let _mass = mass_kg;
    let gradient = gradient_percent / 100.0;

    // Base speed at flat ground (approximation)
    // At 200W, roughly 30 km/h (8.33 m/s) on flat
    // Using cube root relationship: speed ∝ power^(1/3)
    // Calibrated so 200W gives ~8.5 m/s
    let base_speed_mps = (power / 200.0).powf(0.33) * 8.5;

    // Gradient effect
    // Each 1% gradient reduces speed significantly
    let gradient_factor = if gradient > 0.0 {
        1.0 / (1.0 + gradient * 4.0)
    } else {
        1.0 + gradient.abs() * 0.5 // Faster on descents
    };

    (base_speed_mps * gradient_factor).clamp(1.0, 25.0) // 1-25 m/s range
}

/// Calculate speed from physics model
pub fn calculate_speed_physics(
    power_watts: f32,
    gradient_percent: f32,
    mass_kg: f32,
    cda: f32,
    crr: f32,
) -> f32 {
    // Full physics model
    // P = v * (CdA * rho * v² / 2 + m * g * sin(theta) * v + Crr * m * g * cos(theta) * v)

    const AIR_DENSITY: f32 = 1.225; // kg/m³
    const GRAVITY: f32 = 9.81; // m/s²

    let gradient_rad = (gradient_percent / 100.0).atan();
    let sin_grade = gradient_rad.sin();
    let cos_grade = gradient_rad.cos();

    // Iterative solution (Newton-Raphson)
    let mut v = 5.0; // Initial guess: 5 m/s

    for _ in 0..10 {
        let aero_resistance = 0.5 * cda * AIR_DENSITY * v * v;
        let gravity_resistance = mass_kg * GRAVITY * sin_grade;
        let rolling_resistance = crr * mass_kg * GRAVITY * cos_grade;

        let total_resistance = aero_resistance + gravity_resistance + rolling_resistance;
        let power_needed = v * total_resistance;

        let f = power_needed - power_watts;
        let df = total_resistance + cda * AIR_DENSITY * v * v;

        if df.abs() < 0.001 {
            break;
        }

        v -= f / df;
        v = v.clamp(0.5, 30.0);
    }

    v
}

/// Simple random number (0.0-1.0)
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / 4_294_967_295.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_speed_flat() {
        let speed = calculate_speed(200, 0.0, 75.0);
        assert!(speed > 3.0 && speed < 20.0); // Reasonable flat speed (m/s)
    }

    #[test]
    fn test_calculate_speed_climb() {
        let flat_speed = calculate_speed(200, 0.0, 75.0);
        let climb_speed = calculate_speed(200, 8.0, 75.0);
        assert!(climb_speed < flat_speed); // Slower on climb
    }

    #[test]
    fn test_calculate_speed_descent() {
        let flat_speed = calculate_speed(200, 0.0, 75.0);
        let descent_speed = calculate_speed(200, -8.0, 75.0);
        assert!(descent_speed > flat_speed); // Faster on descent
    }
}
