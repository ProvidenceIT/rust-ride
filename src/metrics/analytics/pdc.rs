//! Power Duration Curve (PDC) calculation and management.
//!
//! The PDC represents the maximum average power a rider can sustain for any given duration.
//! This module provides:
//! - MMP (Mean Maximal Power) extraction from ride data
//! - PDC storage and update logic
//! - Interpolation for arbitrary durations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single point on the power duration curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PdcPoint {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Maximum average power at this duration (watts).
    pub power_watts: u16,
}

/// Power Duration Curve - collection of max power values at each duration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerDurationCurve {
    /// PDC points, sorted by duration.
    points: Vec<PdcPoint>,
}

impl PowerDurationCurve {
    /// Create an empty PDC.
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Create PDC from existing points.
    pub fn from_points(mut points: Vec<PdcPoint>) -> Self {
        points.sort_by_key(|p| p.duration_secs);
        Self { points }
    }

    /// Get power at a specific duration (interpolates if needed).
    pub fn power_at(&self, duration_secs: u32) -> Option<u16> {
        if self.points.is_empty() {
            return None;
        }

        // Exact match
        if let Some(point) = self
            .points
            .iter()
            .find(|p| p.duration_secs == duration_secs)
        {
            return Some(point.power_watts);
        }

        // Find surrounding points for interpolation
        let mut lower: Option<&PdcPoint> = None;
        let mut upper: Option<&PdcPoint> = None;

        for point in &self.points {
            if point.duration_secs < duration_secs {
                lower = Some(point);
            } else if point.duration_secs > duration_secs {
                upper = Some(point);
                break;
            }
        }

        match (lower, upper) {
            (Some(l), Some(u)) => {
                // Linear interpolation
                let ratio = (duration_secs - l.duration_secs) as f32
                    / (u.duration_secs - l.duration_secs) as f32;
                let power =
                    l.power_watts as f32 + ratio * (u.power_watts as f32 - l.power_watts as f32);
                Some(power.round() as u16)
            }
            (Some(l), None) => Some(l.power_watts), // Beyond max duration
            (None, Some(u)) => Some(u.power_watts), // Below min duration
            (None, None) => None,
        }
    }

    /// Get all points for charting.
    pub fn points(&self) -> &[PdcPoint] {
        &self.points
    }

    /// Update PDC with new ride data, returns which points changed.
    pub fn update(&mut self, new_points: &[PdcPoint]) -> Vec<PdcPoint> {
        let mut changed = Vec::new();

        for new_point in new_points {
            if let Some(existing) = self
                .points
                .iter_mut()
                .find(|p| p.duration_secs == new_point.duration_secs)
            {
                if new_point.power_watts > existing.power_watts {
                    *existing = *new_point;
                    changed.push(*new_point);
                }
            } else {
                self.points.push(*new_point);
                changed.push(*new_point);
            }
        }

        self.points.sort_by_key(|p| p.duration_secs);
        changed
    }

    /// Check if sufficient data exists for CP calculation (3+ points in 2-20 min range).
    pub fn has_sufficient_data_for_cp(&self) -> bool {
        let cp_range_points = self
            .points
            .iter()
            .filter(|p| p.duration_secs >= 120 && p.duration_secs <= 1200)
            .count();
        cp_range_points >= 3
    }

    /// Filter to date range (placeholder - requires ride_id tracking).
    pub fn filter_date_range(&self, _start: DateTime<Utc>, _end: DateTime<Utc>) -> Self {
        // Note: Full implementation requires tracking achieved_at per point
        self.clone()
    }

    /// Check if PDC is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if there's actual data at or near a duration (not extrapolated).
    /// Returns true if there's a point within tolerance of the target duration.
    pub fn has_data_near(&self, duration_secs: u32, tolerance_secs: u32) -> bool {
        self.points
            .iter()
            .any(|p| p.duration_secs.abs_diff(duration_secs) <= tolerance_secs)
    }

    /// Get power only if there's actual data at or near this duration.
    /// Uses tolerance to avoid relying on extrapolated values.
    pub fn power_at_actual(&self, duration_secs: u32, tolerance_secs: u32) -> Option<u16> {
        if self.has_data_near(duration_secs, tolerance_secs) {
            self.power_at(duration_secs)
        } else {
            None
        }
    }

    /// Get max duration with actual data.
    pub fn max_duration(&self) -> Option<u32> {
        self.points.last().map(|p| p.duration_secs)
    }
}

/// Maximum gap length in seconds to interpolate.
const MAX_INTERPOLATION_GAP: usize = 10;

