//! Voice Command Parser with Fuzzy Matching
//!
//! This module provides robust command parsing using Levenshtein distance-based
//! fuzzy matching to handle speech recognition errors and variations.
//!
//! ## Features
//!
//! - Levenshtein distance calculation for fuzzy string matching
//! - Confidence scoring based on edit distance
//! - Common misrecognition mappings (e.g., "paws" -> "pause")
//! - Configurable minimum confidence threshold
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::command_parser::{CommandParser, ParseResult};
//! use rustride::accessibility::voice_control::VoiceCommand;
//!
//! let parser = CommandParser::new();
//! let result = parser.parse("paws workout");
//!
//! if let Some(parse_result) = result {
//!     println!("Command: {:?}", parse_result.command);
//!     println!("Confidence: {:.2}", parse_result.confidence);
//! }
//! ```

use crate::accessibility::voice_control::VoiceCommand;

/// Default minimum confidence threshold (0.0 - 1.0).
/// Commands below this threshold will not be recognized.
pub const DEFAULT_MIN_CONFIDENCE: f32 = 0.6;

/// Maximum Levenshtein distance for a fuzzy match to be considered.
/// Higher values allow more lenient matching but may cause false positives.
const MAX_EDIT_DISTANCE: usize = 3;

/// Result of parsing a phrase into a voice command.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The recognized command.
    pub command: VoiceCommand,
    /// Confidence score (0.0 - 1.0).
    /// Higher values indicate better matches.
    pub confidence: f32,
    /// The original phrase that was parsed.
    pub original_phrase: String,
    /// The matched keyword or phrase.
    pub matched_keyword: String,
}

impl ParseResult {
    /// Create a new parse result.
    pub fn new(
        command: VoiceCommand,
        confidence: f32,
        original_phrase: impl Into<String>,
        matched_keyword: impl Into<String>,
    ) -> Self {
        Self {
            command,
            confidence: confidence.clamp(0.0, 1.0),
            original_phrase: original_phrase.into(),
            matched_keyword: matched_keyword.into(),
        }
    }

    /// Check if this result meets the minimum confidence threshold.
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }
}

/// Command definition with associated keywords.
struct CommandDef {
    /// The command this definition maps to.
    command: VoiceCommand,
    /// Primary keywords that map to this command (exact matches get high confidence).
    primary_keywords: &'static [&'static str],
    /// Secondary keywords with slightly lower confidence.
    secondary_keywords: &'static [&'static str],
}

/// Common misrecognitions mapping to their correct forms.
/// Format: (misrecognized, correct)
static MISRECOGNITION_MAP: &[(&str, &str)] = &[
    // Pause misrecognitions
    ("paws", "pause"),
    ("paus", "pause"),
    ("pas", "pause"),
    ("paused", "pause"),
    ("pauze", "pause"),
    ("pawse", "pause"),
    ("pouse", "pause"),
    // Resume misrecognitions
    ("resoom", "resume"),
    ("resumé", "resume"),
    ("resum", "resume"),
    ("rezume", "resume"),
    ("resumed", "resume"),
    // Start misrecognitions
    ("starred", "start"),
    ("startt", "start"),
    ("stort", "start"),
    ("stard", "start"),
    ("started", "start"),
    // Stop/End misrecognitions
    ("stopp", "stop"),
    ("stope", "stop"),
    ("stopped", "stop"),
    ("and", "end"),
    ("ant", "end"),
    ("ended", "end"),
    // Skip/Next misrecognitions
    ("skipt", "skip"),
    ("skipped", "skip"),
    ("nex", "next"),
    ("necks", "next"),
    ("nextt", "next"),
    // Increase/Decrease misrecognitions
    ("encrase", "increase"),
    ("increese", "increase"),
    ("decrase", "decrease"),
    ("decreese", "decrease"),
    ("op", "up"),
    ("don", "down"),
    ("doun", "down"),
    // Status misrecognitions
    ("statis", "status"),
    ("statuss", "status"),
    ("metric", "metrics"),
    // Lap misrecognitions
    ("lab", "lap"),
    ("lapp", "lap"),
    ("lapt", "lap"),
    ("lack", "lap"),
    ("lock", "lap"),
];

