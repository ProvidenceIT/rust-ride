//! Sensor setup screen implementation.
//!
//! T045: Implement sensor discovery list widget
//! T046: Implement sensor pairing confirmation dialog
//! T009-3.4: Add connection quality indicators to sensor setup screen
//! T009-4.4: Add sensor conflict resolution dialog
//! T009-5.4: Add power meter calibration dialog
//! T009-6.4: Add in-app troubleshooting tips

use std::collections::HashMap;

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::sensors::ant::dongle::{AntDongle, DongleStatus};
use crate::sensors::calibration::{
    CalibrationRequest, CalibrationType, is_calibratable_sensor,
};
use crate::sensors::conflict::{DataType, SensorConflict};
use crate::sensors::health::HealthStatus;
use crate::sensors::quality::{QualityLevel, QualityStats};
use crate::sensors::troubleshooting::{
    get_no_sensors_tips, get_poor_signal_tips, get_power_meter_tips,
    get_ant_plus_tips, IssueDetector, TroubleshootingTip, TipPriority,
};
use crate::sensors::types::{ConnectionState, DiscoveredSensor, Protocol, SensorState, SensorType};
use crate::ui::dialogs::calibration::{
    CalibrationDialog, CalibrationDialogAction, CalibrationDialogState,
};
use crate::ui::dialogs::sensor_conflict::{
    ConflictNotificationBanner, ConflictResolutionAction, SensorConflictDialog,
    SensorConflictDialogState,
};
use crate::ui::widgets::connection_quality::ConnectionQualityIndicator;

use super::Screen;

/// Sensor setup screen state.
pub struct SensorSetupScreen {
    /// Whether discovery is active
    pub is_scanning: bool,
    /// List of discovered sensors
    pub discovered_sensors: Vec<DiscoveredSensor>,
    /// List of connected sensors
    pub connected_sensors: Vec<SensorState>,
    /// Selected sensor for pairing dialog
    pub selected_sensor: Option<usize>,
    /// Show pairing confirmation dialog
    pub show_pairing_dialog: bool,
    /// ANT+ enabled status
    pub ant_enabled: bool,
    /// Detected ANT+ dongles
    pub ant_dongles: Vec<AntDongle>,
    /// Show protocol choice dialog (for dual-protocol sensors)
    pub show_protocol_dialog: bool,
    /// Sensor for protocol choice (device_id, ble_sensor, ant_sensor)
    pub protocol_choice_sensor:
        Option<(String, Option<DiscoveredSensor>, Option<DiscoveredSensor>)>,
    /// Connection quality stats per device (device_id -> QualityStats)
    pub quality_stats: HashMap<String, QualityStats>,
    /// State for the sensor conflict resolution dialog
    pub conflict_dialog_state: SensorConflictDialogState,
    /// Active sensor conflicts (data_type -> conflict)
    pub active_conflicts: Vec<SensorConflict>,
    /// Last conflict resolution action (for external handling)
    pub last_conflict_action: Option<ConflictResolutionAction>,
    /// Issue detector for contextual troubleshooting tips
    pub issue_detector: IssueDetector,
    /// Whether to show expanded troubleshooting panel
    pub show_troubleshooting_panel: bool,
    /// Dismissed tip titles (to avoid showing same tip repeatedly)
    dismissed_tips: std::collections::HashSet<String>,
    /// State for the calibration dialog
    pub calibration_dialog_state: CalibrationDialogState,
    /// Last calibration action (for external handling)
    pub last_calibration_action: Option<CalibrationDialogAction>,
    /// Pending calibration requests
    pub pending_calibration_requests: Vec<CalibrationRequest>,
}

impl Default for SensorSetupScreen {
    fn default() -> Self {
        Self {
            is_scanning: false,
            discovered_sensors: Vec::new(),
            connected_sensors: Vec::new(),
            selected_sensor: None,
            show_pairing_dialog: false,
            ant_enabled: false,
            ant_dongles: Vec::new(),
            show_protocol_dialog: false,
            protocol_choice_sensor: None,
            quality_stats: HashMap::new(),
            conflict_dialog_state: SensorConflictDialogState::new(),
            active_conflicts: Vec::new(),
            last_conflict_action: None,
            issue_detector: IssueDetector::new(),
            show_troubleshooting_panel: false,
            dismissed_tips: std::collections::HashSet::new(),
            calibration_dialog_state: CalibrationDialogState::new(),
            last_calibration_action: None,
            pending_calibration_requests: Vec::new(),
        }
    }
}

impl SensorSetupScreen {
    /// Create a new sensor setup screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a newly discovered sensor to the list.
    pub fn add_discovered_sensor(&mut self, sensor: DiscoveredSensor) {
        // Check if sensor already exists (by device_id)
        if !self
            .discovered_sensors
            .iter()
            .any(|s| s.device_id == sensor.device_id)
        {
            self.discovered_sensors.push(sensor);
        }
    }

