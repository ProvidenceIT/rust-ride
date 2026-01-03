//! UI widgets for reusable components.

pub mod accessible_button;
pub mod achievement_notification;
pub mod chat_panel;
pub mod connection_quality;
pub mod dynamics_display;
pub mod ftp_notification;
pub mod gradient_display;
pub mod gradient_settings;
pub mod hud;
pub mod level_up_notification;
pub mod metric_display;
pub mod participant_list;
pub mod pdc_chart;
pub mod plan_detail;
pub mod power_curve_chart;
pub mod sensor_status;
pub mod shortcut_overlay;
pub mod smo2_display;
pub mod sync_notification;
pub mod tilt_indicator;
pub mod upcoming_workouts;
pub mod voice_indicator;
pub mod weather_widget;
pub mod zone_indicator;

pub use accessible_button::{
    accessible_button, accessible_icon_button, AccessibleButton, AccessibleButtonStyle,
    AccessibleIconButton, TouchFeedbackStyle, MIN_TOUCH_TARGET,
};
pub use achievement_notification::{
    AchievementBadge, AchievementNotificationConfig, AchievementNotificationWidget,
};
pub use chat_panel::{ChatMessage, ChatOverlay, ChatPanel, ChatPanelAction, ChatPanelConfig};
pub use connection_quality::{
    CompactConnectionQualityIndicator, ConnectionQualityIndicator,
    ConnectionQualityIndicatorConfig, ConnectionQualityIndicatorResponse, ConnectionQualityPreview,
};
pub use dynamics_display::{BalanceBar, DynamicsDisplay};
pub use ftp_notification::{FtpNotification, FtpNotificationAction};
pub use gradient_display::{GradientDisplay, GradientDisplayConfig, GradientPreview};
pub use gradient_settings::{
    GradientSettingsAction, GradientSettingsPanel, GradientSettingsResponse,
};
pub use level_up_notification::{
    LevelBadge, LevelProgressBar, LevelUpNotificationConfig, LevelUpNotificationWidget,
};
pub use metric_display::{MetricDisplay, MetricSize};
pub use participant_list::{CompactParticipantRow, ParticipantList, ParticipantListConfig};
pub use pdc_chart::{KeyPowers, PdcChart, PdcDateFilter};
pub use plan_detail::{PlanDetailAction, PlanDetailConfig, PlanDetailWidget};
pub use power_curve_chart::{
    PowerCurveChart, PowerCurveConfig, PowerCurveSummary, PowerImprovement,
};
pub use sensor_status::{SensorConnectionCard, SensorStatusIndicator, SensorStatusSummary};
pub use shortcut_overlay::ShortcutOverlay;
pub use smo2_display::{SmO2Display, SmO2Placeholder, SmO2WidgetSize};
pub use sync_notification::{
    SyncNotificationAction, SyncNotificationConfig, SyncNotificationItem, SyncNotificationQueue,
    SyncNotificationType, SyncNotificationWidget,
};
pub use tilt_indicator::{TiltIndicator, TiltIndicatorConfig};
pub use upcoming_workouts::{
    UpcomingWorkoutsAction, UpcomingWorkoutsCompact, UpcomingWorkoutsConfig, UpcomingWorkoutsWidget,
};
pub use voice_indicator::{
    CompactVoiceIndicator, ConfirmationCue, VoiceIndicator, VoiceIndicatorResponse,
    VoiceIndicatorState,
};
pub use weather_widget::{WeatherPlaceholder, WeatherWidget, WeatherWidgetSize};
pub use zone_indicator::{ZoneIndicator, ZoneType};
pub use hud::{
    HudConnectionQualityConfig, HudConnectionQualityIndicator, HudConnectionQualityResponse,
    HudConnectionState, InlineHudSensorStatus,
};