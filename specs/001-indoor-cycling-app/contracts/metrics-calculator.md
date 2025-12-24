# Contract: Metrics Calculator

**Module**: `src/metrics/`
**Responsibility**: Real-time and post-ride training metric calculations

## Public Interface

### MetricsCalculator

Real-time metrics aggregation and calculation.

```rust
pub struct MetricsCalculator {
    // Internal state
}

impl MetricsCalculator {
    /// Create a new metrics calculator
    pub fn new(config: MetricsConfig) -> Self;

    /// Configure user's training zones
    pub fn set_zones(&mut self, power_zones: PowerZones, hr_zones: Option<HRZones>);

    /// Set user's FTP for calculations
    pub fn set_ftp(&mut self, ftp: u16);

    /// Reset all running calculations (for new ride)
    pub fn reset(&mut self);

    /// Process a new sensor reading
    /// Returns aggregated metrics for display
    pub fn process(&mut self, reading: SensorReading) -> AggregatedMetrics;

    /// Get current power zone (1-7)
    pub fn current_power_zone(&self, power: u16) -> u8;

    /// Get current heart rate zone (1-5)
    pub fn current_hr_zone(&self, hr: u8) -> Option<u8>;

    /// Get running normalized power
    pub fn normalized_power(&self) -> Option<u16>;

    /// Get running TSS
    pub fn tss(&self) -> Option<f32>;

    /// Get running intensity factor
    pub fn intensity_factor(&self) -> Option<f32>;
}
```

### Configuration

```rust
pub struct MetricsConfig {
    /// Window size for short-term power average (default: 3 seconds)
    pub power_avg_window_short: usize,

    /// Window size for NP calculation (default: 30 seconds)
    pub power_avg_window_np: usize,

    /// Smoothing factor for speed calculations
    pub speed_smoothing: f32,

    /// Enable noise filtering for power readings
    pub filter_power_noise: bool,

    /// Power spike threshold (readings above this are filtered)
    pub power_spike_threshold: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            power_avg_window_short: 3,
            power_avg_window_np: 30,
            speed_smoothing: 0.3,
            filter_power_noise: true,
            power_spike_threshold: 2000,
        }
    }
}
```

### Output Types

```rust
/// Aggregated metrics for UI display
pub struct AggregatedMetrics {
    /// Timestamp of this aggregation
    pub timestamp: Instant,

    /// Power metrics
    pub power: PowerMetrics,

    /// Heart rate metrics
    pub heart_rate: Option<HeartRateMetrics>,

    /// Cadence (rpm)
    pub cadence: Option<u8>,

    /// Speed (km/h)
    pub speed: Option<f32>,

    /// Distance from start (meters)
    pub distance: f64,

    /// Elapsed time
    pub elapsed_time: Duration,

    /// Calories burned
    pub calories: u32,
}

pub struct PowerMetrics {
    /// Instantaneous power (watts)
    pub instant: Option<u16>,

    /// 3-second rolling average
    pub avg_3s: Option<u16>,

    /// Current power zone (1-7)
    pub zone: Option<u8>,

    /// Zone color for display
    pub zone_color: Option<Color>,

    /// Running normalized power
    pub normalized: Option<u16>,

    /// Running TSS
    pub tss: Option<f32>,

    /// Running intensity factor
    pub intensity_factor: Option<f32>,
}

pub struct HeartRateMetrics {
    /// Current heart rate (bpm)
    pub current: u8,

    /// Current HR zone (1-5)
    pub zone: Option<u8>,

    /// Zone color for display
    pub zone_color: Option<Color>,
}
```

## Zone Calculation

### Coggan Power Zones (7-zone model)

