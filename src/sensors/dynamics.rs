//! Cycling dynamics sensor parsing and data types.
//!
//! This module handles extended cycling power data including left/right power balance,
//! pedal smoothness, and torque effectiveness from dual-sided power meters.
//!
//! T045: Create CyclingDynamicsData, LeftRightBalance, PedalSmoothness, TorqueEffectiveness types
//! T046: Implement extended Cycling Power Service parsing for L/R balance
//! T047: Implement CyclingDynamicsProvider trait

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::sensors::SensorError;

/// Left/Right power balance data.
///
/// Represents the power distribution between left and right pedals.
/// Values are percentages that should sum to 100%.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LeftRightBalance {
    /// Left pedal power percentage (0-100)
    pub left_percent: f32,
    /// Right pedal power percentage (0-100)
    pub right_percent: f32,
    /// Whether the reference is from the left pedal (true) or right (false)
    pub reference_is_left: bool,
}

impl Default for LeftRightBalance {
    fn default() -> Self {
        Self {
            left_percent: 50.0,
            right_percent: 50.0,
            reference_is_left: true,
        }
    }
}

impl LeftRightBalance {
    /// Create a new balance from the reference side percentage.
    pub fn from_reference(reference_percent: f32, reference_is_left: bool) -> Self {
        let clamped = reference_percent.clamp(0.0, 100.0);
        let other = 100.0 - clamped;

        if reference_is_left {
            Self {
                left_percent: clamped,
                right_percent: other,
                reference_is_left,
            }
        } else {
            Self {
                left_percent: other,
                right_percent: clamped,
                reference_is_left,
            }
        }
    }

    /// Get the imbalance percentage (positive = left dominant, negative = right dominant).
    pub fn imbalance(&self) -> f32 {
        self.left_percent - self.right_percent
    }

    /// Check if the balance is within a reasonable range (e.g., within 10% of 50/50).
    pub fn is_balanced(&self, threshold: f32) -> bool {
        self.imbalance().abs() <= threshold
    }
}

/// Pedal smoothness data.
///
/// Represents how smoothly power is applied throughout the pedal stroke.
/// Higher values indicate smoother power delivery.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PedalSmoothness {
    /// Left pedal smoothness percentage (0-100)
    pub left_percent: f32,
    /// Right pedal smoothness percentage (0-100)
    pub right_percent: f32,
    /// Combined/average smoothness (0-100)
    pub combined_percent: f32,
}

impl Default for PedalSmoothness {
    fn default() -> Self {
        Self {
            left_percent: 0.0,
            right_percent: 0.0,
            combined_percent: 0.0,
        }
    }
}

impl PedalSmoothness {
    /// Create from left and right values.
    pub fn new(left: f32, right: f32) -> Self {
        Self {
            left_percent: left.clamp(0.0, 100.0),
            right_percent: right.clamp(0.0, 100.0),
            combined_percent: ((left + right) / 2.0).clamp(0.0, 100.0),
        }
    }

    /// Create from combined value only (single-sided power meter).
    pub fn from_combined(combined: f32) -> Self {
        let clamped = combined.clamp(0.0, 100.0);
        Self {
            left_percent: clamped,
            right_percent: clamped,
            combined_percent: clamped,
        }
    }
}

/// Torque effectiveness data.
///
/// Represents how effectively the rider converts torque into forward motion.
/// Higher values indicate more effective pedaling technique.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TorqueEffectiveness {
    /// Left pedal torque effectiveness percentage (0-100+)
    pub left_percent: f32,
    /// Right pedal torque effectiveness percentage (0-100+)
    pub right_percent: f32,
    /// Combined/average torque effectiveness
    pub combined_percent: f32,
}

impl Default for TorqueEffectiveness {
    fn default() -> Self {
        Self {
            left_percent: 0.0,
            right_percent: 0.0,
            combined_percent: 0.0,
        }
    }
}

impl TorqueEffectiveness {
    /// Create from left and right values.
    pub fn new(left: f32, right: f32) -> Self {
        Self {
            left_percent: left.max(0.0),
            right_percent: right.max(0.0),
            combined_percent: ((left + right) / 2.0).max(0.0),
        }
    }