    /// Update connection state for a sensor.
    pub fn update_connection_state(&mut self, device_id: &str, state: ConnectionState) {
        // If connecting/connected, move from discovered to connected list
        if state == ConnectionState::Connected {
            if let Some(idx) = self
                .discovered_sensors
                .iter()
                .position(|s| s.device_id == device_id)
            {
                let sensor = self.discovered_sensors.remove(idx);
                let sensor_state = SensorState {
                    id: uuid::Uuid::new_v4(),
                    device_id: sensor.device_id,
                    name: sensor.name,
                    sensor_type: sensor.sensor_type,
                    protocol: sensor.protocol,
                    connection_state: state,
                    signal_strength: sensor.signal_strength,
                    battery_level: None,
                    last_data_at: None,
                    is_primary: self.connected_sensors.is_empty(), // First sensor is primary
                };
                self.connected_sensors.push(sensor_state);
            }
        } else if state == ConnectionState::Disconnected {
            // Remove from connected list
            self.connected_sensors.retain(|s| s.device_id != device_id);
        } else {
            // Update state of existing connected sensor
            if let Some(sensor) = self
                .connected_sensors
                .iter_mut()
                .find(|s| s.device_id == device_id)
            {
                sensor.connection_state = state;
            }
        }
    }

    /// Set whether scanning is active.
    pub fn set_scanning(&mut self, scanning: bool) {
        self.is_scanning = scanning;
    }

    /// Update the list of ANT+ dongles.
    pub fn set_ant_dongles(&mut self, dongles: Vec<AntDongle>) {
        self.ant_dongles = dongles;
    }

    /// Set ANT+ enabled state.
    pub fn set_ant_enabled(&mut self, enabled: bool) {
        self.ant_enabled = enabled;
    }

    /// Update connection quality stats for a sensor.
    pub fn update_quality_stats(&mut self, device_id: &str, stats: QualityStats) {
        self.quality_stats.insert(device_id.to_string(), stats);
    }

    /// Update quality stats for multiple sensors.
    pub fn update_all_quality_stats(&mut self, stats: Vec<QualityStats>) {
        for stat in stats {
            self.quality_stats.insert(stat.device_id.clone(), stat);
        }
    }

    /// Clear quality stats for a sensor.
    pub fn clear_quality_stats(&mut self, device_id: &str) {
        self.quality_stats.remove(device_id);
    }

    /// Get quality stats for a sensor.
    pub fn get_quality_stats(&self, device_id: &str) -> Option<&QualityStats> {
        self.quality_stats.get(device_id)
    }

