//! Weather display widget for showing current conditions.
//!
//! T098: Weather widget with temperature, conditions icon, humidity display

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::integrations::weather::{WeatherCondition, WeatherData, WeatherUnits};

/// Size variants for weather display.
#[derive(Debug, Clone, Copy, Default)]
pub enum WeatherWidgetSize {
    /// Compact display (just temp and icon)
    Compact,
    /// Standard display
    #[default]
    Standard,
    /// Detailed display with wind and pressure
    Detailed,
}

/// A widget for displaying current weather conditions.
pub struct WeatherWidget<'a> {
    /// The weather data to display
    data: &'a WeatherData,
    /// Temperature units
    units: WeatherUnits,
    /// Display size
    size: WeatherWidgetSize,
    /// Show "feels like" temperature
    show_feels_like: bool,
}

impl<'a> WeatherWidget<'a> {
    /// Create a new weather widget.
    pub fn new(data: &'a WeatherData, units: WeatherUnits) -> Self {
        Self {
            data,
            units,
            size: WeatherWidgetSize::default(),
            show_feels_like: true,
        }
    }

    /// Set the display size.
    pub fn with_size(mut self, size: WeatherWidgetSize) -> Self {
        self.size = size;
        self
    }

    /// Set whether to show feels like temperature.
    pub fn with_feels_like(mut self, show: bool) -> Self {
        self.show_feels_like = show;
        self
    }

    /// Get temperature font size based on widget size.
    fn temp_font_size(&self) -> f32 {
        match self.size {
            WeatherWidgetSize::Compact => 20.0,
            WeatherWidgetSize::Standard => 28.0,
            WeatherWidgetSize::Detailed => 32.0,
        }
    }

    /// Get label font size based on widget size.
    fn label_font_size(&self) -> f32 {
        match self.size {
            WeatherWidgetSize::Compact => 10.0,
            WeatherWidgetSize::Standard => 12.0,
            WeatherWidgetSize::Detailed => 14.0,
        }
    }

    /// Get icon font size based on widget size.
    fn icon_font_size(&self) -> f32 {
        match self.size {
            WeatherWidgetSize::Compact => 18.0,
            WeatherWidgetSize::Standard => 24.0,
            WeatherWidgetSize::Detailed => 32.0,
        }
    }

    /// Get color for temperature.
    fn temp_color(&self) -> Color32 {
        let temp = self.data.temperature;
        match self.units {
            WeatherUnits::Metric => {
                if temp < 0.0 {
                    Color32::from_rgb(100, 149, 237) // Icy blue
                } else if temp < 10.0 {
                    Color32::from_rgb(135, 206, 250) // Light blue
                } else if temp < 20.0 {
                    Color32::from_rgb(144, 238, 144) // Light green
                } else if temp < 30.0 {
                    Color32::from_rgb(255, 165, 0) // Orange
                } else {
                    Color32::from_rgb(255, 99, 71) // Red
                }
            }
            WeatherUnits::Imperial => {
                if temp < 32.0 {
                    Color32::from_rgb(100, 149, 237)
                } else if temp < 50.0 {
                    Color32::from_rgb(135, 206, 250)
                } else if temp < 68.0 {
                    Color32::from_rgb(144, 238, 144)
                } else if temp < 86.0 {
                    Color32::from_rgb(255, 165, 0)
                } else {
                    Color32::from_rgb(255, 99, 71)
                }
            }
        }
    }