    /// Create from combined value only.
    pub fn from_combined(combined: f32) -> Self {
        let clamped = combined.max(0.0);
        Self {
            left_percent: clamped,
            right_percent: clamped,
            combined_percent: clamped,
        }
    }
}

/// Power phase angle data for force vector analysis.
///
/// Represents the crank angle range where positive torque is applied.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PowerPhase {
    /// Start angle in degrees (0-360)
    pub start_angle: f32,
    /// End angle in degrees (0-360)
    pub end_angle: f32,
    /// Peak power angle in degrees (0-360)
    pub peak_angle: Option<f32>,
}

impl Default for PowerPhase {
    fn default() -> Self {
        Self {
            start_angle: 0.0,
            end_angle: 180.0,
            peak_angle: Some(90.0),
        }
    }
}

impl PowerPhase {
    /// Create a new power phase.
    pub fn new(start: f32, end: f32, peak: Option<f32>) -> Self {
        Self {
            start_angle: start % 360.0,
            end_angle: end % 360.0,
            peak_angle: peak.map(|p| p % 360.0),
        }
    }

    /// Get the arc length in degrees.
    pub fn arc_length(&self) -> f32 {
        let diff = self.end_angle - self.start_angle;
        if diff < 0.0 {
            360.0 + diff
        } else {
            diff
        }
    }
}

/// Complete cycling dynamics data from a dual-sided power meter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CyclingDynamicsData {
    /// Left/right power balance
    pub balance: LeftRightBalance,
    /// Pedal smoothness
    pub smoothness: PedalSmoothness,
    /// Torque effectiveness
    pub torque_effectiveness: TorqueEffectiveness,
    /// Left pedal power phase (optional, from advanced sensors)
    pub left_power_phase: Option<PowerPhase>,
    /// Right pedal power phase (optional, from advanced sensors)
    pub right_power_phase: Option<PowerPhase>,
    /// Timestamp of this reading
    #[serde(skip)]
    pub timestamp: Option<Instant>,
}

impl Default for CyclingDynamicsData {
    fn default() -> Self {
        Self {
            balance: LeftRightBalance::default(),
            smoothness: PedalSmoothness::default(),
            torque_effectiveness: TorqueEffectiveness::default(),
            left_power_phase: None,
            right_power_phase: None,
            timestamp: None,
        }
    }
}

impl CyclingDynamicsData {
    /// Create a new dynamics data instance.
    pub fn new(
        balance: LeftRightBalance,
        smoothness: PedalSmoothness,
        torque_effectiveness: TorqueEffectiveness,
    ) -> Self {
        Self {
            balance,
            smoothness,
            torque_effectiveness,
            left_power_phase: None,
            right_power_phase: None,
            timestamp: Some(Instant::now()),
        }
    }

    /// Create with power phase data (for advanced sensors like Shimano/Assioma).
    pub fn with_power_phases(
        mut self,
        left_phase: Option<PowerPhase>,
        right_phase: Option<PowerPhase>,
    ) -> Self {
        self.left_power_phase = left_phase;
        self.right_power_phase = right_phase;
        self
    }

    /// Check if this data has advanced power phase information.
    pub fn has_power_phases(&self) -> bool {
        self.left_power_phase.is_some() || self.right_power_phase.is_some()
    }
}

/// Session averages for cycling dynamics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DynamicsAverages {
    /// Average left balance percentage
    pub avg_left_balance: f32,
    /// Average right balance percentage
    pub avg_right_balance: f32,
    /// Average left pedal smoothness
    pub avg_left_smoothness: f32,
    /// Average right pedal smoothness
    pub avg_right_smoothness: f32,
    /// Average left torque effectiveness
    pub avg_left_torque_eff: f32,
    /// Average right torque effectiveness
    pub avg_right_torque_eff: f32,
    /// Number of samples in the average
    pub sample_count: u32,
}

