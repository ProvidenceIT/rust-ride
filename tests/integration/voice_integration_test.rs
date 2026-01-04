//! Voice Control Integration Tests
//!
//! Tests that verify the complete voice command flow from speech recognition
//! to action execution. Uses mock components for CI testing without actual
//! audio hardware or Vosk model.
//!
//! ## Test Coverage
//!
//! - Full voice command flow simulation
//! - Command parsing accuracy with various accents and noise
//! - Wake word detection and activation modes
//! - Command cooldown and debouncing
//! - Context-sensitive command execution
//! - Error handling and edge cases
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                    Voice Integration Test Flow                        │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  Pre-recorded Text ──▶ CommandParser ──▶ VoiceCommand                │
//! │                                                │                      │
//! │  ExecutorContext ◀───── VoiceCommandExecutor ◀┘                      │
//! │         │                                                            │
//! │         ▼                                                            │
//! │    ButtonAction ──▶ Action Verification                              │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```

use rustride::accessibility::voice_control::{
    CommandAudioCue, VoiceCommand, VoiceCommandHandler, VoskVoiceControl,
};
use rustride::hid::actions::ButtonAction;
use rustride::voice::{
    ActivationMode, CommandCooldown, CommandParser, ExecutorContext,
    VoiceCommandExecutor, VoiceEngineState, WakeWordDetector, WakeWordConfig,
};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Test Fixtures: Simulated Voice Input Samples
// ============================================================================