    /// Get sensors with poor quality connections.
    pub fn get_poor_quality_sensors(&self) -> Vec<&SensorState> {
        self.connected_sensors
            .iter()
            .filter(|s| {
                self.quality_stats
                    .get(&s.device_id)
                    .map(|q| q.level == QualityLevel::Poor)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Check if any sensor has poor connection quality.
    pub fn has_poor_quality_sensors(&self) -> bool {
        self.quality_stats.values().any(|q| q.level == QualityLevel::Poor)
    }

    // =========================================================================
    // Conflict Management
    // =========================================================================

    /// Update active conflicts from the conflict detector.
    pub fn update_conflicts(&mut self, conflicts: Vec<SensorConflict>) {
        self.active_conflicts = conflicts;
    }

    /// Add a new conflict or update an existing one.
    pub fn add_conflict(&mut self, conflict: SensorConflict) {
        // Replace if same data type exists
        if let Some(existing) = self.active_conflicts
            .iter_mut()
            .find(|c| c.data_type == conflict.data_type)
        {
            *existing = conflict;
        } else {
            self.active_conflicts.push(conflict);
        }
    }

    /// Remove a conflict by data type.
    pub fn remove_conflict(&mut self, data_type: DataType) {
        self.active_conflicts.retain(|c| c.data_type != data_type);
    }

    /// Get unresolved conflicts.
    pub fn unresolved_conflicts(&self) -> Vec<&SensorConflict> {
        self.active_conflicts
            .iter()
            .filter(|c| !c.is_resolved && c.is_active())
            .collect()
    }

    /// Check if there are any unresolved conflicts.
    pub fn has_unresolved_conflicts(&self) -> bool {
        self.active_conflicts
            .iter()
            .any(|c| !c.is_resolved && c.is_active())
    }

    /// Get the number of unresolved conflicts.
    pub fn unresolved_conflict_count(&self) -> usize {
        self.active_conflicts
            .iter()
            .filter(|c| !c.is_resolved && c.is_active())
            .count()
    }

    /// Open the conflict resolution dialog for a specific data type.
    pub fn open_conflict_dialog(&mut self, data_type: DataType) {
        if let Some(conflict) = self.active_conflicts
            .iter()
            .find(|c| c.data_type == data_type)
            .cloned()
        {
            self.conflict_dialog_state.open(conflict);
        }
    }

    /// Open the conflict resolution dialog for a specific conflict.
    pub fn open_conflict_dialog_for(&mut self, conflict: SensorConflict) {
        self.conflict_dialog_state.open(conflict);
    }

    /// Take the last conflict action (consumes it).
    pub fn take_conflict_action(&mut self) -> Option<ConflictResolutionAction> {
        self.last_conflict_action.take()
    }

    // =========================================================================
    // Troubleshooting Management
    // =========================================================================

    /// Record a signal quality issue for a sensor.
    pub fn record_quality_issue(&mut self, device_id: &str, sensor_name: &str, quality: QualityLevel, rssi: Option<i16>) {
        self.issue_detector.record_quality_issue(device_id, sensor_name, quality, rssi);
    }

    /// Record a connection health issue for a sensor.
    pub fn record_health_issue(&mut self, device_id: &str, sensor_name: &str, status: HealthStatus) {
        self.issue_detector.record_health_issue(device_id, sensor_name, status);
    }

    /// Record a discovery issue for a missing sensor.
    pub fn record_discovery_issue(&mut self, sensor_name: &str, sensor_type: SensorType) {
        self.issue_detector.record_discovery_issue(sensor_name, sensor_type);
    }

    /// Record a battery issue for a sensor.
    pub fn record_battery_issue(&mut self, device_id: &str, sensor_name: &str, level: u8) {
        self.issue_detector.record_battery_issue(device_id, sensor_name, level);
    }

    /// Record that no ANT+ dongle is available.
    pub fn record_ant_dongle_missing(&mut self) {
        self.issue_detector.record_ant_dongle_missing();
    }

    /// Clear all detected issues.
    pub fn clear_issues(&mut self) {
        self.issue_detector.clear();
    }

    /// Check if there are any detected issues.
    pub fn has_issues(&self) -> bool {
        self.issue_detector.has_issues()
    }

    /// Dismiss a troubleshooting tip by title.
    pub fn dismiss_tip(&mut self, title: &str) {
        self.dismissed_tips.insert(title.to_string());
    }

    /// Check if a tip has been dismissed.
    pub fn is_tip_dismissed(&self, title: &str) -> bool {
        self.dismissed_tips.contains(title)
    }

    /// Toggle the troubleshooting panel visibility.
    pub fn toggle_troubleshooting_panel(&mut self) {
        self.show_troubleshooting_panel = !self.show_troubleshooting_panel;
    }

    // =========================================================================
    // Calibration Management
    // =========================================================================

    /// Open the calibration dialog for a sensor.
    pub fn open_calibration_dialog(&mut self, sensor: &SensorState) {
        self.calibration_dialog_state.open(
            sensor.device_id.clone(),
            sensor.name.clone(),
            sensor.protocol,
        );
    }

    /// Open the calibration dialog with calibration type selection.
    pub fn open_calibration_dialog_with_selection(&mut self, sensor: &SensorState) {
        self.calibration_dialog_state.open_with_selection(
            sensor.device_id.clone(),
            sensor.name.clone(),
            sensor.protocol,
        );
    }

    /// Open the calibration dialog for a specific calibration type.
    pub fn open_calibration_dialog_with_type(
        &mut self,
        sensor: &SensorState,
        calibration_type: CalibrationType,
    ) {
        self.calibration_dialog_state.open_with_type(
            sensor.device_id.clone(),
            sensor.name.clone(),
            sensor.protocol,
            calibration_type,
        );
    }

    /// Close the calibration dialog.
    pub fn close_calibration_dialog(&mut self) {
        self.calibration_dialog_state.close();
    }

    /// Check if the calibration dialog is visible.
    pub fn is_calibration_dialog_visible(&self) -> bool {
        self.calibration_dialog_state.visible
    }

    /// Check if a calibration is in progress.
    pub fn is_calibrating(&self) -> bool {
        self.calibration_dialog_state.is_calibrating()
    }

    /// Update the calibration with a successful result.
    pub fn complete_calibration(&mut self, offset_value: Option<i32>) {
        self.calibration_dialog_state.complete(offset_value);
    }

    /// Update the calibration with a failure.
    pub fn fail_calibration(&mut self, error_message: String) {
        self.calibration_dialog_state.fail(error_message);
    }

    /// Advance the calibration to the next step.
    pub fn advance_calibration_step(&mut self) {
        self.calibration_dialog_state.advance_step();
    }

    /// Take the last calibration action (consumes it).
    pub fn take_calibration_action(&mut self) -> Option<CalibrationDialogAction> {
        self.last_calibration_action.take()
    }

    /// Take the next pending calibration request (consumes it).
    pub fn take_pending_calibration_request(&mut self) -> Option<CalibrationRequest> {
        if !self.pending_calibration_requests.is_empty() {
            Some(self.pending_calibration_requests.remove(0))
        } else {
            None
        }
    }

    /// Check if there are pending calibration requests.
    pub fn has_pending_calibration_requests(&self) -> bool {
        !self.pending_calibration_requests.is_empty()
    }

    /// Get connected sensors that can be calibrated.
    pub fn get_calibratable_sensors(&self) -> Vec<&SensorState> {
        self.connected_sensors
            .iter()
            .filter(|s| {
                s.connection_state == ConnectionState::Connected
                    && is_calibratable_sensor(s.sensor_type)
            })
            .collect()
    }

    /// Check if a sensor with the same name exists with a different protocol.
    /// Returns Some((device_id, ble_sensor, ant_sensor)) if dual-protocol detected.
    pub fn find_dual_protocol_sensor(
        &self,
        sensor: &DiscoveredSensor,
    ) -> Option<(String, DiscoveredSensor, DiscoveredSensor)> {
        // Check if there's another sensor with same name but different protocol
        for existing in &self.discovered_sensors {
            if existing.name == sensor.name && existing.device_id != sensor.device_id {
                let is_ble = matches!(
                    sensor.protocol,
                    Protocol::BleFtms
                        | Protocol::BleCyclingPower
                        | Protocol::BleHeartRate
                        | Protocol::BleCsc
                );
                let existing_is_ble = matches!(
                    existing.protocol,
                    Protocol::BleFtms
                        | Protocol::BleCyclingPower
                        | Protocol::BleHeartRate
                        | Protocol::BleCsc
                );

                // One is BLE, one is ANT+
                if is_ble != existing_is_ble {
                    let (ble, ant) = if is_ble {
                        (sensor.clone(), existing.clone())
                    } else {
                        (existing.clone(), sensor.clone())
                    };
                    return Some((sensor.name.clone(), ble, ant));
                }
            }
        }
        None
    }

    /// Show protocol choice dialog for dual-protocol sensors.
    pub fn show_protocol_choice(
        &mut self,
        base_name: String,
        ble: DiscoveredSensor,
        ant: DiscoveredSensor,
    ) {
        self.protocol_choice_sensor = Some((base_name, Some(ble), Some(ant)));
        self.show_protocol_dialog = true;
    }

    /// Render the sensor setup screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    next_screen = Some(Screen::Home);
                }
                ui.heading("Sensor Setup");
            });