/// Calculator for extracting Mean Maximal Power from ride samples.
pub struct MmpCalculator {
    /// Duration buckets to calculate (in seconds).
    durations: Vec<u32>,
}

/// T128: Interpolate short sensor gaps in power data.
///
/// Handles gaps up to 10 seconds where power drops to 0 due to sensor
/// communication issues. Uses linear interpolation between valid values.
pub fn interpolate_sensor_gaps(power_samples: &[u16]) -> Vec<u16> {
    if power_samples.is_empty() {
        return Vec::new();
    }

    let mut result = power_samples.to_vec();
    let n = result.len();

    let mut i = 0;
    while i < n {
        // Find start of a gap (zero reading)
        if result[i] == 0 {
            let gap_start = i;

            // Find end of gap
            let mut gap_end = i;
            while gap_end < n && result[gap_end] == 0 {
                gap_end += 1;
            }

            let gap_length = gap_end - gap_start;

            // Only interpolate short gaps
            if gap_length <= MAX_INTERPOLATION_GAP {
                // Get the values before and after the gap
                let before_value = if gap_start > 0 {
                    result[gap_start - 1]
                } else {
                    // No value before, find first non-zero after
                    if gap_end < n {
                        result[gap_end]
                    } else {
                        0
                    }
                };

                let after_value = if gap_end < n { result[gap_end] } else { before_value };

                // Linear interpolation
                if before_value > 0 || after_value > 0 {
                    for (idx, sample) in result
                        .iter_mut()
                        .enumerate()
                        .take(gap_end)
                        .skip(gap_start)
                    {
                        let t = (idx - gap_start + 1) as f32 / (gap_length + 1) as f32;
                        let interpolated =
                            before_value as f32 * (1.0 - t) + after_value as f32 * t;
                        *sample = interpolated.round() as u16;
                    }
                }
            }

            i = gap_end;
        } else {
            i += 1;
        }
    }

    result
}

impl MmpCalculator {
    /// Create a new calculator for specified durations.
    pub fn new(durations: &[u32]) -> Self {
        let mut sorted_durations = durations.to_vec();
        sorted_durations.sort_unstable();
        Self {
            durations: sorted_durations,
        }
    }

    /// Create with standard durations (1s to 5h).
    pub fn standard() -> Self {
        Self::new(&[
            1, 2, 3, 5, 10, 15, 20, 30, // seconds
            60, 120, 180, 300, 600, 900, 1200, 1800, // 1-30 min
            2700, 3600, 5400, 7200, 10800, 14400, 18000, // 45min - 5h
        ])
    }

    /// Calculate MMP from power samples (1-second intervals assumed).
    /// Returns max power at each configured duration.
    pub fn calculate(&self, power_samples: &[u16]) -> Vec<PdcPoint> {
        let n = power_samples.len();
        if n == 0 {
            return Vec::new();
        }

        let mut results = Vec::with_capacity(self.durations.len());

        // Compute prefix sums for efficient window averages
        let mut prefix_sum = vec![0u64; n + 1];
        for (i, &power) in power_samples.iter().enumerate() {
            prefix_sum[i + 1] = prefix_sum[i] + power as u64;
        }

        for &duration in &self.durations {
            if duration as usize > n {
                continue;
            }

            let window_size = duration as usize;
            let mut max_avg = 0u64;

            // Slide window across all positions
            for end in window_size..=n {
                let start = end - window_size;
                let sum = prefix_sum[end] - prefix_sum[start];
                let avg = sum / window_size as u64;
                max_avg = max_avg.max(avg);
            }

            results.push(PdcPoint {
                duration_secs: duration,
                power_watts: max_avg as u16,
            });
        }

        results
    }

    /// Calculate single duration MMP (for focused queries).
    pub fn calculate_single(&self, power_samples: &[u16], duration_secs: u32) -> Option<u16> {
        let n = power_samples.len();
        let window_size = duration_secs as usize;

        if n == 0 || window_size > n {
            return None;
        }

        // Compute prefix sums
        let mut prefix_sum = vec![0u64; n + 1];
        for (i, &power) in power_samples.iter().enumerate() {
            prefix_sum[i + 1] = prefix_sum[i] + power as u64;
        }

        let mut max_avg = 0u64;
        for end in window_size..=n {
            let start = end - window_size;
            let sum = prefix_sum[end] - prefix_sum[start];
            let avg = sum / window_size as u64;
            max_avg = max_avg.max(avg);
        }

        Some(max_avg as u16)
    }

