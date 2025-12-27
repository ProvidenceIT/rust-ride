//! Glossary of cycling and training terms.
//!
//! Provides definitions for technical terms shown as tooltips during onboarding.

use std::collections::HashMap;

/// A glossary term with definition.
#[derive(Debug, Clone)]
pub struct GlossaryTerm {
    /// The term (e.g., "FTP")
    pub term: String,
    /// Full name (e.g., "Functional Threshold Power")
    pub full_name: Option<String>,
    /// Short definition
    pub definition: String,
}

impl GlossaryTerm {
    /// Create a new glossary term.
    pub fn new(term: impl Into<String>, definition: impl Into<String>) -> Self {
        Self {
            term: term.into(),
            full_name: None,
            definition: definition.into(),
        }
    }

    /// Create a term with a full name (for acronyms).
    pub fn with_full_name(
        term: impl Into<String>,
        full_name: impl Into<String>,
        definition: impl Into<String>,
    ) -> Self {
        Self {
            term: term.into(),
            full_name: Some(full_name.into()),
            definition: definition.into(),
        }
    }
}

/// Glossary of cycling and training terms.
pub struct Glossary {
    terms: HashMap<String, GlossaryTerm>,
}

impl Default for Glossary {
    fn default() -> Self {
        Self::new()
    }
}

impl Glossary {
    /// Create a new glossary with default terms.
    pub fn new() -> Self {
        let mut glossary = Self {
            terms: HashMap::new(),
        };
        glossary.add_default_terms();
        glossary
    }

    /// Add the default cycling/training terms.
    fn add_default_terms(&mut self) {
        // Power-related terms
        self.add(GlossaryTerm::with_full_name(
            "FTP",
            "Functional Threshold Power",
            "The highest average power you can sustain for approximately one hour. Used to set training zones.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "NP",
            "Normalized Power",
            "A weighted average power that accounts for variability in your effort. Better reflects the physiological cost of variable efforts.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "TSS",
            "Training Stress Score",
            "A measure of training load that accounts for both intensity and duration. 100 TSS = 1 hour at FTP.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "IF",
            "Intensity Factor",
            "The ratio of Normalized Power to FTP. IF of 1.0 means you averaged your FTP.",
        ));

        // Sensor/Protocol terms
        self.add(GlossaryTerm::with_full_name(
            "FTMS",
            "Fitness Machine Service",
            "A Bluetooth standard for smart trainers that allows power targets and resistance control.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "ANT+",
            "ANT+ Protocol",
            "A wireless protocol commonly used by cycling sensors. Similar to Bluetooth but optimized for fitness devices.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "BLE",
            "Bluetooth Low Energy",
            "A wireless protocol used by many modern cycling sensors and smart trainers.",
        ));

        // Training terms
        self.add(GlossaryTerm::new(
            "ERG Mode",
            "Trainer automatically adjusts resistance to maintain a target power, regardless of cadence.",
        ));

        self.add(GlossaryTerm::new(
            "Power Zone",
            "Training intensity ranges based on your FTP. Zone 1 is recovery, Zone 7 is max effort.",
        ));

        self.add(GlossaryTerm::new(
            "Cadence",
            "Pedaling speed measured in revolutions per minute (RPM).",
        ));

        self.add(GlossaryTerm::new(
            "Sweet Spot",
            "Training intensity between 88-94% of FTP. Provides high training benefit with manageable fatigue.",
        ));

        self.add(GlossaryTerm::new(
            "Interval",
            "A structured work period followed by recovery. Common in structured training.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "VO2max",
            "Maximal Oxygen Uptake",
            "The maximum rate at which your body can use oxygen during intense exercise. A key fitness indicator.",
        ));

        // Equipment terms
        self.add(GlossaryTerm::new(
            "Smart Trainer",
            "An indoor trainer that can measure power and control resistance, often with ERG mode.",
        ));

        self.add(GlossaryTerm::new(
            "Power Meter",
            "A device that measures the power you produce, typically in the cranks, pedals, or hub.",
        ));

        self.add(GlossaryTerm::with_full_name(
            "HRM",
            "Heart Rate Monitor",
            "A sensor (usually chest strap) that measures your heart rate in beats per minute.",
        ));
    }

    /// Add a term to the glossary.
    pub fn add(&mut self, term: GlossaryTerm) {
        self.terms.insert(term.term.to_lowercase(), term);
    }

    /// Get a term by name (case-insensitive).
    pub fn get(&self, term: &str) -> Option<&GlossaryTerm> {
        self.terms.get(&term.to_lowercase())
    }

    /// Get just the definition for a term.
    pub fn get_definition(&self, term: &str) -> Option<&str> {
        self.get(term).map(|t| t.definition.as_str())
    }

    /// Get all terms.
    pub fn all_terms(&self) -> impl Iterator<Item = &GlossaryTerm> {
        self.terms.values()
    }

    /// Check if a term exists.
    pub fn contains(&self, term: &str) -> bool {
        self.terms.contains_key(&term.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glossary_lookup() {
        let glossary = Glossary::new();

        let ftp = glossary.get("FTP").expect("FTP should exist");
        assert_eq!(ftp.full_name.as_deref(), Some("Functional Threshold Power"));

        // Case insensitive
        assert!(glossary.get("ftp").is_some());
        assert!(glossary.get("Ftp").is_some());
    }

    #[test]
    fn test_glossary_definition() {
        let glossary = Glossary::new();

        let def = glossary.get_definition("ERG Mode");
        assert!(def.is_some());
        assert!(def.unwrap().contains("resistance"));
    }
}