            ui.add_space(16.0);

            // Scanning controls and ANT+ status
            ui.horizontal(|ui| {
                if self.is_scanning {
                    if ui.button("Stop Scanning").clicked() {
                        self.is_scanning = false;
                    }
                    ui.spinner();
                    ui.label("Scanning for sensors...");
                } else if ui.button("Start Scanning").clicked() {
                    self.is_scanning = true;
                    // TODO: Trigger actual BLE scan
                }

                ui.separator();

                // ANT+ toggle
                let ant_available = !self.ant_dongles.is_empty();
                ui.add_enabled_ui(ant_available, |ui| {
                    if ui.checkbox(&mut self.ant_enabled, "ANT+").changed() {
                        // TODO: Toggle ANT+ scanning
                    }
                });

                // ANT+ dongle status indicator
                if ant_available {
                    let dongle = &self.ant_dongles[0];
                    let status_text = match &dongle.status {
                        DongleStatus::Detected => "Detected",
                        DongleStatus::Initializing => "Initializing...",
                        DongleStatus::Ready => "Ready",
                        DongleStatus::Error(_e) => "Error",
                        DongleStatus::Disconnected => "Disconnected",
                    };
                    let status_color = match &dongle.status {
                        DongleStatus::Ready => Color32::from_rgb(52, 168, 83),
                        DongleStatus::Initializing | DongleStatus::Detected => {
                            Color32::from_rgb(251, 188, 4)
                        }
                        DongleStatus::Error(_) | DongleStatus::Disconnected => {
                            Color32::from_rgb(234, 67, 53)
                        }
                    };
                    ui.label(
                        RichText::new(format!("📡 {}", status_text))
                            .color(status_color)
                            .small(),
                    );
                } else {
                    ui.label(
                        RichText::new("📡 No ANT+ dongle")
                            .color(Color32::GRAY)
                            .small(),
                    );
                }
            });

            ui.add_space(16.0);
            ui.separator();

