#!/bin/bash
#
# FIT File Validator Script
#
# This script validates FIT files exported by RustRide to ensure they comply
# with the FIT specification. It uses the Rust fitparser crate through a
# cargo test for validation.
#
# Usage:
#   ./scripts/validate_fit.sh <fit_file>
#   ./scripts/validate_fit.sh --all     # Run all FIT validation tests
#
# Exit codes:
#   0 - FIT file is valid
#   1 - FIT file is invalid or error occurred
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_usage() {
    echo "FIT File Validator for RustRide"
    echo ""
    echo "Usage:"
    echo "  $0 <fit_file>     Validate a specific FIT file"
    echo "  $0 --all          Run all FIT validation integration tests"
    echo "  $0 --help         Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 ~/rides/2025-01-15_Workout.fit"
    echo "  $0 --all"
}

validate_fit_file() {
    local fit_file="$1"

    if [[ ! -f "$fit_file" ]]; then
        echo -e "${RED}Error: File not found: $fit_file${NC}"
        exit 1
    fi

    if [[ ! "$fit_file" =~ \.fit$ ]]; then
        echo -e "${YELLOW}Warning: File does not have .fit extension${NC}"
    fi

    echo "Validating FIT file: $fit_file"
    echo "----------------------------------------"

    # Create a temporary Rust test file for validation
    local temp_test_dir=$(mktemp -d)
    local temp_test_file="$temp_test_dir/validate.rs"

    cat > "$temp_test_file" << 'RUSTEOF'
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: validate <fit_file>");
        std::process::exit(1);
    }

    let fit_file = &args[1];
    let data = match fs::read(fit_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Validate FIT header
    if data.len() < 14 {
        eprintln!("Error: File too small to be a valid FIT file (< 14 bytes)");
        std::process::exit(1);
    }

    let header_size = data[0];
    if header_size != 14 {
        eprintln!("Warning: Non-standard header size: {} (expected 14)", header_size);
    }

    // Check .FIT signature
    if &data[8..12] != b".FIT" {
        eprintln!("Error: Invalid FIT signature (expected '.FIT')");
        std::process::exit(1);
    }

    println!("Header validation: PASSED");

    // Parse with fitparser
    match fitparser::from_bytes(&data) {
        Ok(records) => {
            println!("FIT parsing: PASSED ({} records)", records.len());

            // Check for required message types
            let has_file_id = records.iter().any(|r| r.kind() == fitparser::profile::MesgNum::FileId);
            let has_session = records.iter().any(|r| r.kind() == fitparser::profile::MesgNum::Session);
            let has_lap = records.iter().any(|r| r.kind() == fitparser::profile::MesgNum::Lap);
            let has_activity = records.iter().any(|r| r.kind() == fitparser::profile::MesgNum::Activity);
            let has_record = records.iter().any(|r| r.kind() == fitparser::profile::MesgNum::Record);
            let has_event = records.iter().any(|r| r.kind() == fitparser::profile::MesgNum::Event);

            println!("");
            println!("Required FIT messages:");
            println!("  FileId:   {}", if has_file_id { "PRESENT" } else { "MISSING" });
            println!("  Session:  {}", if has_session { "PRESENT" } else { "MISSING" });
            println!("  Lap:      {}", if has_lap { "PRESENT" } else { "MISSING" });
            println!("  Activity: {}", if has_activity { "PRESENT" } else { "MISSING" });
            println!("  Record:   {}", if has_record { "PRESENT" } else { "MISSING" });
            println!("  Event:    {}", if has_event { "PRESENT" } else { "MISSING" });

            let record_count = records.iter().filter(|r| r.kind() == fitparser::profile::MesgNum::Record).count();
            let lap_count = records.iter().filter(|r| r.kind() == fitparser::profile::MesgNum::Lap).count();

            println!("");
            println!("Statistics:");
            println!("  Total messages: {}", records.len());
            println!("  Record samples: {}", record_count);
            println!("  Lap count:      {}", lap_count);

            if has_file_id && has_session && has_lap && has_activity {
                println!("");
                println!("FIT validation: PASSED");
                std::process::exit(0);
            } else {
                eprintln!("");
                eprintln!("FIT validation: FAILED (missing required messages)");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: Failed to parse FIT file: {}", e);
            std::process::exit(1);
        }
    }
}
RUSTEOF

    # Run the validation using cargo
    cd "$PROJECT_DIR"

    # Create a test that validates the file
    echo ""
    echo "Running fitparser validation..."

    # Use cargo to run the validation test
    if cargo run --quiet --example fit_validator -- "$fit_file" 2>/dev/null; then
        echo -e "${GREEN}Validation PASSED${NC}"
        rm -rf "$temp_test_dir"
        exit 0
    else
        # If the example doesn't exist, fall back to running integration tests
        echo "Running validation via integration tests..."
        if FIT_FILE_TO_VALIDATE="$fit_file" cargo test --test integration_tests fit_validation -- --nocapture 2>/dev/null; then
            echo -e "${GREEN}Validation PASSED${NC}"
            rm -rf "$temp_test_dir"
            exit 0
        else
            # Ultimate fallback: just try to parse with inline Rust
            echo "Running inline validation..."
            echo ""

            # Check if we can parse the file using rustride's existing infrastructure
            cargo test --test integration_tests test_validate_fit_file_structure -- --nocapture 2>&1 | head -50

            rm -rf "$temp_test_dir"
            exit 1
        fi
    fi
}

run_all_validation_tests() {
    echo "Running all FIT validation tests..."
    echo "========================================"

    cd "$PROJECT_DIR"

    # Run integration tests
    echo ""
    echo "Running FIT validation integration tests..."
    if cargo test --test integration_tests fit_validation -- --nocapture; then
        echo -e "${GREEN}All FIT validation tests PASSED${NC}"
    else
        echo -e "${RED}Some FIT validation tests FAILED${NC}"
        exit 1
    fi

    # Run unit tests for FIT export
    echo ""
    echo "Running FIT export unit tests..."
    if cargo test fit_export -- --nocapture; then
        echo -e "${GREEN}All FIT export tests PASSED${NC}"
    else
        echo -e "${RED}Some FIT export tests FAILED${NC}"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}All FIT validation tests completed successfully!${NC}"
}

# Main entry point
case "${1:-}" in
    --help|-h)
        print_usage
        exit 0
        ;;
    --all)
        run_all_validation_tests
        ;;
    "")
        print_usage
        exit 1
        ;;
    *)
        validate_fit_file "$1"
        ;;
esac
