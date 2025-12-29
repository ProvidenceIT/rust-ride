//! Power profile visualization screen.
//!
//! T054: Create power profile visualization screen.
//!
//! Displays 4D power profile with rolling and lifetime bests,
//! strength/weakness analysis, and rider type classification.

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui, Vec2};

use crate::power_profile::{
    duration_label, DurationComparison, EnergySystem, PowerProfile, ProfileAnalysis,
    ProfileComparer, ReferenceLevel, RiderClassification, RiderType, StrengthLevel,
    PROFILE_DURATIONS,
};

/// Tab selection for power profile screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerProfileTab {
    /// Overview with key metrics
    #[default]
    Overview,
    /// Detailed power curve chart
    PowerCurve,
    /// Strength/weakness analysis
    Analysis,
    /// Comparison to reference curves
    Comparison,
}

impl PowerProfileTab {
    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::PowerCurve => "Power Curve",
            Self::Analysis => "Analysis",
            Self::Comparison => "Compare",
        }
    }
}

/// Power profile screen state.
pub struct PowerProfileScreen {
    /// Current rolling window profile.
    pub rolling_profile: PowerProfile,
    /// Lifetime best profile.
    pub lifetime_profile: PowerProfile,
    /// Profile analysis.
    pub analysis: ProfileAnalysis,
    /// Rider classification.
    pub classification: Option<RiderClassification>,
    /// User weight in kg.
    pub weight_kg: f64,
    /// Current tab.
    pub current_tab: PowerProfileTab,
    /// Show lifetime bests overlay on chart.
    pub show_lifetime: bool,
    /// Show reference curves on chart.
    pub show_reference: bool,
    /// Selected reference level for comparison.
    pub reference_level: ReferenceLevel,
    /// Use female reference curves.
    pub use_female_reference: bool,
    /// Reference comparison result.
    pub comparison: Option<Vec<DurationComparison>>,
}

impl Default for PowerProfileScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerProfileScreen {
    /// Create a new power profile screen.
    pub fn new() -> Self {
        Self {
            rolling_profile: PowerProfile::default(),
            lifetime_profile: PowerProfile::default(),
            analysis: ProfileAnalysis::from_profile(&PowerProfile::default(), None),
            classification: None,
            weight_kg: 70.0,
            current_tab: PowerProfileTab::Overview,
            show_lifetime: true,
            show_reference: false,
            reference_level: ReferenceLevel::Trained,
            use_female_reference: false,
            comparison: None,
        }
    }

    /// Create with existing data.
    pub fn with_data(
        rolling_profile: PowerProfile,
        lifetime_profile: PowerProfile,
        weight_kg: f64,
        classification: Option<RiderClassification>,
    ) -> Self {
        let analysis = ProfileAnalysis::from_profile(&rolling_profile, Some(weight_kg));
        let comparer = ProfileComparer::new(weight_kg, false);
        let comparison = comparer.compare(&rolling_profile);

        Self {
            rolling_profile,
            lifetime_profile,
            analysis,
            classification,
            weight_kg,
            current_tab: PowerProfileTab::Overview,
            show_lifetime: true,
            show_reference: false,
            reference_level: ReferenceLevel::Trained,
            use_female_reference: false,
            comparison: Some(comparison.duration_comparisons),
        }
    }

    /// Update the comparison when settings change.
    fn update_comparison(&mut self) {
        let comparer = ProfileComparer::new(self.weight_kg, self.use_female_reference);
        let comparison = comparer.compare(&self.rolling_profile);
        self.comparison = Some(comparison.duration_comparisons);
    }

    /// Show the power profile screen.
    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            self.show_header(ui);
            ui.add_space(8.0);
            self.show_tabs(ui);
            ui.add_space(12.0);

