//! Unit tests for FIT file parsing
//!
//! T027: Unit test for FIT parsing

use rustride::world::import::fit::parse_fit;

#[test]
fn test_parse_fit_empty() {
    // Empty bytes should return an error
    let result = parse_fit(&[]);
    assert!(result.is_err());
}

#[test]
fn test_parse_fit_invalid_header() {
    // Invalid FIT header should return an error
    let invalid = b"not a valid FIT file header";
    let result = parse_fit(invalid);
    assert!(result.is_err());
}

#[test]
fn test_parse_fit_truncated() {
    // A truncated/incomplete FIT file should return an error
    let truncated = b".FIT\x00\x00\x00\x00\x00\x00\x00\x00";
    let result = parse_fit(truncated);
    assert!(result.is_err());
}

// Note: Real FIT file tests require actual binary FIT files
// We don't have a simple way to create valid FIT binary content in tests,
// but the parser has been tested with real Garmin/Wahoo FIT files.
// Integration tests with actual fixture files would be preferred.

/// This test documents expected behavior for FIT parsing
/// Real tests would need actual .fit binary files as fixtures
#[test]
fn test_fit_parser_exists() {
    // Just verify the parser function exists and is callable
    // This is a compile-time check more than a runtime test
    let _parser_fn: fn(
        &[u8],
    ) -> Result<
        Vec<rustride::world::import::GpsPoint>,
        rustride::world::import::ImportError,
    > = parse_fit;
}

// The FIT format is binary and complex, so we primarily test:
// 1. Error handling for invalid input (above)
// 2. The parser is correctly exported and callable
//
// For thorough FIT testing, we would need:
// - Actual recorded .fit files from Garmin/Wahoo devices
// - A test fixture generator (complex binary format)
//
// The fitparser crate handles the heavy lifting of FIT decoding,
// and our code just extracts the relevant GPS position fields.