```rust
impl PowerZones {
    /// Calculate zones from FTP using Coggan model
    pub fn from_ftp(ftp: u16) -> Self {
        Self {
            z1_recovery: ZoneRange {
                min_percent: 0,
                max_percent: 55,
                min_watts: 0,
                max_watts: (ftp as f32 * 0.55) as u16,
                color: Color::GRAY,
                name: "Recovery".to_string(),
            },
            z2_endurance: ZoneRange {
                min_percent: 56,
                max_percent: 75,
                min_watts: (ftp as f32 * 0.56) as u16,
                max_watts: (ftp as f32 * 0.75) as u16,
                color: Color::BLUE,
                name: "Endurance".to_string(),
            },
            z3_tempo: ZoneRange {
                min_percent: 76,
                max_percent: 90,
                min_watts: (ftp as f32 * 0.76) as u16,
                max_watts: (ftp as f32 * 0.90) as u16,
                color: Color::GREEN,
                name: "Tempo".to_string(),
            },
            z4_threshold: ZoneRange {
                min_percent: 91,
                max_percent: 105,
                min_watts: (ftp as f32 * 0.91) as u16,
                max_watts: (ftp as f32 * 1.05) as u16,
                color: Color::YELLOW,
                name: "Threshold".to_string(),
            },
            z5_vo2max: ZoneRange {
                min_percent: 106,
                max_percent: 120,
                min_watts: (ftp as f32 * 1.06) as u16,
                max_watts: (ftp as f32 * 1.20) as u16,
                color: Color::ORANGE,
                name: "VO2max".to_string(),
            },
            z6_anaerobic: ZoneRange {
                min_percent: 121,
                max_percent: 150,
                min_watts: (ftp as f32 * 1.21) as u16,
                max_watts: (ftp as f32 * 1.50) as u16,
                color: Color::RED,
                name: "Anaerobic".to_string(),
            },
            z7_neuromuscular: ZoneRange {
                min_percent: 151,
                max_percent: 255,
                min_watts: (ftp as f32 * 1.51) as u16,
                max_watts: u16::MAX,
                color: Color::PURPLE,
                name: "Neuromuscular".to_string(),
            },
            custom: false,
        }
    }

    pub fn get_zone(&self, power: u16) -> u8 {
        if power <= self.z1_recovery.max_watts { 1 }
        else if power <= self.z2_endurance.max_watts { 2 }
        else if power <= self.z3_tempo.max_watts { 3 }
        else if power <= self.z4_threshold.max_watts { 4 }
        else if power <= self.z5_vo2max.max_watts { 5 }
        else if power <= self.z6_anaerobic.max_watts { 6 }
        else { 7 }
    }
}
```

### Karvonen Heart Rate Zones (5-zone model)

```rust
impl HRZones {
    /// Calculate zones using Karvonen formula
    /// HRR = Max HR - Resting HR
    /// Target = ((HRR × intensity%) + Resting HR)
    pub fn from_max_resting(max_hr: u8, resting_hr: u8) -> Self {
        let hrr = max_hr as f32 - resting_hr as f32;

        Self {
            z1_recovery: HRZoneRange {
                min_bpm: resting_hr,
                max_bpm: (resting_hr as f32 + hrr * 0.60) as u8,
                color: Color::BLUE,
                name: "Recovery".to_string(),
            },
            z2_aerobic: HRZoneRange {
                min_bpm: (resting_hr as f32 + hrr * 0.60) as u8,
                max_bpm: (resting_hr as f32 + hrr * 0.70) as u8,
                color: Color::GREEN,
                name: "Aerobic".to_string(),
            },
            z3_tempo: HRZoneRange {
                min_bpm: (resting_hr as f32 + hrr * 0.70) as u8,
                max_bpm: (resting_hr as f32 + hrr * 0.80) as u8,
                color: Color::YELLOW,
                name: "Tempo".to_string(),
            },
            z4_threshold: HRZoneRange {
                min_bpm: (resting_hr as f32 + hrr * 0.80) as u8,
                max_bpm: (resting_hr as f32 + hrr * 0.90) as u8,
                color: Color::ORANGE,
                name: "Threshold".to_string(),
            },
            z5_maximum: HRZoneRange {
                min_bpm: (resting_hr as f32 + hrr * 0.90) as u8,
                max_bpm: max_hr,
                color: Color::RED,
                name: "Maximum".to_string(),
            },
            custom: false,
        }
    }

    pub fn get_zone(&self, hr: u8) -> u8 {
        if hr <= self.z1_recovery.max_bpm { 1 }
        else if hr <= self.z2_aerobic.max_bpm { 2 }
        else if hr <= self.z3_tempo.max_bpm { 3 }
        else if hr <= self.z4_threshold.max_bpm { 4 }
        else { 5 }
    }
}
```

## Training Metrics Algorithms

### Normalized Power (NP)

Algorithm:
1. Calculate 30-second rolling average of power
2. Raise each averaged value to the 4th power
3. Take the mean of those values
4. Take the 4th root of the mean

