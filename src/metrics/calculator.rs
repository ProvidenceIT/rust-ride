//! Metrics calculator for real-time training metrics.
//!
//! T039: Implement MetricsCalculator struct
//! T040: Implement process() to aggregate SensorReading
//! T041: Define AggregatedMetrics, PowerMetrics structs
//! T091-T094: TSS, IF, NP, calorie calculations

use crate::metrics::smoothing::{NormalizedPowerCalculator, PowerFilter, RollingAverage};
use crate::metrics::zones::{HRZones, PowerZones};
use crate::sensors::types::SensorReading;
use std::time::{Duration, Instant};

/// Aggregated metrics from all sensors for display.
#[derive(Debug, Clone, Default)]
pub struct AggregatedMetrics {
    /// Aggregation timestamp
    pub timestamp: Option<Instant>,
    /// Current instantaneous power
    pub power_instant: Option<u16>,
    /// 3-second rolling average power
    pub power_3s_avg: Option<u16>,
    /// 30-second rolling average power (for NP)
    pub power_30s_avg: Option<u16>,
    /// Current cadence
    pub cadence: Option<u8>,
    /// Current heart rate
    pub heart_rate: Option<u8>,
    /// Current speed in km/h
    pub speed: Option<f32>,
    /// Total distance in meters
    pub distance: f64,
    /// Total ride time
    pub elapsed_time: Duration,
    /// Total calories
    pub calories: u32,
    /// Current power zone (1-7)
    pub power_zone: Option<u8>,
    /// Current HR zone (1-5)
    pub hr_zone: Option<u8>,
    /// Running Normalized Power
    pub normalized_power: Option<u16>,
    /// Running TSS
    pub tss: Option<f32>,
    /// Running Intensity Factor
    pub intensity_factor: Option<f32>,
}

/// Power-specific metrics.
#[derive(Debug, Clone, Default)]
pub struct PowerMetrics {
    /// Instantaneous power
    pub instant: Option<u16>,
    /// 3-second average
    pub avg_3s: Option<u16>,
    /// 30-second average
    pub avg_30s: Option<u16>,
    /// Current zone (1-7)
    pub zone: Option<u8>,
    /// Average power for the ride
    pub avg: Option<u16>,
    /// Maximum power
    pub max: Option<u16>,
    /// Normalized Power
    pub normalized: Option<u16>,
}

/// Calculates real-time training metrics from sensor data.
pub struct MetricsCalculator {
    /// Power filter
    power_filter: PowerFilter,
    /// 3-second rolling average
    power_3s: RollingAverage,
    /// 30-second rolling average (for display)
    power_30s: RollingAverage,
    /// Normalized Power calculator
    np_calculator: NormalizedPowerCalculator,
    /// Power zones
    power_zones: Option<PowerZones>,
    /// HR zones
    hr_zones: Option<HRZones>,
    /// User's FTP
    ftp: u16,
    /// Total power sum for average
    power_sum: u64,
    /// Power sample count
    power_count: u32,
    /// Maximum power seen
    max_power: u16,
    /// Total distance in meters
    total_distance: f64,
    /// Total calories
    total_calories: u32,
    /// Ride start time
    start_time: Option<Instant>,
    /// Current aggregated metrics
    current_metrics: AggregatedMetrics,
}

impl MetricsCalculator {
    /// Create a new metrics calculator.
    pub fn new(ftp: u16) -> Self {
        Self {
            power_filter: PowerFilter::new(),
            power_3s: RollingAverage::three_second(),
            power_30s: RollingAverage::thirty_second(),
            np_calculator: NormalizedPowerCalculator::new(),
            power_zones: Some(PowerZones::from_ftp(ftp)),
            hr_zones: None,
            ftp,
            power_sum: 0,
            power_count: 0,
            max_power: 0,
            total_distance: 0.0,
            total_calories: 0,
            start_time: None,
            current_metrics: AggregatedMetrics::default(),
        }
    }

    /// Set power zones.
    pub fn set_power_zones(&mut self, zones: PowerZones) {
        self.power_zones = Some(zones);
    }

    /// Set HR zones.
    pub fn set_hr_zones(&mut self, zones: HRZones) {
        self.hr_zones = Some(zones);
    }

    /// Update FTP and recalculate zones.
    pub fn set_ftp(&mut self, ftp: u16) {
        self.ftp = ftp;
        self.power_zones = Some(PowerZones::from_ftp(ftp));
    }

