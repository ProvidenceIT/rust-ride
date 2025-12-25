//! Analytics screen for training science metrics.
//!
//! T036: Create analytics screen with PDC display
//! T058: Add CP/W' display section
//! T091: Add ATL/CTL/TSB history chart
//! T101: Add VO2max display section
//! T111: Add Sweet Spot recommendations section
//! T125: Add power profile radar chart

use egui::{RichText, Ui};

use crate::metrics::analytics::{
    critical_power::CpModel,
    pdc::PowerDurationCurve,
    rider_type::{PowerProfile, RiderClassifier, RiderType},
    sweet_spot::{SweetSpotRecommender, WorkoutRecommendation},
    training_load::{AcwrStatus, DailyLoad, TrainingLoadCalculator},
    vo2max::{FitnessLevel, Vo2maxCalculator, Vo2maxResult},
};
use crate::ui::widgets::pdc_chart::{KeyPowers, PdcChart, PdcDateFilter};

/// Analytics screen tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnalyticsTab {
    /// Power Duration Curve
    #[default]
    PowerCurve,
    /// Critical Power / W'
    CriticalPower,
    /// Training Load (ATL/CTL/ACWR)
    TrainingLoad,
    /// Fitness metrics (VO2max, Rider Type)
    Fitness,
    /// Workout recommendations
    Recommendations,
}

impl AnalyticsTab {
    fn label(&self) -> &'static str {
        match self {
            AnalyticsTab::PowerCurve => "Power Curve",
            AnalyticsTab::CriticalPower => "CP / W'",
            AnalyticsTab::TrainingLoad => "Training Load",
            AnalyticsTab::Fitness => "Fitness",
            AnalyticsTab::Recommendations => "Recommendations",
        }
    }
}

/// Analytics screen state.
pub struct AnalyticsScreen {
    /// Current tab
    current_tab: AnalyticsTab,
    /// PDC date filter
    pdc_filter: PdcDateFilter,
    /// Power Duration Curve data
    pdc: Option<PowerDurationCurve>,
    /// Critical Power model
    cp_model: Option<CpModel>,
    /// Current training load
    current_load: Option<DailyLoad>,
    /// VO2max result
    vo2max: Option<Vo2maxResult>,
    /// Rider type classification
    rider_type: Option<RiderType>,
    /// Power profile
    power_profile: Option<PowerProfile>,
    /// Workout recommendation
    recommendation: Option<WorkoutRecommendation>,
    /// User FTP
    ftp: u16,
    /// User weight in kg
    weight_kg: f32,
    /// TTE input power
    tte_input_power: String,
}

impl Default for AnalyticsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyticsScreen {
    /// Create a new analytics screen.
    pub fn new() -> Self {
        Self {
            current_tab: AnalyticsTab::default(),
            pdc_filter: PdcDateFilter::default(),
            pdc: None,
            cp_model: None,
            current_load: None,
            vo2max: None,
            rider_type: None,
            power_profile: None,
            recommendation: None,
            ftp: 200,
            weight_kg: 70.0,
            tte_input_power: "350".to_string(),
        }
    }

    /// Set PDC data.
    pub fn set_pdc(&mut self, pdc: PowerDurationCurve) {
        self.pdc = Some(pdc);
    }

    /// Set CP model.
    pub fn set_cp_model(&mut self, model: CpModel) {
        self.cp_model = Some(model);
    }

    /// Set current training load.
    pub fn set_training_load(&mut self, load: DailyLoad) {
        self.current_load = Some(load);
    }

    /// Set VO2max result.
    pub fn set_vo2max(&mut self, result: Vo2maxResult) {
        self.vo2max = Some(result);
    }

    /// Set rider classification.
    pub fn set_rider_type(&mut self, rider_type: RiderType, profile: PowerProfile) {
        self.rider_type = Some(rider_type);
        self.power_profile = Some(profile);
    }

    /// Set workout recommendation.
    pub fn set_recommendation(&mut self, rec: WorkoutRecommendation) {
        self.recommendation = Some(rec);
    }

    /// Set user FTP.
    pub fn set_ftp(&mut self, ftp: u16) {
        self.ftp = ftp;
    }

    /// Set user weight.
    pub fn set_weight(&mut self, weight_kg: f32) {
        self.weight_kg = weight_kg;
    }