            match self.current_tab {
                PowerProfileTab::Overview => self.show_overview(ui),
                PowerProfileTab::PowerCurve => self.show_power_curve(ui),
                PowerProfileTab::Analysis => self.show_analysis(ui),
                PowerProfileTab::Comparison => self.show_comparison(ui),
            }
        });
    }

    /// Show header with rider type and FTP.
    fn show_header(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Power Profile");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Rider type badge
                if let Some(ref classification) = self.classification {
                    let rider_type = &classification.rider_type;
                    let color = rider_type_color(*rider_type);
                    ui.label(
                        RichText::new(rider_type.display_name())
                            .color(color)
                            .strong()
                            .size(16.0),
                    );

                    if classification.confidence < 0.7 {
                        ui.label(
                            RichText::new("(low confidence)")
                                .color(Color32::from_gray(120))
                                .size(12.0),
                        );
                    }

                    ui.separator();
                }

                // FTP display
                if let Some(ftp) = self.analysis.estimated_ftp {
                    let wpk = ftp as f64 / self.weight_kg;
                    ui.label(
                        RichText::new(format!("FTP: {}W ({:.2} W/kg)", ftp, wpk))
                            .strong()
                            .size(14.0),
                    );
                }
            });
        });
    }

    /// Show tab buttons.
    fn show_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for tab in [
                PowerProfileTab::Overview,
                PowerProfileTab::PowerCurve,
                PowerProfileTab::Analysis,
                PowerProfileTab::Comparison,
            ] {
                let selected = self.current_tab == tab;
                if ui.selectable_label(selected, tab.label()).clicked() {
                    self.current_tab = tab;
                }
            }
        });
    }

    /// Show overview tab with key metrics.
    fn show_overview(&self, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Rider type section
                if let Some(ref classification) = self.classification {
                    self.show_rider_type_section(ui, classification);
                    ui.add_space(16.0);
                }

                // Key power metrics
                self.show_key_metrics(ui);
                ui.add_space(16.0);

                // Energy system scores
                self.show_energy_system_overview(ui);
            });
    }

    /// Show rider type section.
    fn show_rider_type_section(&self, ui: &mut Ui, classification: &RiderClassification) {
        let rider_type = &classification.rider_type;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                let color = rider_type_color(*rider_type);
                ui.label(
                    RichText::new(rider_type.display_name())
                        .color(color)
                        .strong()
                        .size(20.0),
                );

                if let Some(secondary) = &classification.secondary_type {
                    ui.label(RichText::new("/").weak());
                    ui.label(
                        RichText::new(secondary.display_name())
                            .color(rider_type_color(*secondary))
                            .size(16.0),
                    );
                }
            });

            ui.add_space(4.0);
            ui.label(rider_type.description());
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Training Focus: {}", rider_type.training_focus()))
                    .weak()
                    .italics(),
            );
        });
    }

    /// Show key power metrics.
    fn show_key_metrics(&self, ui: &mut Ui) {
        ui.label(RichText::new("Key Metrics").strong().size(16.0));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            // Sprint (5s)
            self.show_metric_card(ui, "5s Sprint", 5);

            // 1-minute
            self.show_metric_card(ui, "1 Min", 60);

            // 5-minute (VO2max)
            self.show_metric_card(ui, "5 Min (VO2max)", 300);

            // 20-minute (FTP proxy)
            self.show_metric_card(ui, "20 Min (FTP)", 1200);
        });
    }

    /// Show a single metric card.
    fn show_metric_card(&self, ui: &mut Ui, label: &str, duration_secs: u32) {
        let rolling = self.rolling_profile.power_at_duration(duration_secs);
        let lifetime = self.lifetime_profile.power_at_duration(duration_secs);

        let (rect, _) = ui.allocate_exact_size(Vec2::new(120.0, 90.0), egui::Sense::hover());

        ui.painter().rect_filled(rect, 4.0, Color32::from_gray(40));

        let content_rect = rect.shrink(8.0);
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

        child_ui.vertical(|ui| {
            ui.label(RichText::new(label).weak().size(11.0));

            if let Some(power) = rolling {
                let wpk = power as f64 / self.weight_kg;
                ui.label(RichText::new(format!("{}W", power)).strong().size(18.0));
                ui.label(RichText::new(format!("{:.2} W/kg", wpk)).weak().size(12.0));
            } else {
                ui.label(RichText::new("--").weak().size(18.0));
            }

            // Lifetime comparison
            if let (Some(r), Some(l)) = (rolling, lifetime) {
                if l > r {
                    let diff = l - r;
                    ui.label(
                        RichText::new(format!("↓{}W from best", diff))
                            .color(Color32::from_rgb(255, 150, 100))
                            .size(10.0),
                    );
                } else if l == r {
                    ui.label(
                        RichText::new("= Lifetime Best!")
                            .color(Color32::from_rgb(100, 255, 100))
                            .size(10.0),
                    );
                }
            }
        });
    }

    /// Show energy system overview.
    fn show_energy_system_overview(&self, ui: &mut Ui) {
        ui.label(RichText::new("Energy Systems").strong().size(16.0));
        ui.add_space(8.0);

        for system in EnergySystem::all() {
            if let Some(score) = self.analysis.energy_system_score(*system) {
                self.show_energy_system_bar(ui, *system, score);
            }
        }
    }

    /// Show a single energy system bar.
    fn show_energy_system_bar(&self, ui: &mut Ui, system: EnergySystem, score: f64) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(system.short_name()).strong().size(12.0));

            // Normalize score to 0-1 range (assuming -30 to +30 range)
            let _normalized = ((score + 30.0) / 60.0).clamp(0.0, 1.0) as f32;

            // Color based on strength
            let strength = StrengthLevel::from_deviation(score);
            let color = strength_color(strength);

            let bar_width = 200.0;
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(bar_width, 16.0), egui::Sense::hover());

            // Background
            ui.painter().rect_filled(rect, 4.0, Color32::from_gray(50));

            // Center line
            let center_x = rect.min.x + bar_width * 0.5;
            ui.painter().line_segment(
                [
                    egui::Pos2::new(center_x, rect.min.y),
                    egui::Pos2::new(center_x, rect.max.y),
                ],
                egui::Stroke::new(1.0, Color32::from_gray(100)),
            );

            // Bar from center
            let bar_rect = if score >= 0.0 {
                egui::Rect::from_min_max(
                    egui::Pos2::new(center_x, rect.min.y + 2.0),
                    egui::Pos2::new(
                        center_x + (bar_width * 0.5 * (score / 30.0).min(1.0) as f32),
                        rect.max.y - 2.0,
                    ),
                )
            } else {
                egui::Rect::from_min_max(
                    egui::Pos2::new(
                        center_x + (bar_width * 0.5 * (score / 30.0).max(-1.0) as f32),
                        rect.min.y + 2.0,
                    ),
                    egui::Pos2::new(center_x, rect.max.y - 2.0),
                )
            };

            if bar_rect.width() > 0.0 {
                ui.painter().rect_filled(bar_rect, 2.0, color);
            }

            // Score text
            ui.label(
                RichText::new(format!("{:+.1}%", score))
                    .color(color)
                    .size(12.0),
            );

            ui.label(
                RichText::new(strength.display_name())
                    .color(color)
                    .size(11.0),
            );
        });
    }

    /// Show power curve tab.
    fn show_power_curve(&mut self, ui: &mut Ui) {
        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_lifetime, "Show Lifetime Bests");
            ui.checkbox(&mut self.show_reference, "Show Reference");

            if self.show_reference {
                egui::ComboBox::from_label("Level")
                    .selected_text(self.reference_level.display_name())
                    .show_ui(ui, |ui| {
                        for level in ReferenceLevel::all() {
                            if ui
                                .selectable_label(
                                    self.reference_level == *level,
                                    level.display_name(),
                                )
                                .clicked()
                            {
                                self.reference_level = *level;
                            }
                        }
                    });
            }
        });

        ui.add_space(12.0);

        // Chart area
        self.show_power_curve_chart(ui);
    }

    /// Show the power curve chart.
    fn show_power_curve_chart(&self, ui: &mut Ui) {
        let available = ui.available_size();
        let chart_height = (available.y - 50.0).max(200.0);
        let chart_width = available.x.min(800.0);

        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(chart_width, chart_height), egui::Sense::hover());

        // Background
        ui.painter().rect_filled(rect, 4.0, Color32::from_gray(30));

        // Chart margins
        let margin = 50.0;
        let chart_rect = egui::Rect::from_min_max(
            egui::Pos2::new(rect.min.x + margin, rect.min.y + 20.0),
            egui::Pos2::new(rect.max.x - 20.0, rect.max.y - margin),
        );

        // Draw grid
        self.draw_chart_grid(ui, chart_rect);

        // Draw reference curves if enabled
        if self.show_reference {
            self.draw_reference_curve(ui, chart_rect);
        }

        // Draw lifetime profile if enabled
        if self.show_lifetime {
            self.draw_profile_curve(
                ui,
                chart_rect,
                &self.lifetime_profile,
                Color32::from_rgb(100, 100, 200),
                true,
            );
        }

        // Draw rolling profile
        self.draw_profile_curve(
            ui,
            chart_rect,
            &self.rolling_profile,
            Color32::from_rgb(100, 200, 255),
            false,
        );

        // Legend
        self.draw_chart_legend(ui, rect);
    }

    /// Draw chart grid lines.
    fn draw_chart_grid(&self, ui: &mut Ui, chart_rect: egui::Rect) {
        let painter = ui.painter();

        // Y-axis (power)
        let max_power = self.get_max_power();
        let power_steps = [0, 200, 400, 600, 800, 1000, 1200, 1400];

        for &power in &power_steps {
            if power as u16 <= max_power {
                let y = chart_rect.max.y - (power as f32 / max_power as f32) * chart_rect.height();
                painter.line_segment(
                    [
                        egui::Pos2::new(chart_rect.min.x, y),
                        egui::Pos2::new(chart_rect.max.x, y),
                    ],
                    egui::Stroke::new(1.0, Color32::from_gray(50)),
                );
                painter.text(
                    egui::Pos2::new(chart_rect.min.x - 5.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}W", power),
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(120),
                );
            }
        }

        // X-axis (duration) - logarithmic scale
        for &duration in &PROFILE_DURATIONS {
            let x = self.duration_to_x(duration, chart_rect);
            painter.line_segment(
                [
                    egui::Pos2::new(x, chart_rect.min.y),
                    egui::Pos2::new(x, chart_rect.max.y),
                ],
                egui::Stroke::new(1.0, Color32::from_gray(50)),
            );
            painter.text(
                egui::Pos2::new(x, chart_rect.max.y + 5.0),
                egui::Align2::CENTER_TOP,
                duration_label(duration),
                egui::FontId::proportional(10.0),
                Color32::from_gray(120),
            );
        }
    }

    /// Draw a profile curve.
    fn draw_profile_curve(
        &self,
        ui: &mut Ui,
        chart_rect: egui::Rect,
        profile: &PowerProfile,
        color: Color32,
        dashed: bool,
    ) {
        let painter = ui.painter();
        let max_power = self.get_max_power();

        let mut points: Vec<egui::Pos2> = Vec::new();

        for &duration in &PROFILE_DURATIONS {
            if let Some(power) = profile.power_at_duration(duration) {
                let x = self.duration_to_x(duration, chart_rect);
                let y = chart_rect.max.y - (power as f32 / max_power as f32) * chart_rect.height();
                points.push(egui::Pos2::new(x, y));
            }
        }

        // Draw line
        if points.len() >= 2 {
            for window in points.windows(2) {
                let stroke = if dashed {
                    egui::Stroke::new(1.5, color.linear_multiply(0.7))
                } else {
                    egui::Stroke::new(2.0, color)
                };
                painter.line_segment([window[0], window[1]], stroke);
            }
        }

        // Draw points
        for point in &points {
            painter.circle_filled(*point, if dashed { 3.0 } else { 4.0 }, color);
        }
    }

    /// Draw reference curve.
    fn draw_reference_curve(&self, ui: &mut Ui, chart_rect: egui::Rect) {
        let painter = ui.painter();
        let max_power = self.get_max_power();

        let comparer = ProfileComparer::new(self.weight_kg, self.use_female_reference);
        let curve = comparer.reference_curve(self.reference_level);
        let reference_points = curve.full_curve(self.weight_kg);

        let points: Vec<egui::Pos2> = reference_points
            .iter()
            .map(|(duration, power)| {
                let x = self.duration_to_x(*duration, chart_rect);
                let y = chart_rect.max.y - (*power as f32 / max_power as f32) * chart_rect.height();
                egui::Pos2::new(x, y)
            })
            .collect();

        // Draw line
        let color = Color32::from_rgba_unmultiplied(255, 200, 100, 150);
        if points.len() >= 2 {
            for window in points.windows(2) {
                painter.line_segment([window[0], window[1]], egui::Stroke::new(1.5, color));
            }
        }
    }

    /// Draw chart legend.
    fn draw_chart_legend(&self, ui: &mut Ui, rect: egui::Rect) {
        let painter = ui.painter();
        let legend_y = rect.min.y + 5.0;
        let mut x = rect.max.x - 200.0;

        // Rolling
        painter.circle_filled(
            egui::Pos2::new(x, legend_y),
            4.0,
            Color32::from_rgb(100, 200, 255),
        );
        painter.text(
            egui::Pos2::new(x + 8.0, legend_y),
            egui::Align2::LEFT_CENTER,
            "Rolling 90-Day",
            egui::FontId::proportional(10.0),
            Color32::from_gray(180),
        );
        x += 80.0;

        if self.show_lifetime {
            painter.circle_filled(
                egui::Pos2::new(x, legend_y),
                3.0,
                Color32::from_rgb(100, 100, 200),
            );
            painter.text(
                egui::Pos2::new(x + 8.0, legend_y),
                egui::Align2::LEFT_CENTER,
                "Lifetime",
                egui::FontId::proportional(10.0),
                Color32::from_gray(180),
            );
        }
    }

    /// Convert duration to X coordinate (log scale).
    fn duration_to_x(&self, duration: u32, chart_rect: egui::Rect) -> f32 {
        let log_min = (5.0_f32).ln();
        let log_max = (3600.0_f32).ln();
        let log_duration = (duration as f32).ln();
        let normalized = (log_duration - log_min) / (log_max - log_min);
        chart_rect.min.x + normalized * chart_rect.width()
    }

    /// Get maximum power for chart scaling.
    fn get_max_power(&self) -> u16 {
        let rolling_max = self.rolling_profile.max_power().unwrap_or(500);
        let lifetime_max = if self.show_lifetime {
            self.lifetime_profile.max_power().unwrap_or(500)
        } else {
            0
        };
        rolling_max.max(lifetime_max).max(500)
    }

    /// Show analysis tab.
    fn show_analysis(&self, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Strengths
                ui.label(
                    RichText::new("Strengths")
                        .strong()
                        .color(Color32::from_rgb(100, 200, 100))
                        .size(16.0),
                );
                ui.add_space(8.0);

                let strengths = self.analysis.get_strengths();
                if strengths.is_empty() {
                    ui.label("No clear strengths identified yet. Keep training!");
                } else {
                    for strength in strengths {
                        self.show_duration_analysis(ui, strength, true);
                    }
                }

                ui.add_space(16.0);

                // Weaknesses
                ui.label(
                    RichText::new("Weaknesses")
                        .strong()
                        .color(Color32::from_rgb(200, 100, 100))
                        .size(16.0),
                );
                ui.add_space(8.0);

                let weaknesses = self.analysis.get_weaknesses();
                if weaknesses.is_empty() {
                    ui.label("No clear weaknesses identified. Well-rounded profile!");
                } else {
                    for weakness in weaknesses {
                        self.show_duration_analysis(ui, weakness, false);
                    }
                }
            });
    }

    /// Show a duration analysis item.
    fn show_duration_analysis(
        &self,
        ui: &mut Ui,
        analysis: &crate::power_profile::DurationStrength,
        is_strength: bool,
    ) {
        let color = if is_strength {
            Color32::from_rgb(100, 200, 100)
        } else {
            Color32::from_rgb(200, 100, 100)
        };

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(duration_label(analysis.duration_secs))
                    .strong()
                    .size(14.0),
            );

            ui.label(format!("{}W", analysis.power_watts));

            if let Some(wpk) = analysis.watts_per_kg {
                ui.label(
                    RichText::new(format!("({:.2} W/kg)", wpk))
                        .weak()
                        .size(12.0),
                );
            }

            ui.label(
                RichText::new(format!("{:+.1}%", analysis.deviation_percent))
                    .color(color)
                    .strong(),
            );

            ui.label(
                RichText::new(analysis.energy_system.short_name())
                    .weak()
                    .size(11.0),
            );
        });
    }

    /// Show comparison tab.
    fn show_comparison(&mut self, ui: &mut Ui) {
        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_female_reference, "Female Reference");

            egui::ComboBox::from_label("Compare to")
                .selected_text(self.reference_level.display_name())
                .show_ui(ui, |ui| {
                    let old_level = self.reference_level;
                    for level in ReferenceLevel::all() {
                        if ui
                            .selectable_label(self.reference_level == *level, level.display_name())
                            .clicked()
                        {
                            self.reference_level = *level;
                        }
                    }
                    if old_level != self.reference_level {
                        self.update_comparison();
                    }
                });
        });

        ui.add_space(12.0);

        // Comparison table
        if let Some(ref comparisons) = self.comparison {
            self.show_comparison_table(ui, comparisons);
        }
    }

    /// Show comparison table.
    fn show_comparison_table(&self, ui: &mut Ui, comparisons: &[DurationComparison]) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Duration").strong().size(12.0));
                    ui.add_space(40.0);
                    ui.label(RichText::new("Power").strong().size(12.0));
                    ui.add_space(30.0);
                    ui.label(RichText::new("W/kg").strong().size(12.0));
                    ui.add_space(20.0);
                    ui.label(RichText::new("Level").strong().size(12.0));
                    ui.add_space(40.0);
                    ui.label(RichText::new("Progress").strong().size(12.0));
                });

                ui.separator();

                // Rows
                for comparison in comparisons {
                    ui.horizontal(|ui| {
                        ui.label(duration_label(comparison.duration_secs));
                        ui.add_space(40.0);
                        ui.label(format!("{}W", comparison.actual_power));
                        ui.add_space(30.0);
                        ui.label(format!("{:.2}", comparison.actual_wpk));
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new(comparison.level.display_name())
                                .color(level_color(comparison.level)),
                        );
                        ui.add_space(40.0);

                        // Progress bar to next level
                        ui.add(
                            egui::ProgressBar::new(comparison.progress_to_next as f32 / 100.0)
                                .text(format!("{:.0}%", comparison.progress_to_next))
                                .desired_width(100.0),
                        );
                    });
                }
            });
    }
}

