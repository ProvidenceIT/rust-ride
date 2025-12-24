//! Zone indicator widget for power and heart rate zones.
//!
//! T108: Implement zone indicator widget with color band

use egui::{Color32, Pos2, Rect, RichText, Ui, Vec2};

use crate::metrics::zones::{Color, HRZones, PowerZones, HR_ZONE_COLORS, POWER_ZONE_COLORS};

/// Convert internal Color to egui Color32.
fn to_color32(color: &Color) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

/// A zone indicator that shows the current zone with a color band.
pub struct ZoneIndicator;

impl ZoneIndicator {
    /// Render a power zone indicator.
    ///
    /// Shows a horizontal bar with all zone colors and highlights the current zone.
    pub fn power_zone(ui: &mut Ui, current_zone: Option<u8>, zones: Option<&PowerZones>) {
        let available_width = ui.available_width();
        let bar_height = 24.0;
        let zone_count = 7u8;

        // Calculate zone widths (equal distribution)
        let zone_width = available_width / zone_count as f32;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, bar_height + 20.0),
            egui::Sense::hover(),
        );

        let rect = response.rect;
        let bar_rect = Rect::from_min_size(rect.min, Vec2::new(available_width, bar_height));

        // Draw zone color bars
        for zone in 1..=zone_count {
            let zone_x = rect.min.x + (zone - 1) as f32 * zone_width;
            let zone_rect = Rect::from_min_size(
                Pos2::new(zone_x, bar_rect.min.y),
                Vec2::new(zone_width, bar_height),
            );

            let color = to_color32(&POWER_ZONE_COLORS[(zone - 1) as usize]);

            // Dim zones that are not current
            let fill_color = if current_zone == Some(zone) {
                color
            } else {
                color.linear_multiply(0.3)
            };

            painter.rect_filled(zone_rect, 0.0, fill_color);

            // Draw zone number
            let text_pos = zone_rect.center();
            let text_color = if current_zone == Some(zone) {
                Color32::WHITE
            } else {
                Color32::from_gray(100)
            };

            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                format!("Z{}", zone),
                egui::FontId::proportional(12.0),
                text_color,
            );
        }

        // Draw current zone label below bar
        if let Some(zone) = current_zone {
            let zone_name = if let Some(zones) = zones {
                zones.get_zone_range(zone).map(|z| z.name.clone())
            } else {
                Some(default_power_zone_name(zone))
            };

            if let Some(name) = zone_name {
                let label_y = bar_rect.max.y + 4.0;
                let zone_color = to_color32(&POWER_ZONE_COLORS[(zone - 1) as usize]);

                painter.text(
                    Pos2::new(rect.center().x, label_y + 8.0),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(14.0),
                    zone_color,
                );
            }
        }
    }

    /// Render a heart rate zone indicator.
    pub fn hr_zone(ui: &mut Ui, current_zone: Option<u8>, _zones: Option<&HRZones>) {
        let available_width = ui.available_width();
        let bar_height = 24.0;
        let zone_count = 5u8;

        let zone_width = available_width / zone_count as f32;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, bar_height + 20.0),
            egui::Sense::hover(),
        );

        let rect = response.rect;
        let bar_rect = Rect::from_min_size(rect.min, Vec2::new(available_width, bar_height));

        // Draw zone color bars
        for zone in 1..=zone_count {
            let zone_x = rect.min.x + (zone - 1) as f32 * zone_width;
            let zone_rect = Rect::from_min_size(
                Pos2::new(zone_x, bar_rect.min.y),
                Vec2::new(zone_width, bar_height),
            );

            let color = to_color32(&HR_ZONE_COLORS[(zone - 1) as usize]);

            let fill_color = if current_zone == Some(zone) {
                color
            } else {
                color.linear_multiply(0.3)
            };

            painter.rect_filled(zone_rect, 0.0, fill_color);

            // Draw zone number
            let text_pos = zone_rect.center();
            let text_color = if current_zone == Some(zone) {
                Color32::WHITE
            } else {
                Color32::from_gray(100)
            };

            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                format!("Z{}", zone),
                egui::FontId::proportional(12.0),
                text_color,
            );
        }

        // Draw current zone label below bar
        if let Some(zone) = current_zone {
            let zone_name = default_hr_zone_name(zone);
            let label_y = bar_rect.max.y + 4.0;
            let zone_color = if zone > 0 && zone <= 5 {
                to_color32(&HR_ZONE_COLORS[(zone - 1) as usize])
            } else {
                Color32::GRAY
            };

            painter.text(
                Pos2::new(rect.center().x, label_y + 8.0),
                egui::Align2::CENTER_CENTER,
                zone_name,
                egui::FontId::proportional(14.0),
                zone_color,
            );
        }
    }

    /// Render a compact zone badge (just the zone number with color).
    pub fn zone_badge(ui: &mut Ui, label: &str, zone: Option<u8>, is_power: bool) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(12.0).weak());

            if let Some(z) = zone {
                let colors = if is_power {
                    &POWER_ZONE_COLORS[..]
                } else {
                    &HR_ZONE_COLORS[..]
                };
                let max_zone = colors.len();

                if z > 0 && (z as usize) <= max_zone {
                    let color = to_color32(&colors[(z - 1) as usize]);
                    let text = format!("Z{}", z);

                    ui.label(RichText::new(text).color(color).strong().size(14.0));
                } else {
                    ui.label(RichText::new("-").weak().size(14.0));
                }
            } else {
                ui.label(RichText::new("-").weak().size(14.0));
            }
        });
    }
}

/// Get default power zone name.
fn default_power_zone_name(zone: u8) -> String {
    match zone {
        1 => "Active Recovery".to_string(),
        2 => "Endurance".to_string(),
        3 => "Tempo".to_string(),
        4 => "Threshold".to_string(),
        5 => "VO2max".to_string(),
        6 => "Anaerobic".to_string(),
        7 => "Neuromuscular".to_string(),
        _ => format!("Zone {}", zone),
    }
}

/// Get default HR zone name.
fn default_hr_zone_name(zone: u8) -> String {
    match zone {
        1 => "Recovery".to_string(),
        2 => "Aerobic".to_string(),
        3 => "Tempo".to_string(),
        4 => "Threshold".to_string(),
        5 => "Maximum".to_string(),
        _ => format!("Zone {}", zone),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_power_zone_names() {
        assert_eq!(default_power_zone_name(1), "Active Recovery");
        assert_eq!(default_power_zone_name(4), "Threshold");
        assert_eq!(default_power_zone_name(7), "Neuromuscular");
    }

    #[test]
    fn test_default_hr_zone_names() {
        assert_eq!(default_hr_zone_name(1), "Recovery");
        assert_eq!(default_hr_zone_name(5), "Maximum");
    }
}
