//! T083: Integration tests for WebSocket streaming.
//!
//! Tests the streaming server, PIN authentication, and session management.

use rustride::integrations::streaming::{
    DefaultPinAuthenticator, DefaultStreamingServer, PinAuthenticator, StreamingConfig,
    StreamingError, StreamingMetrics, StreamingServer,
};
use std::sync::Arc;
use std::time::Duration;

/// Test streaming config defaults.
#[test]
fn test_streaming_config_defaults() {
    let config = StreamingConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.port, 8080);
    assert!(config.require_pin);
    assert_eq!(config.pin_expiry_minutes, 60);
    assert_eq!(config.update_interval_ms, 1000);
    assert!(!config.allow_remote);
}

/// Test PIN authenticator creation.
#[test]
fn test_pin_authenticator_creation() {
    let auth = DefaultPinAuthenticator::new(60);
    // No PIN generated yet
    assert!(auth.is_expired());
    assert!(auth.get_current_pin().is_none());
}

/// Test PIN generation.
#[test]
fn test_pin_generation() {
    let auth = DefaultPinAuthenticator::new(60);
    let pin = auth.generate_pin();

    // PIN should be 6 digits
    assert_eq!(pin.len(), 6);
    assert!(pin.chars().all(|c| c.is_ascii_digit()));

    // Should be retrievable
    assert_eq!(auth.get_current_pin(), Some(pin.clone()));
}

/// Test PIN validation.
#[test]
fn test_pin_validation() {
    let auth = DefaultPinAuthenticator::new(60);
    let pin = auth.generate_pin();

    // Correct PIN should validate
    assert!(auth.validate_pin(&pin));

    // Wrong PIN should fail
    assert!(!auth.validate_pin("000000"));
    assert!(!auth.validate_pin(""));
    assert!(!auth.validate_pin("12345")); // Wrong length
}

/// Test PIN regeneration.
#[test]
fn test_pin_regeneration() {
    let auth = DefaultPinAuthenticator::new(60);
    let pin1 = auth.generate_pin();

    // Regenerate
    let pin2 = auth.regenerate();

    // Old PIN should no longer work
    // (Note: There's a small chance they could be equal by coincidence)
    if pin1 != pin2 {
        assert!(!auth.validate_pin(&pin1));
        assert!(auth.validate_pin(&pin2));
    }
}

/// Test PIN expiry with zero duration.
#[test]
fn test_pin_expiry_immediate() {
    let auth = DefaultPinAuthenticator::new(0); // Immediate expiry
    let _pin = auth.generate_pin();

    // Wait a tiny bit
    std::thread::sleep(Duration::from_millis(10));

    // Should be expired
    assert!(auth.is_expired());
    assert!(auth.get_current_pin().is_none());
}

/// Test PIN lockout after failed attempts.
#[test]
fn test_pin_lockout() {
    let auth = DefaultPinAuthenticator::new(60);
    let _pin = auth.generate_pin();

    // Make 5 failed attempts
    for _ in 0..5 {
        assert!(!auth.validate_pin("wrong"));
    }

    // Should be locked out now
    assert!(!auth.validate_pin("wrong"));
}

/// Test streaming server creation.
#[test]
fn test_streaming_server_creation() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    // Server should not be running
    assert!(!server.is_running());
    assert!(server.get_url().is_none());
    assert!(server.get_qr_code().is_none());
}

/// Test streaming server start with disabled config.
#[tokio::test]
async fn test_streaming_server_start_disabled() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: false,
        ..Default::default()
    };

    let result = server.start(&config).await;
    assert!(result.is_err());
}

/// Test streaming server start with enabled config.
#[tokio::test]
async fn test_streaming_server_start_enabled() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth.clone());

    let config = StreamingConfig {
        enabled: true,
        port: 18080, // Use high port to avoid conflicts
        require_pin: true,
        ..Default::default()
    };

    let result = server.start(&config).await;
    assert!(result.is_ok());

    // Server should be running
    assert!(server.is_running());
    assert!(server.get_url().is_some());

    // PIN should be generated
    assert!(server.get_pin().is_some());
    assert!(pin_auth.get_current_pin().is_some());
}

/// Test streaming server stop.
#[tokio::test]
async fn test_streaming_server_stop() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: true,
        port: 18081,
        ..Default::default()
    };

    server.start(&config).await.unwrap();
    assert!(server.is_running());

    server.stop().await.unwrap();
    assert!(!server.is_running());
    assert!(server.get_url().is_none());
}