            // Two-column layout: Discovered | Connected
            ui.columns(2, |columns| {
                // Left column: Discovered sensors
                columns[0].vertical(|ui| {
                    ui.heading("Discovered Sensors");
                    ui.add_space(8.0);

                    if self.discovered_sensors.is_empty() {
                        if self.is_scanning {
                            ui.label(RichText::new("Searching...").weak());
                        } else {
                            ui.label(RichText::new("No sensors found").weak());
                            ui.label(
                                RichText::new("Start scanning to discover nearby sensors").weak(),
                            );
                        }

                        // T009-6.4: Troubleshooting tips
                        ui.add_space(16.0);
                        self.render_troubleshooting_tips_panel(ui, TroubleshootingContext::NoSensors);
                    } else {
                        // Clone sensors to avoid borrow conflict with mutable self
                        let sensors: Vec<_> = self.discovered_sensors.clone();
                        for (i, sensor) in sensors.iter().enumerate() {
                            self.render_discovered_sensor(ui, sensor, i);
                        }
                    }
                });

                // Right column: Connected sensors
                columns[1].vertical(|ui| {
                    ui.heading("Connected Sensors");
                    ui.add_space(8.0);

                    if self.connected_sensors.is_empty() {
                        ui.label(RichText::new("No sensors connected").weak());
                    } else {
                        // Show conflict notification banner if there are unresolved conflicts
                        let unresolved: Vec<_> = self.active_conflicts
                            .iter()
                            .filter(|c| !c.is_resolved && c.is_active())
                            .cloned()
                            .collect();

                        if !unresolved.is_empty() {
                            if let Some(data_type) = ConflictNotificationBanner::new(&unresolved).show(ui) {
                                // User clicked on a conflict to resolve
                                if let Some(conflict) = unresolved.into_iter().find(|c| c.data_type == data_type) {
                                    self.conflict_dialog_state.open(conflict);
                                }
                            }
                            ui.add_space(8.0);
                        }

                        // Show warning banner if any sensor has poor connection quality
                        if self.has_poor_quality_sensors() {
                            self.render_poor_quality_warning(ui);
                            ui.add_space(8.0);
                        }

                        // Clone sensors to avoid borrow conflict with mutable self
                        let connected: Vec<_> = self.connected_sensors.clone();
                        for sensor in &connected {
                            self.render_connected_sensor(ui, sensor);
                        }
                    }
                });
            });
        });

        // Pairing confirmation dialog
        if self.show_pairing_dialog {
            if let Some(idx) = self.selected_sensor {
                if idx < self.discovered_sensors.len() {
                    // Clone to avoid borrow conflict
                    let sensor = self.discovered_sensors[idx].clone();
                    self.render_pairing_dialog(ui, &sensor);
                }
            }
        }

        // Protocol choice dialog for dual-protocol sensors
        if self.show_protocol_dialog {
            if let Some((name, ble, ant)) = &self.protocol_choice_sensor.clone() {
                self.render_protocol_choice_dialog(ui, name, ble.as_ref(), ant.as_ref());
            }
        }

        // Sensor conflict resolution dialog
        if self.conflict_dialog_state.visible {
            let response = SensorConflictDialog::new(&mut self.conflict_dialog_state).show(ui);
            match response.action {
                ConflictResolutionAction::SelectPrimary { data_type, device_id, remember } => {
                    // Store the action for external handling
                    self.last_conflict_action = Some(ConflictResolutionAction::SelectPrimary {
                        data_type,
                        device_id,
                        remember,
                    });
                }
                ConflictResolutionAction::Cancel => {
                    // Dialog was cancelled, no action needed
                }
                ConflictResolutionAction::None => {
                    // Dialog still open
                }
            }
        }

        // Power meter calibration dialog
        if self.calibration_dialog_state.visible {
            let response = CalibrationDialog::new(&mut self.calibration_dialog_state).show(ui);
            match response.action {
                CalibrationDialogAction::StartCalibration { device_id, calibration_type } => {
                    // Store the action for external handling
                    self.last_calibration_action = Some(CalibrationDialogAction::StartCalibration {
                        device_id: device_id.clone(),
                        calibration_type,
                    });

                    // Create a calibration request for the sensor manager
                    if let Some(process) = self.calibration_dialog_state.current_process() {
                        let request = CalibrationRequest::new(
                            device_id,
                            process.device_name.clone(),
                            process.protocol,
                            calibration_type,
                        );
                        self.pending_calibration_requests.push(request);
                    }
                }
                CalibrationDialogAction::Retry { device_id, calibration_type } => {
                    // Handle retry - similar to start but restarts the process
                    self.last_calibration_action = Some(CalibrationDialogAction::Retry {
                        device_id: device_id.clone(),
                        calibration_type,
                    });

                    // Create a new calibration request for retry
                    if let Some(process) = self.calibration_dialog_state.current_process() {
                        let request = CalibrationRequest::new(
                            device_id,
                            process.device_name.clone(),
                            process.protocol,
                            calibration_type,
                        );
                        self.pending_calibration_requests.push(request);
                    }
                }
                CalibrationDialogAction::Close { record_calibration, notes: _ } => {
                    // Store the action for external handling (to record in CalibrationManager)
                    self.last_calibration_action = Some(CalibrationDialogAction::Close {
                        record_calibration,
                        notes: self.calibration_dialog_state.get_notes(),
                    });
                }
                CalibrationDialogAction::Cancel => {
                    // Dialog was cancelled
                    self.last_calibration_action = Some(CalibrationDialogAction::Cancel);
                }
                CalibrationDialogAction::None => {
                    // Dialog still open
                }
            }
        }

        next_screen
    }

    /// Render a discovered sensor item.
    fn render_discovered_sensor(&mut self, ui: &mut Ui, sensor: &DiscoveredSensor, index: usize) {
        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(12.0)
            .corner_radius(4.0);

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Sensor icon
                let icon = sensor_type_icon(sensor.sensor_type);
                ui.label(RichText::new(icon).size(24.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&sensor.name).strong());
                        // Protocol badge
                        ui.label(protocol_badge(sensor.protocol));
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}", sensor.sensor_type)).weak());
                        if let Some(rssi) = sensor.signal_strength {
                            ui.label(signal_indicator(rssi));
                        }
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Connect").clicked() {
                        self.selected_sensor = Some(index);
                        self.show_pairing_dialog = true;
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    /// Render a connected sensor item.
    fn render_connected_sensor(&mut self, ui: &mut Ui, sensor: &SensorState) {
        let quality_stats = self.quality_stats.get(&sensor.device_id).cloned();
        let is_poor_quality = quality_stats
            .as_ref()
            .map(|q| q.level == QualityLevel::Poor)
            .unwrap_or(false);

        // Check if this sensor can be calibrated
        let can_calibrate = sensor.connection_state == ConnectionState::Connected
            && is_calibratable_sensor(sensor.sensor_type);

        // Use different background color for poor quality connections
        let bg_color = if is_poor_quality {
            Color32::from_rgba_unmultiplied(234, 67, 53, 25) // Red tint for poor connections
        } else {
            ui.visuals().faint_bg_color
        };

        let frame = egui::Frame::new()
            .fill(bg_color)
            .inner_margin(12.0)
            .corner_radius(4.0);

        // Clone sensor info needed for calibration
        let sensor_for_calibration = sensor.clone();

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Sensor icon
                let icon = sensor_type_icon(sensor.sensor_type);
                ui.label(RichText::new(icon).size(24.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&sensor.name).strong());
                        if sensor.is_primary {
                            ui.label(
                                RichText::new("PRIMARY")
                                    .small()
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(connection_status_label(sensor.connection_state));
                        if let Some(battery) = sensor.battery_level {
                            ui.label(battery_indicator(battery));
                        }
                    });

                    // Show quality indicator for connected sensors
                    if sensor.connection_state == ConnectionState::Connected {
                        ui.horizontal(|ui| {
                            if let Some(stats) = &quality_stats {
                                ConnectionQualityIndicator::new()
                                    .with_stats(stats.clone())
                                    .compact()
                                    .show(ui);

                                ui.add_space(4.0);

                                // Show quality level text
                                let quality_color = match stats.level {
                                    QualityLevel::Excellent => Color32::from_rgb(52, 168, 83),
                                    QualityLevel::Good => Color32::from_rgb(102, 187, 106),
                                    QualityLevel::Fair => Color32::from_rgb(251, 188, 4),
                                    QualityLevel::Poor => Color32::from_rgb(234, 67, 53),
                                };
                                ui.label(
                                    RichText::new(format!("{}", stats.level))
                                        .small()
                                        .color(quality_color),
                                );

                                // Show warning icon for poor connections
                                if stats.level == QualityLevel::Poor {
                                    ui.add_space(4.0);
                                    ui.label(RichText::new("⚠").color(Color32::from_rgb(234, 67, 53)));
                                }
                            } else if let Some(rssi) = sensor.signal_strength {
                                // Fallback to RSSI-based signal indicator if no quality stats available
                                ui.label(signal_indicator(rssi));
                            }
                        });
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Disconnect").clicked() {
                        // TODO: Disconnect sensor
                    }

                    // Show calibrate button for power meters and trainers
                    if can_calibrate {
                        ui.add_space(4.0);
                        let calibrate_button = egui::Button::new(
                            RichText::new("⚡ Calibrate").small(),
                        )
                        .fill(Color32::from_rgba_unmultiplied(66, 133, 244, 40));

                        if ui.add(calibrate_button).clicked() {
                            self.calibration_dialog_state.open(
                                sensor_for_calibration.device_id.clone(),
                                sensor_for_calibration.name.clone(),
                                sensor_for_calibration.protocol,
                            );
                        }
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    /// Render a warning banner for poor quality connections.
    fn render_poor_quality_warning(&self, ui: &mut Ui) {
        let poor_sensors = self.get_poor_quality_sensors();
        if poor_sensors.is_empty() {
            return;
        }

        let warning_bg = Color32::from_rgba_unmultiplied(234, 67, 53, 30);
        let warning_border = Color32::from_rgb(234, 67, 53);

        egui::Frame::new()
            .fill(warning_bg)
            .stroke(egui::Stroke::new(1.0, warning_border))
            .inner_margin(10.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(18.0).color(warning_border));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Poor Connection Quality Detected")
                                .strong()
                                .color(warning_border),
                        );
                        let sensor_names: Vec<_> = poor_sensors.iter().map(|s| s.name.as_str()).collect();
                        let names_text = if sensor_names.len() == 1 {
                            format!("{} has a weak signal", sensor_names[0])
                        } else {
                            format!("{} have weak signals", sensor_names.join(", "))
                        };
                        ui.label(RichText::new(names_text).weak());

                        // Show quick troubleshooting tips
                        ui.add_space(4.0);
                        ui.label(RichText::new("Quick fixes:").small().strong());
                        for tip in get_poor_signal_tips().iter().take(3) {
                            ui.label(RichText::new(format!("• {}", tip)).small().weak());
                        }
                    });
                });
            });
    }

    /// Render the troubleshooting tips panel based on context.
    fn render_troubleshooting_tips_panel(&mut self, ui: &mut Ui, context: TroubleshootingContext) {
        let tips = match context {
            TroubleshootingContext::NoSensors => get_no_sensors_tips(),
            TroubleshootingContext::PoorSignal => get_poor_signal_tips(),
            TroubleshootingContext::PowerMeterMissing => get_power_meter_tips(),
            TroubleshootingContext::AntPlus => get_ant_plus_tips(),
        };

        let (title, icon) = match context {
            TroubleshootingContext::NoSensors => ("Troubleshooting Tips", "💡"),
            TroubleshootingContext::PoorSignal => ("Signal Troubleshooting", "📶"),
            TroubleshootingContext::PowerMeterMissing => ("Power Meter Tips", "⚡"),
            TroubleshootingContext::AntPlus => ("ANT+ Troubleshooting", "📡"),
        };

        let bg_color = Color32::from_rgba_unmultiplied(66, 133, 244, 20);
        let border_color = Color32::from_rgb(66, 133, 244);

        egui::Frame::new()
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(12.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(18.0));
                    ui.add_space(4.0);
                    ui.label(RichText::new(title).size(14.0).strong().color(border_color));
                });
                ui.add_space(6.0);

                for tip in tips {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("•").color(Color32::GRAY));
                        ui.add_space(4.0);
                        ui.label(RichText::new(tip).small());
                    });
                }
            });
    }

    /// Render contextual troubleshooting based on detected issues.
    fn render_contextual_troubleshooting(&mut self, ui: &mut Ui) {
        let contextual_tips = self.issue_detector.generate_contextual_tips();
        if contextual_tips.is_empty() {
            return;
        }

        // Filter out dismissed tips
        let active_tips: Vec<_> = contextual_tips
            .into_iter()
            .filter(|t| !self.dismissed_tips.contains(&t.tip.title))
            .collect();

        if active_tips.is_empty() {
            return;
        }

        ui.add_space(8.0);

        // Show the top tip prominently
        if let Some(top_tip) = active_tips.first() {
            self.render_tip_card(ui, &top_tip.tip, &top_tip.context);
        }

        // Show toggle for more tips if there are multiple
        if active_tips.len() > 1 {
            ui.add_space(4.0);
            let toggle_text = if self.show_troubleshooting_panel {
                format!("▼ Hide {} more tips", active_tips.len() - 1)
            } else {
                format!("▶ Show {} more tips", active_tips.len() - 1)
            };

            if ui.small_button(&toggle_text).clicked() {
                self.show_troubleshooting_panel = !self.show_troubleshooting_panel;
            }

            if self.show_troubleshooting_panel {
                for tip in active_tips.iter().skip(1) {
                    self.render_tip_card(ui, &tip.tip, &tip.context);
                }
            }
        }
    }

    /// Render a single tip card.
    fn render_tip_card(&mut self, ui: &mut Ui, tip: &TroubleshootingTip, context: &str) {
        let (bg_color, border_color) = match tip.priority {
            TipPriority::Critical => (
                Color32::from_rgba_unmultiplied(234, 67, 53, 25),
                Color32::from_rgb(234, 67, 53),
            ),
            TipPriority::High => (
                Color32::from_rgba_unmultiplied(251, 188, 4, 25),
                Color32::from_rgb(251, 188, 4),
            ),
            TipPriority::Medium => (
                Color32::from_rgba_unmultiplied(66, 133, 244, 20),
                Color32::from_rgb(66, 133, 244),
            ),
            TipPriority::Low => (
                Color32::from_rgba_unmultiplied(160, 160, 170, 20),
                Color32::from_rgb(160, 160, 170),
            ),
        };

        egui::Frame::new()
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(10.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tip.icon).size(16.0));
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&tip.title).strong().color(border_color));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("✕").on_hover_text("Dismiss").clicked() {
                                    self.dismissed_tips.insert(tip.title.clone());
                                }
                            });
                        });

                        ui.label(RichText::new(context).small().weak());
                        ui.add_space(4.0);

                        // Show first resolution step
                        if let Some(first_step) = tip.resolution.first() {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("→").color(border_color));
                                ui.label(RichText::new(first_step).small());
                            });
                        }
                    });
                });
            });

        ui.add_space(4.0);
    }

    /// Render the pairing confirmation dialog.
    fn render_pairing_dialog(&mut self, ui: &mut Ui, sensor: &DiscoveredSensor) {
        egui::Window::new("Connect Sensor")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(300.0, 150.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    let icon = sensor_type_icon(sensor.sensor_type);
                    ui.label(RichText::new(icon).size(48.0));

                    ui.add_space(8.0);
                    ui.label(RichText::new(&sensor.name).size(18.0).strong());
                    ui.label(format!("{}", sensor.sensor_type));

                    ui.add_space(16.0);
                    ui.label("Connect to this sensor?");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_pairing_dialog = false;
                            self.selected_sensor = None;
                        }

                        ui.add_space(16.0);

                        if ui
                            .add(egui::Button::new("Connect").fill(Color32::from_rgb(66, 133, 244)))
                            .clicked()
                        {
                            // TODO: Actually connect to the sensor
                            self.show_pairing_dialog = false;
                            self.selected_sensor = None;
                        }
                    });
                });
            });
    }

    /// Render protocol choice dialog for dual-protocol sensors.
    fn render_protocol_choice_dialog(
        &mut self,
        ui: &mut Ui,
        name: &str,
        ble: Option<&DiscoveredSensor>,
        ant: Option<&DiscoveredSensor>,
    ) {
        egui::Window::new("Choose Protocol")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(350.0, 200.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label(RichText::new("🔄").size(32.0));
                    ui.add_space(8.0);
                    ui.label(RichText::new(name).size(18.0).strong());
                    ui.add_space(4.0);
                    ui.label("This sensor is available via both BLE and ANT+.");
                    ui.label("Which protocol would you like to use?");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.add_space(20.0);

                        // BLE option
                        ui.vertical(|ui| {
                            ui.set_min_width(120.0);
                            let ble_button =
                                egui::Button::new(RichText::new("BLE (Bluetooth)").size(14.0))
                                    .fill(Color32::from_rgb(0, 122, 255));

                            if ui.add(ble_button).clicked() {
                                if let Some(sensor) = ble {
                                    // Find index of BLE sensor
                                    if let Some(idx) = self
                                        .discovered_sensors
                                        .iter()
                                        .position(|s| s.device_id == sensor.device_id)
                                    {
                                        self.selected_sensor = Some(idx);
                                        self.show_pairing_dialog = true;
                                    }
                                }
                                self.show_protocol_dialog = false;
                                self.protocol_choice_sensor = None;
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Better compatibility").small().weak());
                        });

                        ui.add_space(20.0);

                        // ANT+ option
                        ui.vertical(|ui| {
                            ui.set_min_width(120.0);
                            let ant_button = egui::Button::new(RichText::new("ANT+").size(14.0))
                                .fill(Color32::from_rgb(255, 102, 0));

                            if ui.add(ant_button).clicked() {
                                if let Some(sensor) = ant {
                                    // Find index of ANT+ sensor
                                    if let Some(idx) = self
                                        .discovered_sensors
                                        .iter()
                                        .position(|s| s.device_id == sensor.device_id)
                                    {
                                        self.selected_sensor = Some(idx);
                                        self.show_pairing_dialog = true;
                                    }
                                }
                                self.show_protocol_dialog = false;
                                self.protocol_choice_sensor = None;
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Lower latency").small().weak());
                        });
                    });

                    ui.add_space(16.0);

                    if ui.button("Cancel").clicked() {
                        self.show_protocol_dialog = false;
                        self.protocol_choice_sensor = None;
                    }
                });
            });
    }
}

