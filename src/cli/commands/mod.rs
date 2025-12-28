//! CLI command implementations.
//!
//! Each submodule implements a group of related commands.
//!
//! Phase 4: Added ride and workout commands (T044, T045)
//! Phase 5: Added rides export commands (T052, T053)
//! Phase 6: Added sensors commands (T057-T060)

pub mod daemon;
pub mod ride;
pub mod rides;
pub mod sensors;
pub mod workout;