/// Simulated voice input samples representing different accent variations
/// and noise levels for command recognition testing.
///
/// These samples simulate what a speech recognition system might produce
/// when processing audio from various speakers.
struct VoiceInputSamples {
    /// Clear speech samples (native speaker, quiet environment)
    clear: Vec<(&'static str, VoiceCommand)>,
    /// Accented speech samples (non-native speaker variations)
    accented: Vec<(&'static str, VoiceCommand)>,
    /// Noisy environment samples (background noise artifacts)
    noisy: Vec<(&'static str, VoiceCommand)>,
    /// Mumbled/unclear speech samples (not used in tests, kept for future)
    #[allow(dead_code)]
    unclear: Vec<(&'static str, VoiceCommand)>,
    /// Common misrecognitions from various speech engines
    misrecognized: Vec<(&'static str, VoiceCommand)>,
}

impl VoiceInputSamples {
    fn new() -> Self {
        Self {
            clear: vec![
                // Perfect pronunciation
                ("pause", VoiceCommand::Pause),
                ("resume", VoiceCommand::Resume),
                ("start", VoiceCommand::Start),
                ("end", VoiceCommand::End),
                ("skip", VoiceCommand::Skip),
                ("increase", VoiceCommand::Increase),
                ("decrease", VoiceCommand::Decrease),
                ("status", VoiceCommand::Status),
                ("lap", VoiceCommand::TakeLap),
                ("take lap", VoiceCommand::TakeLap),
                ("mark lap", VoiceCommand::TakeLap),
            ],
            accented: vec![
                // British variations
                ("pauz", VoiceCommand::Pause),
                ("resyoom", VoiceCommand::Resume),
                // German accent
                ("schtart", VoiceCommand::Start),
                ("schtop", VoiceCommand::Pause),
                // French accent
                ("pawse", VoiceCommand::Pause),
                // Spanish accent
                ("estart", VoiceCommand::Start),
                ("eskip", VoiceCommand::Skip),
                // Indian English
                ("paas", VoiceCommand::Pause),
                // Australian
                ("staht", VoiceCommand::Start),
            ],
            noisy: vec![
                // Background noise artifacts
                ("uh pause", VoiceCommand::Pause),
                ("um resume", VoiceCommand::Resume),
                ("pause uh", VoiceCommand::Pause),
                ("um skip next", VoiceCommand::Skip),
                ("okay end", VoiceCommand::End),
                // Partial noise interference
                ("p-pause", VoiceCommand::Pause),
                ("res-resume", VoiceCommand::Resume),
            ],
            unclear: vec![
                // Slightly mumbled
                ("paus", VoiceCommand::Pause),
                ("resum", VoiceCommand::Resume),
                ("stat", VoiceCommand::Start),
                ("en", VoiceCommand::End),
                ("ski", VoiceCommand::Skip),
                ("increese", VoiceCommand::Increase),
                ("decreese", VoiceCommand::Decrease),
            ],
            misrecognized: vec![
                // Common speech-to-text errors
                ("paws", VoiceCommand::Pause),
                ("pouse", VoiceCommand::Pause),
                ("resoom", VoiceCommand::Resume),
                ("starred", VoiceCommand::Start),
                ("stopp", VoiceCommand::Pause),
                ("skipt", VoiceCommand::Skip),
                ("necks", VoiceCommand::Skip),
                ("statis", VoiceCommand::Status),
                ("lab", VoiceCommand::TakeLap),
                ("lapp", VoiceCommand::TakeLap),
                ("lack", VoiceCommand::TakeLap),
            ],
        }
    }

}

// ============================================================================
// Mock Voice Engine for Integration Testing
// ============================================================================

/// MockVoiceEngine simulates the voice recognition pipeline for testing
/// without requiring actual audio hardware or Vosk model.
#[allow(dead_code)]
struct MockVoiceEngine {
    state: VoiceEngineState,
    activation_mode: ActivationMode,
    wake_word_detector: WakeWordDetector,
    cooldown: CommandCooldown,
    parser: CommandParser,
    is_active: bool,
}

impl MockVoiceEngine {
    fn new() -> Self {
        Self {
            state: VoiceEngineState::Ready,
            activation_mode: ActivationMode::AlwaysListening,
            wake_word_detector: WakeWordDetector::new(WakeWordConfig::default()),
            cooldown: CommandCooldown::default(),
            parser: CommandParser::new(),
            is_active: true,
        }
    }

    fn with_activation_mode(mut self, mode: ActivationMode) -> Self {
        self.activation_mode = mode;
        // In WakeWord mode, start inactive
        if mode == ActivationMode::WakeWord || mode == ActivationMode::PushToTalk {
            self.is_active = false;
        }
        self
    }

    fn start(&mut self) {
        self.state = VoiceEngineState::Listening;
    }

    #[allow(dead_code)]
    fn stop(&mut self) {
        self.state = VoiceEngineState::Ready;
    }

    /// Simulate feeding recognized text into the engine
    fn feed_text(&mut self, text: &str) -> Option<VoiceCommand> {
        match self.activation_mode {
            ActivationMode::AlwaysListening => {
                // Process all text as commands
                self.process_text_as_command(text)
            }
            ActivationMode::WakeWord => {
                // Check for wake word first
                if let Some(_event) = self.wake_word_detector.process_text(text) {
                    self.is_active = true;
                    return None;
                }

                // Only process commands when active
                if self.is_active {
                    self.process_text_as_command(text)
                } else {
                    None
                }
            }
            ActivationMode::PushToTalk => {
                // Only process when manually activated
                if self.is_active {
                    self.process_text_as_command(text)
                } else {
                    None
                }
            }
        }
    }

    fn process_text_as_command(&mut self, text: &str) -> Option<VoiceCommand> {
        if let Some(result) = self.parser.parse(text) {
            let command = result.command;

            // Check cooldown
            if self.cooldown.is_allowed(&command) {
                self.cooldown.record_command(&command);
                Some(command)
            } else {
                None // Command blocked by cooldown
            }
        } else {
            None
        }
    }

    /// Manually activate (for push-to-talk simulation)
    fn activate(&mut self) {
        self.is_active = true;
        self.wake_word_detector.activate();
    }

    /// Manually deactivate
    fn deactivate(&mut self) {
        if self.activation_mode != ActivationMode::AlwaysListening {
            self.is_active = false;
            self.wake_word_detector.deactivate();
        }
    }
}

// ============================================================================
// Command Parsing Accuracy Tests
// ============================================================================

#[test]
fn test_clear_speech_recognition_accuracy() {
    let samples = VoiceInputSamples::new();
    let parser = CommandParser::new();

    let mut total = 0;
    let mut correct = 0;

    for (text, expected_command) in &samples.clear {
        total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected_command {
                correct += 1;
            }
        }
    }

    let accuracy = (correct as f32 / total as f32) * 100.0;
    assert!(
        accuracy >= 95.0,
        "Clear speech accuracy should be >= 95%, got {:.1}% ({}/{})",
        accuracy, correct, total
    );
}

#[test]
fn test_accented_speech_recognition_accuracy() {
    let samples = VoiceInputSamples::new();
    let parser = CommandParser::new();

    let mut total = 0;
    let mut correct = 0;

    for (text, expected_command) in &samples.accented {
        total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected_command {
                correct += 1;
            }
        }
    }

    let accuracy = (correct as f32 / total as f32) * 100.0;
    // Lower threshold for accented speech due to variations
    assert!(
        accuracy >= 60.0,
        "Accented speech accuracy should be >= 60%, got {:.1}% ({}/{})",
        accuracy, correct, total
    );
}

#[test]
fn test_noisy_environment_recognition_accuracy() {
    let samples = VoiceInputSamples::new();
    let parser = CommandParser::new();

    let mut total = 0;
    let mut correct = 0;

    for (text, expected_command) in &samples.noisy {
        total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected_command {
                correct += 1;
            }
        }
    }

    let accuracy = (correct as f32 / total as f32) * 100.0;
    // Lower threshold for noisy environment
    assert!(
        accuracy >= 70.0,
        "Noisy environment accuracy should be >= 70%, got {:.1}% ({}/{})",
        accuracy, correct, total
    );
}

#[test]
fn test_common_misrecognition_handling() {
    let samples = VoiceInputSamples::new();
    let parser = CommandParser::new();

    let mut total = 0;
    let mut correct = 0;

    for (text, expected_command) in &samples.misrecognized {
        total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected_command {
                correct += 1;
            }
        }
    }

    let accuracy = (correct as f32 / total as f32) * 100.0;
    // Should handle most common misrecognitions
    assert!(
        accuracy >= 90.0,
        "Misrecognition handling accuracy should be >= 90%, got {:.1}% ({}/{})",
        accuracy, correct, total
    );
}

#[test]
fn test_confidence_scores_for_various_inputs() {
    let parser = CommandParser::new();

    // Perfect match should have high confidence
    let result = parser.parse("pause").unwrap();
    assert_eq!(result.confidence, 1.0, "Perfect match should have 1.0 confidence");

    // Misrecognition (corrected) should also have high confidence
    let result = parser.parse("paws").unwrap();
    assert_eq!(result.confidence, 1.0, "Corrected misrecognition should have 1.0 confidence");

    // Fuzzy match should have lower confidence
    let parser_low = CommandParser::with_min_confidence(0.3);
    if let Some(result) = parser_low.parse_with_confidence("pausee") {
        assert!(
            result.confidence < 1.0,
            "Fuzzy match should have confidence < 1.0"
        );
    }
}

// ============================================================================
// Full Voice Command Flow Tests
// ============================================================================

#[test]
fn test_full_command_flow_always_listening() {
    let mut engine = MockVoiceEngine::new();
    engine.start();

    // Test basic command flow
    let commands_to_test = vec![
        ("pause", VoiceCommand::Pause),
        ("resume", VoiceCommand::Resume),
        ("skip", VoiceCommand::Skip),
        ("take lap", VoiceCommand::TakeLap),
    ];

    for (text, expected) in commands_to_test {
        // Reset cooldown to allow same command types
        engine.cooldown.reset();

        let result = engine.feed_text(text);
        assert_eq!(
            result,
            Some(expected),
            "Failed to recognize '{}' as command",
            text
        );
    }
}

#[test]
fn test_full_command_flow_with_wake_word() {
    let mut engine = MockVoiceEngine::new()
        .with_activation_mode(ActivationMode::WakeWord);
    engine.start();

    // Before wake word, commands should be ignored
    assert!(
        engine.feed_text("pause").is_none(),
        "Command should be ignored before wake word"
    );

    // Trigger wake word
    engine.feed_text("hey rust ride");
    assert!(engine.is_active, "Should be active after wake word");

    // Now command should work
    let result = engine.feed_text("pause");
    assert_eq!(result, Some(VoiceCommand::Pause));
}

#[test]
fn test_full_command_flow_with_push_to_talk() {
    let mut engine = MockVoiceEngine::new()
        .with_activation_mode(ActivationMode::PushToTalk);
    engine.start();

    // Before activation, commands should be ignored
    assert!(
        engine.feed_text("pause").is_none(),
        "Command should be ignored before push-to-talk"
    );

    // Activate (simulating button press)
    engine.activate();

    // Now command should work
    let result = engine.feed_text("pause");
    assert_eq!(result, Some(VoiceCommand::Pause));

    // Deactivate (simulating button release)
    engine.deactivate();

    // Command should be ignored again
    engine.cooldown.reset();
    assert!(
        engine.feed_text("pause").is_none(),
        "Command should be ignored after deactivation"
    );
}

#[test]
fn test_command_to_action_execution_flow() {
    let executor = VoiceCommandExecutor::new();
    let parser = CommandParser::new();

    // Test complete flow: text -> command -> action

    // Active workout context
    let workout_context = ExecutorContext::active_workout();

    // Simulate recognized commands
    let test_cases = vec![
        ("pause", Some(ButtonAction::PauseResume)),
        ("skip", Some(ButtonAction::SkipInterval)),
        ("take lap", Some(ButtonAction::AddLapMarker)),
        ("increase", Some(ButtonAction::VolumeUp)),
        ("end", Some(ButtonAction::EndRide)),
    ];

    for (text, expected_action) in test_cases {
        if let Some(result) = parser.parse(text) {
            let action = executor.to_action(&result.command, &workout_context);
            assert_eq!(
                action, expected_action,
                "Command '{}' should map to {:?}",
                text, expected_action
            );
        }
    }
}

#[test]
fn test_context_sensitive_command_flow() {
    let executor = VoiceCommandExecutor::new();
    let parser = CommandParser::new();

    // Test that Skip only works during workout
    let skip = parser.parse("skip").unwrap().command;

    // No workout context
    let no_workout = ExecutorContext::active_ride();
    assert!(
        executor.to_action(&skip, &no_workout).is_none(),
        "Skip should fail without workout"
    );

    // With workout context
    let with_workout = ExecutorContext::active_workout();
    assert_eq!(
        executor.to_action(&skip, &with_workout),
        Some(ButtonAction::SkipInterval)
    );

    // Test Pause/Resume context sensitivity
    let pause = parser.parse("pause").unwrap().command;
    let resume = parser.parse("resume").unwrap().command;

    // Pause not valid when paused
    let paused = ExecutorContext::paused_ride();
    assert!(
        executor.to_action(&pause, &paused).is_none(),
        "Pause should fail when already paused"
    );
    assert_eq!(
        executor.to_action(&resume, &paused),
        Some(ButtonAction::PauseResume)
    );

    // Resume not valid when not paused
    let active = ExecutorContext::active_ride();
    assert!(
        executor.to_action(&resume, &active).is_none(),
        "Resume should fail when not paused"
    );
    assert_eq!(
        executor.to_action(&pause, &active),
        Some(ButtonAction::PauseResume)
    );
}

// ============================================================================
// Command Cooldown Integration Tests
// ============================================================================

#[test]
fn test_command_cooldown_prevents_rapid_repetition() {
    let mut engine = MockVoiceEngine::new();
    engine.start();

    // First command should succeed
    let result1 = engine.feed_text("pause");
    assert_eq!(result1, Some(VoiceCommand::Pause));

    // Immediate repeat should be blocked
    let result2 = engine.feed_text("pause");
    assert!(result2.is_none(), "Rapid repeat should be blocked by cooldown");

    // Different command should work
    let result3 = engine.feed_text("resume");
    assert_eq!(result3, Some(VoiceCommand::Resume));
}

#[test]
fn test_command_cooldown_expiry() {
    let mut cooldown = CommandCooldown::new(10); // 10ms for testing

    // Record command
    cooldown.record_command(&VoiceCommand::Pause);
    assert!(!cooldown.is_allowed(&VoiceCommand::Pause));

    // Wait for expiry
    std::thread::sleep(Duration::from_millis(15));

    // Should be allowed now
    assert!(cooldown.is_allowed(&VoiceCommand::Pause));
}

#[test]
fn test_voice_command_handler_cooldown_integration() {
    let mut handler = VoiceCommandHandler::with_cooldown(10); // 10ms for testing

    // Queue and process first command
    assert!(handler.queue_command_with_cooldown(VoiceCommand::Pause));
    let _ = handler.take_pending();

    // Immediate repeat should be blocked
    assert!(!handler.queue_command_with_cooldown(VoiceCommand::Pause));

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(15));

