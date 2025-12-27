//! IMU (Inertial Measurement Unit) sensor module for motion tracking.
//!
//! T137: Create MotionSample, Vector3, Quaternion types
//! T138: Implement MotionProvider trait
//!
//! This module provides support for motion tracking sensors (accelerometers,
//! gyroscopes) commonly found in rocker plates and smart trainers.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 3D vector for accelerometer/gyroscope readings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vector3 {
    /// X-axis component (left-right)
    pub x: f32,
    /// Y-axis component (up-down)
    pub y: f32,
    /// Z-axis component (forward-backward)
    pub z: f32,
}

impl Vector3 {
    /// Create a new vector with specified components.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Create a zero vector.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Calculate the magnitude (length) of the vector.
    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalize the vector to unit length.
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            Self {
                x: self.x / mag,
                y: self.y / mag,
                z: self.z / mag,
            }
        } else {
            Self::zero()
        }
    }

    /// Dot product with another vector.
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product with another vector.
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Add two vectors.
    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    /// Subtract another vector from this one.
    pub fn sub(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    /// Multiply by a scalar.
    pub fn scale(&self, s: f32) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
}

/// Quaternion for 3D rotation representation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quaternion {
    /// W component (scalar)
    pub w: f32,
    /// X component (i)
    pub x: f32,
    /// Y component (j)
    pub y: f32,
    /// Z component (k)
    pub z: f32,
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

impl Quaternion {
    /// Create a new quaternion.
    pub fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { w, x, y, z }
    }

    /// Create an identity quaternion (no rotation).
    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Create a quaternion from Euler angles (roll, pitch, yaw in radians).
    pub fn from_euler(roll: f32, pitch: f32, yaw: f32) -> Self {
        let cr = (roll * 0.5).cos();
        let sr = (roll * 0.5).sin();
        let cp = (pitch * 0.5).cos();
        let sp = (pitch * 0.5).sin();
        let cy = (yaw * 0.5).cos();
        let sy = (yaw * 0.5).sin();

        Self {
            w: cr * cp * cy + sr * sp * sy,
            x: sr * cp * cy - cr * sp * sy,
            y: cr * sp * cy + sr * cp * sy,
            z: cr * cp * sy - sr * sp * cy,
        }
    }

    /// Create a quaternion from axis-angle representation.
    pub fn from_axis_angle(axis: Vector3, angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        let axis = axis.normalize();

        Self {
            w: half_angle.cos(),
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
        }
    }

    /// Convert to Euler angles (roll, pitch, yaw in radians).
    pub fn to_euler(&self) -> (f32, f32, f32) {
        // Roll (x-axis rotation)
        let sinr_cosp = 2.0 * (self.w * self.x + self.y * self.z);
        let cosr_cosp = 1.0 - 2.0 * (self.x * self.x + self.y * self.y);
        let roll = sinr_cosp.atan2(cosr_cosp);

        // Pitch (y-axis rotation)
        let sinp = 2.0 * (self.w * self.y - self.z * self.x);
        let pitch = if sinp.abs() >= 1.0 {
            std::f32::consts::FRAC_PI_2.copysign(sinp)
        } else {
            sinp.asin()
        };

        // Yaw (z-axis rotation)
        let siny_cosp = 2.0 * (self.w * self.z + self.x * self.y);
        let cosy_cosp = 1.0 - 2.0 * (self.y * self.y + self.z * self.z);
        let yaw = siny_cosp.atan2(cosy_cosp);

        (roll, pitch, yaw)
    }

    /// Normalize the quaternion to unit length.
    pub fn normalize(&self) -> Self {
        let mag = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if mag > 0.0 {
            Self {
                w: self.w / mag,
                x: self.x / mag,
                y: self.y / mag,
                z: self.z / mag,
            }
        } else {
            Self::identity()
        }
    }

    /// Multiply two quaternions (compose rotations).
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// Conjugate (inverse for unit quaternion).
    pub fn conjugate(&self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Rotate a vector by this quaternion.
    pub fn rotate_vector(&self, v: &Vector3) -> Vector3 {
        let q_vec = Quaternion::new(0.0, v.x, v.y, v.z);
        let result = self.multiply(&q_vec).multiply(&self.conjugate());
        Vector3::new(result.x, result.y, result.z)
    }

    /// Spherical linear interpolation between two quaternions.
    pub fn slerp(&self, other: &Self, t: f32) -> Self {
        let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;

        // Handle negative dot (choose shorter path)
        let other = if dot < 0.0 {
            dot = -dot;
            Quaternion::new(-other.w, -other.x, -other.y, -other.z)
        } else {
            *other
        };

        // If very close, use linear interpolation
        if dot > 0.9995 {
            return Quaternion::new(
                self.w + t * (other.w - self.w),
                self.x + t * (other.x - self.x),
                self.y + t * (other.y - self.y),
                self.z + t * (other.z - self.z),
            )
            .normalize();
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = theta.cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        Quaternion::new(
            s0 * self.w + s1 * other.w,
            s0 * self.x + s1 * other.x,
            s0 * self.y + s1 * other.y,
            s0 * self.z + s1 * other.z,
        )
    }
}

/// A single motion sample from an IMU sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionSample {
    /// Elapsed time since ride start (seconds)
    pub elapsed_seconds: u32,
    /// Accelerometer reading (m/s²)
    pub acceleration: Vector3,
    /// Gyroscope reading (rad/s)
    pub angular_velocity: Vector3,
    /// Computed orientation quaternion
    pub orientation: Quaternion,
    /// Computed tilt angles (roll, pitch in degrees)
    pub tilt_degrees: (f32, f32),
    /// Sensor temperature (Celsius, optional)
    pub temperature: Option<f32>,
}

