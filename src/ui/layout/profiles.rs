//! Layout profile storage and management.
//!
//! Provides CRUD operations for layout profiles with a maximum of 10 profiles.

use super::WidgetType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximum number of layout profiles allowed.
pub const MAX_PROFILES: usize = 10;

/// A widget placement within a layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPlacement {
    /// Widget type
    pub widget_type: WidgetType,
    /// X position (normalized 0-1)
    pub x: f32,
    /// Y position (normalized 0-1)
    pub y: f32,
    /// Width (normalized 0-1)
    pub width: f32,
    /// Height (normalized 0-1)
    pub height: f32,
    /// Whether the widget is visible
    pub visible: bool,
}

impl WidgetPlacement {
    /// Create a new widget placement.
    pub fn new(widget_type: WidgetType, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            widget_type,
            x,
            y,
            width,
            height,
            visible: true,
        }
    }

    /// Get the bounds as an egui Rect (scaled to container size).
    pub fn rect(&self, container_width: f32, container_height: f32) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(self.x * container_width, self.y * container_height),
            egui::vec2(self.width * container_width, self.height * container_height),
        )
    }

    /// Update position from an egui Rect.
    pub fn set_from_rect(&mut self, rect: egui::Rect, container_width: f32, container_height: f32) {
        self.x = rect.min.x / container_width;
        self.y = rect.min.y / container_height;
        self.width = rect.width() / container_width;
        self.height = rect.height() / container_height;
    }

    /// Check if this placement intersects with another.
    pub fn intersects(&self, other: &WidgetPlacement) -> bool {
        !(self.x + self.width <= other.x
            || other.x + other.width <= self.x
            || self.y + self.height <= other.y
            || other.y + other.height <= self.y)
    }
}

/// A layout profile containing widget placements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutProfile {
    /// Unique identifier
    pub id: Uuid,
    /// Profile name
    pub name: String,
    /// Widget placements
    pub widgets: Vec<WidgetPlacement>,
    /// Whether this is the default profile
    pub is_default: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
}

impl LayoutProfile {
    /// Create a new empty profile.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            widgets: Vec::new(),
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create the default layout profile.
    pub fn default_layout() -> Self {
        let mut profile = Self::new("Default");
        profile.is_default = true;

        // Add default widget placements
        profile.widgets = vec![
            // Primary metrics (top row)
            WidgetPlacement::new(WidgetType::Power, 0.0, 0.0, 0.33, 0.2),
            WidgetPlacement::new(WidgetType::HeartRate, 0.33, 0.0, 0.33, 0.2),
            WidgetPlacement::new(WidgetType::Cadence, 0.66, 0.0, 0.34, 0.2),
            // Secondary metrics (second row)
            WidgetPlacement::new(WidgetType::Duration, 0.0, 0.2, 0.25, 0.15),
            WidgetPlacement::new(WidgetType::Distance, 0.25, 0.2, 0.25, 0.15),
            WidgetPlacement::new(WidgetType::Speed, 0.5, 0.2, 0.25, 0.15),
            WidgetPlacement::new(WidgetType::Calories, 0.75, 0.2, 0.25, 0.15),
            // Zone indicator
            WidgetPlacement::new(WidgetType::PowerZone, 0.0, 0.35, 1.0, 0.1),
            // Power graph (bottom)
            WidgetPlacement::new(WidgetType::PowerGraph, 0.0, 0.45, 1.0, 0.55),
        ];

        profile
    }

    /// Add a widget to the profile.
    pub fn add_widget(&mut self, placement: WidgetPlacement) {
        self.widgets.push(placement);
        self.updated_at = Utc::now();
    }

    /// Remove a widget by index.
    pub fn remove_widget(&mut self, index: usize) {
        if index < self.widgets.len() {
            self.widgets.remove(index);
            self.updated_at = Utc::now();
        }
    }

    /// Update a widget placement.
    pub fn update_widget(&mut self, index: usize, placement: WidgetPlacement) {
        if index < self.widgets.len() {
            self.widgets[index] = placement;
            self.updated_at = Utc::now();
        }
    }

    /// Find widgets at a given position.
    pub fn widgets_at_position(&self, x: f32, y: f32) -> Vec<usize> {
        self.widgets
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                w.visible && x >= w.x && x <= w.x + w.width && y >= w.y && y <= w.y + w.height
            })
            .map(|(i, _)| i)
            .collect()
    }
}

