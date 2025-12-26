//! Chat panel widget for group rides.
//!
//! Displays and allows sending chat messages during group rides.

use egui::{Color32, RichText, ScrollArea, Ui};
use uuid::Uuid;

/// Chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender_id: Uuid,
    pub sender_name: String,
    pub message: String,
    pub timestamp: String,
    pub is_local: bool,
}

/// Chat panel actions.
#[derive(Debug, Clone)]
pub enum ChatPanelAction {
    /// Send a message.
    SendMessage(String),
}

/// Chat panel configuration.
#[derive(Debug, Clone)]
pub struct ChatPanelConfig {
    /// Maximum height of the chat panel.
    pub max_height: f32,
    /// Show timestamps.
    pub show_timestamps: bool,
    /// Compact mode.
    pub compact: bool,
}

impl Default for ChatPanelConfig {
    fn default() -> Self {
        Self {
            max_height: 300.0,
            show_timestamps: true,
            compact: false,
        }
    }
}

/// Chat panel widget state.
pub struct ChatPanel {
    /// Configuration.
    config: ChatPanelConfig,
    /// Input message.
    input: String,
    /// Whether panel is expanded.
    expanded: bool,
    /// Scroll to bottom flag.
    scroll_to_bottom: bool,
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatPanel {
    /// Create a new chat panel.
    pub fn new() -> Self {
        Self {
            config: ChatPanelConfig::default(),
            input: String::new(),
            expanded: true,
            scroll_to_bottom: false,
        }
    }

    /// Create with configuration.
    pub fn with_config(config: ChatPanelConfig) -> Self {
        Self {
            config,
            input: String::new(),
            expanded: true,
            scroll_to_bottom: false,
        }
    }

    /// Request scroll to bottom (call after receiving new message).
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to_bottom = true;
    }

    /// Render the chat panel.
    pub fn show(&mut self, ui: &mut Ui, messages: &[ChatMessage]) -> Option<ChatPanelAction> {
        let mut action = None;

        egui::Frame::new()
            .fill(Color32::from_rgb(25, 25, 30))
            .inner_margin(8.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    let header_text = if self.expanded { "Chat ▼" } else { "Chat ▶" };
                    if ui.button(header_text).clicked() {
                        self.expanded = !self.expanded;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} messages", messages.len())).small().weak());
                    });
                });

                if self.expanded {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);

                    // Messages area
                    let scroll_height = self.config.max_height - 60.0;
                    let mut scroll_area = ScrollArea::vertical()
                        .max_height(scroll_height)
                        .auto_shrink([false, false]);

                    if self.scroll_to_bottom {
                        scroll_area = scroll_area.stick_to_bottom(true);
                        self.scroll_to_bottom = false;
                    }

                    scroll_area.show(ui, |ui| {
                        if messages.is_empty() {
                            ui.label(RichText::new("No messages yet").italics().weak());
                        } else {
                            for msg in messages {
                                self.show_message(ui, msg);
                            }
                        }
                    });

                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);

                    // Input area
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.input)
                                .desired_width(ui.available_width() - 60.0)
                                .hint_text("Type a message...")
                        );

                        let send_clicked = ui.button("Send").clicked();
                        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        if (send_clicked || enter_pressed) && !self.input.trim().is_empty() {
                            let message = self.input.trim().to_string();
                            self.input.clear();
                            action = Some(ChatPanelAction::SendMessage(message));
                        }
                    });
                }
            });

        action
    }

    /// Show a single message.
    fn show_message(&self, ui: &mut Ui, msg: &ChatMessage) {
        let _spacing = if self.config.compact { 2.0 } else { 4.0 };

        let bg_color = if msg.is_local {
            Color32::from_rgb(40, 60, 80)
        } else {
            Color32::from_rgb(35, 35, 40)
        };

        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(6.0)
            .outer_margin(2.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Sender name
                    let name_color = if msg.is_local {
                        Color32::from_rgb(150, 200, 255)
                    } else {
                        Color32::from_rgb(200, 200, 200)
                    };
                    ui.label(RichText::new(&msg.sender_name).color(name_color).strong().small());

                    // Timestamp
                    if self.config.show_timestamps {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(&msg.timestamp).small().weak());
                        });
                    }
                });

                // Message text
                ui.label(&msg.message);
            });
    }
}

/// Compact chat overlay for ride screen.
pub struct ChatOverlay {
    /// Maximum messages to show.
    max_messages: usize,
    /// Auto-hide after seconds.
    auto_hide_secs: f32,
    /// Last activity time.
    last_activity: std::time::Instant,
    /// Whether input is focused.
    input_focused: bool,
    /// Input text.
    input: String,
}

impl Default for ChatOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatOverlay {
    /// Create a new chat overlay.
    pub fn new() -> Self {
        Self {
            max_messages: 5,
            auto_hide_secs: 10.0,
            last_activity: std::time::Instant::now(),
            input_focused: false,
            input: String::new(),
        }
    }

    /// Check if overlay should be visible.
    pub fn is_visible(&self) -> bool {
        self.input_focused || self.last_activity.elapsed().as_secs_f32() < self.auto_hide_secs
    }

    /// Mark activity (new message received).
    pub fn mark_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    /// Show the overlay.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        messages: &[ChatMessage],
        position: egui::Pos2,
    ) -> Option<ChatPanelAction> {
        let mut action = None;

        if !self.is_visible() && !self.input_focused {
            // Just show a minimal indicator
            egui::Area::new(egui::Id::new("chat_indicator"))
                .fixed_pos(position)
                .show(ui.ctx(), |ui| {
                    if ui.button("💬").clicked() {
                        self.mark_activity();
                    }
                });
            return None;
        }

        let recent: Vec<_> = messages.iter().rev().take(self.max_messages).collect();

        egui::Area::new(egui::Id::new("chat_overlay"))
            .fixed_pos(position)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(20, 20, 25, 220))
                    .inner_margin(8.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.set_min_width(250.0);
                        ui.set_max_width(300.0);

                        // Recent messages (reversed back to chronological order)
                        for msg in recent.into_iter().rev() {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&msg.sender_name).strong().small());
                                ui.label(RichText::new(&msg.message).small());
                            });
                        }

                        ui.add_space(5.0);

                        // Input
                        ui.horizontal(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.input)
                                    .desired_width(180.0)
                                    .hint_text("Chat...")
                            );

                            self.input_focused = response.has_focus();
                            if response.has_focus() {
                                self.mark_activity();
                            }

                            if ui.button("Send").clicked() && !self.input.trim().is_empty() {
                                action = Some(ChatPanelAction::SendMessage(self.input.trim().to_string()));
                                self.input.clear();
                                self.mark_activity();
                            }
                        });
                    });
            });

        action
    }
}