impl MotionSample {
    /// Create a new motion sample.
    pub fn new(
        elapsed_seconds: u32,
        acceleration: Vector3,
        angular_velocity: Vector3,
        orientation: Quaternion,
    ) -> Self {
        let (roll, pitch, _) = orientation.to_euler();
        Self {
            elapsed_seconds,
            acceleration,
            angular_velocity,
            orientation,
            tilt_degrees: (roll.to_degrees(), pitch.to_degrees()),
            temperature: None,
        }
    }

    /// Get the total acceleration magnitude.
    pub fn total_acceleration(&self) -> f32 {
        self.acceleration.magnitude()
    }

    /// Get the angular velocity magnitude.
    pub fn total_angular_velocity(&self) -> f32 {
        self.angular_velocity.magnitude()
    }

    /// Check if the sample indicates significant motion.
    pub fn is_moving(&self, threshold: f32) -> bool {
        self.total_angular_velocity() > threshold
    }
}

/// Calibration state for an IMU sensor.
#[derive(Debug, Clone, Default)]
pub struct ImuCalibration {
    /// Accelerometer bias offset
    pub accel_bias: Vector3,
    /// Gyroscope bias offset
    pub gyro_bias: Vector3,
    /// Whether calibration is complete
    pub is_calibrated: bool,
    /// Number of samples used for calibration
    pub sample_count: usize,
    /// Maximum detected roll angle during calibration (reserved for future use)
    #[allow(dead_code)]
    pub max_roll: f32,
    /// Maximum detected pitch angle during calibration (reserved for future use)
    #[allow(dead_code)]
    pub max_pitch: f32,
}

impl ImuCalibration {
    /// Create a new uncalibrated state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update calibration with a new sample (should be stationary).
    pub fn update(&mut self, accel: &Vector3, gyro: &Vector3) {
        // Running average for bias estimation
        let n = self.sample_count as f32;
        let n1 = (self.sample_count + 1) as f32;

        self.accel_bias.x = (self.accel_bias.x * n + accel.x) / n1;
        self.accel_bias.y = (self.accel_bias.y * n + accel.y) / n1;
        self.accel_bias.z = (self.accel_bias.z * n + accel.z) / n1;

        self.gyro_bias.x = (self.gyro_bias.x * n + gyro.x) / n1;
        self.gyro_bias.y = (self.gyro_bias.y * n + gyro.y) / n1;
        self.gyro_bias.z = (self.gyro_bias.z * n + gyro.z) / n1;

        self.sample_count += 1;

        // Assume calibrated after enough samples
        if self.sample_count >= 100 {
            // Correct for gravity on the expected axis (usually Z)
            self.accel_bias.z -= 9.81;
            self.is_calibrated = true;
        }
    }

    /// Apply calibration correction to a reading.
    pub fn apply(&self, accel: &Vector3, gyro: &Vector3) -> (Vector3, Vector3) {
        let corrected_accel = accel.sub(&self.accel_bias);
        let corrected_gyro = gyro.sub(&self.gyro_bias);
        (corrected_accel, corrected_gyro)
    }
}