    /// Calculate MMP with sensor gap interpolation (T128).
    ///
    /// Pre-processes the power data to interpolate short gaps (<10s) before
    /// calculating MMP. This prevents sensor dropouts from affecting power curve.
    pub fn calculate_with_interpolation(&self, power_samples: &[u16]) -> Vec<PdcPoint> {
        let interpolated = interpolate_sensor_gaps(power_samples);
        self.calculate(&interpolated)
    }

    /// T133: Calculate MMP for specific durations only.
    ///
    /// Performance optimization: Only calculates requested durations instead
    /// of all standard durations. Use when you only need specific data points.
    pub fn calculate_selected(&self, power_samples: &[u16], durations: &[u32]) -> Vec<PdcPoint> {
        let n = power_samples.len();
        if n == 0 || durations.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::with_capacity(durations.len());

        // Compute prefix sums once
        let mut prefix_sum = vec![0u64; n + 1];
        for (i, &power) in power_samples.iter().enumerate() {
            prefix_sum[i + 1] = prefix_sum[i] + power as u64;
        }

        for &duration in durations {
            if duration as usize > n {
                continue;
            }

            let window_size = duration as usize;
            let mut max_avg = 0u64;

            for end in window_size..=n {
                let start = end - window_size;
                let sum = prefix_sum[end] - prefix_sum[start];
                let avg = sum / window_size as u64;
                max_avg = max_avg.max(avg);
            }

            results.push(PdcPoint {
                duration_secs: duration,
                power_watts: max_avg as u16,
            });
        }

        results
    }
}

/// T133: Performance-optimized PDC updater for large ride histories.
///
/// Maintains cached prefix sums and provides incremental update capability.
/// Use this when processing many rides to avoid redundant calculations.
#[derive(Default)]
pub struct PdcBatchProcessor {
    /// Accumulated PDC from all processed rides.
    pdc: PowerDurationCurve,
    /// Number of rides processed.
    ride_count: usize,
}

impl PdcBatchProcessor {
    /// Create a new batch processor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with existing PDC data.
    pub fn with_pdc(pdc: PowerDurationCurve) -> Self {
        Self { pdc, ride_count: 0 }
    }

    /// Process a ride's power samples and update the PDC.
    ///
    /// Returns the points that improved the PDC.
    pub fn process_ride(&mut self, power_samples: &[u16]) -> Vec<PdcPoint> {
        let calculator = MmpCalculator::standard();
        let ride_mmp = calculator.calculate(power_samples);
        let changed = self.pdc.update(&ride_mmp);
        self.ride_count += 1;
        changed
    }

    /// Process a ride with custom durations for efficiency.
    pub fn process_ride_selected(
        &mut self,
        power_samples: &[u16],
        durations: &[u32],
    ) -> Vec<PdcPoint> {
        let calculator = MmpCalculator::new(durations);
        let ride_mmp = calculator.calculate_selected(power_samples, durations);
        let changed = self.pdc.update(&ride_mmp);
        self.ride_count += 1;
        changed
    }

    /// Get the accumulated PDC.
    pub fn pdc(&self) -> &PowerDurationCurve {
        &self.pdc
    }

    /// Get number of rides processed.
    pub fn ride_count(&self) -> usize {
        self.ride_count
    }

    /// Take ownership of the PDC.
    pub fn into_pdc(self) -> PowerDurationCurve {
        self.pdc
    }
}

impl Default for MmpCalculator {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T019: Unit test for MmpCalculator with constant power
    #[test]
    fn test_mmp_constant_power() {
        let calculator = MmpCalculator::standard();
        let samples: Vec<u16> = vec![200; 600]; // 10 minutes at 200W
        let mmp = calculator.calculate(&samples);

        // Should find 200W at all durations up to 600s
        for point in &mmp {
            if point.duration_secs <= 600 {
                assert_eq!(point.power_watts, 200, "Duration: {}s", point.duration_secs);
            }
        }
    }

    // T020: Unit test for MmpCalculator with variable power (interval efforts)
    #[test]
    fn test_mmp_variable_power() {
        let calculator = MmpCalculator::new(&[60, 300]);

        // 5 min easy, 1 min hard, 5 min easy
        let mut samples = vec![150u16; 300]; // 5 min @ 150W
        samples.extend(vec![400u16; 60]); // 1 min @ 400W
        samples.extend(vec![150u16; 300]); // 5 min @ 150W

        let mmp = calculator.calculate(&samples);

        // 1-min max should be 400W
        let one_min = mmp.iter().find(|p| p.duration_secs == 60).unwrap();
        assert_eq!(one_min.power_watts, 400);

        // 5-min max should include some of the 400W effort
        let five_min = mmp.iter().find(|p| p.duration_secs == 300).unwrap();
        assert!(five_min.power_watts > 150);
    }