impl DynamicsAverages {
    /// Update averages with a new sample.
    pub fn update(&mut self, data: &CyclingDynamicsData) {
        let n = self.sample_count as f32;
        let new_n = n + 1.0;

        // Running average formula: avg = (avg * n + new_value) / (n + 1)
        self.avg_left_balance = (self.avg_left_balance * n + data.balance.left_percent) / new_n;
        self.avg_right_balance = (self.avg_right_balance * n + data.balance.right_percent) / new_n;
        self.avg_left_smoothness =
            (self.avg_left_smoothness * n + data.smoothness.left_percent) / new_n;
        self.avg_right_smoothness =
            (self.avg_right_smoothness * n + data.smoothness.right_percent) / new_n;
        self.avg_left_torque_eff =
            (self.avg_left_torque_eff * n + data.torque_effectiveness.left_percent) / new_n;
        self.avg_right_torque_eff =
            (self.avg_right_torque_eff * n + data.torque_effectiveness.right_percent) / new_n;

        self.sample_count += 1;
    }

    /// Get the average balance imbalance.
    pub fn average_imbalance(&self) -> f32 {
        self.avg_left_balance - self.avg_right_balance
    }
}

/// Parse and provide cycling dynamics data.
///
/// T047: Implement CyclingDynamicsProvider trait
pub trait CyclingDynamicsProvider: Send + Sync {
    /// Check if sensor supports cycling dynamics.
    fn supports_dynamics(&self, sensor_id: &Uuid) -> bool;

    /// Get latest dynamics data.
    fn get_current_dynamics(&self, sensor_id: &Uuid) -> Option<CyclingDynamicsData>;

    /// Subscribe to dynamics updates.
    fn subscribe_dynamics(&self, sensor_id: &Uuid) -> broadcast::Receiver<CyclingDynamicsData>;

    /// Get session averages.
    fn get_session_averages(&self) -> DynamicsAverages;
}

/// Cycling Power Service characteristic UUIDs.
pub mod ble_uuids {
    use uuid::Uuid;

    /// Cycling Power Service UUID
    pub const CYCLING_POWER_SERVICE: Uuid = Uuid::from_u128(0x00001818_0000_1000_8000_00805f9b34fb);
    /// Cycling Power Measurement characteristic
    pub const CYCLING_POWER_MEASUREMENT: Uuid =
        Uuid::from_u128(0x00002a63_0000_1000_8000_00805f9b34fb);
    /// Cycling Power Feature characteristic
    pub const CYCLING_POWER_FEATURE: Uuid = Uuid::from_u128(0x00002a65_0000_1000_8000_00805f9b34fb);
    /// Cycling Power Control Point characteristic
    pub const CYCLING_POWER_CONTROL_POINT: Uuid =
        Uuid::from_u128(0x00002a66_0000_1000_8000_00805f9b34fb);
}

/// Cycling Power Feature flags (from BLE spec).
#[derive(Debug, Clone, Copy, Default)]
pub struct PowerFeatures {
    /// Pedal power balance supported
    pub pedal_power_balance: bool,
    /// Accumulated torque supported
    pub accumulated_torque: bool,
    /// Wheel revolution data supported
    pub wheel_revolution: bool,
    /// Crank revolution data supported
    pub crank_revolution: bool,
    /// Extreme magnitudes supported
    pub extreme_magnitudes: bool,
    /// Extreme angles supported
    pub extreme_angles: bool,
    /// Top/bottom dead spot angles supported
    pub dead_spot_angles: bool,
    /// Accumulated energy supported
    pub accumulated_energy: bool,
    /// Offset compensation supported
    pub offset_compensation: bool,
    /// Cycling power measurement characteristic content masking supported
    pub content_masking: bool,
    /// Multiple sensor locations supported
    pub multiple_locations: bool,
    /// Crank length adjustment supported
    pub crank_length_adjustment: bool,
    /// Chain length adjustment supported
    pub chain_length_adjustment: bool,
    /// Chain weight adjustment supported
    pub chain_weight_adjustment: bool,
    /// Span length adjustment supported
    pub span_length_adjustment: bool,
    /// Sensor measurement context (0 = force, 1 = torque)
    pub measurement_context_torque: bool,
    /// Instantaneous measurement direction supported
    pub instantaneous_direction: bool,
    /// Factory calibration date supported
    pub factory_calibration: bool,
    /// Enhanced offset compensation supported
    pub enhanced_offset_compensation: bool,
    /// Distributed system support (0 = unspecified, 1 = not for distributed, 2 = can be used in distributed, 3 = dual-sided)
    pub distributed_system: u8,
}

