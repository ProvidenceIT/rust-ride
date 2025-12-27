//! Button Mapping
//!
//! Handles mapping of button presses to actions.

use super::actions::{ActionResult, ButtonAction};
use super::HidError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// A button mapping configuration
#[derive(Debug, Clone)]
pub struct ButtonMapping {
    /// Unique ID for this mapping
    pub id: Uuid,
    /// Device this mapping belongs to
    pub device_id: Uuid,
    /// Button code from the device
    pub button_code: u8,
    /// Action to execute
    pub action: ButtonAction,
    /// Optional label for the button
    pub label: Option<String>,
    /// Whether this mapping is enabled
    pub enabled: bool,
}

impl ButtonMapping {
    /// Create a new button mapping
    pub fn new(device_id: Uuid, button_code: u8, action: ButtonAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            device_id,
            button_code,
            action,
            label: None,
            enabled: true,
        }
    }

    /// Set the label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Raw button event from device
#[derive(Debug, Clone)]
pub struct RawButtonEvent {
    /// Device ID
    pub device_id: Uuid,
    /// Button code
    pub button_code: u8,
    /// Whether button is pressed (true) or released (false)
    pub pressed: bool,
    /// When the event occurred
    pub timestamp: Instant,
}

/// Button action event (after mapping)
#[derive(Debug, Clone)]
pub struct ButtonActionEvent {
    /// Device ID
    pub device_id: Uuid,
    /// Mapping ID
    pub mapping_id: Uuid,
    /// The action to execute
    pub action: ButtonAction,
    /// When the event occurred
    pub timestamp: Instant,
}

/// Trait for button input handling
pub trait ButtonInputHandler: Send + Sync {
    /// Register button mappings for a device
    fn register_mappings(&self, device_id: &Uuid, mappings: Vec<ButtonMapping>);

    /// Get mappings for a device
    fn get_mappings(&self, device_id: &Uuid) -> Vec<ButtonMapping>;

    /// Add a single mapping
    fn add_mapping(&self, device_id: &Uuid, mapping: ButtonMapping);

    /// Remove a mapping
    fn remove_mapping(&self, mapping_id: &Uuid);

    /// Update a mapping's action
    fn update_mapping(&self, mapping_id: &Uuid, new_action: ButtonAction);

    /// Clear all mappings for a device
    fn clear_mappings(&self, device_id: &Uuid);

    /// Subscribe to button press events (after mapping)
    fn subscribe_actions(&self) -> broadcast::Receiver<ButtonActionEvent>;

    /// Subscribe to raw button events (for mapping UI)
    fn subscribe_raw(&self) -> broadcast::Receiver<RawButtonEvent>;

    /// Start learning mode (for mapping new buttons)
    fn start_learning_mode(&self, device_id: &Uuid);

    /// Stop learning mode
    fn stop_learning_mode(&self);

    /// Check if in learning mode
    fn is_learning(&self) -> bool;

    /// Get the last learned button code
    fn get_learned_button(&self) -> Option<u8>;
}

/// Default button input handler implementation
pub struct DefaultButtonInputHandler {
    mappings: Arc<RwLock<HashMap<Uuid, Vec<ButtonMapping>>>>,
    action_tx: broadcast::Sender<ButtonActionEvent>,
    raw_tx: broadcast::Sender<RawButtonEvent>,
    learning_mode: Arc<RwLock<Option<Uuid>>>,
    learned_button: Arc<RwLock<Option<u8>>>,
}

impl Default for DefaultButtonInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultButtonInputHandler {
    /// Create a new button input handler
    pub fn new() -> Self {
        let (action_tx, _) = broadcast::channel(100);
        let (raw_tx, _) = broadcast::channel(100);

        Self {
            mappings: Arc::new(RwLock::new(HashMap::new())),
            action_tx,
            raw_tx,
            learning_mode: Arc::new(RwLock::new(None)),
            learned_button: Arc::new(RwLock::new(None)),
        }
    }

