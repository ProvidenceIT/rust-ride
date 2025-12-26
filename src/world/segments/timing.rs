//! Segment timing detection and recording.

use super::{Segment, SegmentTime};
use uuid::Uuid;

/// Segment timing state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingState {
    /// Not in a segment
    Inactive,
    /// Approaching segment start
    Approaching,
    /// Currently in segment
    Active,
    /// Just completed segment
    Completed,
}

/// Active segment timing
#[derive(Debug, Clone)]
pub struct ActiveTiming {
    /// Segment being timed
    pub segment_id: Uuid,
    /// Time started (from ride elapsed time)
    pub start_time_seconds: f64,
    /// Current elapsed time in segment
    pub elapsed_seconds: f64,
    /// Best time to beat (if known)
    pub target_time_seconds: Option<f64>,
    /// Running average power in segment
    pub avg_power_watts: f64,
    /// Running average HR in segment
    pub avg_heart_rate: f64,
    /// Sample count for averages
    sample_count: u32,
}

impl ActiveTiming {
    /// Create new timing session
    pub fn new(segment_id: Uuid, start_time: f64, target: Option<f64>) -> Self {
        Self {
            segment_id,
            start_time_seconds: start_time,
            elapsed_seconds: 0.0,
            target_time_seconds: target,
            avg_power_watts: 0.0,
            avg_heart_rate: 0.0,
            sample_count: 0,
        }
    }

    /// Update timing with new sample
    pub fn update(&mut self, elapsed: f64, power: Option<u16>, hr: Option<u8>) {
        self.elapsed_seconds = elapsed;
        self.sample_count += 1;

        if let Some(p) = power {
            let n = self.sample_count as f64;
            self.avg_power_watts = ((self.avg_power_watts * (n - 1.0)) + p as f64) / n;
        }

        if let Some(h) = hr {
            let n = self.sample_count as f64;
            self.avg_heart_rate = ((self.avg_heart_rate * (n - 1.0)) + h as f64) / n;
        }
    }

    /// Get time delta vs target (negative = ahead, positive = behind)
    pub fn delta_vs_target(&self) -> Option<f64> {
        self.target_time_seconds.map(|target| {
            // Project current pace to full segment
            self.elapsed_seconds - target
        })
    }
}

/// Segment timing manager
pub struct SegmentTimer {
    /// All segments on current route
    segments: Vec<Segment>,
    /// Currently active timing (if any)
    active: Option<ActiveTiming>,
    /// Current state
    state: TimingState,
    /// Distance to next segment start (meters)
    distance_to_next: Option<f64>,
    /// Completed times this ride
    completed_times: Vec<SegmentTime>,
}

impl SegmentTimer {
    /// Create timer for a route's segments
    pub fn new(segments: Vec<Segment>) -> Self {
        Self {
            segments,
            active: None,
            state: TimingState::Inactive,
            distance_to_next: None,
            completed_times: Vec::new(),
        }
    }

