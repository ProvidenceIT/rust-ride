//! UI widgets for reusable components.

pub mod metric_display;
pub mod sensor_status;
pub mod zone_indicator;

pub use metric_display::{MetricDisplay, MetricSize};
pub use sensor_status::{SensorConnectionCard, SensorStatusIndicator, SensorStatusSummary};
pub use zone_indicator::ZoneIndicator;