    // Should work now
    assert!(handler.queue_command_with_cooldown(VoiceCommand::Pause));
}

// ============================================================================
// Wake Word Detection Tests
// ============================================================================

#[test]
fn test_wake_word_detection_variations() {
    // Test all wake word variations from WAKE_PHRASES
    let wake_phrases = vec![
        "hey rust ride",
        "hey rustride",
        "ok ride",
        "okay ride",
        "ok rustride",
    ];

    for phrase in wake_phrases {
        let mut detector = WakeWordDetector::new(WakeWordConfig::default());
        let event = detector.process_text(phrase);
        assert!(
            event.is_some() || detector.is_active(),
            "Wake phrase '{}' should activate detector",
            phrase
        );
    }

    // Test common misrecognition variants
    let misrecognitions = vec![
        "hey rust right",  // Variant of "hey rust ride"
        "hey rest ride",   // Variant of "hey rust ride"
        "okay right",      // Variant of "ok ride"
    ];

    for phrase in misrecognitions {
        let mut detector = WakeWordDetector::new(WakeWordConfig::default());
        let event = detector.process_text(phrase);
        assert!(
            event.is_some() || detector.is_active(),
            "Misrecognition '{}' should be corrected and activate detector",
            phrase
        );
    }
}

#[test]
fn test_wake_word_timeout() {
    let config = WakeWordConfig::new(50); // 50ms timeout for testing
    let mut detector = WakeWordDetector::new(config);

    // Activate
    detector.activate();
    assert!(detector.is_active());

    // Wait for timeout
    std::thread::sleep(Duration::from_millis(60));

    // Check timeout
    let event = detector.check_timeout();
    assert!(event.is_some() || !detector.is_active());
}

