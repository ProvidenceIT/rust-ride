//! Career progression unit tests.
//!
//! T082: Add unit tests for career levels.

use rustride::career::{
    all_level_rewards, all_rewards, cumulative_xp_to_level, is_milestone_level, level_from_xp,
    level_title, next_milestone, rewards_for_level, rewards_up_to_level, xp_for_level,
    CareerStatus, CosmeticInventory, CosmeticType, EquippedCosmetics, LevelUpEvent, RewardType,
    MAX_LEVEL, XP_BASE, XP_GROWTH_RATE,
};

// ============================================================================
// XP Curve Tests
// ============================================================================

#[test]
fn test_xp_curve_constants() {
    // Verify base constants are sensible
    assert!((XP_BASE - 1000.0).abs() < 0.001, "Base XP should be 1000");
    assert!(
        (XP_GROWTH_RATE - 1.15).abs() < 0.001,
        "Growth rate should be 1.15"
    );
    assert_eq!(MAX_LEVEL, 50, "Max level should be 50");
}

#[test]
fn test_xp_curve_progression() {
    // Level 1 is the starting level (0 XP to achieve)
    assert_eq!(xp_for_level(1), 0, "Level 1 should require 0 XP");

    // Level 2 should require 1000 * 1.15^1 = 1150 XP
    assert_eq!(xp_for_level(2), 1150, "Level 2 should require 1150 XP");

    // Later levels should require more
    let level_10_xp = xp_for_level(10);
    let level_50_xp = xp_for_level(50);
    assert!(
        level_10_xp > 3000,
        "Level 10 should require significant XP, got {}",
        level_10_xp
    );
    assert!(
        level_50_xp > level_10_xp,
        "Level 50 should require more than level 10"
    );
}

#[test]
fn test_level_from_xp() {
    // 0 XP = Level 1
    assert_eq!(level_from_xp(0), 1);

    // Just below level 2 threshold (1150 XP) = still level 1
    assert_eq!(level_from_xp(1149), 1);

    // At level 2 threshold = level 2
    assert_eq!(level_from_xp(1150), 2);

    // Very high XP = max level
    let max_xp = cumulative_xp_to_level(MAX_LEVEL) + xp_for_level(MAX_LEVEL) + 1000;
    assert_eq!(level_from_xp(max_xp), MAX_LEVEL);
}

#[test]
fn test_cumulative_xp_round_trip() {
    // cumulative_xp_to_level(N) = sum of xp_for_level(1..N)
    // This is the total XP earned by the time you COMPLETE level N-1
    // At that XP, level_from_xp returns N-1, not N
    //
    // For level 1: cumulative = 0, level_from_xp(0) = 1
    // For level 2: cumulative = 0 (xp_for_level(1)=0), level_from_xp(0) = 1
    // For level 3: cumulative = 1150, level_from_xp(1150) = 2

    // Test level 1
    assert_eq!(cumulative_xp_to_level(1), 0);
    assert_eq!(level_from_xp(0), 1);

    // The cumulative XP to level N gives you enough XP to be at level N-1
    // (because it's the sum of costs to GET TO level N, not to BE level N)
    for level in 3..=MAX_LEVEL {
        let cumulative = cumulative_xp_to_level(level);
        let calculated_level = level_from_xp(cumulative);
        // At cumulative XP for level N, you're at level N-1
        assert_eq!(
            calculated_level,
            level - 1,
            "Cumulative XP {} for level {} should give level {}",
            cumulative,
            level,
            level - 1
        );
    }

    // To actually BE at level N, you need cumulative_xp_to_level(N+1) - 1
    // Or more simply, add 1 XP to cumulative_xp_to_level(level+1)
    for level in 2..MAX_LEVEL {
        let xp_at_level = cumulative_xp_to_level(level + 1);
        let calculated_level = level_from_xp(xp_at_level);
        assert_eq!(
            calculated_level, level,
            "XP {} should give level {}",
            xp_at_level, level
        );
    }
}

// ============================================================================
// Level Definition Tests
// ============================================================================

#[test]
fn test_all_50_levels_defined() {
    let levels = all_level_rewards();
    assert_eq!(levels.len(), 50, "Should have exactly 50 levels defined");

    for i in 1..=50 {
        assert!(
            levels.iter().any(|l| l.level == i),
            "Level {} should be defined",
            i
        );
    }
}

#[test]
fn test_level_titles() {
    assert_eq!(level_title(1), "Beginner");
    assert_eq!(level_title(5), "Enthusiast");
    assert_eq!(level_title(10), "Regular");
    assert_eq!(level_title(20), "Committed");
    assert_eq!(level_title(30), "Expert");
    assert_eq!(level_title(40), "Champion");
    assert_eq!(level_title(50), "Legend");
}

