//! Ride history screen implementation.
//!
//! T129: Create ride history screen with list view
//! T130: Display ride summary cards (date, duration, distance, TSS)
//! T131: Implement sorting by date
//! T132: Implement pagination

use chrono::{Duration, Local, Utc};
use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui, Vec2};
use uuid::Uuid;

use crate::recording::types::Ride;
use crate::storage::config::Units;

use super::Screen;

/// Date range filter options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateFilter {
    /// Last 7 days
    #[default]
    Week,
    /// Last 30 days
    Month,
    /// Last 90 days
    Quarter,
    /// Last 365 days
    Year,
    /// All time
    AllTime,
}

impl DateFilter {
    /// Get display name.
    pub fn label(&self) -> &'static str {
        match self {
            DateFilter::Week => "Last 7 Days",
            DateFilter::Month => "Last 30 Days",
            DateFilter::Quarter => "Last 90 Days",
            DateFilter::Year => "Last Year",
            DateFilter::AllTime => "All Time",
        }
    }

    /// Get duration for filter.
    pub fn duration(&self) -> Option<Duration> {
        match self {
            DateFilter::Week => Some(Duration::days(7)),
            DateFilter::Month => Some(Duration::days(30)),
            DateFilter::Quarter => Some(Duration::days(90)),
            DateFilter::Year => Some(Duration::days(365)),
            DateFilter::AllTime => None,
        }
    }
}

/// Sort options for ride history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Most recent first
    #[default]
    DateDesc,
    /// Oldest first
    DateAsc,
    /// Longest duration first
    DurationDesc,
    /// Highest TSS first
    TssDesc,
}

impl SortOrder {
    /// Get display name.
    pub fn label(&self) -> &'static str {
        match self {
            SortOrder::DateDesc => "Newest First",
            SortOrder::DateAsc => "Oldest First",
            SortOrder::DurationDesc => "Longest",
            SortOrder::TssDesc => "Highest TSS",
        }
    }
}

/// Ride history screen state.
pub struct RideHistoryScreen {
    /// List of rides
    pub rides: Vec<Ride>,
    /// Selected ride ID (for detail view)
    pub selected_ride: Option<Uuid>,
    /// Current date filter
    pub date_filter: DateFilter,
    /// Current sort order
    pub sort_order: SortOrder,
    /// Current page (0-indexed)
    pub current_page: usize,
    /// Items per page
    pub items_per_page: usize,
    /// Unit preference for display
    pub units: Units,
}

impl Default for RideHistoryScreen {
    fn default() -> Self {
        Self {
            rides: Vec::new(),
            selected_ride: None,
            date_filter: DateFilter::default(),
            sort_order: SortOrder::default(),
            current_page: 0,
            items_per_page: 10,
            units: Units::Metric,
        }
    }
}

impl RideHistoryScreen {
    /// Create a new ride history screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set rides to display.
    pub fn set_rides(&mut self, rides: Vec<Ride>) {
        self.rides = rides;
        self.current_page = 0;
    }