/// Motion sensor device information.
#[derive(Debug, Clone)]
pub struct MotionSensorInfo {
    /// Unique device identifier
    pub device_id: Uuid,
    /// Device name
    pub name: String,
    /// Firmware version (if available)
    pub firmware_version: Option<String>,
    /// Accelerometer range (±g)
    pub accel_range: f32,
    /// Gyroscope range (±dps)
    pub gyro_range: f32,
    /// Sample rate (Hz)
    pub sample_rate: u16,
}

impl Default for MotionSensorInfo {
    fn default() -> Self {
        Self {
            device_id: Uuid::new_v4(),
            name: "Unknown IMU".to_string(),
            firmware_version: None,
            accel_range: 16.0,  // ±16g
            gyro_range: 2000.0, // ±2000 dps
            sample_rate: 100,   // 100 Hz
        }
    }
}

/// Motion sensor connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionSensorState {
    /// Disconnected
    Disconnected,
    /// Connecting to sensor
    Connecting,
    /// Connected, awaiting calibration
    AwaitingCalibration,
    /// Calibrating (collecting bias samples)
    Calibrating,
    /// Ready and streaming data
    Ready,
    /// Error state
    Error,
}

/// T138: Trait for motion sensor providers.
pub trait MotionProvider: Send + Sync {
    /// Discover available motion sensors.
    fn discover_motion_sensors(&mut self) -> Vec<MotionSensorInfo>;

    /// Connect to a specific sensor.
    fn connect(&mut self, device_id: &Uuid) -> Result<(), ImuError>;

    /// Disconnect from the current sensor.
    fn disconnect(&mut self);

    /// Start calibration process.
    fn calibrate(&mut self) -> Result<(), ImuError>;

    /// Get current connection state.
    fn get_state(&self) -> MotionSensorState;

    /// Get the latest motion sample.
    fn get_sample(&self) -> Option<MotionSample>;

    /// Get calibration status.
    fn get_calibration(&self) -> &ImuCalibration;

    /// Check if sensor is streaming data.
    fn is_streaming(&self) -> bool;
}

/// Errors that can occur during IMU operations.
#[derive(Debug, Clone)]
pub enum ImuError {
    /// Device not found
    DeviceNotFound,
    /// Connection failed
    ConnectionFailed(String),
    /// Calibration failed
    CalibrationFailed(String),
    /// Sensor not connected
    NotConnected,
    /// Sensor in invalid state
    InvalidState(MotionSensorState),
    /// Communication error
    CommunicationError(String),
}

impl std::fmt::Display for ImuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImuError::DeviceNotFound => write!(f, "Motion sensor device not found"),
            ImuError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            ImuError::CalibrationFailed(msg) => write!(f, "Calibration failed: {}", msg),
            ImuError::NotConnected => write!(f, "Sensor not connected"),
            ImuError::InvalidState(state) => write!(f, "Invalid sensor state: {:?}", state),
            ImuError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
        }
    }
}

impl std::error::Error for ImuError {}

/// Default IMU provider for simulation/testing.
pub struct DefaultMotionProvider {
    state: MotionSensorState,
    calibration: ImuCalibration,
    last_sample: Option<MotionSample>,
    connected_device: Option<MotionSensorInfo>,
}

impl DefaultMotionProvider {
    /// Create a new default provider.
    pub fn new() -> Self {
        Self {
            state: MotionSensorState::Disconnected,
            calibration: ImuCalibration::new(),
            last_sample: None,
            connected_device: None,
        }
    }
}

impl Default for DefaultMotionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MotionProvider for DefaultMotionProvider {
    fn discover_motion_sensors(&mut self) -> Vec<MotionSensorInfo> {
        // Simulated device discovery
        vec![MotionSensorInfo {
            device_id: Uuid::new_v4(),
            name: "Simulated IMU".to_string(),
            firmware_version: Some("1.0.0".to_string()),
            accel_range: 16.0,
            gyro_range: 2000.0,
            sample_rate: 100,
        }]
    }