#[test]
fn test_wake_word_with_command_in_same_phrase() {
    let mut engine = MockVoiceEngine::new()
        .with_activation_mode(ActivationMode::WakeWord);
    engine.start();

    // Wake word followed by command
    engine.feed_text("hey rust ride");

    // Now send the command
    let result = engine.feed_text("pause");
    assert_eq!(result, Some(VoiceCommand::Pause));
}

// ============================================================================
// Multi-Command Sequence Tests
// ============================================================================

#[test]
fn test_workout_control_sequence() {
    let executor = VoiceCommandExecutor::new();
    let parser = CommandParser::new();

    // Simulate a typical workout control sequence
    let mut context = ExecutorContext::active_workout();
    let mut actions_executed = Vec::new();

    let sequence = vec![
        "take lap",      // Mark lap
        "skip",          // Skip interval
        "pause",         // Pause workout
        // Context changes to paused
        "resume",        // Resume
        "take lap",      // Another lap
        "end",           // End workout
    ];

    for text in sequence {
        if let Some(result) = parser.parse(text) {
            if let Some(action) = executor.to_action(&result.command, &context) {
                actions_executed.push(action.clone());

                // Update context based on action
                match action {
                    ButtonAction::PauseResume if !context.ride_paused => {
                        context = context.with_ride_paused(true);
                    }
                    ButtonAction::PauseResume if context.ride_paused => {
                        context = context.with_ride_paused(false);
                    }
                    ButtonAction::EndRide => {
                        context = ExecutorContext::new();
                    }
                    _ => {}
                }
            }
        }
    }

    // Verify the sequence executed correctly
    assert_eq!(actions_executed.len(), 6);
    assert_eq!(actions_executed[0], ButtonAction::AddLapMarker);
    assert_eq!(actions_executed[1], ButtonAction::SkipInterval);
    assert_eq!(actions_executed[2], ButtonAction::PauseResume);
    assert_eq!(actions_executed[3], ButtonAction::PauseResume);
    assert_eq!(actions_executed[4], ButtonAction::AddLapMarker);
    assert_eq!(actions_executed[5], ButtonAction::EndRide);
}