/// Voice command parser with fuzzy matching support.
#[derive(Debug, Clone)]
pub struct CommandParser {
    /// Minimum confidence threshold for accepting a command.
    min_confidence: f32,
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandParser {
    /// Create a new command parser with default settings.
    pub fn new() -> Self {
        Self {
            min_confidence: DEFAULT_MIN_CONFIDENCE,
        }
    }

    /// Create a parser with a custom minimum confidence threshold.
    pub fn with_min_confidence(min_confidence: f32) -> Self {
        Self {
            min_confidence: min_confidence.clamp(0.0, 1.0),
        }
    }

    /// Get the minimum confidence threshold.
    pub fn min_confidence(&self) -> f32 {
        self.min_confidence
    }

    /// Set the minimum confidence threshold.
    pub fn set_min_confidence(&mut self, threshold: f32) {
        self.min_confidence = threshold.clamp(0.0, 1.0);
    }

    /// Parse a phrase into a voice command.
    ///
    /// Returns `Some(ParseResult)` if a command was recognized with sufficient confidence,
    /// or `None` if no command matched or confidence was too low.
    pub fn parse(&self, phrase: &str) -> Option<ParseResult> {
        let result = self.parse_with_confidence(phrase);
        result.filter(|r| r.meets_threshold(self.min_confidence))
    }

    /// Parse a phrase and return result regardless of confidence threshold.
    ///
    /// Use this when you want to handle low-confidence results specially.
    pub fn parse_with_confidence(&self, phrase: &str) -> Option<ParseResult> {
        if phrase.is_empty() {
            return None;
        }

        let normalized = self.normalize_phrase(phrase);
        if normalized.is_empty() {
            return None;
        }

        // Apply misrecognition corrections
        let corrected = self.apply_corrections(&normalized);

        // Try exact matching first (highest confidence)
        if let Some(result) = self.try_exact_match(&corrected, phrase) {
            return Some(result);
        }

        // Try fuzzy matching
        self.try_fuzzy_match(&corrected, phrase)
    }

    /// Normalize a phrase for matching.
    fn normalize_phrase(&self, phrase: &str) -> String {
        phrase
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Apply common misrecognition corrections.
    fn apply_corrections(&self, phrase: &str) -> String {
        let mut result = phrase.to_string();

        for (misrecognition, correction) in MISRECOGNITION_MAP {
            // Replace whole words only
            let words: Vec<&str> = result.split_whitespace().collect();
            let corrected_words: Vec<&str> = words
                .iter()
                .map(|&word| {
                    if word == *misrecognition {
                        *correction
                    } else {
                        word
                    }
                })
                .collect();
            result = corrected_words.join(" ");
        }

        result
    }

    /// Try exact keyword matching.
    fn try_exact_match(&self, phrase: &str, original: &str) -> Option<ParseResult> {
        let command_defs = self.get_command_definitions();

        for def in command_defs {
            // Check primary keywords (high confidence)
            for &keyword in def.primary_keywords {
                if phrase.contains(keyword) {
                    return Some(ParseResult::new(
                        def.command.clone(),
                        1.0,
                        original,
                        keyword,
                    ));
                }
            }

            // Check secondary keywords (slightly lower confidence)
            for &keyword in def.secondary_keywords {
                if phrase.contains(keyword) {
                    return Some(ParseResult::new(
                        def.command.clone(),
                        0.9,
                        original,
                        keyword,
                    ));
                }
            }
        }

        None
    }

    /// Try fuzzy matching using Levenshtein distance.
    fn try_fuzzy_match(&self, phrase: &str, original: &str) -> Option<ParseResult> {
        let words: Vec<&str> = phrase.split_whitespace().collect();
        let command_defs = self.get_command_definitions();

        let mut best_match: Option<ParseResult> = None;
        let mut best_confidence: f32 = 0.0;

        for word in &words {
            for def in &command_defs {
                // Check against primary keywords
                for &keyword in def.primary_keywords {
                    let distance = levenshtein_distance(word, keyword);
                    if distance <= MAX_EDIT_DISTANCE {
                        let confidence = calculate_confidence(word.len(), distance);
                        if confidence > best_confidence {
                            best_confidence = confidence;
                            best_match = Some(ParseResult::new(
                                def.command.clone(),
                                confidence,
                                original,
                                keyword,
                            ));
                        }
                    }
                }

                // Check against secondary keywords (with reduced confidence)
                for &keyword in def.secondary_keywords {
                    let distance = levenshtein_distance(word, keyword);
                    if distance <= MAX_EDIT_DISTANCE {
                        let confidence = calculate_confidence(word.len(), distance) * 0.9;
                        if confidence > best_confidence {
                            best_confidence = confidence;
                            best_match = Some(ParseResult::new(
                                def.command.clone(),
                                confidence,
                                original,
                                keyword,
                            ));
                        }
                    }
                }
            }
        }

        best_match
    }

    /// Get command definitions for matching.
    fn get_command_definitions(&self) -> Vec<CommandDef> {
        vec![
            CommandDef {
                command: VoiceCommand::Start,
                primary_keywords: &["start", "begin", "go"],
                secondary_keywords: &["commence", "initiate", "launch"],
            },
            CommandDef {
                command: VoiceCommand::Pause,
                primary_keywords: &["pause", "stop", "hold"],
                secondary_keywords: &["wait", "halt", "freeze"],
            },
            CommandDef {
                command: VoiceCommand::Resume,
                primary_keywords: &["resume", "continue", "unpause"],
                secondary_keywords: &["proceed", "restart", "carry on"],
            },
            CommandDef {
                command: VoiceCommand::End,
                primary_keywords: &["end", "finish", "done"],
                secondary_keywords: &["complete", "terminate", "quit"],
            },
            CommandDef {
                command: VoiceCommand::Skip,
                primary_keywords: &["skip", "next"],
                secondary_keywords: &["forward", "advance"],
            },
            CommandDef {
                command: VoiceCommand::Increase,
                primary_keywords: &["increase", "up", "more"],
                secondary_keywords: &["raise", "higher", "boost"],
            },
            CommandDef {
                command: VoiceCommand::Decrease,
                primary_keywords: &["decrease", "down", "less"],
                secondary_keywords: &["lower", "reduce", "drop"],
            },
            CommandDef {
                command: VoiceCommand::Status,
                primary_keywords: &["status", "metrics", "how am i doing"],
                secondary_keywords: &["stats", "info", "update", "progress"],
            },
            CommandDef {
                command: VoiceCommand::TakeLap,
                primary_keywords: &["lap", "take lap", "mark lap"],
                secondary_keywords: &["new lap", "split", "mark"],
            },
        ]
    }
}

/// Calculate Levenshtein distance between two strings.
///
/// The Levenshtein distance is the minimum number of single-character edits
/// (insertions, deletions, or substitutions) required to transform one string
/// into another.
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    // Early exit for empty strings
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Use two rows for space efficiency
    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row: Vec<usize> = vec![0; len2 + 1];

    for i in 1..=len1 {
        curr_row[0] = i;

        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };

            curr_row[j] = std::cmp::min(
                std::cmp::min(
                    prev_row[j] + 1,      // deletion
                    curr_row[j - 1] + 1,  // insertion
                ),
                prev_row[j - 1] + cost,   // substitution
            );
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len2]
}