impl Default for LayoutProfile {
    fn default() -> Self {
        Self::default_layout()
    }
}

/// Manager for layout profiles.
pub struct LayoutProfileManager {
    /// All profiles
    profiles: Vec<LayoutProfile>,
    /// Currently active profile ID
    active_profile_id: Uuid,
}

impl Default for LayoutProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutProfileManager {
    /// Create a new profile manager with the default profile.
    pub fn new() -> Self {
        let default_profile = LayoutProfile::default_layout();
        let active_id = default_profile.id;

        Self {
            profiles: vec![default_profile],
            active_profile_id: active_id,
        }
    }

    /// Get all profiles.
    pub fn profiles(&self) -> &[LayoutProfile] {
        &self.profiles
    }

    /// Get the active profile.
    pub fn active_profile(&self) -> &LayoutProfile {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_profile_id)
            .unwrap_or_else(|| self.profiles.first().unwrap())
    }

    /// Get the active profile mutably.
    pub fn active_profile_mut(&mut self) -> &mut LayoutProfile {
        let id = self.active_profile_id;
        // Find the index first to avoid borrow conflicts
        let idx = self.profiles.iter().position(|p| p.id == id).unwrap_or(0);
        &mut self.profiles[idx]
    }

    /// Set the active profile by ID.
    pub fn set_active(&mut self, id: Uuid) -> bool {
        if self.profiles.iter().any(|p| p.id == id) {
            self.active_profile_id = id;
            true
        } else {
            false
        }
    }

    /// Create a new profile.
    pub fn create_profile(&mut self, name: impl Into<String>) -> Result<Uuid, ProfileError> {
        if self.profiles.len() >= MAX_PROFILES {
            return Err(ProfileError::MaxProfilesReached);
        }

        let profile = LayoutProfile::new(name);
        let id = profile.id;
        self.profiles.push(profile);
        Ok(id)
    }

    /// Duplicate an existing profile.
    pub fn duplicate_profile(
        &mut self,
        id: Uuid,
        new_name: impl Into<String>,
    ) -> Result<Uuid, ProfileError> {
        if self.profiles.len() >= MAX_PROFILES {
            return Err(ProfileError::MaxProfilesReached);
        }

        let source = self
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or(ProfileError::NotFound)?
            .clone();

        let mut new_profile = LayoutProfile::new(new_name);
        new_profile.widgets = source.widgets;
        new_profile.is_default = false;

        let new_id = new_profile.id;
        self.profiles.push(new_profile);
        Ok(new_id)
    }

    /// Delete a profile by ID.
    pub fn delete_profile(&mut self, id: Uuid) -> Result<(), ProfileError> {
        let profile = self
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or(ProfileError::NotFound)?;

        if profile.is_default {
            return Err(ProfileError::CannotDeleteDefault);
        }

        self.profiles.retain(|p| p.id != id);

        // If we deleted the active profile, switch to default
        if self.active_profile_id == id {
            self.active_profile_id = self.profiles.first().map(|p| p.id).unwrap();
        }

        Ok(())
    }

    /// Rename a profile.
    pub fn rename_profile(
        &mut self,
        id: Uuid,
        new_name: impl Into<String>,
    ) -> Result<(), ProfileError> {
        let profile = self
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(ProfileError::NotFound)?;

        profile.name = new_name.into();
        profile.updated_at = Utc::now();
        Ok(())
    }

    /// Get a profile by ID.
    pub fn get_profile(&self, id: Uuid) -> Option<&LayoutProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Get a profile mutably by ID.
    pub fn get_profile_mut(&mut self, id: Uuid) -> Option<&mut LayoutProfile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Get the number of profiles.
    pub fn count(&self) -> usize {
        self.profiles.len()
    }

    /// Check if more profiles can be created.
    pub fn can_create_profile(&self) -> bool {
        self.profiles.len() < MAX_PROFILES
    }
}

/// Errors that can occur with profile management.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Maximum number of profiles ({}) reached", MAX_PROFILES)]
    MaxProfilesReached,

    #[error("Profile not found")]
    NotFound,

    #[error("Cannot delete the default profile")]
    CannotDeleteDefault,

    #[error("Profile name already exists")]
    NameExists,
}
