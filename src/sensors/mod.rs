//! Sensor module for BLE and ANT+ device communication.

pub mod ant;
pub mod dynamics;
pub mod ftms;
pub mod fusion;
pub mod imu;
pub mod incline;
pub mod manager;
pub mod smo2;
pub mod types;

pub use dynamics::{
    CyclingDynamicsData, CyclingDynamicsProvider, DefaultDynamicsProvider, DynamicsAverages,
    LeftRightBalance, PedalSmoothness, PowerFeatures, PowerMeasurementData, PowerMeasurementParser,
    PowerPhase, TorqueEffectiveness,
};
pub use fusion::{CadenceFusion, FusionDiagnostics, FusionMode, SensorFusion, SensorFusionConfig};
pub use imu::{
    DefaultMotionProvider, ImuCalibration, ImuError, MotionProvider, MotionSample,
    MotionSensorInfo, MotionSensorState, Quaternion, Vector3,
};
pub use incline::{
    DefaultInclineController, GradientSmoother, GradientState, InclineConfig, InclineController,
    IntensityScaler,
};
pub use manager::SensorManager;
pub use smo2::{
    DefaultSmO2Provider, MuscleLocation, SmO2Error, SmO2Provider, SmO2Reading, SmO2Sensor,
    SmO2Status,
};
pub use types::{
    ConnectionState, DiscoveredSensor, Protocol, SensorConfig, SensorError, SensorEvent,
    SensorReading, SensorState, SensorType,
};
