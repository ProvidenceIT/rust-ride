//! Sensor module for BLE and ANT+ device communication.

pub mod ant;
pub mod cache;
pub mod calibration;
pub mod conflict;
pub mod connection_queue;
pub mod connection_state;
pub mod dual_protocol;
pub mod dynamics;
pub mod ftms;
pub mod fusion;
pub mod health;
pub mod imu;
pub mod incline;
pub mod manager;
pub mod persistence;
pub mod power_meter;
pub mod quality;
pub mod reconnection;
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
pub use cache::{CacheError, CachedSensor, SensorCache};
pub use conflict::{
    ConflictDetector, ConflictDetectorConfig, ConflictError, ConflictInfo, ConflictPreference,
    ConflictPreferenceManager, ConflictSummary, DataSource, DataType, FailoverResult,
    ResolutionStrategy, SensorConflict, get_conflict_preference_path,
};
pub use connection_queue::{ConnectionQueue, ConnectionQueueEntry, SensorPriority};
pub use connection_state::{
    ConnectionLifecycleState, ConnectionStateMachine, ConnectionStateMachineConfig,
    ConnectionStateManager, ConnectionStateStats, InvalidTransitionError, StateTransition,
};
pub use health::{
    ConnectionHealth, ConnectionHealthConfig, ConnectionHealthMonitor, HealthStats, HealthStatus,
};
pub use persistence::{
    ConnectionSession, ConnectionSessionManager, PersistenceError, SessionSensor,
};
pub use quality::{
    ConnectionQuality, ConnectionQualityConfig, ConnectionQualityMonitor, QualityLevel,
    QualityMetrics, QualityStats,
};
pub use reconnection::{
    ExponentialBackoff, ExponentialBackoffConfig, ReconnectionManager, ReconnectionStats,
};
pub use dual_protocol::{
    DetectionResult, DualProtocolBinding, DualProtocolDetector, MatchConfidence,
    PreferenceError, ProtocolPreference, ProtocolPreferenceData, ProtocolPreferenceManager,
    SensorIdentifier, SensorManufacturer, get_preference_path,
};
pub use manager::SensorManager;
pub use power_meter::{
    ExpectedPowerMeter, ExtendedDiscoveryDecision, ExtendedPowerMeterDiscoveryConfig,
    PowerMeterWakeUpConfig, PowerMeterWakeUpDetector, WakeUpDetectionResult,
    WakeUpHint, WakeUpHintType, is_power_protocol, provides_power_data,
    DEFAULT_EXTENDED_DISCOVERY_SECS, DEFAULT_STANDARD_DISCOVERY_SECS,
    EXTENDED_DISCOVERY_THRESHOLD_SECS,
};
pub use calibration::{
    CalibrationData, CalibrationError, CalibrationInstructions, CalibrationManager,
    CalibrationProcess, CalibrationRecord, CalibrationReminder, CalibrationReminderConfig,
    CalibrationReminderType, CalibrationResult, CalibrationStep, CalibrationType,
    get_calibration_path, is_calibratable_sensor, DEFAULT_CALIBRATION_REMINDER_DAYS,
    MAX_CALIBRATION_REMINDER_DAYS, MIN_CALIBRATION_REMINDER_DAYS,
};
pub use smo2::{
    DefaultSmO2Provider, MuscleLocation, SmO2Error, SmO2Provider, SmO2Reading, SmO2Sensor,
    SmO2Status,
};
pub use types::{
    ConnectionState, DiscoveredSensor, DiscoveryPhase, DiscoveryProgress, ParallelDiscoveryResult,
    ProgressiveTimeoutConfig, ProgressiveTimeoutState, Protocol, SensorConfig, SensorError,
    SensorEvent, SensorReading, SensorState, SensorType, StopReason, TimeoutDecision,
};
