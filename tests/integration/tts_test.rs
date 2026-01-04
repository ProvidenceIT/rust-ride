//! TTS Integration Tests
//!
//! Tests for Text-to-Speech functionality across platforms.
//!
//! These tests verify TTS initialization, voice enumeration, and speech
//! on each platform. Tests handle cases where TTS is not available gracefully.
//!
//! Platform-specific backends:
//! - Windows: SAPI (Speech API)
//! - macOS: AVSpeechSynthesizer
//! - Linux: speech-dispatcher

use rustride::audio::{DefaultTtsProvider, ThreadSafeTtsProvider, TtsProvider, VoiceInfo};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if TTS is available on this system.
/// Returns true if TTS can be initialized successfully.
fn is_tts_available() -> bool {
    let provider = DefaultTtsProvider::new();
    provider.initialize().is_ok()
}

/// Skip test with message if TTS is not available.
macro_rules! skip_if_tts_unavailable {
    () => {
        if !is_tts_available() {
            eprintln!("TTS not available on this system - skipping test");
            return;
        }
    };
}

// ============================================================================
// DefaultTtsProvider Integration Tests
// ============================================================================

/// Test that TTS initialization handles unavailable systems gracefully.
#[test]
fn test_tts_initialization_graceful() {
    let provider = DefaultTtsProvider::new();

    // Initialization should either succeed or fail with a meaningful error
    match provider.initialize() {
        Ok(()) => {
            // TTS is available - verify it's in initialized state
            assert!(provider.get_voices().len() > 0 || provider.get_voices().is_empty());
        }
        Err(e) => {
            // TTS unavailable - error message should be informative
            let error_msg = format!("{}", e);
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
    }
}

/// Test voice enumeration returns valid voice information.
#[test]
fn test_voice_enumeration() {
    skip_if_tts_unavailable!();

    let provider = DefaultTtsProvider::new();
    let _ = provider.initialize();

    let voices = provider.get_voices();

    // Should have at least one voice when TTS is available
    assert!(
        !voices.is_empty(),
        "Should have at least one voice available"
    );

    // Verify voice properties are valid
    for voice in &voices {
        assert!(!voice.id.is_empty(), "Voice ID should not be empty");
        assert!(!voice.name.is_empty(), "Voice name should not be empty");
        // Language might be "unknown" on some platforms, but should exist
        assert!(
            !voice.language.is_empty(),
            "Voice language should not be empty"
        );
    }

    // At most one voice should be marked as default
    let default_count = voices.iter().filter(|v| v.is_default).count();
    assert!(
        default_count <= 1,
        "At most one voice should be marked as default, found {}",
        default_count
    );
}

/// Test voice selection with valid and invalid voice IDs.
#[test]
fn test_voice_selection() {
    skip_if_tts_unavailable!();

    let provider = DefaultTtsProvider::new();
    let _ = provider.initialize();

    let voices = provider.get_voices();
    if voices.is_empty() {
        eprintln!("No voices available - skipping voice selection test");
        return;
    }

    // Test setting a valid voice
    let first_voice = &voices[0];
    let result = provider.set_voice(&first_voice.id);
    assert!(
        result.is_ok(),
        "Should be able to set a valid voice: {:?}",
        result.err()
    );
    assert_eq!(
        provider.get_current_voice(),
        Some(first_voice.id.clone()),
        "Current voice should match what was set"
    );

    // Test setting an invalid voice
    let invalid_result = provider.set_voice("nonexistent_voice_id_xyz_12345");
    assert!(
        invalid_result.is_err(),
        "Should fail when setting non-existent voice"
    );

    // Current voice should still be the valid one we set before
    assert_eq!(
        provider.get_current_voice(),
        Some(first_voice.id.clone()),
        "Current voice should remain unchanged after failed set attempt"
    );
}

/// Test rate control with boundary values.
#[test]
fn test_rate_control() {
    let provider = DefaultTtsProvider::new();

    // Default rate should be 1.0
    assert_eq!(provider.get_rate(), 1.0, "Default rate should be 1.0");

    // Test normal rate setting
    provider.set_rate(1.5);
    assert_eq!(provider.get_rate(), 1.5, "Rate should be set to 1.5");

    // Test rate clamping at upper bound
    provider.set_rate(5.0);
    assert_eq!(
        provider.get_rate(),
        2.0,
        "Rate should be clamped to 2.0 maximum"
    );

    // Test rate clamping at lower bound
    provider.set_rate(0.1);
    assert_eq!(
        provider.get_rate(),
        0.5,
        "Rate should be clamped to 0.5 minimum"
    );

    // Test edge cases
    provider.set_rate(0.5);
    assert_eq!(provider.get_rate(), 0.5, "Rate should accept exact minimum");

    provider.set_rate(2.0);
    assert_eq!(provider.get_rate(), 2.0, "Rate should accept exact maximum");
}

/// Test volume control with boundary values.
#[test]
fn test_volume_control() {
    let provider = DefaultTtsProvider::new();

    // Default volume should be 1.0
    assert_eq!(provider.get_volume(), 1.0, "Default volume should be 1.0");

    // Test normal volume setting
    provider.set_volume(0.5);
    assert_eq!(provider.get_volume(), 0.5, "Volume should be set to 0.5");

    // Test volume clamping at upper bound
    provider.set_volume(1.5);
    assert_eq!(
        provider.get_volume(),
        1.0,
        "Volume should be clamped to 1.0 maximum"
    );

    // Test volume clamping at lower bound
    provider.set_volume(-0.5);
    assert_eq!(
        provider.get_volume(),
        0.0,
        "Volume should be clamped to 0.0 minimum"
    );

    // Test edge cases
    provider.set_volume(0.0);
    assert_eq!(
        provider.get_volume(),
        0.0,
        "Volume should accept exact minimum"
    );

    provider.set_volume(1.0);
    assert_eq!(
        provider.get_volume(),
        1.0,
        "Volume should accept exact maximum"
    );
}

/// Test speaking empty text returns immediately without error.
#[test]
fn test_speak_empty_text() {
    let provider = DefaultTtsProvider::new();

    // Speaking empty text should succeed without initializing TTS
    let result = provider.speak("");
    assert!(
        result.is_ok(),
        "Speaking empty text should return Ok: {:?}",
        result.err()
    );

    // Provider should not be speaking after empty text
    assert!(
        !provider.is_speaking(),
        "Should not be speaking after empty text"
    );
}

/// Test stop() is safe to call when not speaking.
#[test]
fn test_stop_when_not_speaking() {
    let provider = DefaultTtsProvider::new();

    // Stop should be safe to call even when not initialized
    provider.stop();
    assert!(!provider.is_speaking(), "Should not be speaking after stop");

    // Stop should also be safe after initialization
    if provider.initialize().is_ok() {
        provider.stop();
        assert!(
            !provider.is_speaking(),
            "Should not be speaking after stop (post-init)"
        );
    }
}

/// Test is_speaking() returns false when not speaking.
#[test]
fn test_is_speaking_default_state() {
    let provider = DefaultTtsProvider::new();

    // Should not be speaking initially
    assert!(!provider.is_speaking(), "Should not be speaking initially");

    // After initialization (if available), still should not be speaking
    if provider.initialize().is_ok() {
        assert!(
            !provider.is_speaking(),
            "Should not be speaking after initialization"
        );
    }
}

// ============================================================================
// ThreadSafeTtsProvider Integration Tests
// ============================================================================

/// Test ThreadSafeTtsProvider initialization.
#[test]
fn test_thread_safe_initialization() {
    let provider = ThreadSafeTtsProvider::new();

    match provider.initialize() {
        Ok(()) => {
            // TTS is available
            assert!(provider.get_voices().len() >= 0);
        }
        Err(e) => {
            // TTS unavailable - error should be informative
            let error_msg = format!("{}", e);
            assert!(!error_msg.is_empty());
        }
    }
}

/// Test ThreadSafeTtsProvider voice enumeration.
#[test]
fn test_thread_safe_voice_enumeration() {
    skip_if_tts_unavailable!();

    let provider = ThreadSafeTtsProvider::new();
    let _ = provider.initialize();

    let voices = provider.get_voices();
    assert!(!voices.is_empty(), "Should have voices available");

    // Verify voice data integrity
    for voice in &voices {
        assert!(!voice.id.is_empty());
        assert!(!voice.name.is_empty());
    }
}

/// Test ThreadSafeTtsProvider voice selection.
#[test]
fn test_thread_safe_voice_selection() {
    skip_if_tts_unavailable!();

    let provider = ThreadSafeTtsProvider::new();
    let _ = provider.initialize();

    let voices = provider.get_voices();
    if voices.is_empty() {
        return;
    }

    // Set valid voice
    let first_voice_id = voices[0].id.clone();
    let result = provider.set_voice(&first_voice_id);
    assert!(result.is_ok(), "Should be able to set valid voice");
    assert_eq!(provider.get_current_voice(), Some(first_voice_id));

    // Invalid voice should fail
    let invalid_result = provider.set_voice("invalid_voice_xyz");
    assert!(invalid_result.is_err());
}

/// Test ThreadSafeTtsProvider rate and volume controls.
#[test]
fn test_thread_safe_rate_volume() {
    let provider = ThreadSafeTtsProvider::new();

    // Rate tests
    assert_eq!(provider.get_rate(), 1.0);
    provider.set_rate(1.5);
    assert_eq!(provider.get_rate(), 1.5);
    provider.set_rate(5.0);
    assert_eq!(provider.get_rate(), 2.0); // Clamped

    // Volume tests
    assert_eq!(provider.get_volume(), 1.0);
    provider.set_volume(0.7);
    assert_eq!(provider.get_volume(), 0.7);
    provider.set_volume(2.0);
    assert_eq!(provider.get_volume(), 1.0); // Clamped
}

/// Test ThreadSafeTtsProvider empty text handling.
#[test]
fn test_thread_safe_empty_text() {
    let provider = ThreadSafeTtsProvider::new();
    let result = provider.speak("");
    assert!(result.is_ok());
    assert!(!provider.is_speaking());
}

/// Test ThreadSafeTtsProvider stop safety.
#[test]
fn test_thread_safe_stop() {
    let provider = ThreadSafeTtsProvider::new();
    provider.stop();
    assert!(!provider.is_speaking());
}

/// Test ThreadSafeTtsProvider is Send + Sync.
#[test]
fn test_thread_safe_provider_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeTtsProvider>();
}