/// Context for which troubleshooting tips to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroubleshootingContext {
    /// No sensors found during discovery.
    NoSensors,
    /// Poor signal quality detected.
    PoorSignal,
    /// Power meter expected but not found.
    PowerMeterMissing,
    /// ANT+ specific issues.
    AntPlus,
}

/// Get an icon for a sensor type.
fn sensor_type_icon(sensor_type: SensorType) -> &'static str {
    match sensor_type {
        SensorType::Trainer | SensorType::SmartTrainer => "🚴",
        SensorType::PowerMeter => "⚡",
        SensorType::HeartRate => "❤",
        SensorType::Cadence | SensorType::CadenceSensor => "🔄",
        SensorType::Speed => "💨",
        SensorType::SpeedCadence => "📊",
        SensorType::SmO2 => "🩸",
        SensorType::Imu => "📐",
    }
}

/// Get a signal strength indicator.
fn signal_indicator(rssi: i16) -> RichText {
    let (bars, color) = if rssi > -50 {
        ("●●●●", Color32::from_rgb(52, 168, 83)) // Excellent
    } else if rssi > -60 {
        ("●●●○", Color32::from_rgb(52, 168, 83)) // Good
    } else if rssi > -70 {
        ("●●○○", Color32::from_rgb(251, 188, 4)) // Fair
    } else {
        ("●○○○", Color32::from_rgb(234, 67, 53)) // Weak
    };

    RichText::new(bars).color(color).small()
}

