//! Onboarding module for first-time user experience.
//!
//! Provides a guided wizard for new users to set up their profile,
//! connect sensors, configure FTP, and learn the UI.

pub mod glossary;
pub mod steps;

use serde::{Deserialize, Serialize};

// Re-export types
pub use glossary::{Glossary, GlossaryTerm};
pub use steps::OnboardingStep;

/// State of the onboarding wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingState {
    /// Whether onboarding has been completed
    pub completed: bool,
    /// Current step in the wizard
    pub current_step: OnboardingStep,
    /// Whether the user chose to skip onboarding
    pub skipped: bool,
    /// Steps that have been completed
    pub completed_steps: Vec<OnboardingStep>,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            completed: false,
            current_step: OnboardingStep::Welcome,
            skipped: false,
            completed_steps: Vec::new(),
        }
    }
}

impl OnboardingState {
    /// Check if a specific step is complete.
    pub fn is_step_complete(&self, step: OnboardingStep) -> bool {
        self.completed_steps.contains(&step)
    }

    /// Mark the current step as complete and advance.
    pub fn complete_current_step(&mut self) {
        if !self.completed_steps.contains(&self.current_step) {
            self.completed_steps.push(self.current_step);
        }

        if let Some(next) = self.current_step.next() {
            self.current_step = next;
        } else {
            self.completed = true;
        }
    }

    /// Go back to the previous step.
    pub fn go_back(&mut self) {
        if let Some(prev) = self.current_step.previous() {
            self.current_step = prev;
        }
    }

    /// Skip the onboarding process.
    pub fn skip(&mut self) {
        self.skipped = true;
        self.completed = true;
    }

    /// Restart the onboarding process.
    pub fn restart(&mut self) {
        *self = Self::default();
    }

    /// Get progress as a percentage (0-100).
    pub fn progress_percent(&self) -> u8 {
        let total = OnboardingStep::all().len();
        let completed = self.completed_steps.len();
        ((completed * 100) / total) as u8
    }
}

/// Onboarding wizard controller.
pub struct OnboardingWizard {
    /// Current state
    state: OnboardingState,
    /// Glossary for tooltips
    glossary: Glossary,
}

impl Default for OnboardingWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardingWizard {
    /// Create a new onboarding wizard.
    pub fn new() -> Self {
        Self {
            state: OnboardingState::default(),
            glossary: Glossary::new(),
        }
    }

    /// Create from existing state.
    pub fn from_state(state: OnboardingState) -> Self {
        Self {
            state,
            glossary: Glossary::new(),
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &OnboardingState {
        &self.state
    }

    /// Get mutable state.
    pub fn state_mut(&mut self) -> &mut OnboardingState {
        &mut self.state
    }

    /// Check if onboarding should be shown.
    pub fn should_show(&self) -> bool {
        !self.state.completed && !self.state.skipped
    }

    /// Get the current step.
    pub fn current_step(&self) -> OnboardingStep {
        self.state.current_step
    }

    /// Advance to the next step.
    pub fn next_step(&mut self) {
        self.state.complete_current_step();
    }

    /// Go back to the previous step.
    pub fn previous_step(&mut self) {
        self.state.go_back();
    }

    /// Skip the onboarding.
    pub fn skip(&mut self) {
        self.state.skip();
    }

    /// Restart the onboarding.
    pub fn restart(&mut self) {
        self.state.restart();
    }

    /// Get the glossary.
    pub fn glossary(&self) -> &Glossary {
        &self.glossary
    }

    /// Get a glossary tooltip for a term.
    pub fn get_tooltip(&self, term: &str) -> Option<&str> {
        self.glossary.get_definition(term)
    }
}

/// Trait for onboarding wizard functionality.
pub trait OnboardingWizardTrait {
    /// Get the current step.
    fn current_step(&self) -> OnboardingStep;

    /// Advance to the next step.
    fn next(&mut self);

    /// Go back to the previous step.
    fn back(&mut self);

    /// Skip the onboarding.
    fn skip(&mut self);

    /// Check if onboarding is complete.
    fn is_complete(&self) -> bool;

    /// Restart the onboarding.
    fn restart(&mut self);
}

impl OnboardingWizardTrait for OnboardingWizard {
    fn current_step(&self) -> OnboardingStep {
        self.current_step()
    }

    fn next(&mut self) {
        self.next_step();
    }

    fn back(&mut self) {
        self.previous_step();
    }

    fn skip(&mut self) {
        self.skip();
    }

    fn is_complete(&self) -> bool {
        self.state.completed
    }

    fn restart(&mut self) {
        self.restart();
    }
}
