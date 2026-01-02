//! Ride history screen implementation.
//!
//! T129: Create ride history screen with list view
//! T130: Display ride summary cards (date, duration, distance, TSS)
//! T131: Implement sorting by date
//! T132: Implement pagination
//! 3.4: Add retry button for failed syncs

use chrono::{Duration, Local, Utc};
use egui::{Align, Color32, CursorIcon, Layout, RichText, ScrollArea, Ui, Vec2};
use std::collections::HashMap;
use uuid::Uuid;

use crate::recording::types::Ride;
use crate::storage::config::Units;

use super::Screen;

/// Actions that can result from the ride history screen.
#[derive(Debug, Clone, PartialEq)]
pub enum RideHistoryAction {
    /// No action
    None,
    /// Navigate to a different screen
    Navigate(Screen),
    /// Retry sync for a specific ride
    RetrySync {
        /// The ride ID to retry sync for
        ride_id: Uuid,
        /// Platform to retry (e.g., "Strava")
        platform: String,
    },
}

/// Strava brand color for sync badges
const STRAVA_ORANGE: Color32 = Color32::from_rgb(252, 82, 0);

/// Sync status for a ride (simplified view for UI)
#[derive(Debug, Clone, PartialEq)]
pub enum RideSyncStatus {
    /// No sync attempted for this ride
    NotSynced,
    /// Upload is pending or in progress
    Pending {
        /// Platform name (e.g., "Strava")
        platform: String,
    },
    /// Upload completed successfully
    Synced {
        /// External activity ID (e.g., Strava activity ID)
        activity_id: String,
        /// Platform name (e.g., "Strava")
        platform: String,
    },
    /// Upload failed
    Failed {
        /// Error message
        error: String,
        /// Number of retry attempts
        retry_count: i32,
        /// Platform name (e.g., "Strava")
        platform: String,
    },
    /// Retry in progress
    Retrying {
        /// Platform name (e.g., "Strava")
        platform: String,
    },
}

impl RideSyncStatus {
    /// Get the display label for this status
    pub fn label(&self) -> &str {
        match self {
            RideSyncStatus::NotSynced => "",
            RideSyncStatus::Pending { .. } => "Syncing...",
            RideSyncStatus::Synced { .. } => "Synced",
            RideSyncStatus::Failed { .. } => "Failed",
            RideSyncStatus::Retrying { .. } => "Retrying...",
        }
    }

    /// Get the display color for this status
    pub fn color(&self) -> Color32 {
        match self {
            RideSyncStatus::NotSynced => Color32::TRANSPARENT,
            RideSyncStatus::Pending { .. } => Color32::from_rgb(251, 188, 4), // Yellow/amber
            RideSyncStatus::Synced { .. } => Color32::from_rgb(52, 168, 83), // Green
            RideSyncStatus::Failed { .. } => Color32::from_rgb(234, 67, 53), // Red
            RideSyncStatus::Retrying { .. } => Color32::from_rgb(251, 188, 4), // Yellow/amber
        }
    }

