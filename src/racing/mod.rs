//! Racing module for virtual race events
//!
//! Provides race event management, synchronized starts, and results tracking.

pub mod countdown;
pub mod events;
pub mod results;

// Re-export commonly used types
pub use countdown::CountdownSync;
pub use events::RaceManager;
pub use results::RaceResults;