    /// Recalculate all derived metrics from PDC.
    pub fn recalculate(&mut self) {
        // Calculate VO2max if we have PDC and weight
        if let Some(pdc) = &self.pdc {
            if let Some(p5m) = pdc.power_at(300) {
                let calc = Vo2maxCalculator::new(self.weight_kg);
                self.vo2max = Some(calc.from_five_minute_power(p5m));
            }

            // Calculate rider type if we have FTP
            if self.ftp > 0 {
                let classifier = RiderClassifier::new(self.ftp);
                let profile = classifier.profile_from_pdc(pdc);
                let rider_type = classifier.classify(&profile);
                self.power_profile = Some(profile);
                self.rider_type = Some(rider_type);
            }
        }

        // Generate recommendation if we have training load and FTP
        if let Some(load) = &self.current_load {
            if self.ftp > 0 {
                let recommender = SweetSpotRecommender::new(self.ftp);
                self.recommendation = Some(recommender.recommend(load));
            }
        }
    }

    /// Show the analytics screen.
    pub fn show(&mut self, ui: &mut Ui) {
        // Tab bar
        ui.horizontal(|ui| {
            for tab in [
                AnalyticsTab::PowerCurve,
                AnalyticsTab::CriticalPower,
                AnalyticsTab::TrainingLoad,
                AnalyticsTab::Fitness,
                AnalyticsTab::Recommendations,
            ] {
                if ui
                    .selectable_label(self.current_tab == tab, tab.label())
                    .clicked()
                {
                    self.current_tab = tab;
                }
            }
        });

        ui.separator();

        // Tab content
        match self.current_tab {
            AnalyticsTab::PowerCurve => self.show_power_curve(ui),
            AnalyticsTab::CriticalPower => self.show_critical_power(ui),
            AnalyticsTab::TrainingLoad => self.show_training_load(ui),
            AnalyticsTab::Fitness => self.show_fitness(ui),
            AnalyticsTab::Recommendations => self.show_recommendations(ui),
        }
    }

    /// Show Power Duration Curve tab.
    fn show_power_curve(&mut self, ui: &mut Ui) {
        ui.heading("Power Duration Curve");

        // Date filter
        ui.horizontal(|ui| {
            ui.label("Period:");
            for filter in [
                PdcDateFilter::Last30Days,
                PdcDateFilter::Last60Days,
                PdcDateFilter::Last90Days,
                PdcDateFilter::AllTime,
            ] {
                if ui
                    .selectable_label(self.pdc_filter == filter, filter.label())
                    .clicked()
                {
                    self.pdc_filter = filter;
                }
            }
        });

        ui.add_space(8.0);

        if let Some(pdc) = &self.pdc {
            // Key powers summary
            let keys = KeyPowers::from_pdc(pdc);
            keys.show(ui);

            ui.add_space(8.0);

            // PDC chart
            PdcChart::new(pdc).height(300.0).show(ui);
        } else {
            ui.label(
                "No power data available. Complete some rides to build your Power Duration Curve.",
            );
        }
    }