```rust
fn calculate_normalized_power(samples: &[u16]) -> Option<u16> {
    if samples.len() < 30 {
        return None;
    }

    // 30-second rolling average
    let rolling_avg: Vec<f64> = samples
        .windows(30)
        .map(|window| window.iter().map(|&p| p as f64).sum::<f64>() / 30.0)
        .collect();

    // 4th power average
    let fourth_power_avg = rolling_avg.iter()
        .map(|p| p.powi(4))
        .sum::<f64>() / rolling_avg.len() as f64;

    // 4th root
    Some(fourth_power_avg.powf(0.25) as u16)
}
```

### Intensity Factor (IF)

```rust
fn calculate_intensity_factor(np: u16, ftp: u16) -> f32 {
    np as f32 / ftp as f32
}
```

### Training Stress Score (TSS)

```rust
fn calculate_tss(duration_seconds: u32, np: u16, ftp: u16) -> f32 {
    let duration_hours = duration_seconds as f32 / 3600.0;
    let if_value = np as f32 / ftp as f32;
    duration_hours * if_value.powi(2) * 100.0
}
```

### Calorie Estimation

```rust
/// Estimate calories from power (cycling efficiency ~25%)
/// Energy (kJ) = Power (W) × Time (s) / 1000
/// Calories ≈ kJ (for cycling, due to ~25% efficiency)
fn calculate_calories(power_watts: u16, duration_seconds: u32) -> u32 {
    let energy_kj = (power_watts as f64 * duration_seconds as f64) / 1000.0;
    energy_kj as u32  // 1 kJ ≈ 1 kcal for cycling
}
```

## Smoothing and Filtering

### Power Smoothing (3-second average)

```rust
struct RollingAverage {
    buffer: VecDeque<u16>,
    window_size: usize,
    sum: u32,
}

impl RollingAverage {
    fn push(&mut self, value: u16) -> u16 {
        self.sum += value as u32;
        self.buffer.push_back(value);

        if self.buffer.len() > self.window_size {
            self.sum -= self.buffer.pop_front().unwrap() as u32;
        }

        (self.sum / self.buffer.len() as u32) as u16
    }
}
```

### Noise Filtering

```rust
fn filter_power_spike(current: u16, previous: u16, threshold: u16) -> Option<u16> {
    // If reading jumps more than threshold from previous, discard
    if current > previous + threshold || current < previous.saturating_sub(threshold) {
        // Could also return previous value as fallback
        None
    } else {
        Some(current)
    }
}
```

### Speed Smoothing (Exponential)

```rust
fn smooth_speed(current: f32, previous: f32, alpha: f32) -> f32 {
    alpha * current + (1.0 - alpha) * previous
}
```

## Unit Conversion

```rust
/// Speed conversions
fn kmh_to_mph(kmh: f32) -> f32 { kmh * 0.621371 }
fn mph_to_kmh(mph: f32) -> f32 { mph * 1.60934 }

/// Distance conversions
fn meters_to_miles(m: f64) -> f64 { m * 0.000621371 }
fn meters_to_km(m: f64) -> f64 { m / 1000.0 }

/// Weight conversions
fn kg_to_lbs(kg: f32) -> f32 { kg * 2.20462 }
fn lbs_to_kg(lbs: f32) -> f32 { lbs * 0.453592 }
```

## Post-Ride Analysis Functions

```rust
/// Calculate average, max, and distribution for a series
pub fn analyze_power_series(samples: &[Option<u16>]) -> PowerAnalysis {
    let valid: Vec<u16> = samples.iter().filter_map(|&p| p).collect();

    if valid.is_empty() {
        return PowerAnalysis::empty();
    }

    PowerAnalysis {
        average: (valid.iter().map(|&p| p as f64).sum::<f64>() / valid.len() as f64) as u16,
        max: *valid.iter().max().unwrap(),
        min: *valid.iter().min().unwrap(),
        normalized: calculate_normalized_power(&valid),
        time_in_zones: calculate_time_in_zones(&valid, &power_zones),
    }
}

pub struct PowerAnalysis {
    pub average: u16,
    pub max: u16,
    pub min: u16,
    pub normalized: Option<u16>,
    pub time_in_zones: [Duration; 7],
}

pub struct TimeInZones {
    pub zone_times: [Duration; 7],  // For power zones 1-7
    pub zone_percentages: [f32; 7], // As percentage of total time
}
```