impl PowerFeatures {
    /// Parse from the 32-bit feature flags.
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 4 {
            return Self::default();
        }

        let flags = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

        Self {
            pedal_power_balance: (flags & 0x0001) != 0,
            accumulated_torque: (flags & 0x0002) != 0,
            wheel_revolution: (flags & 0x0004) != 0,
            crank_revolution: (flags & 0x0008) != 0,
            extreme_magnitudes: (flags & 0x0010) != 0,
            extreme_angles: (flags & 0x0020) != 0,
            dead_spot_angles: (flags & 0x0040) != 0,
            accumulated_energy: (flags & 0x0080) != 0,
            offset_compensation: (flags & 0x0100) != 0,
            content_masking: (flags & 0x0200) != 0,
            multiple_locations: (flags & 0x0400) != 0,
            crank_length_adjustment: (flags & 0x0800) != 0,
            chain_length_adjustment: (flags & 0x1000) != 0,
            chain_weight_adjustment: (flags & 0x2000) != 0,
            span_length_adjustment: (flags & 0x4000) != 0,
            measurement_context_torque: (flags & 0x8000) != 0,
            instantaneous_direction: (flags & 0x10000) != 0,
            factory_calibration: (flags & 0x20000) != 0,
            enhanced_offset_compensation: (flags & 0x40000) != 0,
            distributed_system: ((flags >> 19) & 0x03) as u8,
        }
    }

    /// Check if this is a dual-sided power meter.
    pub fn is_dual_sided(&self) -> bool {
        self.distributed_system == 3
    }

    /// Check if L/R balance data is available.
    pub fn has_balance(&self) -> bool {
        self.pedal_power_balance
    }
}

/// Parser for Cycling Power Measurement characteristic.
///
/// T046: Implement extended Cycling Power Service parsing for L/R balance
pub struct PowerMeasurementParser;

impl PowerMeasurementParser {
    /// Parse a Cycling Power Measurement notification.
    ///
    /// Returns (instant_power, cadence, balance, accumulated_energy, crank_revs).
    pub fn parse(data: &[u8]) -> Result<PowerMeasurementData, SensorError> {
        if data.len() < 4 {
            return Err(SensorError::ParseError("Data too short".to_string()));
        }

        let flags = u16::from_le_bytes([data[0], data[1]]);
        let instant_power = i16::from_le_bytes([data[2], data[3]]) as u16;

        let mut offset = 4;
        let mut result = PowerMeasurementData {
            instant_power,
            ..Default::default()
        };

        // Pedal Power Balance (bit 0)
        if (flags & 0x0001) != 0 && data.len() > offset {
            let balance_raw = data[offset];
            let reference_is_left = (flags & 0x0002) == 0;
            let balance_percent = balance_raw as f32 / 2.0; // 0.5% resolution
            result.balance = Some(LeftRightBalance::from_reference(
                balance_percent,
                reference_is_left,
            ));
            offset += 1;
        }

        // Accumulated Torque (bit 2)
        if (flags & 0x0004) != 0 && data.len() >= offset + 2 {
            result.accumulated_torque = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
            offset += 2;
        }

        // Wheel Revolution Data (bit 4)
        if (flags & 0x0010) != 0 && data.len() >= offset + 6 {
            result.wheel_revolutions = Some(u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
            result.last_wheel_event_time =
                Some(u16::from_le_bytes([data[offset + 4], data[offset + 5]]));
            offset += 6;
        }

        // Crank Revolution Data (bit 5)
        if (flags & 0x0020) != 0 && data.len() >= offset + 4 {
            result.crank_revolutions = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
            result.last_crank_event_time =
                Some(u16::from_le_bytes([data[offset + 2], data[offset + 3]]));
            offset += 4;
        }

        // Extreme Force Magnitudes (bit 6)
        if (flags & 0x0040) != 0 && data.len() >= offset + 4 {
            result.max_force = Some(i16::from_le_bytes([data[offset], data[offset + 1]]));
            result.min_force = Some(i16::from_le_bytes([data[offset + 2], data[offset + 3]]));
            offset += 4;
        }

        // Extreme Torque Magnitudes (bit 7)
        if (flags & 0x0080) != 0 && data.len() >= offset + 4 {
            result.max_torque = Some(i16::from_le_bytes([data[offset], data[offset + 1]]));
            result.min_torque = Some(i16::from_le_bytes([data[offset + 2], data[offset + 3]]));
            offset += 4;
        }

        // Extreme Angles (bit 8)
        if (flags & 0x0100) != 0 && data.len() >= offset + 3 {
            // Packed format: 12 bits for max angle, 12 bits for min angle
            let packed = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], 0]);
            result.max_angle = Some((packed & 0x0FFF) as u16);
            result.min_angle = Some(((packed >> 12) & 0x0FFF) as u16);
            offset += 3;
        }

        // Top Dead Spot Angle (bit 9)
        if (flags & 0x0200) != 0 && data.len() >= offset + 2 {
            result.top_dead_spot_angle = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
            offset += 2;
        }

        // Bottom Dead Spot Angle (bit 10)
        if (flags & 0x0400) != 0 && data.len() >= offset + 2 {
            result.bottom_dead_spot_angle =
                Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
            offset += 2;
        }

        // Accumulated Energy (bit 11)
        if (flags & 0x0800) != 0 && data.len() >= offset + 2 {
            result.accumulated_energy = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
            // offset += 2;
        }

        Ok(result)
    }
}