    /// Process a sensor reading and update metrics.
    pub fn process(&mut self, reading: &SensorReading) -> &AggregatedMetrics {
        let now = reading.timestamp;

        // Start timer on first reading
        if self.start_time.is_none() {
            self.start_time = Some(now);
        }

        // Process power
        if let Some(power) = reading.power_watts {
            if let Some(filtered_power) = self.power_filter.filter(power) {
                // Update rolling averages
                self.current_metrics.power_instant = Some(filtered_power);
                self.current_metrics.power_3s_avg = self.power_3s.add(filtered_power);
                self.current_metrics.power_30s_avg = self.power_30s.add(filtered_power);

                // Update NP
                self.current_metrics.normalized_power = self.np_calculator.add(filtered_power);

                // Update average and max
                self.power_sum += filtered_power as u64;
                self.power_count += 1;
                self.max_power = self.max_power.max(filtered_power);

                // Calculate power zone
                if let Some(zones) = &self.power_zones {
                    self.current_metrics.power_zone =
                        Some(zones.get_zone(self.current_metrics.power_3s_avg.unwrap_or(filtered_power)));
                }

                // Update calories (kJ ≈ kcal for cycling)
                // 1 watt for 1 second = 1 joule
                // Accumulate joules, divide by 1000 for kJ, ≈ kcal
                self.total_calories = (self.power_sum / 1000) as u32;
            }
        }

        // Process heart rate
        if let Some(hr) = reading.heart_rate_bpm {
            self.current_metrics.heart_rate = Some(hr);

            if let Some(zones) = &self.hr_zones {
                self.current_metrics.hr_zone = Some(zones.get_zone(hr));
            }
        }

        // Process cadence
        if let Some(cadence) = reading.cadence_rpm {
            self.current_metrics.cadence = Some(cadence);
        }

        // Process speed
        if let Some(speed) = reading.speed_kmh {
            self.current_metrics.speed = Some(speed);
        }

        // Process distance
        if let Some(delta) = reading.distance_delta_m {
            self.total_distance += delta as f64;
        }

        // Update aggregated metrics
        self.current_metrics.timestamp = Some(now);
        self.current_metrics.distance = self.total_distance;
        self.current_metrics.calories = self.total_calories;

        if let Some(start) = self.start_time {
            self.current_metrics.elapsed_time = now.duration_since(start);
        }

        // Calculate TSS and IF
        if let Some(np) = self.current_metrics.normalized_power {
            if self.ftp > 0 {
                let if_value = np as f32 / self.ftp as f32;
                self.current_metrics.intensity_factor = Some(if_value);

                let duration_hours = self.current_metrics.elapsed_time.as_secs_f32() / 3600.0;
                let tss = duration_hours * if_value * if_value * 100.0;
                self.current_metrics.tss = Some(tss);
            }
        }

        &self.current_metrics
    }

    /// Get the current aggregated metrics.
    pub fn current_metrics(&self) -> &AggregatedMetrics {
        &self.current_metrics
    }

    /// Get power-specific metrics.
    pub fn power_metrics(&self) -> PowerMetrics {
        PowerMetrics {
            instant: self.current_metrics.power_instant,
            avg_3s: self.current_metrics.power_3s_avg,
            avg_30s: self.current_metrics.power_30s_avg,
            zone: self.current_metrics.power_zone,
            avg: if self.power_count > 0 {
                Some((self.power_sum / self.power_count as u64) as u16)
            } else {
                None
            },
            max: if self.max_power > 0 {
                Some(self.max_power)
            } else {
                None
            },
            normalized: self.current_metrics.normalized_power,
        }
    }

    /// Get the average power for the ride.
    pub fn average_power(&self) -> Option<u16> {
        if self.power_count > 0 {
            Some((self.power_sum / self.power_count as u64) as u16)
        } else {
            None
        }
    }

    /// Get the maximum power for the ride.
    pub fn max_power(&self) -> Option<u16> {
        if self.max_power > 0 {
            Some(self.max_power)
        } else {
            None
        }
    }

    /// Reset all metrics for a new ride.
    pub fn reset(&mut self) {
        self.power_filter.reset();
        self.power_3s.reset();
        self.power_30s.reset();
        self.np_calculator.reset();
        self.power_sum = 0;
        self.power_count = 0;
        self.max_power = 0;
        self.total_distance = 0.0;
        self.total_calories = 0;
        self.start_time = None;
        self.current_metrics = AggregatedMetrics::default();
    }
}

/// Estimate calories from power and duration.
///
/// Uses the approximation that 1 kJ of work ≈ 1 kcal burned
/// (accounting for ~25% metabolic efficiency).
pub fn estimate_calories(power_watts: u16, duration_seconds: u32) -> u32 {
    // Energy (kJ) = Power (W) × Time (s) / 1000
    // At ~25% efficiency, calories burned ≈ kJ / 0.25 = kJ × 4
    // But convention in cycling is kJ ≈ kcal (already accounts for efficiency in some way)
    let kilojoules = power_watts as u64 * duration_seconds as u64 / 1000;
    kilojoules as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_reading(power: u16) -> SensorReading {
        SensorReading {
            sensor_id: Uuid::new_v4(),
            timestamp: Instant::now(),
            power_watts: Some(power),
            cadence_rpm: None,
            heart_rate_bpm: None,
            speed_kmh: None,
            distance_delta_m: None,
        }
    }

    #[test]
    fn test_metrics_calculator_basic() {
        let mut calc = MetricsCalculator::new(200);

        let reading = make_reading(200);
        let metrics = calc.process(&reading);

        assert_eq!(metrics.power_instant, Some(200));
    }

    #[test]
    fn test_power_zone_calculation() {
        let mut calc = MetricsCalculator::new(200);

        // Zone 4 (threshold) is 91-105% = 182-210W
        let reading = make_reading(200);
        let metrics = calc.process(&reading);

        assert_eq!(metrics.power_zone, Some(4));
    }

    #[test]
    fn test_calorie_estimation() {
        // 200W for 1 hour = 720 kJ ≈ 720 kcal
        let calories = estimate_calories(200, 3600);
        assert_eq!(calories, 720);
    }
}