    /// Get text representation of condition (fallback if emoji doesn't render).
    fn condition_text(&self) -> &'static str {
        match self.data.condition {
            WeatherCondition::Clear => "Clear",
            WeatherCondition::PartlyCloudy => "Partly Cloudy",
            WeatherCondition::Cloudy => "Cloudy",
            WeatherCondition::Overcast => "Overcast",
            WeatherCondition::Fog => "Foggy",
            WeatherCondition::LightRain => "Light Rain",
            WeatherCondition::Rain => "Rain",
            WeatherCondition::HeavyRain => "Heavy Rain",
            WeatherCondition::Thunderstorm => "Storms",
            WeatherCondition::Snow => "Snow",
            WeatherCondition::Sleet => "Sleet",
            WeatherCondition::Hail => "Hail",
            WeatherCondition::Windy => "Windy",
        }
    }

    /// Render the compact weather display.
    fn show_compact(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Condition emoji
            ui.label(RichText::new(self.data.condition.emoji()).size(self.icon_font_size()));

            // Temperature
            let temp_str = self.data.formatted_temperature(self.units);
            ui.label(
                RichText::new(temp_str)
                    .size(self.temp_font_size())
                    .color(self.temp_color())
                    .strong(),
            );
        });
    }

    /// Render the standard weather display.
    fn show_standard(&self, ui: &mut Ui) {
        let min_size = Vec2::new(140.0, 80.0);

        egui::Frame::new().inner_margin(8.0).show(ui, |ui| {
            ui.set_min_size(min_size);

            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                // Temperature and icon row
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(self.data.condition.emoji()).size(self.icon_font_size()),
                    );
                    let temp_str = self.data.formatted_temperature(self.units);
                    ui.label(
                        RichText::new(temp_str)
                            .size(self.temp_font_size())
                            .color(self.temp_color())
                            .strong(),
                    );
                });

                // Condition text
                ui.label(
                    RichText::new(self.condition_text())
                        .size(self.label_font_size())
                        .weak(),
                );

                // Humidity and feels like
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}%", self.data.humidity))
                            .size(self.label_font_size()),
                    );
                    ui.label(
                        RichText::new("humidity")
                            .size(self.label_font_size())
                            .weak(),
                    );
                });

                if self.show_feels_like {
                    let feels = match self.units {
                        WeatherUnits::Metric => format!("Feels {:.0}°C", self.data.feels_like),
                        WeatherUnits::Imperial => format!("Feels {:.0}°F", self.data.feels_like),
                    };
                    ui.label(RichText::new(feels).size(self.label_font_size()).weak());
                }
            });
        });
    }

    /// Render the detailed weather display.
    fn show_detailed(&self, ui: &mut Ui) {
        let min_size = Vec2::new(200.0, 140.0);

        egui::Frame::new()
            .inner_margin(12.0)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_size(min_size);

                ui.vertical(|ui| {
                    // Header with icon and temp
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(self.data.condition.emoji()).size(self.icon_font_size()),
                        );
                        let temp_str = self.data.formatted_temperature(self.units);
                        ui.label(
                            RichText::new(temp_str)
                                .size(self.temp_font_size())
                                .color(self.temp_color())
                                .strong(),
                        );
                    });

                    // Condition and feels like
                    ui.label(RichText::new(&self.data.description).size(self.label_font_size()));

                    if self.show_feels_like {
                        let feels = match self.units {
                            WeatherUnits::Metric => {
                                format!("Feels like {:.0}°C", self.data.feels_like)
                            }
                            WeatherUnits::Imperial => {
                                format!("Feels like {:.0}°F", self.data.feels_like)
                            }
                        };
                        ui.label(RichText::new(feels).size(self.label_font_size()).weak());
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Details grid
                    egui::Grid::new("weather_details")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            // Humidity
                            ui.label(
                                RichText::new("Humidity")
                                    .size(self.label_font_size())
                                    .weak(),
                            );
                            ui.label(
                                RichText::new(format!("{}%", self.data.humidity))
                                    .size(self.label_font_size()),
                            );
                            ui.end_row();

                            // Wind
                            ui.label(RichText::new("Wind").size(self.label_font_size()).weak());
                            let wind_str = format!(
                                "{} {}",
                                self.data.formatted_wind(self.units),
                                self.data.wind_cardinal()
                            );
                            ui.label(RichText::new(wind_str).size(self.label_font_size()));
                            ui.end_row();

                            // Pressure
                            ui.label(
                                RichText::new("Pressure")
                                    .size(self.label_font_size())
                                    .weak(),
                            );
                            ui.label(
                                RichText::new(format!("{} hPa", self.data.pressure))
                                    .size(self.label_font_size()),
                            );
                            ui.end_row();

                            // UV Index (if available)
                            if let Some(uv) = self.data.uv_index {
                                ui.label(
                                    RichText::new("UV Index")
                                        .size(self.label_font_size())
                                        .weak(),
                                );
                                let uv_color = if uv < 3.0 {
                                    Color32::from_rgb(144, 238, 144)
                                } else if uv < 6.0 {
                                    Color32::from_rgb(255, 255, 0)
                                } else if uv < 8.0 {
                                    Color32::from_rgb(255, 165, 0)
                                } else {
                                    Color32::from_rgb(255, 99, 71)
                                };
                                ui.label(
                                    RichText::new(format!("{:.1}", uv))
                                        .size(self.label_font_size())
                                        .color(uv_color),
                                );
                                ui.end_row();
                            }
                        });
                });
            });
    }

    /// Render the weather widget.
    pub fn show(self, ui: &mut Ui) {
        match self.size {
            WeatherWidgetSize::Compact => self.show_compact(ui),
            WeatherWidgetSize::Standard => self.show_standard(ui),
            WeatherWidgetSize::Detailed => self.show_detailed(ui),
        }
    }
}