    /// Update timing based on current position
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        distance_meters: f64,
        ride_time_seconds: f64,
        power: Option<u16>,
        hr: Option<u8>,
        user_id: Uuid,
        ride_id: Uuid,
        ftp: u16,
        personal_best: Option<f64>,
    ) -> Option<SegmentTime> {
        let mut completed: Option<SegmentTime> = None;

        // Check if we're in any segment
        let in_segment = self.segments.iter().find(|s| {
            distance_meters >= s.start_distance_meters && distance_meters <= s.end_distance_meters
        });

        // Check approaching segments (within 200m)
        let approaching = self.segments.iter().find(|s| {
            let dist_to_start = s.start_distance_meters - distance_meters;
            dist_to_start > 0.0 && dist_to_start <= 200.0
        });

        match (&self.active, in_segment) {
            // Not timing, entered segment
            (None, Some(segment)) => {
                self.active = Some(ActiveTiming::new(
                    segment.id,
                    ride_time_seconds,
                    personal_best,
                ));
                self.state = TimingState::Active;
            }

            // Currently timing, still in segment
            (Some(timing), Some(segment)) if timing.segment_id == segment.id => {
                if let Some(ref mut active) = self.active {
                    let elapsed = ride_time_seconds - active.start_time_seconds;
                    active.update(elapsed, power, hr);
                }
            }

            // Was timing, exited segment
            (Some(timing), None) => {
                let final_time = ride_time_seconds - timing.start_time_seconds;

                let mut segment_time =
                    SegmentTime::new(timing.segment_id, user_id, ride_id, final_time, ftp);

                segment_time = segment_time.with_metrics(
                    if timing.avg_power_watts > 0.0 {
                        Some(timing.avg_power_watts as u16)
                    } else {
                        None
                    },
                    if timing.avg_heart_rate > 0.0 {
                        Some(timing.avg_heart_rate as u8)
                    } else {
                        None
                    },
                );

                // Check if personal best
                if let Some(pb) = personal_best {
                    if final_time < pb {
                        segment_time.is_personal_best = true;
                    }
                } else {
                    // First attempt is always PB
                    segment_time.is_personal_best = true;
                }

                self.completed_times.push(segment_time.clone());
                completed = Some(segment_time);
                self.active = None;
                self.state = TimingState::Completed;
            }

            // Entered different segment (shouldn't happen with valid segments)
            (Some(_), Some(segment)) => {
                self.active = Some(ActiveTiming::new(
                    segment.id,
                    ride_time_seconds,
                    personal_best,
                ));
                self.state = TimingState::Active;
            }

            // Not in any segment
            (None, None) => {
                if approaching.is_some() {
                    self.state = TimingState::Approaching;
                    self.distance_to_next =
                        approaching.map(|s| s.start_distance_meters - distance_meters);
                } else {
                    self.state = TimingState::Inactive;
                    self.distance_to_next = None;
                }
            }
        }

        completed
    }

    /// Get current timing state
    pub fn state(&self) -> TimingState {
        self.state
    }

    /// Get active timing (if any)
    pub fn active(&self) -> Option<&ActiveTiming> {
        self.active.as_ref()
    }

    /// Get distance to next segment
    pub fn distance_to_next(&self) -> Option<f64> {
        self.distance_to_next
    }

    /// Get all completed times from this ride
    pub fn completed_times(&self) -> &[SegmentTime] {
        &self.completed_times
    }

    /// Reset for new ride
    pub fn reset(&mut self) {
        self.active = None;
        self.state = TimingState::Inactive;
        self.distance_to_next = None;
        self.completed_times.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_states() {
        let segment = Segment::new(
            Uuid::new_v4(),
            "Test Segment".to_string(),
            1000.0,
            2000.0,
            50.0,
        );

        let mut timer = SegmentTimer::new(vec![segment]);

        // Before segment
        timer.update(
            500.0,
            0.0,
            Some(200),
            Some(150),
            Uuid::new_v4(),
            Uuid::new_v4(),
            250,
            None,
        );
        assert_eq!(timer.state(), TimingState::Inactive);

        // Approaching
        timer.update(
            850.0,
            5.0,
            Some(200),
            Some(150),
            Uuid::new_v4(),
            Uuid::new_v4(),
            250,
            None,
        );
        assert_eq!(timer.state(), TimingState::Approaching);

        // In segment
        timer.update(
            1500.0,
            10.0,
            Some(200),
            Some(150),
            Uuid::new_v4(),
            Uuid::new_v4(),
            250,
            None,
        );
        assert_eq!(timer.state(), TimingState::Active);
    }

    #[test]
    fn test_active_timing_update() {
        let mut timing = ActiveTiming::new(Uuid::new_v4(), 0.0, Some(60.0));

        timing.update(10.0, Some(200), Some(150));
        timing.update(20.0, Some(220), Some(155));

        assert_eq!(timing.elapsed_seconds, 20.0);
        assert!((timing.avg_power_watts - 210.0).abs() < 0.1);
    }
}
