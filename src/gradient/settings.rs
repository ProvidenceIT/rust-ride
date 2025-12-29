//! Gradient simulation settings.

use serde::{Deserialize, Serialize};

/// User preferences for gradient simulation behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientSettings {
    /// Trainer difficulty multiplier (0.0-1.0, where 1.0 = 100%)
    pub difficulty: f32,
    /// Maximum positive gradient (percent)
    pub max_gradient: f32,
    /// Maximum negative gradient (percent)
    pub min_gradient: f32,
    /// Smoothing window (seconds)
    pub smoothing_secs: u8,
    /// Rolling resistance coefficient
    pub rolling_resistance: f32,
}

impl Default for GradientSettings {
    fn default() -> Self {
        Self {
            difficulty: 1.0,
            max_gradient: 15.0,
            min_gradient: -15.0,
            smoothing_secs: 3,
            rolling_resistance: 0.004,
        }
    }
}

impl GradientSettings {
    /// Create settings with a specific difficulty level (0-100%)
    pub fn with_difficulty(difficulty_percent: u8) -> Self {
        Self {
            difficulty: (difficulty_percent as f32) / 100.0,
            ..Default::default()
        }
    }

    /// Clamp a gradient value to the configured min/max range
    pub fn clamp_gradient(&self, gradient: f32) -> f32 {
        gradient.clamp(self.min_gradient, self.max_gradient)
    }

    /// Apply difficulty scaling to a gradient
    pub fn apply_difficulty(&self, gradient: f32) -> f32 {
        gradient * self.difficulty
    }

    /// Get the effective gradient after clamping and difficulty scaling
    pub fn effective_gradient(&self, raw_gradient: f32) -> f32 {
        self.apply_difficulty(self.clamp_gradient(raw_gradient))
    }

    /// Validate that settings are within acceptable ranges
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(0.0..=1.0).contains(&self.difficulty) {
            return Err("Difficulty must be between 0.0 and 1.0");
        }
        if !(0.0..=25.0).contains(&self.max_gradient) {
            return Err("Max gradient must be between 0% and 25%");
        }
        if !(-25.0..=0.0).contains(&self.min_gradient) {
            return Err("Min gradient must be between -25% and 0%");
        }
        if self.smoothing_secs > 10 {
            return Err("Smoothing must be 10 seconds or less");
        }
        if !(0.001..=0.01).contains(&self.rolling_resistance) {
            return Err("Rolling resistance must be between 0.001 and 0.01");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = GradientSettings::default();
        assert_eq!(settings.difficulty, 1.0);
        assert_eq!(settings.max_gradient, 15.0);
        assert_eq!(settings.min_gradient, -15.0);
        assert_eq!(settings.smoothing_secs, 3);
        assert!((settings.rolling_resistance - 0.004).abs() < 0.0001);
    }

    #[test]
    fn test_clamp_gradient() {
        let settings = GradientSettings::default();
        assert_eq!(settings.clamp_gradient(20.0), 15.0);
        assert_eq!(settings.clamp_gradient(-20.0), -15.0);
        assert_eq!(settings.clamp_gradient(5.0), 5.0);
    }

    #[test]
    fn test_effective_gradient() {
        let settings = GradientSettings {
            difficulty: 0.5,
            ..Default::default()
        };
        // 10% gradient at 50% difficulty = 5% effective
        assert_eq!(settings.effective_gradient(10.0), 5.0);
        // 20% gradient clamped to 15%, then 50% difficulty = 7.5%
        assert_eq!(settings.effective_gradient(20.0), 7.5);
    }

    #[test]
    fn test_validation() {
        let valid = GradientSettings::default();
        assert!(valid.validate().is_ok());

        let invalid_difficulty = GradientSettings {
            difficulty: 1.5,
            ..Default::default()
        };
        assert!(invalid_difficulty.validate().is_err());
    }
}