/// Widget for showing weather is unavailable or loading.
pub struct WeatherPlaceholder {
    message: String,
    size: WeatherWidgetSize,
}

impl WeatherPlaceholder {
    /// Create a placeholder for loading state.
    pub fn loading() -> Self {
        Self {
            message: "Loading weather...".to_string(),
            size: WeatherWidgetSize::Standard,
        }
    }

    /// Create a placeholder for unavailable state.
    pub fn unavailable() -> Self {
        Self {
            message: "Weather unavailable".to_string(),
            size: WeatherWidgetSize::Standard,
        }
    }

    /// Create a placeholder for not configured state.
    pub fn not_configured() -> Self {
        Self {
            message: "Weather not configured".to_string(),
            size: WeatherWidgetSize::Standard,
        }
    }

    /// Set the display size.
    pub fn with_size(mut self, size: WeatherWidgetSize) -> Self {
        self.size = size;
        self
    }

    /// Render the placeholder.
    pub fn show(self, ui: &mut Ui) {
        let font_size = match self.size {
            WeatherWidgetSize::Compact => 12.0,
            WeatherWidgetSize::Standard => 14.0,
            WeatherWidgetSize::Detailed => 16.0,
        };

        ui.label(RichText::new(self.message).size(font_size).weak());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_weather_data() -> WeatherData {
        WeatherData {
            temperature: 22.0,
            feels_like: 24.0,
            humidity: 55,
            condition: WeatherCondition::PartlyCloudy,
            description: "Partly cloudy".to_string(),
            wind_speed: 12.0,
            wind_direction: 180,
            pressure: 1015,
            visibility: 10000,
            uv_index: Some(5.0),
            fetched_at: Utc::now(),
        }
    }

    #[test]
    fn test_weather_widget_creation() {
        let data = sample_weather_data();
        let widget = WeatherWidget::new(&data, WeatherUnits::Metric);
        assert!(widget.show_feels_like);
    }

    #[test]
    fn test_weather_widget_sizes() {
        let data = sample_weather_data();

        let compact =
            WeatherWidget::new(&data, WeatherUnits::Metric).with_size(WeatherWidgetSize::Compact);
        assert_eq!(compact.temp_font_size(), 20.0);

        let standard =
            WeatherWidget::new(&data, WeatherUnits::Metric).with_size(WeatherWidgetSize::Standard);
        assert_eq!(standard.temp_font_size(), 28.0);

        let detailed =
            WeatherWidget::new(&data, WeatherUnits::Metric).with_size(WeatherWidgetSize::Detailed);
        assert_eq!(detailed.temp_font_size(), 32.0);
    }

    #[test]
    fn test_temp_color_metric() {
        let mut data = sample_weather_data();

        // Cold
        data.temperature = -5.0;
        let widget = WeatherWidget::new(&data, WeatherUnits::Metric);
        assert_eq!(widget.temp_color(), Color32::from_rgb(100, 149, 237));

        // Hot
        data.temperature = 35.0;
        let widget = WeatherWidget::new(&data, WeatherUnits::Metric);
        assert_eq!(widget.temp_color(), Color32::from_rgb(255, 99, 71));
    }

    #[test]
    fn test_condition_text() {
        let mut data = sample_weather_data();

        data.condition = WeatherCondition::Clear;
        let widget = WeatherWidget::new(&data, WeatherUnits::Metric);
        assert_eq!(widget.condition_text(), "Clear");

        data.condition = WeatherCondition::Thunderstorm;
        let widget = WeatherWidget::new(&data, WeatherUnits::Metric);
        assert_eq!(widget.condition_text(), "Storms");
    }

    #[test]
    fn test_placeholder_creation() {
        let loading = WeatherPlaceholder::loading();
        assert_eq!(loading.message, "Loading weather...");

        let unavailable = WeatherPlaceholder::unavailable();
        assert_eq!(unavailable.message, "Weather unavailable");

        let not_configured = WeatherPlaceholder::not_configured();
        assert_eq!(not_configured.message, "Weather not configured");
    }
}