/// Parsed Cycling Power Measurement data.
#[derive(Debug, Clone, Default)]
pub struct PowerMeasurementData {
    /// Instantaneous power in watts
    pub instant_power: u16,
    /// Left/right power balance
    pub balance: Option<LeftRightBalance>,
    /// Accumulated torque (1/32 Nm resolution)
    pub accumulated_torque: Option<u16>,
    /// Cumulative wheel revolutions
    pub wheel_revolutions: Option<u32>,
    /// Last wheel event time (1/2048s resolution)
    pub last_wheel_event_time: Option<u16>,
    /// Cumulative crank revolutions
    pub crank_revolutions: Option<u16>,
    /// Last crank event time (1/1024s resolution)
    pub last_crank_event_time: Option<u16>,
    /// Maximum force magnitude (N)
    pub max_force: Option<i16>,
    /// Minimum force magnitude (N)
    pub min_force: Option<i16>,
    /// Maximum torque magnitude (1/32 Nm)
    pub max_torque: Option<i16>,
    /// Minimum torque magnitude (1/32 Nm)
    pub min_torque: Option<i16>,
    /// Maximum angle (degrees)
    pub max_angle: Option<u16>,
    /// Minimum angle (degrees)
    pub min_angle: Option<u16>,
    /// Top dead spot angle (degrees)
    pub top_dead_spot_angle: Option<u16>,
    /// Bottom dead spot angle (degrees)
    pub bottom_dead_spot_angle: Option<u16>,
    /// Accumulated energy (kJ)
    pub accumulated_energy: Option<u16>,
}

impl PowerMeasurementData {
    /// Calculate cadence from crank revolution data.
    ///
    /// Requires previous measurement for delta calculation.
    pub fn calculate_cadence(&self, previous: &PowerMeasurementData) -> Option<u8> {
        let revs = self.crank_revolutions?;
        let time = self.last_crank_event_time?;
        let prev_revs = previous.crank_revolutions?;
        let prev_time = previous.last_crank_event_time?;

        // Handle rollover
        let rev_delta = if revs >= prev_revs {
            revs - prev_revs
        } else {
            (u16::MAX - prev_revs) + revs + 1
        };

        let time_delta = if time >= prev_time {
            time - prev_time
        } else {
            (u16::MAX - prev_time) + time + 1
        };

        if time_delta == 0 {
            return None;
        }

        // Time is in 1/1024 second units
        // Cadence = (revolutions / time_seconds) * 60
        let time_seconds = time_delta as f32 / 1024.0;
        let cadence = (rev_delta as f32 / time_seconds) * 60.0;

        Some(cadence.round().min(255.0) as u8)
    }