/// Get color for rider type.
fn rider_type_color(rider_type: RiderType) -> Color32 {
    match rider_type {
        RiderType::Sprinter => Color32::from_rgb(255, 100, 100),
        RiderType::Puncher => Color32::from_rgb(255, 200, 100),
        RiderType::Rouleur => Color32::from_rgb(100, 200, 255),
        RiderType::Climber => Color32::from_rgb(100, 255, 150),
        RiderType::AllRounder => Color32::from_rgb(200, 150, 255),
        RiderType::Unknown => Color32::from_gray(150),
    }
}

/// Get color for strength level.
fn strength_color(strength: StrengthLevel) -> Color32 {
    match strength {
        StrengthLevel::VeryWeak => Color32::from_rgb(200, 50, 50),
        StrengthLevel::Weak => Color32::from_rgb(200, 100, 100),
        StrengthLevel::Average => Color32::from_gray(150),
        StrengthLevel::Strong => Color32::from_rgb(100, 200, 100),
        StrengthLevel::VeryStrong => Color32::from_rgb(50, 255, 100),
    }
}

/// Get color for reference level.
fn level_color(level: ReferenceLevel) -> Color32 {
    match level {
        ReferenceLevel::Untrained => Color32::from_gray(120),
        ReferenceLevel::Recreational => Color32::from_rgb(100, 150, 100),
        ReferenceLevel::Trained => Color32::from_rgb(100, 200, 100),
        ReferenceLevel::Competitive => Color32::from_rgb(100, 200, 255),
        ReferenceLevel::Elite => Color32::from_rgb(255, 200, 100),
        ReferenceLevel::WorldClass => Color32::from_rgb(255, 100, 255),
    }
}