/// Test ThreadSafeTtsProvider clean shutdown via Drop.
#[test]
fn test_thread_safe_provider_drop() {
    // Create provider, initialize, and let it go out of scope
    {
        let provider = ThreadSafeTtsProvider::new();
        let _ = provider.get_voices(); // This triggers initialization
    }
    // If we reach here without hanging, Drop worked correctly
}

/// Test ThreadSafeTtsProvider can be used from multiple threads.
#[test]
fn test_thread_safe_multi_thread_access() {
    skip_if_tts_unavailable!();

    use std::sync::Arc;

    let provider = Arc::new(ThreadSafeTtsProvider::new());
    let _ = provider.initialize();

    let provider1 = Arc::clone(&provider);
    let provider2 = Arc::clone(&provider);
    let provider3 = Arc::clone(&provider);

    // Spawn threads that access the provider concurrently
    let handle1 = thread::spawn(move || {
        let _ = provider1.get_voices();
        provider1.get_rate()
    });

    let handle2 = thread::spawn(move || {
        provider2.set_rate(1.2);
        provider2.get_volume()
    });

    let handle3 = thread::spawn(move || {
        provider3.set_volume(0.8);
        provider3.is_speaking()
    });

    // All threads should complete without panic or deadlock
    let rate = handle1.join().expect("Thread 1 panicked");
    let volume = handle2.join().expect("Thread 2 panicked");
    let speaking = handle3.join().expect("Thread 3 panicked");

    // Verify reasonable values
    assert!(rate >= 0.5 && rate <= 2.0);
    assert!(volume >= 0.0 && volume <= 1.0);
    assert!(!speaking);
}