    /// Convert to CyclingDynamicsData.
    pub fn to_dynamics_data(&self) -> Option<CyclingDynamicsData> {
        let balance = self.balance?;

        // We don't have smoothness/torque effectiveness from basic BLE data
        // These would come from proprietary extensions or ANT+
        Some(CyclingDynamicsData {
            balance,
            smoothness: PedalSmoothness::default(),
            torque_effectiveness: TorqueEffectiveness::default(),
            left_power_phase: self.max_angle.map(|max| {
                PowerPhase::new(
                    self.min_angle.unwrap_or(0) as f32,
                    max as f32,
                    self.top_dead_spot_angle.map(|a| a as f32),
                )
            }),
            right_power_phase: None,
            timestamp: Some(Instant::now()),
        })
    }
}

/// Default implementation of CyclingDynamicsProvider.
pub struct DefaultDynamicsProvider {
    /// Current dynamics data per sensor
    current_data: std::sync::RwLock<std::collections::HashMap<Uuid, CyclingDynamicsData>>,
    /// Session averages
    session_averages: std::sync::RwLock<DynamicsAverages>,
    /// Broadcast channel for dynamics updates
    tx: broadcast::Sender<CyclingDynamicsData>,
}

impl DefaultDynamicsProvider {
    /// Create a new dynamics provider.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(32);
        Self {
            current_data: std::sync::RwLock::new(std::collections::HashMap::new()),
            session_averages: std::sync::RwLock::new(DynamicsAverages::default()),
            tx,
        }
    }

    /// Update dynamics data for a sensor.
    pub fn update_dynamics(&self, sensor_id: Uuid, data: CyclingDynamicsData) {
        // Update current data
        if let Ok(mut current) = self.current_data.write() {
            current.insert(sensor_id, data.clone());
        }

        // Update session averages
        if let Ok(mut averages) = self.session_averages.write() {
            averages.update(&data);
        }

        // Broadcast update
        let _ = self.tx.send(data);
    }

    /// Reset session averages.
    pub fn reset_session(&self) {
        if let Ok(mut averages) = self.session_averages.write() {
            *averages = DynamicsAverages::default();
        }
    }
}

impl Default for DefaultDynamicsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CyclingDynamicsProvider for DefaultDynamicsProvider {
    fn supports_dynamics(&self, sensor_id: &Uuid) -> bool {
        self.current_data
            .read()
            .map(|data| data.contains_key(sensor_id))
            .unwrap_or(false)
    }

    fn get_current_dynamics(&self, sensor_id: &Uuid) -> Option<CyclingDynamicsData> {
        self.current_data.read().ok()?.get(sensor_id).cloned()
    }

    fn subscribe_dynamics(&self, _sensor_id: &Uuid) -> broadcast::Receiver<CyclingDynamicsData> {
        self.tx.subscribe()
    }