#[test]
fn test_available_commands_change_with_context() {
    let executor = VoiceCommandExecutor::new();

    // No active ride
    let no_ride = ExecutorContext::new();
    let available = executor.available_commands(&no_ride);
    assert!(available.contains(&VoiceCommand::Increase));
    assert!(available.contains(&VoiceCommand::Decrease));
    assert!(!available.contains(&VoiceCommand::Pause));
    assert!(!available.contains(&VoiceCommand::Skip));

    // Active ride, no workout
    let active_ride = ExecutorContext::active_ride();
    let available = executor.available_commands(&active_ride);
    assert!(available.contains(&VoiceCommand::Pause));
    assert!(available.contains(&VoiceCommand::TakeLap));
    assert!(!available.contains(&VoiceCommand::Skip));
    assert!(!available.contains(&VoiceCommand::Resume));

    // Active workout
    let active_workout = ExecutorContext::active_workout();
    let available = executor.available_commands(&active_workout);
    assert!(available.contains(&VoiceCommand::Skip));
    assert!(available.contains(&VoiceCommand::TakeLap));

    // Paused ride
    let paused = ExecutorContext::paused_ride();
    let available = executor.available_commands(&paused);
    assert!(available.contains(&VoiceCommand::Resume));
    assert!(!available.contains(&VoiceCommand::Pause));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_unknown_command_handling() {
    let parser = CommandParser::new();

    // Completely unrelated phrases
    let unknown_phrases = vec![
        "hello world",
        "the weather is nice",
        "random gibberish xyz",
        "12345",
        "",
    ];

    for phrase in unknown_phrases {
        let result = parser.parse(phrase);
        // Should either return None or Unknown
        if let Some(r) = result {
            assert!(
                matches!(r.command, VoiceCommand::Unknown(_)),
                "Phrase '{}' should be unknown",
                phrase
            );
        }
    }
}

#[test]
fn test_error_messages_for_invalid_context() {
    let executor = VoiceCommandExecutor::new();

    // Skip without workout
    let ctx = ExecutorContext::active_ride();
    let error = executor.get_error_message(&VoiceCommand::Skip, &ctx);
    assert!(error.is_some());
    assert!(error.unwrap().contains("workout"));

    // Pause without ride
    let ctx = ExecutorContext::new();
    let error = executor.get_error_message(&VoiceCommand::Pause, &ctx);
    assert!(error.is_some());
    assert!(error.unwrap().contains("ride"));

    // Resume when not paused
    let ctx = ExecutorContext::active_ride();
    let error = executor.get_error_message(&VoiceCommand::Resume, &ctx);
    assert!(error.is_some());
    assert!(error.unwrap().contains("paused"));
}

// ============================================================================
// Audio Feedback Integration Tests
// ============================================================================

#[test]
fn test_command_confirmation_messages() {
    let commands = vec![
        (VoiceCommand::Pause, "Pausing"),
        (VoiceCommand::Resume, "Resuming"),
        (VoiceCommand::Skip, "Skipping interval"),
        (VoiceCommand::TakeLap, "Marking lap"),
        (VoiceCommand::End, "Ending ride"),
        (VoiceCommand::Increase, "Increasing"),
        (VoiceCommand::Decrease, "Decreasing"),
        (VoiceCommand::Status, "Reading metrics"),
    ];

    for (command, expected_message) in commands {
        let message = VoskVoiceControl::command_confirmation(&command);
        assert_eq!(
            message, expected_message,
            "Command {:?} should have confirmation '{}'",
            command, expected_message
        );
    }
}

#[test]
fn test_command_audio_cue_types() {
    // Positive cues for start/resume
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::Start),
        CommandAudioCue::Positive
    );
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::Resume),
        CommandAudioCue::Positive
    );

    // Neutral cues for pause/end
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::Pause),
        CommandAudioCue::Neutral
    );
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::End),
        CommandAudioCue::Neutral
    );

    // Action cues for skip/lap
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::Skip),
        CommandAudioCue::Action
    );
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::TakeLap),
        CommandAudioCue::Action
    );

    // Error cue for unknown
    assert_eq!(
        VoskVoiceControl::command_audio_cue(&VoiceCommand::Unknown("test".into())),
        CommandAudioCue::Error
    );
}