#[test]
fn test_milestone_levels() {
    // Milestones at 10, 20, 25, 30, 40, 50
    assert!(!is_milestone_level(1));
    assert!(!is_milestone_level(5));
    assert!(is_milestone_level(10));
    assert!(is_milestone_level(20));
    assert!(is_milestone_level(25));
    assert!(is_milestone_level(30));
    assert!(is_milestone_level(40));
    assert!(is_milestone_level(50));
}

#[test]
fn test_next_milestone() {
    assert_eq!(next_milestone(1), Some(10));
    assert_eq!(next_milestone(9), Some(10));
    assert_eq!(next_milestone(10), Some(20));
    assert_eq!(next_milestone(20), Some(25));
    assert_eq!(next_milestone(25), Some(30));
    assert_eq!(next_milestone(40), Some(50));
    assert_eq!(next_milestone(50), None);
}

// ============================================================================
// Reward Tests
// ============================================================================

#[test]
fn test_all_rewards_unique_ids() {
    let rewards = all_rewards();
    let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for reward in &rewards {
        assert!(
            ids.insert(&reward.id),
            "Duplicate reward ID found: {}",
            reward.id
        );
    }
}

#[test]
fn test_reward_types_coverage() {
    let rewards = all_rewards();

    // Ensure we have at least one of each type
    let types: std::collections::HashSet<_> = rewards.iter().map(|r| r.reward_type).collect();

    assert!(
        types.contains(&RewardType::JerseyColor),
        "Should have jersey rewards"
    );
    assert!(
        types.contains(&RewardType::BikeFrame),
        "Should have bike frame rewards"
    );
    assert!(
        types.contains(&RewardType::UiTheme),
        "Should have theme rewards"
    );
    assert!(
        types.contains(&RewardType::AccentColor),
        "Should have accent color rewards"
    );
    assert!(
        types.contains(&RewardType::ProfileBadge),
        "Should have badge rewards"
    );
    assert!(
        types.contains(&RewardType::WheelStyle),
        "Should have wheel style rewards"
    );
    assert!(
        types.contains(&RewardType::HelmetStyle),
        "Should have helmet style rewards"
    );
}

#[test]
fn test_rewards_for_level() {
    // Level 1 has no rewards (starting level)
    let level_1_rewards = rewards_for_level(1);
    assert!(level_1_rewards.is_empty(), "Level 1 should have no rewards");

    // Level 10 (milestone) should have rewards
    let level_10_rewards = rewards_for_level(10);
    assert!(!level_10_rewards.is_empty(), "Level 10 should have rewards");
    assert!(
        level_10_rewards.iter().any(|r| r.is_milestone),
        "Level 10 should have milestone reward"
    );

    // Level 50 (max level) should have multiple rewards
    let level_50_rewards = rewards_for_level(50);
    assert!(
        level_50_rewards.len() >= 3,
        "Level 50 should have many rewards"
    );
}

#[test]
fn test_rewards_up_to_level() {
    let level_5_rewards = rewards_up_to_level(5);
    let level_10_rewards = rewards_up_to_level(10);
    let level_50_rewards = rewards_up_to_level(50);

    assert!(
        !level_5_rewards.is_empty(),
        "Should have rewards by level 5"
    );
    assert!(
        level_10_rewards.len() > level_5_rewards.len(),
        "Level 10 should have more rewards than level 5"
    );
    assert!(
        level_50_rewards.len() > level_10_rewards.len(),
        "Level 50 should have more rewards than level 10"
    );

    // Level 50 rewards should include all rewards
    let all = all_rewards();
    assert_eq!(
        level_50_rewards.len(),
        all.len(),
        "Level 50 should have all rewards"
    );
}

// ============================================================================
// Career Status Tests
// ============================================================================

#[test]
fn test_career_status_creation() {
    let status = CareerStatus::new(1, 0);

    assert_eq!(status.user_id, 1);
    assert_eq!(status.total_xp, 0);
    assert_eq!(status.current_level, 1);
    assert_eq!(status.level_title, "Beginner");
    assert!(!status.is_max_level());
}

#[test]
fn test_career_status_with_xp() {
    let status = CareerStatus::new(1, 5000);

    assert!(
        status.current_level > 1,
        "Should be above level 1 with 5000 XP"
    );
    // Progress can be > 1.0 in some edge cases, but should be clamped when displayed
    // Test that progress_bar() method clamps correctly
    assert!(
        status.progress_bar() >= 0.0 && status.progress_bar() <= 1.0,
        "Progress bar should be 0-1"
    );
}

#[test]
fn test_career_status_max_level() {
    // To reach max level (50), we need cumulative XP including xp_for_level(50)
    // cumulative_xp_to_level(50) gives us enough to be at level 49
    // We need to add xp_for_level(50) to reach level 50
    let xp_to_reach_50 = cumulative_xp_to_level(MAX_LEVEL) + xp_for_level(MAX_LEVEL);
    let status = CareerStatus::new(1, xp_to_reach_50 + 100_000);

    assert_eq!(status.current_level, MAX_LEVEL);
    assert!(status.is_max_level());
    assert_eq!(status.level_title, "Legend");
}

