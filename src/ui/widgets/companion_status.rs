//! Companion app status indicator widget.
//!
//! T049: Show connected companions indicator in desktop status bar.
//! Displays the number of connected companion apps and provides
//! a list of connected devices with disconnect options.

use egui::{Color32, RichText, Ui, Vec2};
use uuid::Uuid;

use crate::companion::types::CompanionClient;

/// A compact companion status indicator for the status bar.
///
/// Shows the number of connected companion apps with a mobile phone icon.
/// When no companions are connected, shows a subtle indicator.
pub struct CompanionStatusIndicator {
    /// List of connected companion clients.
    clients: Vec<CompanionClient>,
    /// Whether the companion server is running.
    is_running: bool,
}

impl CompanionStatusIndicator {
    /// Create a new companion status indicator.
    pub fn new(clients: Vec<CompanionClient>, is_running: bool) -> Self {
        Self { clients, is_running }
    }

    /// Create an indicator showing server is running with no clients.
    pub fn running_no_clients() -> Self {
        Self {
            clients: Vec::new(),
            is_running: true,
        }
    }

    /// Create an indicator showing server is not running.
    pub fn not_running() -> Self {
        Self {
            clients: Vec::new(),
            is_running: false,
        }
    }

    /// Get the number of connected clients.
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Check if server is running.
    pub fn is_server_running(&self) -> bool {
        self.is_running
    }

    /// Render the companion status indicator (compact version for top bar).
    pub fn show_compact(&self, ui: &mut Ui) {
        if !self.is_running {
            return;
        }

        let count = self.clients.len();
        let (icon, color) = if count > 0 {
            ("📱", Color32::from_rgb(52, 168, 83)) // Green when connected
        } else {
            ("📱", Color32::from_rgb(160, 160, 170)) // Gray when no connections
        };

        let response = ui.horizontal(|ui| {
            ui.label(RichText::new(icon).color(color));
            if count > 0 {
                ui.label(
                    RichText::new(format!("{}", count))
                        .color(color)
                        .small(),
                );
            }
        });

        // Show tooltip with connected clients on hover
        response.response.on_hover_ui(|ui| {
            self.show_tooltip(ui);
        });
    }

    /// Render the tooltip content.
    fn show_tooltip(&self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;

        // Header
        ui.label(RichText::new("Companion Apps").strong());
        ui.separator();

        if self.clients.is_empty() {
            ui.label(RichText::new("No devices connected").weak());
            ui.label(RichText::new("Open RustRide on your phone to connect").weak().small());
        } else {
            ui.label(
                RichText::new(format!("{} device(s) connected", self.clients.len()))
                    .color(Color32::from_rgb(52, 168, 83)),
            );

            ui.add_space(4.0);

            for client in &self.clients {
                ui.horizontal(|ui| {
                    // Connection status icon
                    let status_icon = if client.is_authenticated {
                        "✓"
                    } else {
                        "○"
                    };
                    let status_color = if client.is_authenticated {
                        Color32::from_rgb(52, 168, 83)
                    } else {
                        Color32::from_rgb(251, 188, 4)
                    };
                    ui.label(RichText::new(status_icon).color(status_color).small());

                    // Client IP (truncated if too long)
                    let addr = if client.remote_addr.len() > 21 {
                        format!("{}...", &client.remote_addr[..18])
                    } else {
                        client.remote_addr.clone()
                    };
                    ui.label(RichText::new(addr).small());
                });
            }
        }
    }
}

/// A detailed companion client card for the settings panel.
///
/// Shows client information with a disconnect button.
pub struct CompanionClientCard<'a> {
    /// The companion client to display.
    client: &'a CompanionClient,
}

impl<'a> CompanionClientCard<'a> {
    /// Create a new companion client card.
    pub fn new(client: &'a CompanionClient) -> Self {
        Self { client }
    }

    /// Render the client card and return true if disconnect was clicked.
    pub fn show(&self, ui: &mut Ui) -> bool {
        let mut disconnect_clicked = false;

        let bg_color = if self.client.is_authenticated {
            Color32::from_rgba_unmultiplied(52, 168, 83, 30) // Green tint
        } else {
            Color32::from_rgba_unmultiplied(251, 188, 4, 30) // Yellow tint
        };

        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 16.0);

                ui.horizontal(|ui| {
                    // Icon
                    ui.label(RichText::new("📱").size(24.0));

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        // IP Address
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&self.client.remote_addr).strong());

                            // Authentication status badge
                            if self.client.is_authenticated {
                                ui.label(
                                    RichText::new(" ✓ Authenticated")
                                        .color(Color32::from_rgb(52, 168, 83))
                                        .small(),
                                );
                            } else {
                                ui.label(
                                    RichText::new(" ○ Pending auth")
                                        .color(Color32::from_rgb(251, 188, 4))
                                        .small(),
                                );
                            }
                        });

                        // Connection details
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Connected: {}", self.client.connected_at))
                                    .weak()
                                    .small(),
                            );

                            if self.client.subscribed_to_metrics {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("📊 Streaming metrics")
                                        .color(Color32::from_rgb(66, 133, 244))
                                        .small(),
                                );
                            }
                        });
                    });

                    // Disconnect button on the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Disconnect").color(Color32::from_rgb(234, 67, 53)))
                            .on_hover_text("Disconnect this device")
                            .clicked()
                        {
                            disconnect_clicked = true;
                        }
                    });
                });
            });

        disconnect_clicked
    }
}