/// Action from power profile screen.
#[derive(Debug, Clone)]
pub enum PowerProfileAction {
    /// Go back to previous screen.
    Back,
    /// View specific duration detail.
    ViewDurationDetail(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_creation() {
        let screen = PowerProfileScreen::new();
        assert_eq!(screen.current_tab, PowerProfileTab::Overview);
        assert!(screen.show_lifetime);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(PowerProfileTab::Overview.label(), "Overview");
        assert_eq!(PowerProfileTab::PowerCurve.label(), "Power Curve");
        assert_eq!(PowerProfileTab::Analysis.label(), "Analysis");
        assert_eq!(PowerProfileTab::Comparison.label(), "Compare");
    }

    #[test]
    fn test_rider_type_colors() {
        for rider_type in RiderType::all() {
            let color = rider_type_color(*rider_type);
            assert!(color.r() > 0 || color.g() > 0 || color.b() > 0);
        }
    }

    #[test]
    fn test_strength_colors() {
        let colors = [
            strength_color(StrengthLevel::VeryWeak),
            strength_color(StrengthLevel::Weak),
            strength_color(StrengthLevel::Average),
            strength_color(StrengthLevel::Strong),
            strength_color(StrengthLevel::VeryStrong),
        ];

        for color in &colors {
            assert!(color.r() > 0 || color.g() > 0 || color.b() > 0);
        }
    }
}