    /// Process a raw button event
    pub async fn process_event(&self, event: RawButtonEvent) {
        // Send raw event
        let _ = self.raw_tx.send(event.clone());

        // Check if in learning mode
        if let Some(_learning_device) = self.learning_mode.read().await.as_ref() {
            if event.pressed {
                *self.learned_button.write().await = Some(event.button_code);
            }
            return;
        }

        // Only process button presses, not releases
        if !event.pressed {
            return;
        }

        // Look up mapping
        let mappings = self.mappings.read().await;
        if let Some(device_mappings) = mappings.get(&event.device_id) {
            if let Some(mapping) = device_mappings
                .iter()
                .find(|m| m.button_code == event.button_code && m.enabled)
            {
                let action_event = ButtonActionEvent {
                    device_id: event.device_id,
                    mapping_id: mapping.id,
                    action: mapping.action.clone(),
                    timestamp: event.timestamp,
                };

                let _ = self.action_tx.send(action_event);
            }
        }
    }
}

impl ButtonInputHandler for DefaultButtonInputHandler {
    fn register_mappings(&self, device_id: &Uuid, mappings: Vec<ButtonMapping>) {
        if let Ok(mut m) = self.mappings.try_write() {
            m.insert(*device_id, mappings);
        }
    }

    fn get_mappings(&self, device_id: &Uuid) -> Vec<ButtonMapping> {
        self.mappings
            .try_read()
            .ok()
            .and_then(|m| m.get(device_id).cloned())
            .unwrap_or_default()
    }

    fn add_mapping(&self, device_id: &Uuid, mapping: ButtonMapping) {
        if let Ok(mut mappings) = self.mappings.try_write() {
            mappings.entry(*device_id).or_default().push(mapping);
        }
    }

    fn remove_mapping(&self, mapping_id: &Uuid) {
        if let Ok(mut mappings) = self.mappings.try_write() {
            for device_mappings in mappings.values_mut() {
                device_mappings.retain(|m| &m.id != mapping_id);
            }
        }
    }

    fn update_mapping(&self, mapping_id: &Uuid, new_action: ButtonAction) {
        if let Ok(mut mappings) = self.mappings.try_write() {
            for device_mappings in mappings.values_mut() {
                if let Some(mapping) = device_mappings.iter_mut().find(|m| &m.id == mapping_id) {
                    mapping.action = new_action;
                    break;
                }
            }
        }
    }

    fn clear_mappings(&self, device_id: &Uuid) {
        if let Ok(mut mappings) = self.mappings.try_write() {
            mappings.remove(device_id);
        }
    }

    fn subscribe_actions(&self) -> broadcast::Receiver<ButtonActionEvent> {
        self.action_tx.subscribe()
    }

    fn subscribe_raw(&self) -> broadcast::Receiver<RawButtonEvent> {
        self.raw_tx.subscribe()
    }

    fn start_learning_mode(&self, device_id: &Uuid) {
        if let Ok(mut learning) = self.learning_mode.try_write() {
            *learning = Some(*device_id);
        }
        if let Ok(mut learned) = self.learned_button.try_write() {
            *learned = None;
        }

        tracing::info!("Started button learning mode for device {:?}", device_id);
    }

    fn stop_learning_mode(&self) {
        if let Ok(mut learning) = self.learning_mode.try_write() {
            *learning = None;
        }

        tracing::info!("Stopped button learning mode");
    }

    fn is_learning(&self) -> bool {
        self.learning_mode
            .try_read()
            .map(|l| l.is_some())
            .unwrap_or(false)
    }

    fn get_learned_button(&self) -> Option<u8> {
        self.learned_button.try_read().ok()?.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_creation() {
        let device_id = Uuid::new_v4();
        let mapping = ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker);

        assert_eq!(mapping.device_id, device_id);
        assert_eq!(mapping.button_code, 1);
        assert!(mapping.enabled);
        assert!(mapping.label.is_none());
    }

    #[test]
    fn test_mapping_with_label() {
        let device_id = Uuid::new_v4();
        let mapping =
            ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker).with_label("Lap Button");

        assert_eq!(mapping.label, Some("Lap Button".to_string()));
    }

    #[test]
    fn test_handler_creation() {
        let handler = DefaultButtonInputHandler::new();
        assert!(!handler.is_learning());
        assert!(handler.get_learned_button().is_none());
    }

    #[test]
    fn test_register_mappings() {
        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        let mappings = vec![
            ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
            ButtonMapping::new(device_id, 2, ButtonAction::PauseResume),
        ];

        handler.register_mappings(&device_id, mappings);

        let retrieved = handler.get_mappings(&device_id);
        assert_eq!(retrieved.len(), 2);
    }
}