/// Calculate confidence score based on word length and edit distance.
///
/// Returns a value between 0.0 and 1.0, where 1.0 is a perfect match.
fn calculate_confidence(word_len: usize, distance: usize) -> f32 {
    if distance == 0 {
        return 1.0;
    }

    if word_len == 0 {
        return 0.0;
    }

    // Calculate similarity ratio
    let max_len = word_len.max(distance);
    let similarity = 1.0 - (distance as f32 / max_len as f32);

    // Apply a penalty for fuzzy matches to prefer exact matches
    (similarity * 0.85).clamp(0.0, 0.95)
}

/// Calculate normalized similarity between two strings (0.0 - 1.0).
///
/// This is useful for comparing how similar two strings are.
pub fn string_similarity(s1: &str, s2: &str) -> f32 {
    let distance = levenshtein_distance(s1, s2);
    let max_len = s1.len().max(s2.len());

    if max_len == 0 {
        return 1.0; // Both empty strings are identical
    }

    1.0 - (distance as f32 / max_len as f32)
}

/// Extended VoiceCommand methods for fuzzy matching.
impl VoiceCommand {
    /// Parse a phrase into a command with confidence score.
    ///
    /// This is an enhanced version of `from_phrase` that uses fuzzy matching
    /// and returns both the command and confidence level.
    pub fn from_phrase_with_confidence(phrase: &str) -> (Self, f32) {
        let parser = CommandParser::new();

        match parser.parse_with_confidence(phrase) {
            Some(result) => (result.command, result.confidence),
            None => (VoiceCommand::Unknown(phrase.to_lowercase()), 0.0),
        }
    }

