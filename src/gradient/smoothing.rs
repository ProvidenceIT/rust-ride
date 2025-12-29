//! Gradient smoothing for realistic trainer control.

use std::collections::VecDeque;

/// Gradient smoother using a moving average window.
pub struct GradientSmoother {
    /// Window size in samples
    window_size: usize,
    /// Buffer of recent gradient values
    buffer: VecDeque<f32>,
    /// Current smoothed value
    current: f32,
}

impl GradientSmoother {
    /// Create a new gradient smoother with the specified window size.
    ///
    /// # Arguments
    /// * `window_size` - Number of samples to average (typically based on update rate)
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size: window_size.max(1),
            buffer: VecDeque::with_capacity(window_size),
            current: 0.0,
        }
    }

    /// Create a smoother from seconds and update rate.
    ///
    /// # Arguments
    /// * `smoothing_secs` - Smoothing window in seconds
    /// * `update_rate_hz` - Update rate in Hz
    pub fn from_seconds(smoothing_secs: u8, update_rate_hz: u8) -> Self {
        let window_size = (smoothing_secs as usize) * (update_rate_hz as usize);
        Self::new(window_size.max(1))
    }

    /// Add a new gradient value and return the smoothed result.
    pub fn add(&mut self, gradient: f32) -> f32 {
        // Add new value
        self.buffer.push_back(gradient);

        // Remove old values if buffer is full
        while self.buffer.len() > self.window_size {
            self.buffer.pop_front();
        }

        // Calculate average
        self.current = if self.buffer.is_empty() {
            0.0
        } else {
            self.buffer.iter().sum::<f32>() / self.buffer.len() as f32
        };

        self.current
    }

    /// Get the current smoothed value without adding a new sample.
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Reset the smoother to initial state.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.current = 0.0;
    }

    /// Get the number of samples currently in the buffer.
    pub fn sample_count(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is full.
    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.window_size
    }
}

/// Exponential moving average smoother for smoother transitions.
#[allow(dead_code)]
pub struct ExponentialSmoother {
    /// Smoothing factor (0.0 to 1.0, lower = smoother)
    alpha: f32,
    /// Current smoothed value
    current: f32,
    /// Whether we have an initial value
    initialized: bool,
}

#[allow(dead_code)]
impl ExponentialSmoother {
    /// Create a new exponential smoother.
    ///
    /// # Arguments
    /// * `alpha` - Smoothing factor (0.0-1.0). Lower values = smoother, slower response.
    ///            Typical values: 0.1 (very smooth) to 0.5 (responsive)
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha: alpha.clamp(0.01, 1.0),
            current: 0.0,
            initialized: false,
        }
    }

    /// Create a smoother from a time constant and update rate.
    ///
    /// # Arguments
    /// * `time_constant_secs` - Time for ~63% of a step change
    /// * `update_rate_hz` - Update rate in Hz
    pub fn from_time_constant(time_constant_secs: f32, update_rate_hz: f32) -> Self {
        // alpha = 1 - e^(-dt/tau) where dt = 1/rate and tau = time_constant
        let dt = 1.0 / update_rate_hz;
        let alpha = 1.0 - (-dt / time_constant_secs).exp();
        Self::new(alpha)
    }

    /// Add a new value and return the smoothed result.
    pub fn add(&mut self, value: f32) -> f32 {
        if !self.initialized {
            self.current = value;
            self.initialized = true;
        } else {
            self.current = self.alpha * value + (1.0 - self.alpha) * self.current;
        }
        self.current
    }

    /// Get the current smoothed value.
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Reset the smoother.
    pub fn reset(&mut self) {
        self.current = 0.0;
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_smoother() {
        let mut smoother = GradientSmoother::new(3);

        // First value
        let result = smoother.add(10.0);
        assert!((result - 10.0).abs() < 0.01);

        // Add more values
        let result = smoother.add(10.0);
        assert!((result - 10.0).abs() < 0.01);

        // Add a different value
        let result = smoother.add(20.0);
        // Average of 10, 10, 20 = 13.33
        assert!((result - 13.33).abs() < 0.1);
    }

    #[test]
    fn test_smoother_from_seconds() {
        // 3 seconds at 1 Hz = 3 samples
        let smoother = GradientSmoother::from_seconds(3, 1);
        assert_eq!(smoother.window_size, 3);

        // 2 seconds at 10 Hz = 20 samples
        let smoother = GradientSmoother::from_seconds(2, 10);
        assert_eq!(smoother.window_size, 20);
    }

    #[test]
    fn test_exponential_smoother() {
        let mut smoother = ExponentialSmoother::new(0.5);

        // First value is taken directly
        let result = smoother.add(10.0);
        assert!((result - 10.0).abs() < 0.01);

        // Second value is blended
        let result = smoother.add(20.0);
        // 0.5 * 20 + 0.5 * 10 = 15
        assert!((result - 15.0).abs() < 0.01);

        // Third value
        let result = smoother.add(20.0);
        // 0.5 * 20 + 0.5 * 15 = 17.5
        assert!((result - 17.5).abs() < 0.01);
    }

    #[test]
    fn test_smoother_reset() {
        let mut smoother = GradientSmoother::new(3);
        smoother.add(10.0);
        smoother.add(20.0);
        assert_eq!(smoother.sample_count(), 2);

        smoother.reset();
        assert_eq!(smoother.sample_count(), 0);
        assert_eq!(smoother.current(), 0.0);
    }
}
