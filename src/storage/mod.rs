//! Storage module for database and configuration.

pub mod achievements_store;
pub mod analytics_store;
pub mod config;
pub mod database;
pub mod gradient_store;
pub mod hardware_store;
pub mod ml_store;
pub mod plan_store;
pub mod power_profile_store;
pub mod rewards_store;
pub mod schema;
pub mod social_store;
pub mod sync_store;
pub mod xp_store;

pub use analytics_store::AnalyticsStore;
pub use crate::companion::types::CompanionConfig;
pub use config::{AppConfig, DashboardLayout, MetricType, Theme, UiSettings, Units, UserProfile};
pub use database::{Database, DatabaseError};
pub use hardware_store::{
    HardwareStore, StoredAntDongle, StoredAudioSettings, StoredButtonMapping,
    StoredDualProtocolBinding, StoredFanProfile, StoredHidDevice, StoredPlatformSync,
    StoredSyncRecord as HwStoredSyncRecord, StoredVideoSync,
};
pub use sync_store::{
    delete_fit_from_queue, get_upload_queue_dir, load_fit_from_queue, save_fit_for_queue,
    StoredSyncRecord, StoredUploadQueueEntry, SyncRecordStatus, SyncStore,
};
pub use ml_store::{CachedPrediction, FatigueStateRecord, MlStore, WorkoutRecommendationRecord};
pub use social_store::{
    ActivitySummary, ChatMessageRecord, Club, ClubMembership, GroupRideParticipant,
    GroupRideRecord, Rider, SocialStore,
};

// Competitive features storage (Feature 010)
pub use achievements_store::{AchievementsStore, UserAchievementRecord};
pub use gradient_store::{GradientSettingsRecord, GradientStore};
pub use plan_store::{PlanAssignmentRecord, PlanStore};
pub use power_profile_store::{PowerProfilePointRecord, PowerProfileRecord, PowerProfileStore};
pub use rewards_store::{RewardsStore, UserRewardRecord};
pub use xp_store::{UserXpRecord, XpStore};