    /// Get filtered and sorted rides.
    fn filtered_rides(&self) -> Vec<&Ride> {
        let cutoff = self.date_filter.duration().map(|d| Utc::now() - d);

        let mut filtered: Vec<_> = self
            .rides
            .iter()
            .filter(|r| match cutoff {
                Some(dt) => r.started_at >= dt,
                None => true,
            })
            .collect();

        // Sort
        match self.sort_order {
            SortOrder::DateDesc => filtered.sort_by(|a, b| b.started_at.cmp(&a.started_at)),
            SortOrder::DateAsc => filtered.sort_by(|a, b| a.started_at.cmp(&b.started_at)),
            SortOrder::DurationDesc => {
                filtered.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds))
            }
            SortOrder::TssDesc => {
                filtered.sort_by(|a, b| b.tss.partial_cmp(&a.tss).unwrap_or(std::cmp::Ordering::Equal))
            }
        }

        filtered
    }

    /// Calculate summary statistics for filtered rides.
    fn calculate_summary(&self) -> (usize, u32, f64, f32) {
        let filtered = self.filtered_rides();
        let count = filtered.len();
        let total_duration: u32 = filtered.iter().map(|r| r.duration_seconds).sum();
        let total_distance: f64 = filtered.iter().map(|r| r.distance_meters).sum();
        let total_tss: f32 = filtered.iter().filter_map(|r| r.tss).sum();

        (count, total_duration, total_distance, total_tss)
    }

    /// Render the ride history screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        // Header
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                next_screen = Some(Screen::Home);
            }
            ui.heading("Ride History");
        });

        ui.separator();

        // Filters bar
        self.render_filters(ui);

        ui.add_space(8.0);

        // Summary stats
        self.render_summary(ui);

        ui.add_space(8.0);
        ui.separator();

        // Ride list
        let filtered = self.filtered_rides();
        let total_pages = (filtered.len() + self.items_per_page - 1) / self.items_per_page;
        let start = self.current_page * self.items_per_page;
        let end = (start + self.items_per_page).min(filtered.len());
        let page_rides: Vec<_> = filtered[start..end].to_vec();

        ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            if page_rides.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(RichText::new("No rides found").size(18.0).weak());
                    ui.add_space(8.0);
                    ui.label(RichText::new("Start recording to see your rides here").weak());
                });
            } else {
                for ride in page_rides {
                    if self.render_ride_card(ui, ride) {
                        self.selected_ride = Some(ride.id);
                        next_screen = Some(Screen::RideDetail);
                    }
                    ui.add_space(4.0);
                }
            }
        });

        // Pagination
        if total_pages > 1 {
            ui.add_space(8.0);
            self.render_pagination(ui, total_pages);
        }

        next_screen
    }

    /// Render the filters bar.
    fn render_filters(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Date filter
            ui.label("Show:");
            egui::ComboBox::from_id_salt("date_filter")
                .selected_text(self.date_filter.label())
                .show_ui(ui, |ui| {
                    for filter in [
                        DateFilter::Week,
                        DateFilter::Month,
                        DateFilter::Quarter,
                        DateFilter::Year,
                        DateFilter::AllTime,
                    ] {
                        if ui
                            .selectable_label(self.date_filter == filter, filter.label())
                            .clicked()
                        {
                            self.date_filter = filter;
                            self.current_page = 0;
                        }
                    }
                });

            ui.separator();

            // Sort order
            ui.label("Sort:");
            egui::ComboBox::from_id_salt("sort_order")
                .selected_text(self.sort_order.label())
                .show_ui(ui, |ui| {
                    for order in [
                        SortOrder::DateDesc,
                        SortOrder::DateAsc,
                        SortOrder::DurationDesc,
                        SortOrder::TssDesc,
                    ] {
                        if ui
                            .selectable_label(self.sort_order == order, order.label())
                            .clicked()
                        {
                            self.sort_order = order;
                        }
                    }
                });
        });
    }

    /// Render summary statistics.
    fn render_summary(&self, ui: &mut Ui) {
        let (count, total_duration, total_distance, total_tss) = self.calculate_summary();

        let hours = total_duration / 3600;
        let minutes = (total_duration % 3600) / 60;

        let (distance_val, distance_unit) = match self.units {
            Units::Metric => (total_distance / 1000.0, "km"),
            Units::Imperial => (total_distance / 1000.0 * 0.621371, "mi"),
        };

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(count.to_string()).size(24.0).strong());
                    ui.label(RichText::new("Rides").weak());
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("{}h {}m", hours, minutes)).size(24.0).strong());
                    ui.label(RichText::new("Total Time").weak());
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("{:.0} {}", distance_val, distance_unit)).size(24.0).strong());
                    ui.label(RichText::new("Distance").weak());
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("{:.0}", total_tss)).size(24.0).strong());
                    ui.label(RichText::new("Total TSS").weak());
                });
            });
        });
    }

    /// Render a single ride card. Returns true if clicked.
    fn render_ride_card(&self, ui: &mut Ui, ride: &Ride) -> bool {
        let mut clicked = false;

        let response = ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                // Date and time
                ui.vertical(|ui| {
                    let local = ride.started_at.with_timezone(&Local);
                    ui.label(RichText::new(local.format("%b %d, %Y").to_string()).size(14.0).strong());
                    ui.label(RichText::new(local.format("%H:%M").to_string()).weak());
                });

                ui.add_space(16.0);

                // Duration
                ui.vertical(|ui| {
                    let hours = ride.duration_seconds / 3600;
                    let minutes = (ride.duration_seconds % 3600) / 60;
                    let duration_str = if hours > 0 {
                        format!("{}h {}m", hours, minutes)
                    } else {
                        format!("{}m", minutes)
                    };
                    ui.label(RichText::new(duration_str).size(16.0));
                    ui.label(RichText::new("Duration").size(11.0).weak());
                });

                ui.add_space(16.0);

                // Distance
                ui.vertical(|ui| {
                    let (dist, unit) = match self.units {
                        Units::Metric => (ride.distance_meters / 1000.0, "km"),
                        Units::Imperial => (ride.distance_meters / 1000.0 * 0.621371, "mi"),
                    };
                    ui.label(RichText::new(format!("{:.1} {}", dist, unit)).size(16.0));
                    ui.label(RichText::new("Distance").size(11.0).weak());
                });

                ui.add_space(16.0);

                // Power
                if let Some(power) = ride.avg_power {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{}W", power)).size(16.0));
                        ui.label(RichText::new("Avg Power").size(11.0).weak());
                    });
                    ui.add_space(16.0);
                }

                // TSS
                if let Some(tss) = ride.tss {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{:.0}", tss)).size(16.0));
                        ui.label(RichText::new("TSS").size(11.0).weak());
                    });
                }

                // Workout indicator
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("→").weak());

                    if ride.workout_id.is_some() {
                        ui.label(
                            RichText::new("Workout")
                                .size(11.0)
                                .color(Color32::from_rgb(66, 133, 244)),
                        );
                    }
                });
            });
        });

        if response.response.interact(egui::Sense::click()).clicked() {
            clicked = true;
        }

        clicked
    }

    /// Render pagination controls.
    fn render_pagination(&mut self, ui: &mut Ui, total_pages: usize) {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                if ui.add_enabled(self.current_page > 0, egui::Button::new("← Previous")).clicked() {
                    self.current_page = self.current_page.saturating_sub(1);
                }

                ui.label(format!("Page {} of {}", self.current_page + 1, total_pages));

                if ui
                    .add_enabled(self.current_page < total_pages - 1, egui::Button::new("Next →"))
                    .clicked()
                {
                    self.current_page += 1;
                }
            });
        });
    }
}