/// Get a battery indicator.
fn battery_indicator(level: u8) -> RichText {
    let (icon, color) = if level > 80 {
        ("🔋", Color32::from_rgb(52, 168, 83))
    } else if level > 40 {
        ("🔋", Color32::from_rgb(251, 188, 4))
    } else if level > 20 {
        ("🪫", Color32::from_rgb(255, 128, 0))
    } else {
        ("🪫", Color32::from_rgb(234, 67, 53))
    };

    RichText::new(format!("{} {}%", icon, level))
        .color(color)
        .small()
}

/// Get a connection status label.
fn connection_status_label(state: ConnectionState) -> RichText {
    match state {
        ConnectionState::Connected => {
            RichText::new("● Connected").color(Color32::from_rgb(52, 168, 83))
        }
        ConnectionState::Connecting => {
            RichText::new("◐ Connecting...").color(Color32::from_rgb(251, 188, 4))
        }
        ConnectionState::Reconnecting => {
            RichText::new("◐ Reconnecting...").color(Color32::from_rgb(255, 128, 0))
        }
        ConnectionState::Disconnected => {
            RichText::new("○ Disconnected").color(Color32::from_rgb(160, 160, 170))
        }
    }
}

/// Get a protocol badge for BLE/ANT+.
fn protocol_badge(protocol: Protocol) -> RichText {
    let (text, color) = match protocol {
        Protocol::BleFtms
        | Protocol::BleCyclingPower
        | Protocol::BleHeartRate
        | Protocol::BleCsc => {
            ("BLE", Color32::from_rgb(0, 122, 255)) // Blue for BLE
        }
        Protocol::AntHeartRate
        | Protocol::AntPower
        | Protocol::AntFec
        | Protocol::AntSpeedCadence => {
            ("ANT+", Color32::from_rgb(255, 102, 0)) // Orange for ANT+
        }
    };

    RichText::new(text)
        .small()
        .color(color)
        .background_color(Color32::from_gray(40))
}