/// A list of connected companion clients with disconnect functionality.
pub struct CompanionClientList<'a> {
    /// List of connected companion clients.
    clients: &'a [CompanionClient],
}

/// Response from showing the companion client list.
pub struct CompanionClientListResponse {
    /// Session ID of client to disconnect (if any).
    pub disconnect_session_id: Option<Uuid>,
}

impl<'a> CompanionClientList<'a> {
    /// Create a new companion client list.
    pub fn new(clients: &'a [CompanionClient]) -> Self {
        Self { clients }
    }

    /// Render the client list and return any disconnect requests.
    pub fn show(&self, ui: &mut Ui) -> CompanionClientListResponse {
        let mut disconnect_session_id = None;

        if self.clients.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📱").size(18.0).weak());
                ui.add_space(4.0);
                ui.label(RichText::new("No companion apps connected").weak());
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new("Open the RustRide app on your phone to connect")
                    .weak()
                    .small(),
            );
        } else {
            // Header with count
            ui.horizontal(|ui| {
                ui.label(RichText::new("Connected Devices").strong());
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("({})", self.clients.len()))
                        .color(Color32::from_rgb(52, 168, 83)),
                );
            });

            ui.add_space(8.0);

            // Client cards
            for client in self.clients {
                if CompanionClientCard::new(client).show(ui) {
                    disconnect_session_id = Some(client.session_id);
                }
                ui.add_space(8.0);
            }
        }

        CompanionClientListResponse {
            disconnect_session_id,
        }
    }
}

/// Inline status indicator for top bars (very compact).
///
/// Shows just a mobile icon with count, similar to InlineHudSensorStatus.
pub struct InlineCompanionStatus {
    /// Number of connected clients.
    count: usize,
    /// Whether server is running.
    is_running: bool,
}

impl InlineCompanionStatus {
    /// Create a new inline companion status.
    pub fn new(count: usize, is_running: bool) -> Self {
        Self { count, is_running }
    }

    /// Show the inline status.
    pub fn show(&self, ui: &mut Ui) {
        if !self.is_running {
            return;
        }

        let color = if self.count > 0 {
            Color32::from_rgb(52, 168, 83) // Green
        } else {
            Color32::from_rgb(120, 120, 130) // Muted gray
        };

        ui.horizontal(|ui| {
            let label = if self.count > 0 {
                format!("📱{}", self.count)
            } else {
                "📱".to_string()
            };
            let response = ui.label(RichText::new(&label).color(color).size(12.0));

            // Tooltip
            let tooltip = if self.count > 0 {
                format!("{} companion app(s) connected", self.count)
            } else {
                "Companion server running - no devices connected".to_string()
            };
            response.on_hover_text(tooltip);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_client(addr: &str, authenticated: bool) -> CompanionClient {
        CompanionClient {
            session_id: Uuid::new_v4(),
            connected_at: "2026-01-03T10:00:00Z".to_string(),
            remote_addr: addr.to_string(),
            is_authenticated: authenticated,
            subscribed_to_metrics: false,
        }
    }

    #[test]
    fn test_companion_status_indicator_new() {
        let clients = vec![
            make_test_client("192.168.1.100:54321", true),
            make_test_client("192.168.1.101:54322", false),
        ];
        let indicator = CompanionStatusIndicator::new(clients, true);

        assert_eq!(indicator.client_count(), 2);
        assert!(indicator.is_server_running());
    }

    #[test]
    fn test_companion_status_indicator_running_no_clients() {
        let indicator = CompanionStatusIndicator::running_no_clients();

        assert_eq!(indicator.client_count(), 0);
        assert!(indicator.is_server_running());
    }

    #[test]
    fn test_companion_status_indicator_not_running() {
        let indicator = CompanionStatusIndicator::not_running();

        assert_eq!(indicator.client_count(), 0);
        assert!(!indicator.is_server_running());
    }

    #[test]
    fn test_inline_companion_status_new() {
        let status = InlineCompanionStatus::new(3, true);
        assert_eq!(status.count, 3);
        assert!(status.is_running);
    }

    #[test]
    fn test_companion_client_list_empty() {
        let clients: Vec<CompanionClient> = vec![];
        let list = CompanionClientList::new(&clients);
        // Just verify it can be created with empty list
        assert!(list.clients.is_empty());
    }

    #[test]
    fn test_companion_client_list_with_clients() {
        let clients = vec![
            make_test_client("192.168.1.100:54321", true),
            make_test_client("192.168.1.101:54322", true),
        ];
        let list = CompanionClientList::new(&clients);
        assert_eq!(list.clients.len(), 2);
    }
}