    fn get_session_averages(&self) -> DynamicsAverages {
        self.session_averages
            .read()
            .map(|a| a.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_left_right_balance_from_reference() {
        let balance = LeftRightBalance::from_reference(52.0, true);
        assert_eq!(balance.left_percent, 52.0);
        assert_eq!(balance.right_percent, 48.0);
        assert!(balance.reference_is_left);

        let balance = LeftRightBalance::from_reference(48.0, false);
        assert_eq!(balance.left_percent, 52.0);
        assert_eq!(balance.right_percent, 48.0);
        assert!(!balance.reference_is_left);
    }

    #[test]
    fn test_balance_imbalance() {
        let balance = LeftRightBalance::from_reference(55.0, true);
        assert_eq!(balance.imbalance(), 10.0);

        let balance = LeftRightBalance::from_reference(45.0, true);
        assert_eq!(balance.imbalance(), -10.0);
    }

    #[test]
    fn test_balance_is_balanced() {
        let balance = LeftRightBalance::from_reference(52.0, true);
        assert!(balance.is_balanced(5.0));
        assert!(!balance.is_balanced(3.0));
    }

    #[test]
    fn test_pedal_smoothness_new() {
        let smoothness = PedalSmoothness::new(85.0, 83.0);
        assert_eq!(smoothness.left_percent, 85.0);
        assert_eq!(smoothness.right_percent, 83.0);
        assert_eq!(smoothness.combined_percent, 84.0);
    }

    #[test]
    fn test_torque_effectiveness_new() {
        let te = TorqueEffectiveness::new(72.5, 74.5);
        assert_eq!(te.left_percent, 72.5);
        assert_eq!(te.right_percent, 74.5);
        assert_eq!(te.combined_percent, 73.5);
    }

    #[test]
    fn test_power_phase_arc_length() {
        let phase = PowerPhase::new(0.0, 180.0, Some(90.0));
        assert_eq!(phase.arc_length(), 180.0);

        let phase = PowerPhase::new(270.0, 90.0, None);
        assert_eq!(phase.arc_length(), 180.0);
    }

    #[test]
    fn test_dynamics_averages_update() {
        let mut averages = DynamicsAverages::default();

        let data1 = CyclingDynamicsData::new(
            LeftRightBalance::from_reference(52.0, true),
            PedalSmoothness::new(80.0, 82.0),
            TorqueEffectiveness::new(70.0, 72.0),
        );

        let data2 = CyclingDynamicsData::new(
            LeftRightBalance::from_reference(54.0, true),
            PedalSmoothness::new(84.0, 86.0),
            TorqueEffectiveness::new(74.0, 76.0),
        );

        averages.update(&data1);
        assert_eq!(averages.sample_count, 1);
        assert_eq!(averages.avg_left_balance, 52.0);

        averages.update(&data2);
        assert_eq!(averages.sample_count, 2);
        assert_eq!(averages.avg_left_balance, 53.0);
    }

    #[test]
    fn test_power_features_from_bytes() {
        // Dual-sided power meter with balance support
        let data = [0x01, 0x00, 0x18, 0x00]; // bit 0 (balance) + bits 19-20 = 3 (dual-sided)
        let features = PowerFeatures::from_bytes(&data);
        assert!(features.pedal_power_balance);
        assert_eq!(features.distributed_system, 3);
        assert!(features.is_dual_sided());
    }

    #[test]
    fn test_power_measurement_parse_basic() {
        // Flags: 0x0000 (no optional fields), Power: 250W
        let data = [0x00, 0x00, 0xFA, 0x00];
        let result = PowerMeasurementParser::parse(&data).unwrap();
        assert_eq!(result.instant_power, 250);
        assert!(result.balance.is_none());
    }

    #[test]
    fn test_power_measurement_parse_with_balance() {
        // Flags: 0x0001 (balance present, reference=left), Power: 200W, Balance: 104 (52%)
        let data = [0x01, 0x00, 0xC8, 0x00, 0x68];
        let result = PowerMeasurementParser::parse(&data).unwrap();
        assert_eq!(result.instant_power, 200);
        let balance = result.balance.unwrap();
        assert_eq!(balance.left_percent, 52.0);
        assert_eq!(balance.right_percent, 48.0);
        assert!(balance.reference_is_left);
    }

    #[test]
    fn test_default_dynamics_provider() {
        let provider = DefaultDynamicsProvider::new();
        let sensor_id = Uuid::new_v4();

        // Initially no data
        assert!(!provider.supports_dynamics(&sensor_id));
        assert!(provider.get_current_dynamics(&sensor_id).is_none());

        // Add data
        let data = CyclingDynamicsData::new(
            LeftRightBalance::from_reference(51.0, true),
            PedalSmoothness::new(80.0, 82.0),
            TorqueEffectiveness::new(70.0, 72.0),
        );
        provider.update_dynamics(sensor_id, data);

        // Now has data
        assert!(provider.supports_dynamics(&sensor_id));
        let retrieved = provider.get_current_dynamics(&sensor_id).unwrap();
        assert_eq!(retrieved.balance.left_percent, 51.0);

        // Session averages updated
        let averages = provider.get_session_averages();
        assert_eq!(averages.sample_count, 1);
        assert_eq!(averages.avg_left_balance, 51.0);
    }
}