#[test]
fn test_career_status_update() {
    let mut status = CareerStatus::new(1, 0);
    assert_eq!(status.current_level, 1);

    // Update with XP that should cause level up
    status.update(2000);

    assert!(status.current_level > 1, "Should have leveled up");
    assert_eq!(status.total_xp, 2000);
}

// ============================================================================
// Cosmetic System Tests
// ============================================================================

#[test]
fn test_cosmetic_inventory_defaults() {
    let inventory = CosmeticInventory::with_defaults();

    assert!(inventory.is_unlocked("jersey_default"));
    assert!(inventory.is_unlocked("bike_default"));
    assert!(inventory.is_unlocked("theme_default"));
    assert!(!inventory.is_unlocked("premium_jersey"));
}

#[test]
fn test_cosmetic_unlock_and_equip() {
    let mut inventory = CosmeticInventory::with_defaults();

    // Can't equip locked item
    assert!(!inventory.equip(CosmeticType::Jersey, "locked_jersey"));

    // Unlock and then equip
    inventory.unlock("locked_jersey");
    assert!(inventory.is_unlocked("locked_jersey"));
    assert!(inventory.equip(CosmeticType::Jersey, "locked_jersey"));

    // Verify equipped
    assert_eq!(
        inventory.equipped().get(CosmeticType::Jersey),
        Some(&"locked_jersey".to_string())
    );
}

#[test]
fn test_equipped_cosmetics() {
    let mut equipped = EquippedCosmetics::default_equipment();

    // Should have defaults
    assert!(equipped.jersey.is_some());
    assert!(equipped.bike_frame.is_some());
    assert!(equipped.theme.is_some());

    // Badge should be None by default
    assert!(equipped.badge.is_none());

    // Equip badge
    equipped.equip(CosmeticType::Badge, "test_badge");
    assert_eq!(equipped.badge, Some("test_badge".to_string()));

    // Unequip
    equipped.unequip(CosmeticType::Badge);
    assert!(equipped.badge.is_none());
}

#[test]
fn test_cosmetic_type_conversion() {
    assert_eq!(
        CosmeticType::from_reward_type(RewardType::JerseyColor),
        CosmeticType::Jersey
    );
    assert_eq!(
        CosmeticType::from_reward_type(RewardType::BikeFrame),
        CosmeticType::BikeFrame
    );
    assert_eq!(
        CosmeticType::from_reward_type(RewardType::UiTheme),
        CosmeticType::Theme
    );
    assert_eq!(
        CosmeticType::from_reward_type(RewardType::ProfileBadge),
        CosmeticType::Badge
    );
}

// ============================================================================
// Level Up Event Tests
// ============================================================================

#[test]
fn test_level_up_event() {
    let event = LevelUpEvent::new(5, 6, 1000, 5500);

    assert_eq!(event.old_level, 5);
    assert_eq!(event.new_level, 6);
    assert_eq!(event.levels_gained, 1);
    assert_eq!(event.xp_gained, 1000);
    assert!(!event.is_milestone);
}

#[test]
fn test_multi_level_up_event() {
    let event = LevelUpEvent::new(8, 12, 5000, 15000);

    assert_eq!(event.old_level, 8);
    assert_eq!(event.new_level, 12);
    assert_eq!(event.levels_gained, 4);
    assert!(!event.is_milestone); // 12 is not a milestone
}

#[test]
fn test_milestone_level_up_event() {
    let event = LevelUpEvent::new(9, 10, 2000, 8000);

    assert!(event.is_milestone, "Level 10 should be a milestone");
    assert!(event.notification_message().contains("Milestone"));
}

#[test]
fn test_celebration_messages() {
    // Test different level ranges
    let event_5 = LevelUpEvent::new(4, 5, 1000, 4000);
    assert_eq!(event_5.celebration_message(), "Keep riding!");

    let event_10 = LevelUpEvent::new(9, 10, 1500, 8000);
    assert!(event_10.celebration_message().contains("Double digits"));

    let event_50 = LevelUpEvent::new(49, 50, 50000, 1_000_000);
    assert!(event_50.celebration_message().contains("LEGEND"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_progression_simulation() {
    let mut status = CareerStatus::new(1, 0);
    let all_reward_defs = all_rewards();

    // Simulate gaining XP
    let result = status.add_xp(1500, &all_reward_defs);

    assert!(result.is_level_up, "Should level up with 1500 XP");
    assert!(result.new_level >= 2, "Should be at least level 2");
    assert!(!result.new_rewards.is_empty(), "Should unlock some rewards");
}

#[test]
fn test_reward_count_at_max_level() {
    let all_rewards = rewards_up_to_level(MAX_LEVEL);

    // Should have a reasonable number of rewards (50+ based on level definitions)
    assert!(
        all_rewards.len() >= 50,
        "Should have at least 50 rewards unlocked by max level, got {}",
        all_rewards.len()
    );
}
