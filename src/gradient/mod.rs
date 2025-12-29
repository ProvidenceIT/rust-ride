//! Gradient simulation module for gradient-responsive resistance.
//!
//! Provides gradient calculation from GPX routes and trainer resistance control
//! based on virtual elevation changes.

mod calculator;
mod controller;
mod error;
mod resistance;
mod route;
mod settings;
mod smoothing;
mod types;

pub use calculator::GradientCalculator;
pub use controller::GradientController;
pub use error::GradientError;
pub use resistance::ResistanceController;
pub use route::GpxRoute;
pub use settings::GradientSettings;
pub use smoothing::GradientSmoother;
pub use types::{GradientPoint, GradientResult, GradientSegment, RoutePoint};
