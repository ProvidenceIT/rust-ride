//! Integration test modules.

mod achievement_flow_test;
mod analytics_export_test;
mod analytics_integration_test;
mod audio_pipeline_test;
mod fit_validation_test;
mod garmin_sync_test;
mod gradient_ride_test;
mod hid_test;
mod mqtt_fan_test;
mod npc_test;
mod power_profile_test;
mod profile_export_integration;
mod reconnection_stress_test;
mod ride_recording_test;
mod route_import_test;
mod segment_test;
mod sensor_flow_test;
mod sensor_mock;
mod streaming_test;
mod sync_integration_test;
mod trainingpeaks_sync_test;
mod tts_test;
mod weather_test;
mod workout_audio_test;
// TODO: workout_execution_test needs API updates for CadenceTarget and SegmentType changes
// mod workout_execution_test;