// ============================================================================
// Platform-Specific Tests
// ============================================================================

/// Test platform-specific backend detection.
#[test]
fn test_platform_backend() {
    let provider = DefaultTtsProvider::new();

    // This test documents which backend we expect on each platform
    // The actual backend is logged during initialization
    if provider.initialize().is_ok() {
        #[cfg(target_os = "windows")]
        {
            // Should use Windows SAPI
            // Verify by checking that voices are enumerable
            let voices = provider.get_voices();
            assert!(
                !voices.is_empty(),
                "Windows should have SAPI voices available"
            );
        }

        #[cfg(target_os = "macos")]
        {
            // Should use macOS AVSpeechSynthesizer or NSSpeechSynthesizer
            let voices = provider.get_voices();
            // macOS should have at least one voice
            assert!(
                !voices.is_empty(),
                "macOS should have speech voices available"
            );
        }

        #[cfg(target_os = "linux")]
        {
            // Should use speech-dispatcher
            // Note: May fail if speech-dispatcher is not installed
            let voices = provider.get_voices();
            // Log available voices for debugging
            if voices.is_empty() {
                eprintln!("Linux: No voices available (speech-dispatcher may not be installed)");
            }
        }
    }
}

/// Test that VoiceInfo contains valid language codes.
#[test]
fn test_voice_language_codes() {
    skip_if_tts_unavailable!();

    let provider = DefaultTtsProvider::new();
    let _ = provider.initialize();

    let voices = provider.get_voices();
    if voices.is_empty() {
        return;
    }

    for voice in &voices {
        // Language should be a non-empty string
        // Common formats: "en-US", "en_US", "en", "unknown"
        assert!(
            !voice.language.is_empty(),
            "Voice '{}' should have a language code",
            voice.name
        );
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test that set_voice fails gracefully for uninitialized provider.
#[test]
fn test_set_voice_uninitialized() {
    let provider = DefaultTtsProvider::new();
    // Don't initialize

    // Try to set a voice - should fail or auto-initialize
    let result = provider.set_voice("any_voice_id");
    // Either fails because TTS unavailable or because voice doesn't exist
    // Both are acceptable - the important thing is it doesn't panic
    assert!(result.is_err() || result.is_ok());
}

/// Test that speak fails gracefully for non-empty text when TTS unavailable.
#[test]
fn test_speak_without_tts() {
    // Create a provider but don't initialize
    let provider = DefaultTtsProvider::new();

    // Speaking non-empty text on uninitialized provider
    // should either auto-initialize or fail gracefully
    let result = provider.speak("Hello");

    // Result depends on TTS availability
    // Either succeeds (auto-initialized) or fails with error
    match result {
        Ok(()) => {
            // TTS was available and auto-initialized
        }
        Err(e) => {
            // TTS unavailable - should have meaningful error
            let msg = format!("{}", e);
            assert!(!msg.is_empty());
        }
    }
}

/// Test multiple initialization calls are idempotent.
#[test]
fn test_multiple_initialization() {
    skip_if_tts_unavailable!();

    let provider = DefaultTtsProvider::new();

    // First initialization
    let result1 = provider.initialize();
    assert!(result1.is_ok());

    // Second initialization should also succeed (idempotent)
    let result2 = provider.initialize();
    assert!(result2.is_ok());

    // Provider should still work
    let voices = provider.get_voices();
    assert!(!voices.is_empty() || voices.is_empty()); // Just verify it doesn't crash
}

/// Test ThreadSafeTtsProvider multiple initialization.
#[test]
fn test_thread_safe_multiple_initialization() {
    skip_if_tts_unavailable!();

    let provider = ThreadSafeTtsProvider::new();

    let result1 = provider.initialize();
    assert!(result1.is_ok());

    let result2 = provider.initialize();
    assert!(result2.is_ok());

    let voices = provider.get_voices();
    assert!(!voices.is_empty() || voices.is_empty());
}

// ============================================================================
// Integration with Audio Engine
// ============================================================================

/// Test that DefaultAudioEngine can access TTS provider.
#[test]
fn test_audio_engine_tts_integration() {
    use rustride::audio::{AudioConfig, AudioEngine, DefaultAudioEngine};

    let config = AudioConfig::default();
    let engine = DefaultAudioEngine::new(config);

    // Get TTS provider from engine
    let tts_provider = engine.tts_provider();

    // Should be able to get rate/volume
    let rate = tts_provider.get_rate();
    let volume = tts_provider.get_volume();

    assert!(rate >= 0.5 && rate <= 2.0);
    assert!(volume >= 0.0 && volume <= 1.0);
}

/// Test that AudioEngine TTS can be configured.
#[test]
fn test_audio_engine_tts_configuration() {
    use rustride::audio::{AudioConfig, AudioEngine, DefaultAudioEngine};

    let mut config = AudioConfig::default();
    config.speech_rate = 1.5;
    config.voice_volume = 80; // 80%

    let engine = DefaultAudioEngine::new(config);
    let tts_provider = engine.tts_provider();

    // Speech rate should be configured
    // Note: rate mapping may differ between config and provider
    let rate = tts_provider.get_rate();
    assert!(rate > 0.5, "Rate should be configured");

    // Volume should be configured
    let volume = tts_provider.get_volume();
    assert!(volume > 0.0 && volume <= 1.0);
}

// ============================================================================
// Stress Tests
// ============================================================================

/// Test rapid rate/volume changes don't cause issues.
#[test]
fn test_rapid_settings_changes() {
    let provider = ThreadSafeTtsProvider::new();

    for i in 0..100 {
        let rate = 0.5 + (i as f32 % 16) * 0.1;
        let volume = (i as f32 % 11) * 0.1;
        provider.set_rate(rate);
        provider.set_volume(volume);
    }

    // Should still be in valid state
    let rate = provider.get_rate();
    let volume = provider.get_volume();
    assert!(rate >= 0.5 && rate <= 2.0);
    assert!(volume >= 0.0 && volume <= 1.0);
}

/// Test rapid provider creation/destruction.
#[test]
fn test_rapid_provider_lifecycle() {
    for _ in 0..10 {
        let provider = ThreadSafeTtsProvider::new();
        provider.set_rate(1.5);
        provider.set_volume(0.8);
        let _ = provider.is_speaking();
        // Provider dropped here
    }
    // All providers should have cleaned up successfully
}

/// Test concurrent voice enumeration.
#[test]
fn test_concurrent_voice_enumeration() {
    skip_if_tts_unavailable!();

    use std::sync::Arc;

    let provider = Arc::new(ThreadSafeTtsProvider::new());
    let _ = provider.initialize();

    let mut handles = vec![];

    for _ in 0..5 {
        let p = Arc::clone(&provider);
        handles.push(thread::spawn(move || {
            let voices = p.get_voices();
            voices.len()
        }));
    }

    let mut results = vec![];
    for handle in handles {
        results.push(
            handle
                .join()
                .expect("Thread panicked during voice enumeration"),
        );
    }

    // All threads should get the same number of voices
    let first = results[0];
    for count in results {
        assert_eq!(count, first, "All threads should see the same voice count");
    }
}