    fn connect(&mut self, device_id: &Uuid) -> Result<(), ImuError> {
        self.state = MotionSensorState::Connecting;

        // Simulate connection
        self.connected_device = Some(MotionSensorInfo {
            device_id: *device_id,
            name: "Simulated IMU".to_string(),
            ..Default::default()
        });

        self.state = MotionSensorState::AwaitingCalibration;
        Ok(())
    }

    fn disconnect(&mut self) {
        self.state = MotionSensorState::Disconnected;
        self.connected_device = None;
        self.last_sample = None;
    }

    fn calibrate(&mut self) -> Result<(), ImuError> {
        if self.state != MotionSensorState::AwaitingCalibration {
            return Err(ImuError::InvalidState(self.state));
        }

        self.state = MotionSensorState::Calibrating;

        // Simulate calibration with default values
        self.calibration = ImuCalibration {
            accel_bias: Vector3::zero(),
            gyro_bias: Vector3::zero(),
            is_calibrated: true,
            sample_count: 100,
            max_roll: 0.0,
            max_pitch: 0.0,
        };

        self.state = MotionSensorState::Ready;
        Ok(())
    }

    fn get_state(&self) -> MotionSensorState {
        self.state
    }

    fn get_sample(&self) -> Option<MotionSample> {
        self.last_sample.clone()
    }

    fn get_calibration(&self) -> &ImuCalibration {
        &self.calibration
    }

    fn is_streaming(&self) -> bool {
        self.state == MotionSensorState::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector3_magnitude() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        assert!((v.magnitude() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_vector3_normalize() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.magnitude() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vector3_dot() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        let b = Vector3::new(0.0, 1.0, 0.0);
        assert!((a.dot(&b)).abs() < 0.001); // Perpendicular
    }

    #[test]
    fn test_vector3_cross() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        let b = Vector3::new(0.0, 1.0, 0.0);
        let c = a.cross(&b);
        assert!((c.z - 1.0).abs() < 0.001); // Z axis
    }

    #[test]
    fn test_quaternion_identity() {
        let q = Quaternion::identity();
        let v = Vector3::new(1.0, 2.0, 3.0);
        let rotated = q.rotate_vector(&v);

        assert!((rotated.x - v.x).abs() < 0.001);
        assert!((rotated.y - v.y).abs() < 0.001);
        assert!((rotated.z - v.z).abs() < 0.001);
    }

    #[test]
    fn test_quaternion_from_euler() {
        // 90 degree rotation around Z axis
        let q = Quaternion::from_euler(0.0, 0.0, std::f32::consts::FRAC_PI_2);
        let v = Vector3::new(1.0, 0.0, 0.0);
        let rotated = q.rotate_vector(&v);

        // X axis should become Y axis
        assert!(rotated.x.abs() < 0.001);
        assert!((rotated.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_quaternion_to_euler() {
        let q = Quaternion::from_euler(0.5, 0.3, 0.1);
        let (roll, pitch, yaw) = q.to_euler();

        assert!((roll - 0.5).abs() < 0.01);
        assert!((pitch - 0.3).abs() < 0.01);
        assert!((yaw - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_motion_sample_total_acceleration() {
        let sample = MotionSample::new(
            0,
            Vector3::new(0.0, 0.0, 9.81),
            Vector3::zero(),
            Quaternion::identity(),
        );

        assert!((sample.total_acceleration() - 9.81).abs() < 0.01);
    }

    #[test]
    fn test_imu_calibration() {
        let mut cal = ImuCalibration::new();
        assert!(!cal.is_calibrated);

        // Simulate stationary readings
        for _ in 0..100 {
            cal.update(
                &Vector3::new(0.1, 0.05, 9.81),
                &Vector3::new(0.01, -0.02, 0.005),
            );
        }

        assert!(cal.is_calibrated);
    }

    #[test]
    fn test_default_motion_provider() {
        let mut provider = DefaultMotionProvider::new();

        assert_eq!(provider.get_state(), MotionSensorState::Disconnected);

        let sensors = provider.discover_motion_sensors();
        assert!(!sensors.is_empty());

        let device_id = sensors[0].device_id;
        provider.connect(&device_id).unwrap();

        assert_eq!(provider.get_state(), MotionSensorState::AwaitingCalibration);

        provider.calibrate().unwrap();
        assert_eq!(provider.get_state(), MotionSensorState::Ready);
        assert!(provider.is_streaming());

        provider.disconnect();
        assert_eq!(provider.get_state(), MotionSensorState::Disconnected);
    }
}
