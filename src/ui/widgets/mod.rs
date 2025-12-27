//! UI widgets for reusable components.

pub mod chat_panel;
pub mod dynamics_display;
pub mod ftp_notification;
pub mod metric_display;
pub mod participant_list;
pub mod pdc_chart;
pub mod sensor_status;
pub mod shortcut_overlay;
pub mod smo2_display;
pub mod tilt_indicator;
pub mod weather_widget;
pub mod zone_indicator;

pub use chat_panel::{ChatMessage, ChatOverlay, ChatPanel, ChatPanelAction, ChatPanelConfig};
pub use dynamics_display::{BalanceBar, DynamicsDisplay};
pub use ftp_notification::{FtpNotification, FtpNotificationAction};
pub use metric_display::{MetricDisplay, MetricSize};
pub use participant_list::{CompactParticipantRow, ParticipantList, ParticipantListConfig};
pub use pdc_chart::{KeyPowers, PdcChart, PdcDateFilter};
pub use sensor_status::{SensorConnectionCard, SensorStatusIndicator, SensorStatusSummary};
pub use smo2_display::{SmO2Display, SmO2Placeholder, SmO2WidgetSize};
pub use tilt_indicator::{TiltIndicator, TiltIndicatorConfig};
pub use weather_widget::{WeatherPlaceholder, WeatherWidget, WeatherWidgetSize};
pub use shortcut_overlay::ShortcutOverlay;
pub use zone_indicator::ZoneIndicator;