    /// Parse with custom minimum confidence threshold.
    ///
    /// Returns `Unknown` if confidence is below threshold.
    pub fn from_phrase_with_threshold(phrase: &str, min_confidence: f32) -> (Self, f32) {
        let parser = CommandParser::with_min_confidence(min_confidence);

        match parser.parse(phrase) {
            Some(result) => (result.command, result.confidence),
            None => {
                // Get the confidence anyway for diagnostics
                let fallback_parser = CommandParser::with_min_confidence(0.0);
                match fallback_parser.parse_with_confidence(phrase) {
                    Some(result) => (VoiceCommand::Unknown(phrase.to_lowercase()), result.confidence),
                    None => (VoiceCommand::Unknown(phrase.to_lowercase()), 0.0),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("pause", "pause"), 0);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_levenshtein_distance_empty() {
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", "world"), 5);
    }

    #[test]
    fn test_levenshtein_distance_single_edit() {
        // Substitution
        assert_eq!(levenshtein_distance("pause", "paws"), 2);
        // Insertion
        assert_eq!(levenshtein_distance("pause", "pauses"), 1);
        // Deletion
        assert_eq!(levenshtein_distance("pause", "paus"), 1);
    }

    #[test]
    fn test_levenshtein_distance_multiple_edits() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_string_similarity() {
        assert_eq!(string_similarity("pause", "pause"), 1.0);
        assert!(string_similarity("pause", "paws") > 0.5);
        assert!(string_similarity("pause", "pause") > string_similarity("pause", "paws"));
    }

    #[test]
    fn test_parser_exact_match() {
        let parser = CommandParser::new();

        let result = parser.parse("pause").unwrap();
        assert_eq!(result.command, VoiceCommand::Pause);
        assert_eq!(result.confidence, 1.0);

        let result = parser.parse("resume workout").unwrap();
        assert_eq!(result.command, VoiceCommand::Resume);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_parser_misrecognition_correction() {
        let parser = CommandParser::new();

        // "paws" should be corrected to "pause"
        let result = parser.parse("paws").unwrap();
        assert_eq!(result.command, VoiceCommand::Pause);
        assert_eq!(result.confidence, 1.0);

        // "resoom" should be corrected to "resume"
        let result = parser.parse("resoom").unwrap();
        assert_eq!(result.command, VoiceCommand::Resume);
    }

    #[test]
    fn test_parser_fuzzy_match() {
        let parser = CommandParser::new();

        // "pausee" is close to "pause"
        let result = parser.parse("pausee").unwrap();
        assert_eq!(result.command, VoiceCommand::Pause);
        assert!(result.confidence < 1.0);
        assert!(result.confidence >= DEFAULT_MIN_CONFIDENCE);
    }

    #[test]
    fn test_parser_below_threshold() {
        let parser = CommandParser::with_min_confidence(0.99);

        // Fuzzy match should be below high threshold
        let result = parser.parse("pauseeeee");
        assert!(result.is_none());
    }

    #[test]
    fn test_parser_empty_input() {
        let parser = CommandParser::new();
        assert!(parser.parse("").is_none());
        assert!(parser.parse("   ").is_none());
    }

    #[test]
    fn test_parser_all_commands() {
        let parser = CommandParser::new();

        let test_cases = [
            ("start", VoiceCommand::Start),
            ("begin", VoiceCommand::Start),
            ("go", VoiceCommand::Start),
            ("pause", VoiceCommand::Pause),
            ("stop", VoiceCommand::Pause),
            ("hold", VoiceCommand::Pause),
            ("resume", VoiceCommand::Resume),
            ("continue", VoiceCommand::Resume),
            ("unpause", VoiceCommand::Resume),
            ("end", VoiceCommand::End),
            ("finish", VoiceCommand::End),
            ("done", VoiceCommand::End),
            ("skip", VoiceCommand::Skip),
            ("next", VoiceCommand::Skip),
            ("increase", VoiceCommand::Increase),
            ("up", VoiceCommand::Increase),
            ("more", VoiceCommand::Increase),
            ("decrease", VoiceCommand::Decrease),
            ("down", VoiceCommand::Decrease),
            ("less", VoiceCommand::Decrease),
            ("status", VoiceCommand::Status),
            ("metrics", VoiceCommand::Status),
            ("lap", VoiceCommand::TakeLap),
            ("take lap", VoiceCommand::TakeLap),
            ("mark lap", VoiceCommand::TakeLap),
        ];

        for (phrase, expected_command) in test_cases {
            let result = parser.parse(phrase).unwrap_or_else(|| {
                panic!("Failed to parse '{}' as a command", phrase)
            });
            assert_eq!(
                result.command, expected_command,
                "Phrase '{}' expected {:?} but got {:?}",
                phrase, expected_command, result.command
            );
        }
    }

    #[test]
    fn test_parser_case_insensitive() {
        let parser = CommandParser::new();

        let result = parser.parse("PAUSE").unwrap();
        assert_eq!(result.command, VoiceCommand::Pause);

        let result = parser.parse("Resume").unwrap();
        assert_eq!(result.command, VoiceCommand::Resume);
    }

    #[test]
    fn test_parser_with_noise() {
        let parser = CommandParser::new();

        let result = parser.parse("please pause the workout").unwrap();
        assert_eq!(result.command, VoiceCommand::Pause);

        let result = parser.parse("can you skip to next interval").unwrap();
        assert_eq!(result.command, VoiceCommand::Skip);
    }

    #[test]
    fn test_from_phrase_with_confidence() {
        let (command, confidence) = VoiceCommand::from_phrase_with_confidence("pause");
        assert_eq!(command, VoiceCommand::Pause);
        assert_eq!(confidence, 1.0);

        let (command, confidence) = VoiceCommand::from_phrase_with_confidence("paws");
        assert_eq!(command, VoiceCommand::Pause);
        assert_eq!(confidence, 1.0); // After correction
    }

    #[test]
    fn test_from_phrase_with_threshold() {
        // High confidence match
        let (command, confidence) = VoiceCommand::from_phrase_with_threshold("pause", 0.5);
        assert_eq!(command, VoiceCommand::Pause);
        assert_eq!(confidence, 1.0);

        // Low confidence threshold test
        let (command, _) = VoiceCommand::from_phrase_with_threshold("xyz123", 0.9);
        assert!(matches!(command, VoiceCommand::Unknown(_)));
    }

    #[test]
    fn test_common_misrecognitions() {
        let parser = CommandParser::new();

        // Test various common misrecognitions
        let misrecognitions = [
            ("paws", VoiceCommand::Pause),
            ("pawse", VoiceCommand::Pause),
            ("resoom", VoiceCommand::Resume),
            ("starred", VoiceCommand::Start),
            ("stopp", VoiceCommand::Pause),
            ("skipt", VoiceCommand::Skip),
            ("necks", VoiceCommand::Skip),
            ("statis", VoiceCommand::Status),
            ("lab", VoiceCommand::TakeLap),
            ("lapp", VoiceCommand::TakeLap),
            ("lack", VoiceCommand::TakeLap),
        ];

        for (phrase, expected_command) in misrecognitions {
            let result = parser.parse(phrase);
            assert!(
                result.is_some(),
                "Failed to parse misrecognition '{}'",
                phrase
            );
            assert_eq!(
                result.unwrap().command,
                expected_command,
                "Misrecognition '{}' didn't map correctly",
                phrase
            );
        }
    }

    #[test]
    fn test_parse_result_meets_threshold() {
        let result = ParseResult::new(VoiceCommand::Pause, 0.8, "paws", "pause");
        assert!(result.meets_threshold(0.5));
        assert!(result.meets_threshold(0.8));
        assert!(!result.meets_threshold(0.9));
    }

    #[test]
    fn test_parser_min_confidence_setter() {
        let mut parser = CommandParser::new();
        assert_eq!(parser.min_confidence(), DEFAULT_MIN_CONFIDENCE);

        parser.set_min_confidence(0.8);
        assert_eq!(parser.min_confidence(), 0.8);

        // Test clamping
        parser.set_min_confidence(1.5);
        assert_eq!(parser.min_confidence(), 1.0);

        parser.set_min_confidence(-0.5);
        assert_eq!(parser.min_confidence(), 0.0);
    }

    #[test]
    fn test_secondary_keywords() {
        let parser = CommandParser::new();

        // Secondary keywords should have slightly lower confidence
        let result = parser.parse("raise").unwrap();
        assert_eq!(result.command, VoiceCommand::Increase);
        assert!(result.confidence < 1.0);
        assert!(result.confidence >= 0.9);
    }

    #[test]
    fn test_lap_command_variations() {
        let parser = CommandParser::new();

        // Test primary lap keywords
        let result = parser.parse("lap").unwrap();
        assert_eq!(result.command, VoiceCommand::TakeLap);
        assert_eq!(result.confidence, 1.0);

        let result = parser.parse("take lap").unwrap();
        assert_eq!(result.command, VoiceCommand::TakeLap);
        assert_eq!(result.confidence, 1.0);

        let result = parser.parse("mark lap").unwrap();
        assert_eq!(result.command, VoiceCommand::TakeLap);
        assert_eq!(result.confidence, 1.0);

        // Test with surrounding words
        let result = parser.parse("please take lap now").unwrap();
        assert_eq!(result.command, VoiceCommand::TakeLap);

        // Test misrecognitions
        let result = parser.parse("lab").unwrap();
        assert_eq!(result.command, VoiceCommand::TakeLap);

        let result = parser.parse("lapp").unwrap();
        assert_eq!(result.command, VoiceCommand::TakeLap);
    }
}