    /// Show Critical Power tab.
    fn show_critical_power(&mut self, ui: &mut Ui) {
        ui.heading("Critical Power Model");

        if let Some(model) = &self.cp_model {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Critical Power");
                    ui.heading(format!("{}W", model.cp));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("W' (Anaerobic Capacity)");
                    ui.heading(format!("{}kJ", model.w_prime / 1000));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("Model Fit (R²)");
                    ui.heading(format!("{:.2}", model.r_squared));
                });
            });

            ui.add_space(16.0);

            // TTE Prediction
            ui.group(|ui| {
                ui.label("Time to Exhaustion Prediction");
                ui.horizontal(|ui| {
                    ui.label("Power:");
                    ui.text_edit_singleline(&mut self.tte_input_power);
                    ui.label("W");
                });

                if let Ok(power) = self.tte_input_power.parse::<u16>() {
                    if let Some(tte) = model.time_to_exhaustion(power) {
                        let mins = tte.as_secs() / 60;
                        let secs = tte.as_secs() % 60;
                        ui.label(format!("Time to exhaustion: {}:{:02}", mins, secs));
                    } else {
                        ui.label("Power is at or below CP - sustainable indefinitely");
                    }
                }
            });
        } else {
            ui.label(
                "Insufficient data for CP model. Need at least 3 efforts in the 2-20 minute range.",
            );
        }
    }

    /// Show Training Load tab.
    fn show_training_load(&mut self, ui: &mut Ui) {
        ui.heading("Training Load");

        if let Some(load) = &self.current_load {
            let calc = TrainingLoadCalculator::new();
            let acwr = calc.acwr(load.atl, load.ctl);

            // ACWR status
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("ACWR");
                    let color = status_color(acwr.status);
                    ui.heading(RichText::new(format!("{:.2}", acwr.ratio)).color(color));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("Status");
                    let color = status_color(acwr.status);
                    ui.heading(RichText::new(status_label(acwr.status)).color(color));
                });
            });

            ui.add_space(8.0);
            ui.label(acwr.recommendation());

            ui.add_space(16.0);

            // ATL/CTL/TSB values
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("ATL (Fatigue)");
                    ui.heading(format!("{:.0}", load.atl));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("CTL (Fitness)");
                    ui.heading(format!("{:.0}", load.ctl));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("TSB (Form)");
                    let tsb_color = if load.tsb > 0.0 {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };
                    ui.heading(RichText::new(format!("{:+.0}", load.tsb)).color(tsb_color));
                });
            });
        } else {
            ui.label("No training load data available. Complete some rides to start tracking.");
            ui.add_space(8.0);
            ui.label("Note: ACWR requires at least 28 days of data for meaningful results.");
        }
    }

    /// Show Fitness tab.
    fn show_fitness(&mut self, ui: &mut Ui) {
        ui.heading("Fitness Metrics");

        // VO2max section
        ui.group(|ui| {
            ui.label("VO2max Estimate");
            if let Some(vo2max) = &self.vo2max {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("VO2max");
                        ui.heading(format!("{:.1} ml/kg/min", vo2max.vo2max));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("Classification");
                        ui.heading(fitness_level_label(vo2max.classification));
                    });
                });
                ui.add_space(4.0);
                ui.label(vo2max.classification.description());
            } else {
                ui.label("Need 5-minute max power data to estimate VO2max.");
            }
        });

        ui.add_space(16.0);

        // Rider Type section
        ui.group(|ui| {
            ui.label("Rider Classification");
            if let (Some(rider_type), Some(profile)) = (&self.rider_type, &self.power_profile) {
                ui.heading(rider_type_label(*rider_type));
                ui.add_space(8.0);

                // Power profile
                ui.label("Power Profile (% of FTP):");
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("5s");
                        ui.strong(format!("{:.0}%", profile.neuromuscular));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("1min");
                        ui.strong(format!("{:.0}%", profile.anaerobic));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("5min");
                        ui.strong(format!("{:.0}%", profile.vo2max));
                    });
                });

                ui.add_space(8.0);
                ui.label(rider_type.training_recommendations());
            } else {
                ui.label("Need sufficient PDC data to classify rider type.");
            }
        });
    }

    /// Show Recommendations tab.
    fn show_recommendations(&mut self, ui: &mut Ui) {
        ui.heading("Workout Recommendations");

        if let Some(rec) = &self.recommendation {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Recommended Zone");
                        ui.heading(rec.zone.ftp_range());
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("Duration");
                        ui.heading(format!("{}min", rec.duration_min));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("Expected TSS");
                        ui.heading(format!("{}", rec.expected_tss));
                    });
                });

                ui.add_space(8.0);
                ui.label(&rec.rationale);

                ui.add_space(8.0);
                ui.label("Suggested Workout:");
                ui.code(&rec.structure);
            });
        } else {
            ui.label("Complete some rides to get personalized workout recommendations.");
        }
    }
}

/// Get color for ACWR status.
fn status_color(status: AcwrStatus) -> egui::Color32 {
    match status {
        AcwrStatus::Undertrained => egui::Color32::from_rgb(100, 149, 237),
        AcwrStatus::Optimal => egui::Color32::from_rgb(50, 205, 50),
        AcwrStatus::Caution => egui::Color32::from_rgb(255, 165, 0),
        AcwrStatus::HighRisk => egui::Color32::from_rgb(220, 20, 60),
    }
}

/// Get label for ACWR status.
fn status_label(status: AcwrStatus) -> &'static str {
    match status {
        AcwrStatus::Undertrained => "Undertrained",
        AcwrStatus::Optimal => "Optimal",
        AcwrStatus::Caution => "Caution",
        AcwrStatus::HighRisk => "High Risk",
    }
}

/// Get label for fitness level.
fn fitness_level_label(level: FitnessLevel) -> &'static str {
    match level {
        FitnessLevel::Untrained => "Untrained",
        FitnessLevel::Recreational => "Recreational",
        FitnessLevel::Trained => "Trained",
        FitnessLevel::WellTrained => "Well-Trained",
        FitnessLevel::Elite => "Elite",
        FitnessLevel::WorldClass => "World-Class",
    }
}

/// Get label for rider type.
fn rider_type_label(rt: RiderType) -> &'static str {
    match rt {
        RiderType::Sprinter => "Sprinter",
        RiderType::Pursuiter => "Pursuiter",
        RiderType::TimeTrialist => "Time Trialist",
        RiderType::AllRounder => "All-Rounder",
        RiderType::Unknown => "Unknown",
    }
}