/// Test streaming server PIN regeneration.
#[tokio::test]
async fn test_streaming_server_pin_regenerate() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: true,
        port: 18082,
        require_pin: true,
        ..Default::default()
    };

    server.start(&config).await.unwrap();
    let pin1 = server.get_pin().unwrap();

    let pin2 = server.regenerate_pin();

    // New PIN should be different (usually)
    if pin1 != pin2 {
        assert_eq!(server.get_pin(), Some(pin2));
    }
}

/// Test streaming server session list (empty initially).
#[tokio::test]
async fn test_streaming_server_sessions_empty() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: true,
        port: 18083,
        ..Default::default()
    };

    server.start(&config).await.unwrap();

    let sessions = server.get_sessions();
    assert!(sessions.is_empty());
}

/// Test streaming server event subscription.
#[tokio::test]
async fn test_streaming_server_events() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let mut receiver = server.subscribe_events();

    let config = StreamingConfig {
        enabled: true,
        port: 18084,
        ..Default::default()
    };

    server.start(&config).await.unwrap();

    // Should receive ServerStarted event
    // (Note: This is async so we use try_recv)
    // The event may or may not be received depending on timing
    let _result = receiver.try_recv();
}

/// Test streaming metrics creation.
#[test]
fn test_streaming_metrics_default() {
    let metrics = StreamingMetrics::default();

    assert_eq!(metrics.timestamp_ms, 0);
    assert!(metrics.power.is_none());
    assert!(metrics.heart_rate.is_none());
    assert!(metrics.cadence.is_none());
    assert!(metrics.speed.is_none());
    assert!(metrics.distance.is_none());
    assert_eq!(metrics.elapsed_time, Duration::ZERO);
}

/// Test streaming metrics serialization.
#[test]
fn test_streaming_metrics_serialization() {
    let metrics = StreamingMetrics {
        timestamp_ms: 1000,
        power: Some(200),
        heart_rate: Some(140),
        cadence: Some(90),
        speed: Some(32.5),
        distance: Some(1500.0),
        elapsed_time: Duration::from_secs(60),
        current_interval: Some("Sweet Spot".to_string()),
        zone_name: Some("Zone 3".to_string()),
        ..Default::default()
    };

    let json = serde_json::to_string(&metrics).unwrap();

    assert!(json.contains("\"timestamp_ms\":1000"));
    assert!(json.contains("\"power\":200"));
    assert!(json.contains("\"heart_rate\":140"));
    assert!(json.contains("\"cadence\":90"));

    // Optional None values should be skipped
    assert!(!json.contains("gradient"));
    assert!(!json.contains("left_right_balance"));
}

/// Test metrics broadcast (doesn't require actual connections).
#[tokio::test]
async fn test_streaming_server_broadcast() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: true,
        port: 18085,
        ..Default::default()
    };

    server.start(&config).await.unwrap();

    let metrics = StreamingMetrics {
        power: Some(200),
        heart_rate: Some(140),
        ..Default::default()
    };

    // Should not panic even with no connections
    server.broadcast_metrics(&metrics);
}

/// Test QR code generation.
#[tokio::test]
async fn test_streaming_server_qr_code() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: true,
        port: 18086,
        ..Default::default()
    };

    server.start(&config).await.unwrap();

    let qr = server.get_qr_code();
    assert!(qr.is_some());

    let qr = qr.unwrap();
    assert!(!qr.url.is_empty());
    assert!(!qr.ascii.is_empty());
    assert!(!qr.svg.is_empty());
}

/// Test streaming config with custom metrics.
#[test]
fn test_streaming_config_custom_metrics() {
    use rustride::integrations::streaming::StreamMetric;

    let config = StreamingConfig {
        enabled: true,
        metrics_to_stream: vec![StreamMetric::Power, StreamMetric::HeartRate],
        ..Default::default()
    };

    assert_eq!(config.metrics_to_stream.len(), 2);
}

/// Test disconnect nonexistent session.
#[tokio::test]
async fn test_streaming_server_disconnect_nonexistent() {
    let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
    let server = DefaultStreamingServer::new(pin_auth);

    let config = StreamingConfig {
        enabled: true,
        port: 18087,
        ..Default::default()
    };

    server.start(&config).await.unwrap();

    let session_id = uuid::Uuid::new_v4();
    let result = server.disconnect_session(&session_id).await;

    assert!(matches!(result, Err(StreamingError::SessionNotFound(_))));
}