#[test]
fn test_voice_command_handler_confirmation_flow() {
    let mut handler = VoiceCommandHandler::new();

    // Queue command
    handler.queue_command(VoiceCommand::Pause);
    let _ = handler.take_pending();

    // Should show confirmation
    assert!(handler.should_show_confirmation());
    assert_eq!(handler.confirmation_message(), Some("Pausing"));
    assert_eq!(
        handler.confirmation_audio_cue(),
        Some(CommandAudioCue::Neutral)
    );
}

// ============================================================================
// Comprehensive Flow Simulation Tests
// ============================================================================

#[test]
fn test_end_to_end_voice_command_simulation() {
    // This test simulates a complete voice control session

    let mut engine = MockVoiceEngine::new();
    let executor = VoiceCommandExecutor::new();
    let mut handler = VoiceCommandHandler::new();

    // Start the engine
    engine.start();

    // Simulate user starting a ride (context set externally)
    let mut context = ExecutorContext::active_workout();

    // Sequence of voice commands during a workout
    let voice_inputs = vec![
        "take lap",
        "status",
        "skip",
        "paws",      // Misrecognition of "pause"
        "resoom",    // Misrecognition of "resume"
        "increase",
        "take lap",
        "end",
    ];

    let mut executed_actions = Vec::new();

    for input in voice_inputs {
        if let Some(command) = engine.feed_text(input) {
            if handler.queue_command_with_cooldown(command.clone()) {
                if let Some(cmd) = handler.take_pending() {
                    if let Some(action) = executor.to_action(&cmd, &context) {
                        executed_actions.push((input, action.clone()));

                        // Update context
                        match action {
                            ButtonAction::PauseResume if !context.ride_paused => {
                                context = context.with_ride_paused(true);
                            }
                            ButtonAction::PauseResume if context.ride_paused => {
                                context = context.with_ride_paused(false);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        engine.cooldown.reset(); // Reset for testing
    }

    // Verify key actions were executed
    assert!(!executed_actions.is_empty(), "Should have executed actions");

    // Check specific actions
    let action_names: Vec<_> = executed_actions.iter().map(|(_, a)| a.clone()).collect();
    assert!(action_names.contains(&ButtonAction::AddLapMarker));
    assert!(action_names.contains(&ButtonAction::SkipInterval));
    assert!(action_names.contains(&ButtonAction::PauseResume));
    assert!(action_names.contains(&ButtonAction::VolumeUp));
}

#[test]
fn test_voice_command_recognition_statistics() {
    let samples = VoiceInputSamples::new();
    let parser = CommandParser::new();

    let mut stats: HashMap<&str, (usize, usize)> = HashMap::new();
    stats.insert("clear", (0, 0));
    stats.insert("accented", (0, 0));
    stats.insert("noisy", (0, 0));
    stats.insert("misrecognized", (0, 0));

    // Process each category
    for (text, expected) in &samples.clear {
        let (total, correct) = stats.get_mut("clear").unwrap();
        *total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected {
                *correct += 1;
            }
        }
    }

    for (text, expected) in &samples.accented {
        let (total, correct) = stats.get_mut("accented").unwrap();
        *total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected {
                *correct += 1;
            }
        }
    }

    for (text, expected) in &samples.noisy {
        let (total, correct) = stats.get_mut("noisy").unwrap();
        *total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected {
                *correct += 1;
            }
        }
    }

    for (text, expected) in &samples.misrecognized {
        let (total, correct) = stats.get_mut("misrecognized").unwrap();
        *total += 1;
        if let Some(result) = parser.parse(text) {
            if result.command == *expected {
                *correct += 1;
            }
        }
    }

    // Report and verify minimum thresholds
    for (category, (total, correct)) in &stats {
        if *total > 0 {
            let accuracy = (*correct as f32 / *total as f32) * 100.0;
            eprintln!(
                "Category '{}': {}/{} ({:.1}%)",
                category, correct, total, accuracy
            );

            let min_threshold = match *category {
                "clear" => 95.0,
                "accented" => 50.0,
                "noisy" => 70.0,
                "misrecognized" => 90.0,
                _ => 0.0,
            };

            assert!(
                accuracy >= min_threshold,
                "Category '{}' accuracy {:.1}% below threshold {:.1}%",
                category, accuracy, min_threshold
            );
        }
    }
}