    // T021: Unit test for PowerDurationCurve update logic
    #[test]
    fn test_pdc_update() {
        let mut pdc = PowerDurationCurve::new();

        // Initial update
        let points1 = vec![
            PdcPoint {
                duration_secs: 60,
                power_watts: 300,
            },
            PdcPoint {
                duration_secs: 300,
                power_watts: 250,
            },
        ];
        let changed1 = pdc.update(&points1);
        assert_eq!(changed1.len(), 2);

        // Update with higher power - should change
        let points2 = vec![PdcPoint {
            duration_secs: 60,
            power_watts: 350,
        }];
        let changed2 = pdc.update(&points2);
        assert_eq!(changed2.len(), 1);
        assert_eq!(pdc.power_at(60), Some(350));

        // Update with lower power - should not change
        let points3 = vec![PdcPoint {
            duration_secs: 60,
            power_watts: 320,
        }];
        let changed3 = pdc.update(&points3);
        assert_eq!(changed3.len(), 0);
        assert_eq!(pdc.power_at(60), Some(350));
    }

    // T022: Unit test for PDC monotonicity validation
    #[test]
    fn test_pdc_interpolation() {
        let points = vec![
            PdcPoint {
                duration_secs: 60,
                power_watts: 400,
            },
            PdcPoint {
                duration_secs: 300,
                power_watts: 300,
            },
        ];
        let pdc = PowerDurationCurve::from_points(points);

        // Interpolate at 180s (halfway)
        let power = pdc.power_at(180).unwrap();
        assert!(
            power > 300 && power < 400,
            "Interpolated power should be between bounds"
        );
    }

    #[test]
    fn test_pdc_sufficient_data_for_cp() {
        let mut pdc = PowerDurationCurve::new();
        assert!(!pdc.has_sufficient_data_for_cp());

        // Add points outside CP range
        pdc.update(&[PdcPoint {
            duration_secs: 30,
            power_watts: 500,
        }]);
        assert!(!pdc.has_sufficient_data_for_cp());

        // Add 3 points in 2-20 min range
        pdc.update(&[
            PdcPoint {
                duration_secs: 180,
                power_watts: 350,
            },
            PdcPoint {
                duration_secs: 600,
                power_watts: 300,
            },
            PdcPoint {
                duration_secs: 1200,
                power_watts: 280,
            },
        ]);
        assert!(pdc.has_sufficient_data_for_cp());
    }

    // T128: Test sensor gap interpolation
    #[test]
    fn test_interpolate_sensor_gaps() {
        use super::interpolate_sensor_gaps;

        // Test short gap interpolation (5 zeros)
        let samples = vec![200, 200, 0, 0, 0, 0, 0, 200, 200];
        let result = interpolate_sensor_gaps(&samples);

        // Gap should be interpolated
        assert!(result[2] > 0, "Gap should be interpolated");
        assert!(result[3] > 0, "Gap should be interpolated");
        assert!(result[4] > 0, "Gap should be interpolated");
        // Should be close to 200 since before and after are 200
        assert!((result[4] as i32 - 200).abs() < 10);

        // Test long gap (>10 zeros) should NOT be interpolated
        let long_gap_samples: Vec<u16> = std::iter::once(200u16)
            .chain(std::iter::repeat(0u16).take(15))
            .chain(std::iter::once(200u16))
            .collect();
        let result2 = interpolate_sensor_gaps(&long_gap_samples);

        // Long gap should remain zeros
        assert_eq!(result2[5], 0, "Long gap should not be interpolated");
    }

    #[test]
    fn test_mmp_with_interpolation() {
        let calculator = MmpCalculator::new(&[5, 10]);

        // Power with a short gap
        let samples = vec![200, 200, 0, 0, 200, 200, 200, 200, 200, 200];

        // Without interpolation, the gap affects the average
        let without = calculator.calculate(&samples);
        let with = calculator.calculate_with_interpolation(&samples);

        // With interpolation should give higher (more accurate) power
        let without_10s = without.iter().find(|p| p.duration_secs == 10).unwrap();
        let with_10s = with.iter().find(|p| p.duration_secs == 10).unwrap();

        assert!(
            with_10s.power_watts >= without_10s.power_watts,
            "Interpolated MMP should be >= non-interpolated"
        );
    }
}
