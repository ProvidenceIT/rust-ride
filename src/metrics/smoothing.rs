//! Power smoothing and filtering algorithms.
//!
//! T019: Implement 3-second rolling average
//! T020: Implement power spike filtering (>2000W)

use std::collections::VecDeque;

/// Rolling average calculator for power smoothing.
#[derive(Debug)]
pub struct RollingAverage {
    /// Buffer of recent values
    buffer: VecDeque<u16>,
    /// Window size in samples
    window_size: usize,
    /// Running sum for efficient calculation
    sum: u32,
}

impl RollingAverage {
    /// Create a new rolling average with the given window size.
    pub fn new(window_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(window_size),
            window_size,
            sum: 0,
        }
    }

    /// Create a 3-second rolling average (default for power display).
    pub fn three_second() -> Self {
        Self::new(3)
    }

    /// Create a 30-second rolling average (for Normalized Power calculation).
    pub fn thirty_second() -> Self {
        Self::new(30)
    }

    /// Add a new value and return the current average.
    pub fn add(&mut self, value: u16) -> Option<u16> {
        // Add new value
        self.buffer.push_back(value);
        self.sum += value as u32;

        // Remove oldest if over window size
        if self.buffer.len() > self.window_size {
            if let Some(old) = self.buffer.pop_front() {
                self.sum -= old as u32;
            }
        }

        self.average()
    }

    /// Get the current average without adding a value.
    pub fn average(&self) -> Option<u16> {
        if self.buffer.is_empty() {
            None
        } else {
            Some((self.sum / self.buffer.len() as u32) as u16)
        }
    }

    /// Check if the buffer is full (has enough samples for a valid average).
    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.window_size
    }

    /// Reset the rolling average.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.sum = 0;
    }

    /// Get the number of samples in the buffer.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Power filter for removing noise and spikes.
#[derive(Debug)]
pub struct PowerFilter {
    /// Maximum valid power value
    max_power: u16,
    /// Minimum valid power value
    min_power: u16,
    /// Previous valid value (for spike detection)
    last_valid: Option<u16>,
    /// Maximum allowed change between samples
    max_delta: Option<u16>,
}

impl PowerFilter {
    /// Create a new power filter with default settings.
    ///
    /// Default max power is 2000W (anything above is noise).
    pub fn new() -> Self {
        Self {
            max_power: 2000,
            min_power: 0,
            last_valid: None,
            max_delta: None,
        }
    }

    /// Create a power filter with custom max power.
    pub fn with_max_power(max_power: u16) -> Self {
        Self {
            max_power,
            ..Self::new()
        }
    }

    /// Create a power filter with spike detection.
    ///
    /// `max_delta` is the maximum allowed change between consecutive samples.
    /// Typical value: 200-300W for detecting sensor glitches.
    pub fn with_spike_detection(max_power: u16, max_delta: u16) -> Self {
        Self {
            max_power,
            max_delta: Some(max_delta),
            ..Self::new()
        }
    }

    /// Filter a power value.
    ///
    /// Returns `None` if the value should be discarded (noise/spike).
    /// Returns `Some(value)` if the value is valid.
    pub fn filter(&mut self, power: u16) -> Option<u16> {
        // Check absolute limits
        if power > self.max_power || power < self.min_power {
            return None;
        }

        // Check spike detection
        if let Some(max_delta) = self.max_delta {
            if let Some(last) = self.last_valid {
                let delta = (power as i32 - last as i32).unsigned_abs() as u16;
                if delta > max_delta {
                    // This looks like a spike - skip it
                    return None;
                }
            }
        }

        // Value is valid
        self.last_valid = Some(power);
        Some(power)
    }

    /// Reset the filter state.
    pub fn reset(&mut self) {
        self.last_valid = None;
    }
}

impl Default for PowerFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalized Power calculation.
///
/// NP = 4th root of average of (30-second rolling average power)^4
#[derive(Debug)]
pub struct NormalizedPowerCalculator {
    /// 30-second rolling average
    rolling_avg: RollingAverage,
    /// Sum of 4th powers
    sum_fourth_power: f64,
    /// Count of samples
    count: u32,
}

impl NormalizedPowerCalculator {
    /// Create a new Normalized Power calculator.
    pub fn new() -> Self {
        Self {
            rolling_avg: RollingAverage::thirty_second(),
            sum_fourth_power: 0.0,
            count: 0,
        }
    }

    /// Add a power sample and return the current NP.
    pub fn add(&mut self, power: u16) -> Option<u16> {
        // Add to 30-second rolling average
        if let Some(avg) = self.rolling_avg.add(power) {
            // Only count once we have a full 30-second window
            if self.rolling_avg.is_full() {
                let fourth_power = (avg as f64).powi(4);
                self.sum_fourth_power += fourth_power;
                self.count += 1;
            }
        }

        self.normalized_power()
    }

    /// Get the current Normalized Power.
    pub fn normalized_power(&self) -> Option<u16> {
        if self.count == 0 {
            return None;
        }

        let avg_fourth_power = self.sum_fourth_power / self.count as f64;
        let np = avg_fourth_power.powf(0.25);

        Some(np.round() as u16)
    }

    /// Reset the calculator.
    pub fn reset(&mut self) {
        self.rolling_avg.reset();
        self.sum_fourth_power = 0.0;
        self.count = 0;
    }
}

impl Default for NormalizedPowerCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_average_3s() {
        let mut avg = RollingAverage::three_second();

        // First value
        assert_eq!(avg.add(200), Some(200));
        assert!(!avg.is_full());

        // Second value
        assert_eq!(avg.add(220), Some(210));

        // Third value - now full
        assert_eq!(avg.add(240), Some(220));
        assert!(avg.is_full());

        // Fourth value - first drops off
        // (220 + 240 + 260) / 3 = 240
        assert_eq!(avg.add(260), Some(240));
    }

    #[test]
    fn test_power_filter_max() {
        let mut filter = PowerFilter::new();

        // Valid values
        assert_eq!(filter.filter(200), Some(200));
        assert_eq!(filter.filter(500), Some(500));
        assert_eq!(filter.filter(2000), Some(2000));

        // Invalid (above max)
        assert_eq!(filter.filter(2001), None);
        assert_eq!(filter.filter(5000), None);
    }

    #[test]
    fn test_power_filter_spike_detection() {
        let mut filter = PowerFilter::with_spike_detection(2000, 200);

        // First value always valid
        assert_eq!(filter.filter(200), Some(200));

        // Normal change
        assert_eq!(filter.filter(250), Some(250));

        // Spike (too big a jump)
        assert_eq!(filter.filter(500), None);

        // Still at 250, so this is valid
        assert_eq!(filter.filter(300), Some(300));
    }

    #[test]
    fn test_normalized_power() {
        let mut np_calc = NormalizedPowerCalculator::new();

        // Add 30 samples at constant power
        for _ in 0..30 {
            np_calc.add(200);
        }

        // After 30 seconds of constant 200W, NP should be close to 200
        let np = np_calc.normalized_power();
        assert!(np.is_some());
        assert!((np.unwrap() as i32 - 200).abs() <= 1);

        // Add higher power samples
        for _ in 0..30 {
            np_calc.add(300);
        }

        // NP should increase
        let np2 = np_calc.normalized_power().unwrap();
        assert!(np2 > 200);
    }
}