    /// Get the external activity URL (for Strava activities)
    pub fn activity_url(&self) -> Option<String> {
        match self {
            RideSyncStatus::Synced {
                activity_id,
                platform,
            } => {
                if platform.to_lowercase() == "strava" {
                    Some(format!("https://www.strava.com/activities/{}", activity_id))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get the platform name for this status (if applicable)
    pub fn platform(&self) -> Option<&str> {
        match self {
            RideSyncStatus::NotSynced => None,
            RideSyncStatus::Pending { platform } => Some(platform),
            RideSyncStatus::Synced { platform, .. } => Some(platform),
            RideSyncStatus::Failed { platform, .. } => Some(platform),
            RideSyncStatus::Retrying { platform } => Some(platform),
        }
    }
}

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

/// Result from rendering a ride card.
#[derive(Debug, Clone)]
enum RideCardResult {
    /// No interaction
    None,
    /// Card was clicked (navigate to detail)
    Clicked,
    /// Retry sync was requested
    RetrySync {
        /// Platform to retry
        platform: String,
    },
}

/// Result from rendering a sync badge.
#[derive(Debug, Clone)]
enum SyncBadgeResult {
    /// No interaction
    None,
    /// Retry was clicked
    Retry {
        /// Platform to retry
        platform: String,
    },
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
    /// Sync status for each ride (keyed by ride ID)
    pub sync_status: HashMap<Uuid, RideSyncStatus>,
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
            sync_status: HashMap::new(),
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

    /// Set sync status for a specific ride.
    pub fn set_sync_status(&mut self, ride_id: Uuid, status: RideSyncStatus) {
        self.sync_status.insert(ride_id, status);
    }

    /// Set sync status for multiple rides at once.
    pub fn set_sync_statuses(&mut self, statuses: HashMap<Uuid, RideSyncStatus>) {
        self.sync_status = statuses;
    }

    /// Get sync status for a specific ride.
    pub fn get_sync_status(&self, ride_id: &Uuid) -> &RideSyncStatus {
        self.sync_status
            .get(ride_id)
            .unwrap_or(&RideSyncStatus::NotSynced)
    }

    /// Clear all sync status.
    pub fn clear_sync_status(&mut self) {
        self.sync_status.clear();
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
            SortOrder::TssDesc => filtered.sort_by(|a, b| {
                b.tss
                    .partial_cmp(&a.tss)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
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
    pub fn show(&mut self, ui: &mut Ui) -> RideHistoryAction {
        let mut action = RideHistoryAction::None;

        // Header
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                action = RideHistoryAction::Navigate(Screen::Home);
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
        let total_pages = filtered.len().div_ceil(self.items_per_page);
        let start = self.current_page * self.items_per_page;
        let end = (start + self.items_per_page).min(filtered.len());
        // Clone the rides to avoid borrow conflicts
        let page_rides: Vec<Ride> = filtered[start..end].iter().map(|&r| r.clone()).collect();
        drop(filtered); // Explicitly drop the borrowed references

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
                for ride in &page_rides {
                    let card_result = self.render_ride_card(ui, ride);
                    match card_result {
                        RideCardResult::Clicked => {
                            self.selected_ride = Some(ride.id);
                            action = RideHistoryAction::Navigate(Screen::RideDetail);
                        }
                        RideCardResult::RetrySync { platform } => {
                            action = RideHistoryAction::RetrySync {
                                ride_id: ride.id,
                                platform,
                            };
                        }
                        RideCardResult::None => {}
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

        action
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
                    ui.label(
                        RichText::new(format!("{}h {}m", hours, minutes))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label(RichText::new("Total Time").weak());
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.0} {}", distance_val, distance_unit))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label(RichText::new("Distance").weak());
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.0}", total_tss))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label(RichText::new("Total TSS").weak());
                });
            });
        });
    }

    /// Render a single ride card. Returns the result of user interaction.
    fn render_ride_card(&self, ui: &mut Ui, ride: &Ride) -> RideCardResult {
        let mut result = RideCardResult::None;
        let mut badge_result = SyncBadgeResult::None;

        let response = ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                // Date and time
                ui.vertical(|ui| {
                    let local = ride.started_at.with_timezone(&Local);
                    ui.label(
                        RichText::new(local.format("%b %d, %Y").to_string())
                            .size(14.0)
                            .strong(),
                    );
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

                // Workout indicator and sync status
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("→").weak());

                    if ride.workout_id.is_some() {
                        ui.label(
                            RichText::new("Workout")
                                .size(11.0)
                                .color(Color32::from_rgb(66, 133, 244)),
                        );
                        ui.add_space(8.0);
                    }

                    // Sync status badge
                    let sync_status = self.get_sync_status(&ride.id);
                    if !matches!(sync_status, RideSyncStatus::NotSynced) {
                        badge_result = self.render_sync_badge(ui, sync_status);
                    }
                });
            });
        });

        // Check for retry action first (takes priority)
        if let SyncBadgeResult::Retry { platform } = badge_result {
            return RideCardResult::RetrySync { platform };
        }

        // Otherwise check for card click
        if response.response.interact(egui::Sense::click()).clicked() {
            result = RideCardResult::Clicked;
        }

        result
    }

    /// Render a sync status badge for a ride. Returns action if retry button clicked.
    fn render_sync_badge(&self, ui: &mut Ui, status: &RideSyncStatus) -> SyncBadgeResult {
        match status {
            RideSyncStatus::NotSynced => SyncBadgeResult::None,
            RideSyncStatus::Pending { platform } => {
                // Pending/syncing badge with spinner-like indicator
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⟳").color(status.color()).size(12.0));
                    ui.label(
                        RichText::new(status.label())
                            .size(11.0)
                            .color(status.color()),
                    );
                });
                // Show platform on hover
                ui.ctx()
                    .clone()
                    .on_hover_text(format!("Uploading to {}", platform));
                SyncBadgeResult::None
            }
            RideSyncStatus::Synced {
                activity_id,
                platform,
            } => {
                // Synced badge with clickable Strava link
                let response = ui
                    .horizontal(|ui| {
                        // Strava icon (S for now, but could be actual icon)
                        if platform.to_lowercase() == "strava" {
                            ui.label(RichText::new("S").color(STRAVA_ORANGE).size(12.0).strong());
                        } else {
                            ui.label(RichText::new("✓").color(status.color()).size(12.0));
                        }
                        ui.label(
                            RichText::new(status.label())
                                .size(11.0)
                                .color(status.color()),
                        );
                    })
                    .response;

                // Make clickable if it's a Strava activity
                if let Some(url) = status.activity_url() {
                    let response = response.interact(egui::Sense::click());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if response.on_hover_text(format!("Open {} activity", platform)).clicked() {
                        if let Err(e) = open::that(&url) {
                            tracing::warn!("Failed to open Strava activity: {}", e);
                        }
                    }
                } else {
                    response.on_hover_text(format!("Activity ID: {}", activity_id));
                }
                SyncBadgeResult::None
            }
            RideSyncStatus::Failed {
                error,
                retry_count,
                platform,
            } => {
                let mut result = SyncBadgeResult::None;

                // Failed badge with error tooltip and retry button
                ui.horizontal(|ui| {
                    ui.label(RichText::new("!").color(status.color()).size(12.0).strong());
                    ui.label(
                        RichText::new(status.label())
                            .size(11.0)
                            .color(status.color()),
                    );

                    ui.add_space(4.0);

                    // Retry button
                    let retry_button = egui::Button::new(
                        RichText::new("↻ Retry").size(10.0).color(Color32::WHITE),
                    )
                    .fill(STRAVA_ORANGE)
                    .min_size(Vec2::new(50.0, 18.0));

                    // Show retry count and error on hover
                    let hover_text = if *retry_count > 0 {
                        format!(
                            "Error: {}\nRetries: {}\nClick to retry upload to {}",
                            error, retry_count, platform
                        )
                    } else {
                        format!("Error: {}\nClick to retry upload to {}", error, platform)
                    };

                    if ui.add(retry_button).on_hover_text(hover_text).clicked() {
                        result = SyncBadgeResult::Retry {
                            platform: platform.clone(),
                        };
                    }
                });

                result
            }
            RideSyncStatus::Retrying { platform } => {
                // Retrying badge with spinner
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(status.label())
                            .size(11.0)
                            .color(status.color()),
                    );
                });
                // Show platform on hover
                ui.ctx()
                    .clone()
                    .on_hover_text(format!("Retrying upload to {}", platform));
                SyncBadgeResult::None
            }
        }
    }

    /// Render pagination controls.
    fn render_pagination(&mut self, ui: &mut Ui, total_pages: usize) {
        ui.horizontal(|ui| {
            ui.with_layout(
                Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    if ui
                        .add_enabled(self.current_page > 0, egui::Button::new("← Previous"))
                        .clicked()
                    {
                        self.current_page = self.current_page.saturating_sub(1);
                    }

                    ui.label(format!("Page {} of {}", self.current_page + 1, total_pages));

                    if ui
                        .add_enabled(
                            self.current_page < total_pages - 1,
                            egui::Button::new("Next →"),
                        )
                        .clicked()
                    {
                        self.current_page += 1;
                    }
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ride_sync_status_label() {
        assert_eq!(RideSyncStatus::NotSynced.label(), "");
        assert_eq!(
            RideSyncStatus::Pending {
                platform: "Strava".to_string()
            }
            .label(),
            "Syncing..."
        );
        assert_eq!(
            RideSyncStatus::Synced {
                activity_id: "123".to_string(),
                platform: "Strava".to_string(),
            }
            .label(),
            "Synced"
        );
        assert_eq!(
            RideSyncStatus::Failed {
                error: "Network error".to_string(),
                retry_count: 1,
                platform: "Strava".to_string(),
            }
            .label(),
            "Failed"
        );
        assert_eq!(
            RideSyncStatus::Retrying {
                platform: "Strava".to_string()
            }
            .label(),
            "Retrying..."
        );
    }

    #[test]
    fn test_ride_sync_status_color() {
        // NotSynced should be transparent
        assert_eq!(RideSyncStatus::NotSynced.color(), Color32::TRANSPARENT);

        // Pending should be amber/yellow
        let pending_color = RideSyncStatus::Pending {
            platform: "Strava".to_string(),
        }
        .color();
        assert_eq!(pending_color, Color32::from_rgb(251, 188, 4));

        // Synced should be green
        let synced_color = RideSyncStatus::Synced {
            activity_id: "123".to_string(),
            platform: "Strava".to_string(),
        }
        .color();
        assert_eq!(synced_color, Color32::from_rgb(52, 168, 83));

        // Failed should be red
        let failed_color = RideSyncStatus::Failed {
            error: "Error".to_string(),
            retry_count: 0,
            platform: "Strava".to_string(),
        }
        .color();
        assert_eq!(failed_color, Color32::from_rgb(234, 67, 53));

        // Retrying should be amber/yellow
        let retrying_color = RideSyncStatus::Retrying {
            platform: "Strava".to_string(),
        }
        .color();
        assert_eq!(retrying_color, Color32::from_rgb(251, 188, 4));
    }

    #[test]
    fn test_ride_sync_status_activity_url() {
        // NotSynced has no URL
        assert!(RideSyncStatus::NotSynced.activity_url().is_none());

        // Pending has no URL
        assert!(RideSyncStatus::Pending {
            platform: "Strava".to_string()
        }
        .activity_url()
        .is_none());

        // Synced with Strava has URL
        let strava_synced = RideSyncStatus::Synced {
            activity_id: "12345".to_string(),
            platform: "Strava".to_string(),
        };
        assert_eq!(
            strava_synced.activity_url(),
            Some("https://www.strava.com/activities/12345".to_string())
        );

        // Synced with case-insensitive Strava has URL
        let strava_synced_lower = RideSyncStatus::Synced {
            activity_id: "67890".to_string(),
            platform: "strava".to_string(),
        };
        assert_eq!(
            strava_synced_lower.activity_url(),
            Some("https://www.strava.com/activities/67890".to_string())
        );

        // Synced with other platform has no URL (for now)
        let other_synced = RideSyncStatus::Synced {
            activity_id: "abc123".to_string(),
            platform: "TrainingPeaks".to_string(),
        };
        assert!(other_synced.activity_url().is_none());

        // Failed has no URL
        let failed = RideSyncStatus::Failed {
            error: "Error".to_string(),
            retry_count: 0,
            platform: "Strava".to_string(),
        };
        assert!(failed.activity_url().is_none());
    }

    #[test]
    fn test_ride_sync_status_platform() {
        // NotSynced has no platform
        assert!(RideSyncStatus::NotSynced.platform().is_none());

        // Pending has platform
        assert_eq!(
            RideSyncStatus::Pending {
                platform: "Strava".to_string()
            }
            .platform(),
            Some("Strava")
        );

        // Synced has platform
        assert_eq!(
            RideSyncStatus::Synced {
                activity_id: "123".to_string(),
                platform: "Strava".to_string()
            }
            .platform(),
            Some("Strava")
        );

        // Failed has platform
        assert_eq!(
            RideSyncStatus::Failed {
                error: "Error".to_string(),
                retry_count: 0,
                platform: "Strava".to_string()
            }
            .platform(),
            Some("Strava")
        );

        // Retrying has platform
        assert_eq!(
            RideSyncStatus::Retrying {
                platform: "Strava".to_string()
            }
            .platform(),
            Some("Strava")
        );
    }

    #[test]
    fn test_ride_history_screen_sync_status() {
        let mut screen = RideHistoryScreen::new();
        let ride_id = Uuid::new_v4();

        // Initial state - not synced
        assert_eq!(screen.get_sync_status(&ride_id), &RideSyncStatus::NotSynced);

        // Set pending status
        screen.set_sync_status(
            ride_id,
            RideSyncStatus::Pending {
                platform: "Strava".to_string(),
            },
        );
        assert!(matches!(
            screen.get_sync_status(&ride_id),
            RideSyncStatus::Pending { .. }
        ));

        // Set synced status
        let synced = RideSyncStatus::Synced {
            activity_id: "12345".to_string(),
            platform: "Strava".to_string(),
        };
        screen.set_sync_status(ride_id, synced.clone());
        assert_eq!(screen.get_sync_status(&ride_id), &synced);
    }

    #[test]
    fn test_ride_history_screen_set_sync_statuses() {
        let mut screen = RideHistoryScreen::new();
        let ride_id_1 = Uuid::new_v4();
        let ride_id_2 = Uuid::new_v4();

        let mut statuses = HashMap::new();
        statuses.insert(
            ride_id_1,
            RideSyncStatus::Pending {
                platform: "Strava".to_string(),
            },
        );
        statuses.insert(
            ride_id_2,
            RideSyncStatus::Synced {
                activity_id: "123".to_string(),
                platform: "Strava".to_string(),
            },
        );

        screen.set_sync_statuses(statuses);

        assert!(matches!(
            screen.get_sync_status(&ride_id_1),
            RideSyncStatus::Pending { .. }
        ));
        assert!(matches!(
            screen.get_sync_status(&ride_id_2),
            RideSyncStatus::Synced { .. }
        ));
    }

    #[test]
    fn test_ride_history_screen_clear_sync_status() {
        let mut screen = RideHistoryScreen::new();
        let ride_id = Uuid::new_v4();

        screen.set_sync_status(
            ride_id,
            RideSyncStatus::Pending {
                platform: "Strava".to_string(),
            },
        );
        assert!(matches!(
            screen.get_sync_status(&ride_id),
            RideSyncStatus::Pending { .. }
        ));

        screen.clear_sync_status();
        assert_eq!(screen.get_sync_status(&ride_id), &RideSyncStatus::NotSynced);
    }

    #[test]
    fn test_ride_sync_status_equality() {
        assert_eq!(RideSyncStatus::NotSynced, RideSyncStatus::NotSynced);
        assert_eq!(
            RideSyncStatus::Pending {
                platform: "Strava".to_string()
            },
            RideSyncStatus::Pending {
                platform: "Strava".to_string()
            }
        );
        assert_eq!(
            RideSyncStatus::Synced {
                activity_id: "123".to_string(),
                platform: "Strava".to_string(),
            },
            RideSyncStatus::Synced {
                activity_id: "123".to_string(),
                platform: "Strava".to_string(),
            }
        );
        assert_ne!(
            RideSyncStatus::NotSynced,
            RideSyncStatus::Pending {
                platform: "Strava".to_string()
            }
        );
    }

    #[test]
    fn test_ride_history_action_variants() {
        // Test action variants exist and can be created
        let none = RideHistoryAction::None;
        assert!(matches!(none, RideHistoryAction::None));

        let navigate = RideHistoryAction::Navigate(Screen::Home);
        assert!(matches!(navigate, RideHistoryAction::Navigate(_)));

        let retry = RideHistoryAction::RetrySync {
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
        };
        assert!(matches!(retry, RideHistoryAction::RetrySync { .. }));
    }

    #[test]
    fn test_ride_sync_status_failed_with_retry_count() {
        let failed = RideSyncStatus::Failed {
            error: "Network timeout".to_string(),
            retry_count: 3,
            platform: "Strava".to_string(),
        };

        assert_eq!(failed.label(), "Failed");
        assert_eq!(failed.color(), Color32::from_rgb(234, 67, 53));
        assert_eq!(failed.platform(), Some("Strava"));
        assert!(failed.activity_url().is_none());

        // Verify retry count is preserved
        if let RideSyncStatus::Failed { retry_count, .. } = failed {
            assert_eq!(retry_count, 3);
        } else {
            panic!("Expected Failed variant");
        }
    }
}
